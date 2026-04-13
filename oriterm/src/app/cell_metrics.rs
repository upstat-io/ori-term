//! Cell-metric propagation from the app layer to mux panes.
//!
//! Cell pixel dimensions (`cell_w`, `cell_h`) are derived from font
//! rasterization and change at runtime when the user switches font
//! size, the window crosses a DPI boundary, or a config reload swaps
//! the font family. `Term::set_cell_dimensions` on the IO-thread side
//! recomputes cell coverage for `FixedPixels` image placements so the
//! renderer continues to crop/position them correctly.
//!
//! Pane-creation call sites inline `mux.set_cell_dimensions(pid, w, h)`
//! directly (the mux reference is already in scope next to the
//! existing `set_pane_theme`/`set_image_config` calls, and inlining
//! avoids a second `self.mux` borrow). This module exposes a
//! broadcast helper for the two all-window paths — `sync_grid_layout`
//! and `handle_dpi_change` — that need to fan out to every pane
//! across every tab.

use oriterm_mux::PaneId;
use winit::window::WindowId;

use crate::session::TabId;

use super::App;

impl App {
    /// Send the given cell metrics to every pane across every tab in
    /// the given winit window.
    ///
    /// Cell metrics are per-window (DPI + font size are both per-window
    /// state), so broadcasting to the whole window keeps inactive-tab
    /// panes in sync. Images placed in a background tab still need
    /// correct coverage when the user switches back.
    /// Return the current focused window's cell dimensions in pixels
    /// (rounded, clamped to `>= 1`). Fallback `(8, 16)` matches
    /// `Term::set_cell_dimensions`'s default assumption when no
    /// renderer is attached (e.g. during early init).
    pub(in crate::app) fn current_cell_dims(&self) -> (u16, u16) {
        let Some(ctx) = self.focused_ctx() else {
            return (8, 16);
        };
        let Some(renderer) = ctx.renderer.as_ref() else {
            return (8, 16);
        };
        let cell = renderer.cell_metrics();
        (
            cell.width.round().max(1.0) as u16,
            cell.height.round().max(1.0) as u16,
        )
    }

    pub(in crate::app) fn broadcast_cell_metrics_to_window(
        &mut self,
        winit_id: WindowId,
        cell_w: u16,
        cell_h: u16,
    ) {
        // Short-circuit when nothing changed. Per TPR-07-002-gemini
        // (2026-04-13): `sync_grid_layout` fires on every layout pass
        // including every tick of an interactive drag-resize. If we
        // broadcast unconditionally, every pane gets inserted into
        // `snapshot_dirty` and (in daemon mode) has its pushed
        // snapshot invalidated via IPC on every tick — O(N panes × M
        // resize events) of wasted work when font/DPI haven't changed.
        let Some(ctx) = self.windows.get(&winit_id) else {
            return;
        };
        if ctx.last_broadcast_cell_dims == Some((cell_w, cell_h)) {
            return;
        }

        // Collect all pane IDs up front to avoid holding both a session
        // reference and a mux reference at the same time.
        let session_wid = ctx.window.session_window_id();
        let Some(session_window) = self.session.get_window(session_wid) else {
            return;
        };
        let tab_ids: Vec<TabId> = session_window.tabs().to_vec();

        let mut pane_ids: Vec<PaneId> = Vec::new();
        for tab_id in tab_ids {
            if let Some(tab) = self.session.get_tab(tab_id) {
                pane_ids.extend(tab.all_panes());
            }
        }

        let Some(mux) = self.mux.as_mut() else {
            return;
        };
        for pane_id in pane_ids {
            mux.set_cell_dimensions(pane_id, cell_w, cell_h);
        }

        // Record the broadcast so subsequent identical calls short-circuit.
        if let Some(ctx) = self.windows.get_mut(&winit_id) {
            ctx.last_broadcast_cell_dims = Some((cell_w, cell_h));
        }
    }
}
