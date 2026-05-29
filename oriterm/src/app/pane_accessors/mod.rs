//! Per-pane state accessors: selection, mark cursor, input, focus transfer.
//!
//! Extracted from `mod.rs` to keep the parent under the 500-line limit.

use oriterm_core::{Selection, SelectionPoint, TermMode};
use oriterm_mux::{MarkCursor, PaneId};
use winit::window::WindowId;

use crate::app::App;
use crate::app::window_context::WindowContext;
use crate::session::{SessionRegistry, WindowId as SessionWindowId};

/// Pure helper underlying [`App::is_pane_in_focused_tab`].
///
/// Decides whether `pane_id` lives in the currently focused window's
/// active tab. Returns `false` when no window is focused, when the
/// pane has no owning window, when the owning window is not the
/// focused one, when the focused window has no active tab, or when
/// the pane's owning tab differs from the focused tab. The function
/// reads only the session registry — no mutation, no IO — so it can
/// be exercised against synthetic registries in unit tests.
pub(super) fn is_pane_in_focused_tab_impl(
    active_window: Option<SessionWindowId>,
    session: &SessionRegistry,
    pane_id: PaneId,
) -> bool {
    let Some(active_session_wid) = active_window else {
        return false;
    };
    let Some(owning_session_wid) = session.window_for_pane(pane_id) else {
        return false;
    };
    if owning_session_wid != active_session_wid {
        return false;
    }
    let Some(window) = session.get_window(owning_session_wid) else {
        return false;
    };
    let Some(active_tab_id) = window.active_tab() else {
        return false;
    };
    let Some(pane_tab_id) = session.tab_for_pane(pane_id) else {
        return false;
    };
    active_tab_id == pane_tab_id
}

impl App {
    // Active-pane resolution

    /// The active pane's ID for a specific winit window.
    ///
    /// Resolves the session window from the winit window context, then walks
    /// the local session model (window → active tab → active pane) to find
    /// the `PaneId`. Used by window-specific operations (resize, DPI change).
    pub(super) fn active_pane_id_for_window(&self, winit_id: WindowId) -> Option<PaneId> {
        let ctx = self.windows.get(&winit_id)?;
        let session_wid = ctx.window.session_window_id();
        let win = self.session.get_window(session_wid)?;
        let tab_id = win.active_tab()?;
        let tab = self.session.get_tab(tab_id)?;
        Some(tab.active_pane())
    }

    /// The active pane's ID, derived from the local session model.
    pub(super) fn active_pane_id(&self) -> Option<PaneId> {
        let win_id = self.active_window?;
        let win = self.session.get_window(win_id)?;
        let tab_id = win.active_tab()?;
        let tab = self.session.get_tab(tab_id)?;
        Some(tab.active_pane())
    }

    /// Whether `pane_id` lives in the currently focused window's active tab.
    ///
    /// Broader than `active_pane_id() == Some(pane_id)` — passes for ANY
    /// pane in the visually-focused tab, including non-active splits.
    /// Bell/notification gates compare against the tab the user is
    /// "looking at," not the single keyboard-focused pane within it.
    /// Delegates to [`is_pane_in_focused_tab_impl`] (the free function
    /// is the testable form; the App method is the production caller).
    pub(super) fn is_pane_in_focused_tab(&self, pane_id: PaneId) -> bool {
        is_pane_in_focused_tab_impl(self.active_window, &self.session, pane_id)
    }

    /// Terminal mode flags for a pane.
    ///
    /// Delegates to [`MuxBackend::pane_mode`] — embedded mode reads the
    /// lock-free atomic cache, daemon mode reads the cached snapshot.
    pub(super) fn pane_mode(&self, pane_id: PaneId) -> Option<TermMode> {
        self.mux
            .as_ref()?
            .pane_mode(pane_id)
            .map(TermMode::from_bits_truncate)
    }

    /// Read the terminal mode, locking briefly.
    ///
    /// Returns `None` if no active pane is present.
    pub(super) fn terminal_mode(&self) -> Option<TermMode> {
        let id = self.active_pane_id()?;
        self.pane_mode(id)
    }

