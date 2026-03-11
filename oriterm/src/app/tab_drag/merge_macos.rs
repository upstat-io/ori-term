//! macOS tab tear-off drag tracking and merge detection.
//!
//! Unlike Windows (which uses a blocking `drag_window()` modal loop), macOS
//! tracks the torn-off window manually: cursor move events on the source
//! window update the window position, and mouse-up ends the drag.
//!
//! Merge detection follows Chrome's approach: on every cursor move during
//! drag, check if the torn-off window's tab bar overlaps a target window's
//! tab bar region. If so, merge immediately (no need to wait for mouse-up).

use winit::window::WindowId;

use crate::window_manager::WindowKind;
use crate::window_manager::platform::macos;
use oriterm_ui::widgets::tab_bar::constants::{TAB_BAR_HEIGHT, TAB_LEFT_MARGIN};

use crate::app::App;

/// Minimum distance (in screen points) the cursor must travel from the
/// tear-off origin before a merge is allowed. Prevents the tab from
/// immediately snapping back into the source window.
const MIN_MERGE_DISTANCE: i32 = 50;

/// Vertical expansion of the tab strip bounds for merge detection (pixels).
/// Matches Chrome's `kVerticalDetachMagnetism`.
const MERGE_MAGNETISM: i32 = 15;

impl App {
    /// Track the torn-off window under the cursor and check for merge.
    ///
    /// Called from `CursorMoved` and `about_to_wait` in the event loop.
    /// Uses global cursor position (not the event's window-relative position)
    /// because the cursor is over the torn-off window, not the source window.
    ///
    /// Like Chrome, merge detection happens continuously during drag — when
    /// the torn-off window's tab bar overlaps a target's tab bar, the merge
    /// triggers immediately without waiting for mouse-up.
    ///
    /// Returns `true` if a torn-off drag is active (event consumed).
    pub(in crate::app) fn update_torn_off_drag(&mut self) -> bool {
        let Some(pending) = &mut self.torn_off_pending else {
            return false;
        };
        let winit_id = pending.winit_id;
        let mouse_offset = pending.mouse_offset;

        if !self.windows.contains_key(&winit_id) {
            return false;
        }

        // cursor_screen_pos() returns macOS screen points (logical).
        // Tab bar constants are also in logical points.
        let grab_x = TAB_LEFT_MARGIN + mouse_offset;
        let grab_y = TAB_BAR_HEIGHT / 2.0;

        let (cx, cy) = macos::cursor_screen_pos();
        let pos_x = cx as f64 - grab_x as f64;
        let pos_y = cy as f64 - grab_y as f64;

        if let Some(ctx) = self.windows.get(&winit_id) {
            ctx.window
                .window()
                .set_outer_position(winit::dpi::LogicalPosition::new(pos_x, pos_y));
        }

        // Enable merges once cursor has traveled far enough from the
        // tear-off point. Once enabled, stays enabled permanently —
        // the user can freely drag back to merge without the distance
        // check blocking them.
        if !pending.merge_enabled {
            let (ox, oy) = pending.tear_off_origin;
            let dx = (cx - ox).abs();
            let dy = (cy - oy).abs();
            if dx >= MIN_MERGE_DISTANCE || dy >= MIN_MERGE_DISTANCE {
                pending.merge_enabled = true;
            }
        }
        let merge_enabled = pending.merge_enabled;

        // Chrome-style continuous merge: check if the torn-off window's
        // tab bar now overlaps a target window's tab bar region.
        if merge_enabled {
            let probe_y = pos_y as i32 + (TAB_BAR_HEIGHT / 2.0) as i32;
            let target =
                self.find_merge_target_macos(winit_id, (cx, probe_y));
            if target.is_some() {
                self.execute_merge(target);
            }
        }

        true
    }

    /// Finish the torn-off drag on mouse-up.
    ///
    /// If no continuous merge happened during drag, the torn-off window
    /// stays as a separate window. Consumes `torn_off_pending`.
    pub(in crate::app) fn check_torn_off_merge(&mut self) {
        // Just consume the pending state. Merge already happened during
        // drag if the user positioned correctly, or the window stays.
        let _ = self.torn_off_pending.take();
    }

