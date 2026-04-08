//! Bounded-poll synchronization primitives for [`PtySession`].
//!
//! All polling consumers (`wait_for`, `wait_for_child_exit_inner`, and the
//! variants added by Section 04 of the tack-conformance plan:
//! `wait_for_with_context`, `wait_for_any`) delegate to the private
//! [`poll_until`] helper that captures the deadline + drain + idle-sleep
//! skeleton in exactly one place. Two earlier sites duplicated the loop
//! body — see the `LEAK:algorithmic-duplication` finding documented at
//! `plans/tack-conformance/section-04-scenario-framework.md` for the
//! architectural rationale.
//!
//! The 10 ms idle sleep on empty drain is the bounded-poll invariant
//! that prevents hot-spinning when the reader thread has closed its
//! channel but the predicate is still false (e.g., the race window
//! between PTY reader EOF and child `try_wait` observing exit).

use std::io::Write;
use std::time::{Duration, Instant};

use super::PtySession;

/// Outcome of a single [`poll_until`] check pass.
///
/// `pub(crate)` so consumers in sibling modules
/// (`tack_framework::runner::phase`) can build their own predicates
/// against the canonical bounded-poll skeleton instead of inlining
/// a parallel deadline loop. Visibility was widened from
/// `pub(super)` in 05.0.b's feat commit so `runner/phase.rs` can
/// implement the phase-capture loop without duplicating the
/// drain + idle-sleep + deadline machinery (which would be
/// `LEAK:algorithmic-duplication` per impl-hygiene.md).
pub(crate) enum PollStep<T> {
    /// Predicate not yet satisfied — keep polling.
    NotYet,
    /// Predicate satisfied — return this payload.
    Done(T),
}

/// Bounded-poll skeleton: calls `check`, drains PTY output, sleeps
/// briefly when nothing was drained, and honors a hard deadline.
///
/// Returns `Some(payload)` when `check` emits [`PollStep::Done`] before
/// the deadline. Returns `None` when the deadline passes without a
/// successful check.
///
/// This is the SINGLE canonical home for the bounded-poll pattern shared
/// by every `PtySession` waiter. The 10 ms idle sleep on empty drain is
/// what prevents hot-spinning when the reader thread has closed its
/// channel but the predicate is still false. Every consumer (`wait_for`,
/// `wait_for_child_exit_inner`, and the Section 04
/// `wait_for_with_context` / `wait_for_any` primitives) delegates here.
///
/// **`pub(crate)` since 05.0.b.** Widened from `pub(super)` so the
/// new `tack_framework::runner::phase::phase_capture_loop` (added in
/// 05.0.b's feat commit) can call this directly. Inlining a parallel
/// deadline loop in phase.rs would push the bounded-poll pattern past
/// the impl-hygiene.md "3+ instances = always extract" threshold (the
/// existing consumers are `wait_for_with_context`, `wait_for_any`, and
/// `wait_for_child_exit_inner`; phase capture would be the fourth).
/// Crucially, `poll_until` does NOT call any post-match quiesce —
/// the `self.wait(200)` quiet period that
/// [`PtySession::wait_for_with_context`] applies lives at the call
/// site AFTER `poll_until` returns. Phase capture skips that call
/// site entirely, so the same loop body powers both the
/// stable-screen 200 ms quiesce and the phase-capture zero-quiesce
/// paths without any branching here.
pub(crate) fn poll_until<T, P>(session: &mut PtySession, timeout_ms: u64, mut check: P) -> Option<T>
where
    P: FnMut(&mut PtySession) -> PollStep<T>,
{
    const IDLE_SLEEP: Duration = Duration::from_millis(10);
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);
    loop {
        if let PollStep::Done(payload) = check(session) {
            return Some(payload);
        }
        if Instant::now() >= deadline {
            return None;
        }
        if session.drain_blocking(50) == 0 {
            std::thread::sleep(IDLE_SLEEP);
        }
    }
}

impl PtySession {
    /// Drain all currently-buffered PTY output into Term, writing
    /// captured `PtyWrite` responses back to the PTY.
    pub fn drain(&mut self) -> usize {
        let mut total = 0;
        while let Ok(data) = self.rx.try_recv() {
            total += self.feed_and_flush(&data);
        }
        total
    }

    /// Block until data arrives or `timeout_ms` expires, then drain
    /// everything else still in the channel.
    pub fn drain_blocking(&mut self, timeout_ms: u64) -> usize {
        let mut total = 0;
        if let Ok(data) = self.rx.recv_timeout(Duration::from_millis(timeout_ms)) {
            total += self.feed_and_flush(&data);
        }
        total + self.drain()
    }

