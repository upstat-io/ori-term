//! Per-pane state accessors: selection, mark cursor, input, focus transfer.
//!
//! Extracted from `mod.rs` to keep the parent under the 500-line limit.

use oriterm_core::grid::StableRowIndex;
use oriterm_core::{Selection, SelectionPoint};
use oriterm_mux::{MarkCursor, PaneId};
use winit::window::WindowId;

use crate::app::App;

impl App {
    // Per-pane selection accessors

    /// The active selection for a pane, if any.
    pub(super) fn pane_selection(&self, pane_id: PaneId) -> Option<&Selection> {
        self.pane_selections.get(&pane_id)
    }

    /// Replace or create a selection for a pane.
    pub(super) fn set_pane_selection(&mut self, pane_id: PaneId, sel: Selection) {
        self.pane_selections.insert(pane_id, sel);
    }

    /// Clear the selection for a pane.
    pub(super) fn clear_pane_selection(&mut self, pane_id: PaneId) {
        self.pane_selections.remove(&pane_id);
    }

    /// Update the endpoint of an existing selection (drag).
    pub(super) fn update_pane_selection_end(&mut self, pane_id: PaneId, end: SelectionPoint) {
        if let Some(sel) = self.pane_selections.get_mut(&pane_id) {
            sel.end = end;
        }
    }

    // Per-pane mark cursor accessors

    /// Whether mark mode is active for a pane.
    pub(super) fn is_mark_mode(&self, pane_id: PaneId) -> bool {
        self.mark_cursors.contains_key(&pane_id)
    }

    /// The mark cursor for a pane, if mark mode is active.
    pub(super) fn pane_mark_cursor(&self, pane_id: PaneId) -> Option<MarkCursor> {
        self.mark_cursors.get(&pane_id).copied()
    }

    /// Enter mark mode for a pane, placing the cursor at the terminal cursor.
    ///
    /// Scrolls to bottom first, refreshes the snapshot, then reads the
    /// terminal cursor position from snapshot data.
    pub(super) fn enter_mark_mode(&mut self, pane_id: PaneId) {
        if self.mark_cursors.contains_key(&pane_id) {
            return;
        }
        let Some(mux) = self.mux.as_mut() else { return };
        mux.scroll_to_bottom(pane_id);
        if mux.is_pane_snapshot_dirty(pane_id) || mux.pane_snapshot(pane_id).is_none() {
            mux.refresh_pane_snapshot(pane_id);
        }
        if let Some(snapshot) = self.mux.as_ref().and_then(|m| m.pane_snapshot(pane_id)) {
            let mc = MarkCursor {
                row: StableRowIndex(snapshot.stable_row_base + snapshot.cursor.row as u64),
                col: snapshot.cursor.col as usize,
            };
            self.mark_cursors.insert(pane_id, mc);
        }
    }

    /// Exit mark mode for a pane.
    pub(super) fn exit_mark_mode(&mut self, pane_id: PaneId) {
        self.mark_cursors.remove(&pane_id);
    }

    /// Send input bytes to a pane.
    ///
    /// Delegates to [`MuxBackend::send_input`], which writes to the local PTY
    /// in embedded mode or sends through IPC in daemon mode.
    pub(super) fn write_pane_input(&mut self, pane_id: PaneId, data: &[u8]) {
        if let Some(mux) = self.mux.as_mut() {
            mux.send_input(pane_id, data);
        }
    }

    /// If `winit_id` was the focused window, transfer focus to the next available.
    ///
    /// Updates both `focused_window_id` (winit) and `active_window` (mux).
    pub(super) fn transfer_focus_from(&mut self, winit_id: WindowId) {
        if self.focused_window_id == Some(winit_id) {
            self.focused_window_id = self.windows.keys().next().copied();
            self.active_window = self.focused_window_id.and_then(|id| {
                self.windows
                    .get(&id)
                    .map(|ctx| ctx.window.session_window_id())
            });
        }
    }

    /// Select all content in the active pane.
    ///
    /// Tries shell input selection first (OSC 133 zones), falling back to
    /// selecting the entire buffer (scrollback + visible).
    pub(super) fn select_all_in_pane(&mut self) {
        let Some(pane_id) = self.active_pane_id() else {
            return;
        };
        // Try shell input selection first (OSC 133 zones).
        let input_sel = self
            .mux
            .as_ref()
            .and_then(|m| m.select_command_input(pane_id));
        if let Some(sel) = input_sel {
            self.set_pane_selection(pane_id, sel);
            return;
        }
        // No shell input zone — select entire buffer.
        let Some(mux) = self.mux.as_mut() else { return };
        if mux.pane_snapshot(pane_id).is_none() || mux.is_pane_snapshot_dirty(pane_id) {
            mux.refresh_pane_snapshot(pane_id);
        }
        if let Some(snap) = self.mux.as_ref().and_then(|m| m.pane_snapshot(pane_id)) {
            let grid = super::snapshot_grid::SnapshotGrid::new(snap);
            let sel = super::mark_mode::select_all(&grid);
            self.set_pane_selection(pane_id, sel);
        }
    }

    /// Tab index for a given pane within the active window's tab list.
    ///
    /// Traverses local session: pane → tab → window tab list to find the position.
    pub(super) fn tab_index_for_pane(&self, pane_id: PaneId) -> Option<usize> {
        let tab_id = self.session.tab_for_pane(pane_id)?;
        let win_id = self.active_window?;
        let win = self.session.get_window(win_id)?;
        win.tabs().iter().position(|&t| t == tab_id)
    }
}
