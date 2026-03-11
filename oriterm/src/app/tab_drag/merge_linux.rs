//! Linux tab tear-off drag tracking and merge detection.
//!
//! Like macOS, Linux uses manual tracking instead of compositor-managed
//! `drag_window()` to enable merge detection. Cursor move events on the
//! source window (implicit grab on X11, pointer grab on Wayland) update
//! the torn-off window's position, and mouse-up ends the drag.
//!
//! Merge detection follows Chrome's approach: on every cursor move during
//! drag, check if the torn-off window's tab bar overlaps a target window's
//! tab bar region. If so, merge immediately.

use winit::window::WindowId;

use crate::window_manager::WindowKind;
use oriterm_ui::widgets::tab_bar::constants::{TAB_BAR_HEIGHT, TAB_LEFT_MARGIN};

use crate::app::App;

/// Minimum distance (in physical pixels) the cursor must travel from the
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
    /// Computes screen cursor position from the source window's outer
    /// position plus the winit-tracked cursor position (implicit grab
    /// delivers events to the button-down window on both X11 and Wayland).
    ///
    /// Returns `true` if a torn-off drag is active (event consumed).
    pub(in crate::app) fn update_torn_off_drag(&mut self) -> bool {
        // Extract Copy fields to break the borrow chain.
        let Some(pending) = &self.torn_off_pending else {
            return false;
        };
        let winit_id = pending.winit_id;
        let mouse_offset = pending.mouse_offset;
        let merge_enabled = pending.merge_enabled;
        let tear_off_origin = pending.tear_off_origin;

        if !self.windows.contains_key(&winit_id) {
            return false;
        }

        // Compute screen cursor from source window outer position + local cursor.
        let Some((cx, cy)) = self.screen_cursor_pos() else {
            return true;
        };

        // Compute grab offset in physical pixels.
        let scale = self
            .windows
            .get(&winit_id)
            .map_or(1.0, |ctx| ctx.window.scale_factor().factor() as f32);
        let grab_x = ((TAB_LEFT_MARGIN + mouse_offset) * scale).round() as i32;
        let grab_y = ((TAB_BAR_HEIGHT / 2.0) * scale).round() as i32;

        let pos_x = cx - grab_x;
        let pos_y = cy - grab_y;

        if let Some(ctx) = self.windows.get(&winit_id) {
            ctx.window
                .window()
                .set_outer_position(winit::dpi::PhysicalPosition::new(pos_x, pos_y));
        }

        // Enable merges once cursor has traveled far enough from the
        // tear-off point. Once enabled, stays enabled permanently.
        if !merge_enabled {
            let (ox, oy) = tear_off_origin;
            let dx = (cx - ox).abs();
            let dy = (cy - oy).abs();
            if dx >= MIN_MERGE_DISTANCE || dy >= MIN_MERGE_DISTANCE {
                if let Some(p) = &mut self.torn_off_pending {
                    p.merge_enabled = true;
                }
            }
        }

        let should_merge = merge_enabled
            || self
                .torn_off_pending
                .as_ref()
                .is_some_and(|p| p.merge_enabled);

        // Chrome-style continuous merge: check if the torn-off window's
        // tab bar now overlaps a target window's tab bar region.
        if should_merge {
            let target = self.find_merge_target_linux(winit_id, (cx, cy));
            if target.is_some() {
                self.execute_merge_linux(target);
            }
        }

        true
    }

    /// Finish the torn-off drag on mouse-up.
    ///
    /// If no continuous merge happened during drag, the torn-off window
    /// stays as a separate window. Consumes `torn_off_pending`.
    pub(in crate::app) fn check_torn_off_merge(&mut self) {
        let _ = self.torn_off_pending.take();
    }

    /// Execute a merge: move the tab from the torn-off window to the target.
    fn execute_merge_linux(&mut self, target: Option<(WindowId, f64)>) {
        let Some(pending) = self.torn_off_pending.take() else {
            return;
        };
        let winit_id = pending.winit_id;
        let tab_id = pending.tab_id;

        let Some((target_wid, screen_x)) = target else {
            return;
        };

        if !self.windows.contains_key(&winit_id) || !self.windows.contains_key(&target_wid) {
            return;
        }

        let idx = self.compute_drop_index_linux(target_wid, screen_x);
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

        self.pump_mux_events();
        self.remove_empty_window(winit_id);

        // Activate and focus the target window.
        if let Some(ctx) = self.windows.get(&target_wid) {
            self.active_window = Some(ctx.window.session_window_id());
            ctx.window.window().focus_window();
        }
        self.focused_window_id = Some(target_wid);

        self.sync_tab_bar_for_window(target_wid);
        self.refresh_platform_rects(target_wid);
        self.resize_all_panes();

        if let Some(ctx) = self.windows.get_mut(&target_wid) {
            ctx.pane_cache.invalidate_all();
            ctx.cached_dividers = None;
            ctx.dirty = true;
        }
    }

    /// Find a merge target window whose tab bar region contains the cursor.
    ///
    /// Uses winit's `outer_position()` + `outer_size()` to compute window
    /// bounds (frameless windows have no server-side decoration gap).
    /// Returns `(target_winit_id, screen_x)` or `None`.
    fn find_merge_target_linux(
        &self,
        exclude: WindowId,
        cursor: (i32, i32),
    ) -> Option<(WindowId, f64)> {
        let (cx, cy) = cursor;
        for managed in self.window_manager.windows_of_kind(WindowKind::is_primary) {
            let wid = managed.winit_id;
            if wid == exclude {
                continue;
            }
            let Some(ctx) = self.windows.get(&wid) else {
                continue;
            };
            let scale = ctx.window.scale_factor().factor() as f32;
            let tab_bar_h = (TAB_BAR_HEIGHT * scale).round() as i32;
            let Ok(pos) = ctx.window.window().outer_position() else {
                continue;
            };
            let size = ctx.window.window().outer_size();
            let l = pos.x;
            let t = pos.y;
            let r = pos.x + size.width as i32;

            let in_x = cx >= l && cx < r;
            let zone_top = t - MERGE_MAGNETISM;
            let zone_bottom = t + tab_bar_h + MERGE_MAGNETISM;
            let in_y = cy >= zone_top && cy < zone_bottom;
            if in_x && in_y {
                return Some((wid, cx as f64));
            }
        }
        None
    }

    /// Compute the drop index for inserting a tab at a screen X position.
    fn compute_drop_index_linux(&self, target: WindowId, screen_x: f64) -> usize {
        let Some(ctx) = self.windows.get(&target) else {
            return 0;
        };
        let scale = ctx.window.scale_factor().factor();
        let target_left = ctx
            .window
            .window()
            .outer_position()
            .map_or(0.0, |p| p.x as f64);
        let local_x = screen_x - target_left;
        let tab_width = ctx.tab_bar.layout().base_tab_width() as f64 * scale;
        let left_margin = TAB_LEFT_MARGIN as f64 * scale;
        let tab_count = ctx.tab_bar.layout().tab_count();
        let raw = ((local_x - left_margin + tab_width / 2.0) / tab_width.max(1.0)).floor();
        raw.clamp(0.0, tab_count as f64) as usize
    }
}
