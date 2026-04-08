//! Child-exit and quit primitives for [`PtySession`].
//!
//! Owns the post-`spawn` half of the session lifecycle: observing child
//! termination, asserting the child actually exited (not just that the
//! parent stopped reading), and the state-aware quit loops Section 04
//! of the tack-conformance plan needs for `tack`. The `wait_for_child_exit`
//! family delegates to [`super::sync::poll_until`] so the bounded-poll
//! discipline lives in exactly one place.

use portable_pty::ExitStatus;

use super::PtySession;
use super::sync::{PollStep, poll_until};

impl PtySession {
    /// Wait until the child process exits, with a hard timeout.
    ///
    /// Returns the [`ExitStatus`] on clean exit. Panics with the
    /// current grid contents on timeout — the panic message tells
    /// the test author exactly what was on screen when the child
    /// failed to exit.
    ///
    /// Used by tack/vttest tests after sending `q\n` (or whatever the
    /// tool's quit key is) to assert the child actually terminated,
    /// not just that the parent stopped reading bytes.
    ///
    /// Implementation note: this polls
    /// [`portable_pty::Child::try_wait`] (Unix: `waitpid(WNOHANG)`;
    /// Windows: `GetExitCodeProcess` — NOT `WaitForSingleObject`,
    /// see `crates/portable-pty/src/win/mod.rs` `WinChild::is_complete`).
    /// The bounded-poll skeleton lives in [`super::sync::poll_until`],
    /// so the 10 ms anti-hot-spin discipline is shared with every
    /// other waiter.
    ///
    /// Delegates to [`Self::wait_for_child_exit_inner`] so the test
    /// suite can pin the bounded-poll invariant via a deterministic
    /// iteration counter (see
    /// `pty_session_wait_for_child_exit_bounded_poll_invariant`).
    pub fn wait_for_child_exit(&mut self, timeout_ms: u64) -> ExitStatus {
        self.wait_for_child_exit_inner(timeout_ms, || {})
    }

    /// Inner body of [`Self::wait_for_child_exit`] — see that method
    /// for the bounded-poll contract. The only reason this exists as
    /// a separate function is the `on_iter` callback: the public
    /// wrapper passes a no-op (zero cost), and the test suite passes
    /// an iteration counter to pin the invariant via
    /// `pty_session_wait_for_child_exit_bounded_poll_invariant` in
    /// the sibling tests file.
    pub(crate) fn wait_for_child_exit_inner<F: FnMut()>(
        &mut self,
        timeout_ms: u64,
        mut on_iter: F,
    ) -> ExitStatus {
        let result = poll_until::<ExitStatus, _>(self, timeout_ms, |session| {
            on_iter();
            match session.child.try_wait() {
                Ok(Some(status)) => PollStep::Done(status),
                Ok(None) => PollStep::NotYet,
                Err(e) => panic!("PtySession::wait_for_child_exit: try_wait error: {e}"),
            }
        });
        match result {
            Some(status) => status,
            None => panic!(
                "child did not exit within {timeout_ms}ms.\nGrid:\n{}",
                self.grid_text()
            ),
        }
    }

    /// State-aware quit loop for `tack`-style submenu nesting.
    ///
    /// **Two-phase exit observation.**
    ///
    /// Phase 1 (in-loop, `max_iterations` send budget):
    ///   1. Calls [`Self::send_raw`] with `b"q"` — bare `q`, NO
    ///      newline. Tack reads input character-by-character in raw
    ///      mode, and a trailing `\n` gets queued as a second
    ///      keystroke that confuses the next menu's input state
    ///      (verified empirically — `q\n` from a nested sub-menu
    ///      causes the second `q` to be ignored). Errors from the
    ///      writer are swallowed because the child may have already
    ///      exited on the previous `q` and the PTY writer will
    ///      return EPIPE / `ERROR_BROKEN_PIPE`.
    ///   2. `drain_blocking(150)` — bounded drain of any PTY
    ///      output produced by tack acknowledging the `q`. Bounded
    ///      so we don't block indefinitely if tack produces no
    ///      repaint between iterations.
    ///   3. `try_wait()` — if the child has exited, return the
    ///      [`ExitStatus`] immediately, before the next `q`.
    ///
    /// Phase 2 (after the iteration loop):
    ///   - Falls through to [`Self::wait_for_child_exit`] with a
    ///     2 s timeout for canonical bounded-poll exit observation.
    ///     Even if Phase 1 sent every `q` it was supposed to and
    ///     observed `try_wait() == None` after the last one, the
    ///     child may still be milliseconds away from exiting (tack
    ///     does final cleanup before its main returns); Phase 2 is
    ///     the canonical place to wait for that exit. If the child
    ///     truly never exits, [`Self::wait_for_child_exit`] panics
    ///     with the current grid as its diagnostic message.
    ///
    /// Why a loop instead of a fixed `q × N`: `tack` accepts a
    /// variable number of `q`s depending on which sub-menu the test
    /// left it in. A fixed count either over-sends (writing to a
    /// closed PTY after the child has exited) or under-sends
    /// (leaves tack alive). The loop with early-exit on `try_wait`
    /// terminates the moment the child is observed to have exited.
    ///
    /// Why [`Self::send_raw`] and not [`Self::send`]: the canonical
    /// `send()` runs `wait(300)` internally, which defeats the point
    /// of a state-aware loop — we don't want to burn 300 ms per
    /// iteration waiting for a repaint when we could be polling
    /// `try_wait()`.
    ///
    /// The default `max_iterations` for tack is 5 — enough for
    /// main-menu → submenu → sub-submenu nesting plus one extra
    /// for safety.
    pub fn quit_tack(&mut self, max_iterations: u32) -> ExitStatus {
        // Phase 1: send q's to traverse the menu hierarchy. We send
        // a bare `q` (no newline) per iteration: tack reads input
        // character-by-character in raw mode, and a trailing `\n`
        // gets queued as a second keystroke that confuses the next
        // menu's input state (verified empirically — `q\n` from a
        // nested sub-menu causes the second q to be ignored).
        for _ in 0..max_iterations {
            self.send_raw(b"q");
            // Bounded drain so we don't block indefinitely if tack
            // produces no repaint between q's. We don't need a
            // post-drain sleep here because Phase 2 below will
            // bound-poll for the actual exit with the canonical
            // 10 ms anti-hot-spin discipline.
            let _ = self.drain_blocking(150);
            if let Ok(Some(status)) = self.child.try_wait() {
                return status;
            }
        }
        // Phase 2: all q's are buffered into the PTY. Tack may
        // still be processing the last few menu transitions or
        // running its exit cleanup. Hand off to
        // [`Self::wait_for_child_exit`] for the canonical
        // bounded-poll observation of the actual exit — if tack is
        // truly stuck (no more menu states to quit) the 2 s
        // deadline fires and the panic message includes the grid
        // showing where tack stopped.
        self.wait_for_child_exit(2_000)
    }
}

#[cfg(test)]
mod tests;