    /// Feed `data` through the VTE processor, then write any captured
    /// `PtyWrite` responses back to the PTY. Shared core of [`Self::drain`]
    /// and [`Self::drain_blocking`].
    fn feed_and_flush(&mut self, data: &[u8]) -> usize {
        self.proc.advance(&mut self.term, data);
        for resp in self.term.event_listener().take_responses() {
            // Best-effort: writer errors close the test session naturally
            // via Drop, so swallowing here is correct for test setup.
            let _ = self.writer.write_all(resp.as_bytes());
        }
        let _ = self.writer.flush();
        data.len()
    }

    /// Wait until no new PTY output arrives for `quiet_ms`.
    ///
    /// Uses blocking recv to avoid missing data that arrives between
    /// drain and sleep — important for multi-step DA/DSR handshakes
    /// where the queryer sends a follow-up after receiving a response.
    pub fn wait(&mut self, quiet_ms: u64) {
        loop {
            if self.drain_blocking(quiet_ms) == 0 {
                break;
            }
        }
    }

    /// Wait until `needle` appears anywhere in `grid_text()`, with a
    /// hard timeout. On timeout, panics with the message returned by
    /// `ctx(grid)` — the closure receives the captured grid so callers
    /// can build messages that mention navigation step index, sub-menu
    /// state, or any other context the bare [`Self::wait_for`] doesn't
    /// know about.
    ///
    /// [`Self::wait_for`] (the existing default-message public method)
    /// delegates to this helper. All other consumers (`TackNavigator`,
    /// `ScenarioRunner`) call `wait_for_with_context` directly.
    ///
    /// Internally builds a [`poll_until`] predicate that emits
    /// [`PollStep::Done`] when `grid_text().contains(needle)`; all
    /// deadline/sleep/drain bookkeeping lives in [`poll_until`] so
    /// this method cannot drift from `wait_for_any` or
    /// `wait_for_child_exit_inner`.
    pub fn wait_for_with_context<F>(&mut self, needle: &str, timeout_ms: u64, ctx: F)
    where
        F: Fn(&str) -> String,
    {
        let found = poll_until::<(), _>(self, timeout_ms, |session| {
            if session.grid_text().contains(needle) {
                PollStep::Done(())
            } else {
                PollStep::NotYet
            }
        });
        if found.is_some() {
            self.wait(200);
            return;
        }
        panic!("{}", ctx(&self.grid_text()));
    }

    /// Wait until `needle` appears anywhere in `grid_text()`, with a
    /// hard timeout. Panics with the current grid on timeout — kept as
    /// the default ergonomic for code that doesn't need a richer
    /// message. Delegates to [`Self::wait_for_with_context`].
    pub fn wait_for(&mut self, needle: &str, timeout_ms: u64) {
        self.wait_for_with_context(needle, timeout_ms, |grid| {
            format!("timed out waiting for {needle:?} after {timeout_ms}ms.\nGrid:\n{grid}")
        });
    }

    /// Wait until ANY anchor in `anchors` appears in `grid_text()`,
    /// with a hard timeout.
    ///
    /// Returns `Some(idx)` — the index into `anchors` of the first
    /// anchor that matched — on success. Returns `None` on timeout.
    /// Does NOT panic on timeout: the caller decides how to surface
    /// the failure (the navigator builds a panic message that lists
    /// every anchor it tried; lower-level consumers can log and
    /// continue).
    ///
    /// Semantic contract: anchor-to-index ordering is preserved, so
    /// `MenuStep::or_wait_for`'s slice index is meaningful to the
    /// navigator. If two anchors match simultaneously on the same
    /// poll iteration, the LOWER index wins (primary anchor preferred
    /// over alternates). Empty `anchors` slice is treated as a
    /// malformed call and returns `None` immediately.
    ///
    /// Internally consumes the same [`poll_until`] helper as
    /// [`Self::wait_for_with_context`] — no parallel deadline loop, no
    /// `catch_unwind`, no unwind-safety gymnastics.
    pub fn wait_for_any(&mut self, anchors: &[&str], timeout_ms: u64) -> Option<usize> {
        if anchors.is_empty() {
            return None;
        }
        let matched = poll_until::<usize, _>(self, timeout_ms, |session| {
            let text = session.grid_text();
            for (idx, anchor) in anchors.iter().enumerate() {
                if text.contains(anchor) {
                    return PollStep::Done(idx);
                }
            }
            PollStep::NotYet
        });
        if matched.is_some() {
            self.wait(200);
        }
        matched
    }
}

#[cfg(test)]
mod tests;
