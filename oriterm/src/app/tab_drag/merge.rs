//! Merge detection and seamless drag continuation after OS-level tab drag.
//!
//! After the OS modal drag loop completes (or detects a merge target during
//! `WM_MOVING`), this module checks the result: if the cursor was over another
//! window's tab bar, the tab is merged into that window. If the merge happened
//! during the drag (live), a new in-bar drag state is synthesized for seamless
//! continuation.

use winit::window::WindowId;

use crate::session::TabId;
use crate::window_manager::WindowKind;
use oriterm_ui::platform_windows::{self, OsDragResult};
use oriterm_ui::widgets::tab_bar::constants::{CONTROLS_ZONE_WIDTH, TAB_BAR_HEIGHT};

use super::{DragPhase, TabDragState};
use crate::app::App;

impl App {
    /// Check for a completed OS drag and handle merge or show.
    ///
    /// Called from `about_to_wait` every event loop iteration. If no pending
    /// tear-off exists, returns immediately.
    pub(in crate::app) fn check_torn_off_merge(&mut self) {
        let Some(pending) = &self.torn_off_pending else {
            return;
        };
        let winit_id = pending.winit_id;
        let tab_id = pending.tab_id;
        let mouse_offset = pending.mouse_offset;

        // Poll for the OS drag result.
        let result = {
            let Some(ctx) = self.windows.get(&winit_id) else {
                self.torn_off_pending = None;
                return;
            };
            platform_windows::take_os_drag_result(ctx.window.window())
        };
        let Some(result) = result else {
            return;
        };
        self.torn_off_pending = None;

        let (cursor, is_live) = match result {
            OsDragResult::MergeDetected { cursor } => (cursor, true),
            OsDragResult::DragEnded { cursor } => (cursor, false),
        };

        // Chrome uses ~15px magnetism for merge detection after drag end.
        let scale = self
            .windows
            .get(&winit_id)
            .map_or(1.0, |ctx| ctx.window.scale_factor().factor() as f32);
        let magnetism = if is_live {
            0
        } else {
            (15.0 * scale).round() as i32
        };

        let target = self.find_merge_target(winit_id, cursor, magnetism);

        if let Some((target_wid, screen_x)) = target {
            let (scale, target_left) = self.windows.get(&target_wid).map_or((1.0, 0.0), |ctx| {
                let s = ctx.window.scale_factor().factor();
                let l = platform_windows::visible_frame_bounds(ctx.window.window())
                    .map_or(0.0, |(l, _, _, _)| l as f64);
                (s, l)
            });
            let idx = self.compute_drop_index_for_target(target_wid, screen_x, target_left, scale);

            self.execute_tab_merge(winit_id, tab_id, target_wid, idx);

            // Seamless drag continuation if the merge was live.
            if is_live {
                self.begin_seamless_drag_after_merge(target_wid, tab_id, cursor, mouse_offset);
            }
        } else {
            // No merge — show the torn window.
            if let Some(ctx) = self.windows.get(&winit_id) {
                platform_windows::show_window(ctx.window.window());
            }
        }
    }

    /// Find a merge target window whose tab bar zone contains the cursor.
    ///
    /// Returns `(target_winit_id, screen_x)` or `None`. `magnetism` expands
    /// the tab bar zone vertically for post-drag detection. Only considers
    /// primary windows (`Main` + `TearOff`) via the `WindowManager` registry.
    fn find_merge_target(
        &self,
        exclude: WindowId,
        cursor: (i32, i32),
        magnetism: i32,
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
            let controls_w = (CONTROLS_ZONE_WIDTH * scale).round() as i32;
            if let Some((l, t, r, _)) = platform_windows::visible_frame_bounds(ctx.window.window())
            {
                let in_x = cx >= l && cx < r - controls_w;
                let in_y = cy >= t - magnetism && cy < t + tab_bar_h + magnetism;
                if in_x && in_y {
                    return Some((wid, cx as f64));
                }
            }
        }
        None
    }

    /// Start a new in-bar drag on the target window for seamless continuation.
    ///
    /// After a live merge, the user's mouse is still held down. This creates
    /// a new `TabDragState` on the target window so dragging continues without
    /// releasing the button.
    fn begin_seamless_drag_after_merge(
        &mut self,
        target_wid: WindowId,
        tab_id: TabId,
        cursor: (i32, i32),
        mouse_offset: f32,
    ) {
        let (tab_index, logical_x, logical_y) = {
            let Some(ctx) = self.windows.get(&target_wid) else {
                return;
            };
            let scale = ctx.window.scale_factor().factor() as f32;

            // Convert screen cursor to target window local coords.
            let (tgt_left, tgt_top) = platform_windows::visible_frame_bounds(ctx.window.window())
                .map_or((0, 0), |(l, t, _, _)| (l, t));
            let local_x = (cursor.0 - tgt_left) as f32;
            let local_y = (cursor.1 - tgt_top) as f32;
            let lx = local_x / scale;
            let ly = local_y / scale;

            // Resolve tab index in target window.
            let idx = {
                let session_wid = ctx.window.session_window_id();
                self.session
                    .get_window(session_wid)
                    .and_then(|win| win.tabs().iter().position(|&t| t == tab_id))
                    .unwrap_or(0)
            };

            (idx, lx, ly)
        };

        // Create drag state (suppress_next_release absorbs stale WM_LBUTTONUP).
        let state = TabDragState {
            tab_id,
            original_index: tab_index,
            current_index: tab_index,
            origin_x: logical_x,
            origin_y: logical_y,
            phase: DragPhase::DraggingInBar,
            mouse_offset_in_tab: mouse_offset,
            tab_bar_y: 0.0,
            tab_bar_bottom: TAB_BAR_HEIGHT,
            suppress_next_release: true,
        };

        // Install drag state on target window.
        if let Some(ctx) = self.windows.get_mut(&target_wid) {
            ctx.tab_drag = Some(state);
            // Set drag visual.
            let layout = ctx.tab_bar.layout();
            let max_x = (layout.tabs_end() - layout.base_tab_width()).max(0.0);
            let visual_x = (logical_x - mouse_offset).clamp(0.0, max_x);
            ctx.tab_bar.set_drag_visual(Some((tab_index, visual_x)));
            ctx.root.mark_dirty();
        }

        // Synthesize mouse-down (OS modal loop consumed the original).
        self.mouse
            .set_button_down(winit::event::MouseButton::Left, true);

        // Focus + activate the target window and acquire width lock.
        self.focused_window_id = Some(target_wid);
        if let Some(ctx) = self.windows.get(&target_wid) {
            self.active_window = Some(ctx.window.session_window_id());
            let tw = ctx.tab_bar.layout().base_tab_width();
            self.acquire_tab_width_lock(tw);
        }
    }
}
