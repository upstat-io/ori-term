//! Platform-independent merge helpers shared by all merge modules.
//!
//! Extracted from `merge.rs`, `merge_linux.rs`, and `merge_macos.rs`
//! to avoid duplicating the tab-move logic and drop-index computation
//! across three platform files.

use winit::window::WindowId;

use oriterm_ui::widgets::tab_bar::constants::TAB_LEFT_MARGIN;

use crate::app::App;
use crate::session::TabId;

/// Compute the drop index for inserting a tab at a local X coordinate.
///
/// `local_x` is the cursor X relative to the target window's left edge
/// (in the coordinate system matching `tab_width` and `left_margin`).
/// All three values must be in the same units (physical or logical).
pub(super) fn compute_drop_index(
    local_x: f64,
    tab_width: f64,
    tab_count: usize,
    left_margin: f64,
) -> usize {
    let raw = ((local_x - left_margin + tab_width / 2.0) / tab_width.max(1.0)).floor();
    raw.clamp(0.0, tab_count as f64) as usize
}

impl App {
    /// Execute the common tab merge: move tab, clean up source, focus target.
    ///
    /// After a merge target is found and a drop index computed, all three
    /// platforms perform the same sequence: move the tab in the session,
    /// drain events, remove the empty source window, focus the target,
    /// sync tab bars, resize panes, and mark the target dirty.
    pub(super) fn execute_tab_merge(
        &mut self,
        source_winit_id: WindowId,
        tab_id: TabId,
        target_wid: WindowId,
        drop_index: usize,
    ) {
        let target_session_wid = self
            .windows
            .get(&target_wid)
            .map(|c| c.window.session_window_id());

        // Move tab from source window to target (local session).
        if let Some(dest_wid) = target_session_wid {
            let src_wid = self.session.window_for_tab(tab_id);
            if let Some(wid) = src_wid {
                if let Some(win) = self.session.get_window_mut(wid) {
                    win.remove_tab(tab_id);
                }
            }
            if let Some(win) = self.session.get_window_mut(dest_wid) {
                win.insert_tab_at(drop_index, tab_id);
                // Activate the merged tab so it becomes the focused tab.
                let idx = win
                    .tabs()
                    .iter()
                    .position(|&t| t == tab_id)
                    .unwrap_or(drop_index);
                win.set_active_tab_idx(idx);
            }
        }

        // Drain mux notifications from the move.
        self.pump_mux_events();

        // Remove the torn-off window (now empty).
        self.remove_empty_window(source_winit_id);

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

    /// Compute the drop index using the target window's tab bar layout.
    ///
    /// `screen_x` is the cursor X in screen coordinates. `target_left` is
    /// the target window's left edge in the same units. `scale` converts
    /// layout constants to screen units (pass `1.0` for logical coordinates).
    pub(super) fn compute_drop_index_for_target(
        &self,
        target: WindowId,
        screen_x: f64,
        target_left: f64,
        scale: f64,
    ) -> usize {
        let Some(ctx) = self.windows.get(&target) else {
            return 0;
        };
        let local_x = screen_x - target_left;
        let tab_width = ctx.tab_bar.layout().base_tab_width() as f64 * scale;
        let left_margin = TAB_LEFT_MARGIN as f64 * scale;
        let tab_count = ctx.tab_bar.layout().tab_count();
        compute_drop_index(local_x, tab_width, tab_count, left_margin)
    }
}