    /// Whether DEC Locator reporting (DECELR Ps=1/Ps=2) is active on the
    /// active pane. Drives the additive locator-observation dispatch in
    /// `handle_mouse_input` — independent of `terminal_mode` / mouse
    /// tracking. Returns `false` with no active pane.
    pub(super) fn dec_locator_active(&self) -> bool {
        let Some(id) = self.active_pane_id() else {
            return false;
        };
        self.mux
            .as_ref()
            .is_some_and(|mux| mux.pane_dec_locator_active(id))
    }

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
    /// Uses the IO thread's `EnterMarkMode` reply command which atomically
    /// scrolls to bottom and reads the cursor position from authoritative
    /// terminal state — avoiding stale snapshot data.
    pub(super) fn enter_mark_mode(&mut self, pane_id: PaneId) {
        if self.mark_cursors.contains_key(&pane_id) {
            return;
        }
        let Some(mux) = self.mux.as_mut() else { return };
        if let Some(mc) = mux.enter_mark_mode(pane_id) {
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

    /// Dispatch a semantic mouse input to a pane.
    ///
    /// Routes through [`MuxBackend::send_mouse_input`] per Decision 10
    /// Option A apex. The embedded backend pipes the event into the
    /// pane's IO thread so `Term::handle_mouse_input` encodes + emits
    /// via `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::MouseEvent,
    /// .. })`; the daemon backend's default impl currently encodes
    /// locally + falls back to `send_input` (kindless on the IPC wire).
    pub(super) fn write_pane_mouse_input(
        &mut self,
        pane_id: PaneId,
        event: &oriterm_core::encode::mouse::MouseEvent,
    ) {
        if let Some(mux) = self.mux.as_mut() {
            mux.send_mouse_input(pane_id, event);
        }
    }

    /// Dispatch a semantic mouse input for DEC Locator observation ONLY.
    ///
    /// Routes through [`MuxBackend::observe_pane_locator_input`], which
    /// records the pane's locator position (`Term::observe_locator_input`,
    /// Step A) WITHOUT encoding a mouse report. Used by the locator-only
    /// dispatch path so the encoder cannot fire when mouse-tracking is off
    /// OR suppressed by the shift-to-select bypass.
    pub(super) fn observe_pane_locator_input(
        &mut self,
        pane_id: PaneId,
        event: &oriterm_core::encode::mouse::MouseEvent,
    ) {
        if let Some(mux) = self.mux.as_mut() {
            mux.observe_pane_locator_input(pane_id, event);
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

    /// Mutable [`WindowContext`] for the session window owning `session_wid`.
    ///
    /// Returns `None` if no winit window is currently mapped to that session
    /// window (e.g., window closed mid-tear-off). Used by mux notification
    /// handlers (`PaneBell`, `DesktopNotification`, `CommandComplete`) to
    /// route per-pane UI updates to the correct OWNING window regardless of
    /// which window is focused. Single canonical home for the
    /// `windows.values_mut() + session_window_id() == session_wid` walk
    /// pattern.
    pub(super) fn owning_window_ctx_mut(
        &mut self,
        session_wid: SessionWindowId,
    ) -> Option<&mut WindowContext> {
        self.windows
            .values_mut()
            .find(|ctx| ctx.window.session_window_id() == session_wid)
    }

    /// Ring the tab-bar bell pulse on the OWNING window of `pane_id`.
    ///
    /// Resolves the pane's owning window via `pane_position`, walks to the
    /// owning [`WindowContext`] via `owning_window_ctx_mut`, and rings the
    /// bell on the correct tab. No-op if the pane is not in any tab or no
    /// winit window is mapped to the owning session window.
    ///
    /// Single canonical home for the bell-pulse pattern shared by the
    /// `PaneBell`, `DesktopNotification`, and `CommandComplete` arms in
    /// `mux_pump/mod.rs`.
    pub(super) fn ring_owning_window_tab_bell(&mut self, pane_id: PaneId, now: std::time::Instant) {
        let Some(pos) = self.session.pane_position(pane_id) else {
            return;
        };
        if let Some(ctx) = self.owning_window_ctx_mut(pos.window_id) {
            ctx.tab_bar.ring_bell(pos.tab_index, now);
        }
    }
}

#[cfg(test)]
mod tests;
