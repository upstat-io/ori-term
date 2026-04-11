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
pub(super) enum PollStep<T> {
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
/// This is the SINGLE canonical home for the chunk-at-a-time
/// bounded-poll pattern shared by every `PtySession` ANCHOR-based
/// waiter (`wait_for_with_context`, `wait_for_any`,
/// `wait_for_child_exit_inner`). The 10 ms idle sleep on empty
/// drain is what prevents hot-spinning when the reader thread has
/// closed its channel but the predicate is still false.
///
/// **NOT used by phase capture.** 05.1's empirical investigation
/// found that chunk-at-a-time drain processes tack's modes-test
/// output atomically (the OS PTY buffer batches the entire ~500-byte
/// sweep into one chunk on a fast Linux host), so by the time the
/// next `poll_until` iteration runs the grid has already jumped
/// past every transient cap label. Phase capture lives in
/// [`super::PtySession::drain_until`] which feeds the VTE processor
/// byte-by-byte and stops at the first match. The two are
/// fundamentally different algorithms (chunk-at-a-time anchor poll
/// vs. byte-at-a-time mid-flow capture) so the impl-hygiene.md
/// "3+ instances = always extract" rule does not apply.
pub(super) fn poll_until<T, P>(session: &mut PtySession, timeout_ms: u64, mut check: P) -> Option<T>
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
    /// `PtyWrite` and OSC responses back to the PTY. Shared core of
    /// [`Self::drain`] and [`Self::drain_blocking`].
    fn feed_and_flush(&mut self, data: &[u8]) -> usize {
        self.proc.advance(&mut self.term, data);
        for resp in self.term.event_listener().take_responses() {
            // Best-effort: writer errors close the test session naturally
            // via Drop, so swallowing here is correct for test setup.
            let _ = self.writer.write_all(resp.as_bytes());
        }
        self.write_osc_responses_back();
        let _ = self.writer.flush();
        data.len()
    }

    /// Drain every OSC response (color + clipboard load) captured by the
    /// listener since the last flush and write each one back through the
    /// PTY master. Kept private because Section 06.0.c callers reach it
    /// through [`Self::feed_and_flush`] / [`Self::drain_until`] — no
    /// direct public API.
    ///
    /// Best-effort writes: errors are silently dropped (same policy as
    /// the `take_responses` write-back in `feed_and_flush`) because the
    /// test session's lifecycle is driven by `Drop`, not by fallible
    /// write surfaces inside drain.
    fn write_osc_responses_back(&mut self) {
        for resp in self.term.event_listener().take_osc_responses() {
            let _ = self.writer.write_all(resp.as_bytes());
        }
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

    /// Drain bytes from the PTY incrementally — feeding the VTE
    /// processor ONE byte at a time and checking `grid_text()` after
    /// each byte — until `needle` appears in the grid OR the deadline
    /// expires OR `max_bytes` is consumed without a match.
    ///
    /// Returns `Some(grid_text)` captured the instant the needle
    /// first appears. Returns `None` on deadline / max-bytes
    /// exhaustion / channel closure.
    ///
    /// **Why byte-by-byte (not chunk-at-a-time).** The default drain
    /// methods ([`Self::drain`], [`Self::drain_blocking`]) feed
    /// whole `Vec<u8>` chunks to the VTE processor in one
    /// `proc.advance` call. On a fast host, the OS PTY buffer
    /// batches all of tack's modes-test output (~500 bytes, 7+
    /// cap-result lines) into a single chunk. The chunk-at-a-time
    /// drain processes it atomically: the grid jumps from "no
    /// output" to "everything except the last few lines has scrolled
    /// off" in one step. By the time the consumer's poll loop runs,
    /// the only visible cap is the LAST one printed (`(os) Done`).
    ///
    /// The byte-by-byte feed processes each tack-emitted byte
    /// independently. After the closing `)` of `(am)` is fed, the
    /// grid contains `(am)` on a stable line; the next byte (newline,
    /// then the next cap's bytes) hasn't been processed yet. Checking
    /// `grid_text().contains("(am)")` at that moment captures the
    /// intermediate state BEFORE the next cap line scrolls
    /// `(am)` off. This is the only way to catch mid-flow tack
    /// content on a fast host.
    ///
    /// **Lost bytes are intentional.** When the needle is found
    /// mid-chunk, the remaining bytes in the chunk are NOT fed to
    /// the VTE processor — they're discarded. The kernel back-
    /// pressures tack on subsequent writes, and `quit_tack` flushes
    /// any remaining output during teardown. This is correct for
    /// phase capture: the consumer wanted the moment `needle`
    /// appeared, and any later bytes would only scroll the captured
    /// content out of view.
    ///
    /// **Performance note.** Each byte triggers a `grid_text()` call
    /// (~80×24 char build = ~2 KB allocation per byte). For 500
    /// bytes that's ~1 MB of test-only allocation. Acceptable for a
    /// dev-time test framework; not appropriate for production code.
    pub fn drain_until(
        &mut self,
        needle: &str,
        max_bytes: usize,
        timeout_ms: u64,
    ) -> Option<String> {
        if needle.is_empty() {
            return None;
        }
        // Fast path: needle may already be in the grid from earlier
        // drain calls (e.g., the navigator's pre-existing-anchor
        // guard ran a drain that pulled in pre-trigger content).
        if self.grid_text().contains(needle) {
            return Some(self.grid_text());
        }

        let deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let mut bytes_consumed = 0usize;
        loop {
            let now = Instant::now();
            if now >= deadline {
                return None;
            }
            if bytes_consumed >= max_bytes {
                return None;
            }

            // Block briefly for the next chunk. Capping at 50 ms
            // matches the existing drain_blocking blocking budget so
            // CI hosts under load aren't woken too aggressively.
            //
            // Distinguish Timeout from Disconnected: a Timeout
            // means "no chunk arrived in this slice, try again
            // until the deadline expires"; a Disconnected means
            // "the reader thread has closed the channel because
            // the child exited or the PTY closed, and no future
            // chunk will ever arrive — return None immediately
            // so phase capture surfaces the disconnect rather
            // than waiting out the full timeout budget"
            let remaining = deadline.saturating_duration_since(now);
            let block_ms = remaining.as_millis().min(50) as u64;
            let chunk = match self.rx.recv_timeout(Duration::from_millis(block_ms)) {
                Ok(chunk) => chunk,
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return None,
            };

            // Feed each byte through the VTE processor
            // independently and check the grid after each one.
            for byte in &chunk {
                self.proc
                    .advance(&mut self.term, std::slice::from_ref(byte));
                bytes_consumed += 1;
                // Flush any captured PTY responses promptly so
                // tack's DA/DSR handshakes complete in lockstep.
                for resp in self.term.event_listener().take_responses() {
                    let _ = self.writer.write_all(resp.as_bytes());
                }
                // Mirror the OSC response flush — a query that fires
                // in the middle of a phase must complete its round-trip
                // before the next tack byte advances the menu state.
                self.write_osc_responses_back();
                let grid = self.grid_text();
                if grid.contains(needle) {
                    let _ = self.writer.flush();
                    return Some(grid);
                }
                if bytes_consumed >= max_bytes {
                    let _ = self.writer.flush();
                    return None;
                }
            }
            let _ = self.writer.flush();
        }
    }
}

#[cfg(test)]
mod tests;