    /// Execute a merge: move the tab from the torn-off window to the target.
    fn execute_merge(&mut self, target: Option<(WindowId, f64)>) {
        let Some(pending) = self.torn_off_pending.take() else {
            return;
        };
        let winit_id = pending.winit_id;
        let tab_id = pending.tab_id;

        let Some((target_wid, screen_x)) = target else {
            return;
        };

        // Verify windows still exist.
        if !self.windows.contains_key(&winit_id)
            || !self.windows.contains_key(&target_wid)
        {
            return;
        }

        let idx = self.compute_drop_index_macos(target_wid, screen_x);
        let target_session_wid = self
            .windows
            .get(&target_wid)
            .map(|c| c.window.session_window_id());

        // Move tab from torn window to target (local session).
        if let Some(dest_wid) = target_session_wid {
            let src_wid = self.session.window_for_tab(tab_id);
            if let Some(wid) = src_wid {
                if let Some(win) = self.session.get_window_mut(wid) {
                    win.remove_tab(tab_id);
                }
            }
            if let Some(win) = self.session.get_window_mut(dest_wid) {
                win.insert_tab_at(idx, tab_id);
            }
        }

        // Drain mux notifications from the move.
        self.pump_mux_events();

        // Remove the torn-off window (now empty).
        self.remove_empty_window(winit_id);

        // Activate and focus the target window.
        if let Some(ctx) = self.windows.get(&target_wid) {
            self.active_window = Some(ctx.window.session_window_id());
            ctx.window.window().focus_window();
        }
        self.focused_window_id = Some(target_wid);

        // Sync tab bars and refresh platform hit test rects.
        self.sync_tab_bar_for_window(target_wid);
        self.refresh_platform_rects(target_wid);

        // Resize panes in the moved tab to fit the target window.
        self.resize_all_panes();

        // Mark target dirty.
        if let Some(ctx) = self.windows.get_mut(&target_wid) {
            ctx.pane_cache.invalidate_all();
            ctx.cached_dividers = None;
            ctx.dirty = true;
        }
    }

    /// Find a merge target window whose tab bar region contains the probe point.
    ///
    /// Returns `(target_winit_id, screen_x)` or `None`. The merge zone is
    /// the tab bar bounds ± `MERGE_MAGNETISM` (matching Chrome's approach).
    fn find_merge_target_macos(
        &self,
        exclude: WindowId,
        probe: (i32, i32),
    ) -> Option<(WindowId, f64)> {
        let (px, py) = probe;
        let tab_h = TAB_BAR_HEIGHT as i32;
        for managed in self.window_manager.windows_of_kind(WindowKind::is_primary) {
            let wid = managed.winit_id;
            if wid == exclude {
                continue;
            }
            let Some(ctx) = self.windows.get(&wid) else {
                continue;
            };
            if let Some((l, t, r, _b)) = macos::window_frame_bounds(ctx.window.window()) {
                let in_x = px >= l && px < r;
                // Tab bar spans [t, t + tab_h]. Expand by magnetism.
                let zone_top = t - MERGE_MAGNETISM;
                let zone_bottom = t + tab_h + MERGE_MAGNETISM;
                let in_y = py >= zone_top && py < zone_bottom;
                if in_x && in_y {
                    return Some((wid, px as f64));
                }
            }
        }
        None
    }

    /// Compute the drop index for inserting a tab at a screen X position.
    fn compute_drop_index_macos(&self, target: WindowId, screen_x: f64) -> usize {
        let Some(ctx) = self.windows.get(&target) else {
            return 0;
        };
        // All coordinates are in screen points (logical). Tab bar layout
        // values are also in logical points — no scale conversion needed.
        let target_left = macos::window_frame_bounds(ctx.window.window())
            .map_or(0.0, |(l, _, _, _)| l as f64);
        let local_x = screen_x - target_left;
        let tab_width = ctx.tab_bar.layout().base_tab_width() as f64;
        let left_margin = TAB_LEFT_MARGIN as f64;
        let tab_count = ctx.tab_bar.layout().tab_count();
        let raw = ((local_x - left_margin + tab_width / 2.0) / tab_width.max(1.0)).floor();
        raw.clamp(0.0, tab_count as f64) as usize
    }
}
