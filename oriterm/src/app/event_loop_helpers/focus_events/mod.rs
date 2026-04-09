//! Focus-event escape sequences and the `App` methods that emit
//! them in response to winit `WindowEvent::Focused`.
//!
//! Extracted from `event_loop_helpers/mod.rs` in TPR-06-004 to
//! keep the parent file under the 500-line hygiene limit AND to
//! give Section 06.5 Track B's `kxIN`/`kxOUT` cross-crate tests a
//! testable seam (see [`focus_event_seq_for_mode`] — the pure
//! decision function the `App` method delegates to).

use oriterm_core::TermMode;

use super::App;

/// Focus-in escape sequence emitted when the terminal has
/// `FOCUS_IN_OUT` (DECSET 1004) enabled and winit reports
/// `WindowEvent::Focused(true)`. Cross-references the `kxIN` cap
/// declaration in `extra/ori_term.info:214` (`kxIN=\E[I`).
pub(super) const FOCUS_IN_SEQ: &[u8] = b"\x1b[I";

/// Focus-out escape sequence emitted when winit reports
/// `WindowEvent::Focused(false)` and the terminal has
/// `FOCUS_IN_OUT` enabled. Cross-references the `kxOUT` cap
/// declaration in `extra/ori_term.info:214` (`kxOUT=\E[O`).
pub(super) const FOCUS_OUT_SEQ: &[u8] = b"\x1b[O";

/// Pure focus-event decision function: given a terminal mode
/// snapshot and the new focus state, return the bytes that
/// `send_focus_event` should write to the PTY (or `None` if the
/// terminal has not enabled `FOCUS_IN_OUT` and no event should
/// be emitted).
///
/// Section 06.5's TPR-06-002 finding asked for an actual test of
/// the emission path — not just the constants. This function is
/// the testable kernel of [`App::send_focus_event`]: tests can
/// feed synthetic `TermMode` snapshots and assert the returned
/// byte sequence matches the expected `FOCUS_IN_SEQ` /
/// `FOCUS_OUT_SEQ` constants AND covers the
/// "`FOCUS_IN_OUT` not enabled → no emission" branch. The
/// remaining App-bound logic in `send_focus_event` (active-pane
/// lookup, mode lookup, `write_pane_input` call) is the
/// integration glue that the cross-crate test seam cannot reach
/// without constructing a real `App`; the pure function carries
/// the load-bearing emission decision.
#[must_use]
pub(super) fn focus_event_seq_for_mode(mode: TermMode, focused: bool) -> Option<&'static [u8]> {
    if !mode.contains(TermMode::FOCUS_IN_OUT) {
        return None;
    }
    Some(if focused { FOCUS_IN_SEQ } else { FOCUS_OUT_SEQ })
}

impl App {
    /// Send a focus-in or focus-out escape sequence to the active pane.
    ///
    /// Only sends when the terminal has `FOCUS_IN_OUT` mode enabled
    /// (mode 1004). The decision logic — including which byte
    /// sequence to emit and whether to emit at all — lives in
    /// [`focus_event_seq_for_mode`] so the Section 06.5 Track B
    /// `kxIN`/`kxOUT` tests can pin the emission decision without
    /// constructing a real `App`.
    pub(in crate::app) fn send_focus_event(&mut self, focused: bool) {
        let Some(pane_id) = self.active_pane_id() else {
            return;
        };
        let Some(mode) = self.pane_mode(pane_id) else {
            return;
        };
        if let Some(seq) = focus_event_seq_for_mode(mode, focused) {
            self.write_pane_input(pane_id, seq);
        }
    }

    /// Flush a pending focus-out event.
    ///
    /// Called from `about_to_wait` and from `Focused(true)` handlers.
    /// Checks if focus moved to a child dialog — if so, the focus-
    /// out is suppressed because the terminal is still "active" from
    /// the user's perspective.
    pub(in crate::app) fn flush_pending_focus_out(&mut self) {
        let Some(pending) = self.pending_focus_out.take() else {
            return;
        };
        // If focus moved to a child dialog of the window that lost
        // focus, suppress the PTY focus-out escape sequence.
        if self.window_manager.focused_is_child_of(pending.window_id) {
            return;
        }
        self.send_focus_event(false);
    }
}

#[cfg(test)]
mod tests;
