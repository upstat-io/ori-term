//! Phase-capture `ScenarioRunner` API: `run_phase`, `run_phase_at`.
//!
//! These methods drive tack scenarios where the captured fact is
//! visible only briefly mid-run — the modes test sweep that prints
//! `(am)`, `(bce)`, `(bw)`, ... and scrolls each line off the
//! 24-row viewport in under ~500 ms. The stable-screen
//! [`super::stable`] path cannot capture these because the 300 ms
//! [`crate::session::PtySession::send`] quiesce + 200 ms
//! [`crate::session::PtySession::wait_for`] post-match quiesce lets
//! the line scroll off before the runner returns control.
//!
//! # Why a new primitive (and not just a tighter `wait_for`)
//!
//! Section 04's `prepare_and_navigate` calls
//! `session.send(...)` per [`super::super::spec::MenuStep`]. `send`
//! calls `wait(300)` internally — a 300 ms quiet-period drain after
//! every keystroke. After the last `MenuStep`, `prepare_and_navigate`
//! calls `session.wait_for(spec.ready_anchor, 5_000)` which
//! delegates to `wait_for_with_context`. On a successful match,
//! `wait_for_with_context` calls `self.wait(200)` — another 200 ms
//! quiet period before returning. So between sending the final
//! navigation key and the test reading `grid_text()`, there is a
//! minimum of 500 ms of post-write quiesce.
//!
//! The fix is NOT to weaken the quiesce inside `send` /
//! `wait_for_*` — those are pinned by the existing 198 vttest tests
//! and Section 04's stable-screen contract. The fix is an ADDITIVE
//! new primitive that:
//!
//! - Uses [`crate::session::PtySession::send_raw`] (no built-in
//!   quiesce) for the phase trigger.
//! - Polls `grid_text()` for the phase anchor on the canonical
//!   bounded-poll skeleton with NO post-match quiesce.
//! - Captures `grid_text()` immediately on first match.
//! - Quits tack via `quit_tack` after the capture, same exit-status
//!   assertion as `run_at`.
//!
//! Both code paths share `prepare_and_navigate`'s spawn + main-menu
//! wait + terminfo lifetime; the divergence is in the post-navigate
//! phase loop. This is structurally compatible with
//! [`super::ScenarioRunner`] and does not regress the stable-screen
//! path.
//!
//! # Algorithmic distinction from `poll_until`
//!
//! The first cut of [`phase_capture_loop`] (05.0.b) consumed
//! [`crate::session::poll_until`] directly to share the
//! bounded-poll skeleton with the four existing waiters
//! (`wait_for_with_context`, `wait_for_any`, `wait_for_child_exit_inner`,
//! and the original phase loop). Empirical probing during 05.1
//! showed this approach is fundamentally incompatible with how
//! tack writes the modes-test sweep on a fast Linux host:
//!
//! - Tack runs the entire am/bce/bw/km/mir/msgr/xenl sweep in
//!   under 1 ms total wall-clock.
//! - The OS PTY buffer + reader-thread channel batch the entire
//!   ~500-byte output into ONE `Vec<u8>` chunk.
//! - `poll_until` calls `drain_blocking` which feeds the whole
//!   chunk to `vte::Processor::advance` in ONE call.
//! - VTE processes the chunk atomically — by the time `poll_until`
//!   re-checks the grid, every cap except the last (`(os) Done`)
//!   has scrolled off the 24-row viewport. There is no observable
//!   intermediate state.
//!
//! The `poll_until` skeleton is the right primitive for ANCHOR-based
//! synchronization (wait until tack draws the next prompt). It is
//! the WRONG primitive for MID-FLOW capture where the consumer
//! needs to observe a transient state between two PTY writes that
//! happen in the same kernel buffer.
//!
//! [`phase_capture_loop`] now uses
//! [`crate::session::PtySession::drain_until`] — a byte-by-byte
//! drain that feeds the VTE processor ONE byte at a time and
//! checks the grid after each byte, stopping at the first match
//! and discarding any unread bytes in the chunk. This is a
//! DIFFERENT algorithm from `poll_until` (incremental feed +
//! per-byte check vs. chunk-at-a-time + per-iteration check), so
//! the "3+ instances of the same skeleton" rule
//! does not apply: the two algorithms serve different purposes
//! and live in two distinct canonical homes (`session::poll_until`
//! for chunk-based polling, `session::PtySession::drain_until` for
//! byte-by-byte phase capture).
//!
//! # No post-match quiesce
//!
//! [`drain_until`] returns the captured grid the instant the
//! needle appears — no `wait(200)` call follows. The byte-by-byte
//! feed and the immediate-return-on-first-match are both essential
//! to phase capture: the consumer wanted the moment `needle`
//! appeared, and any later bytes would scroll the captured content
//! out of view.
//!
//! # Sentinel detection
//!
//! Like the stable path, `run_phase_at` calls
//! [`super::assert_no_unverified_sentinels`] BEFORE any I/O so a
//! [`super::super::spec::PhaseSpec`] that still references
//! [`super::super::spec::unverified_menu_key`] /
//! [`super::super::spec::unverified_anchor`] panics with a
//! referral to 05.0 instead of leaking sentinel bytes to tack.

use std::time::Instant;

use super::super::navigator::TackNavigator;
use super::super::spec::PhaseSpec;
use super::{
    MAIN_MENU_READY_TIMEOUT_MS, READY_ANCHOR_TIMEOUT_MS, ScenarioOutcome, ScenarioRunner,
    TACK_MAIN_MENU_PROMPT, assert_no_unverified_sentinels, finish_and_assert,
};
use crate::session::PtySession;
use crate::terminfo::TerminfoEnv;

/// CI-safe default phase capture timeout. 5 s matches the existing
/// [`READY_ANCHOR_TIMEOUT_MS`]. Phase scenarios that need a larger
/// budget set [`PhaseSpec::phase_timeout_ms`] directly.
pub(super) const PHASE_DEFAULT_TIMEOUT_MS: u64 = 5_000;

/// Hard upper bound on bytes [`phase_capture_loop`] will feed
/// before giving up. 64 KiB is well above tack's full modes-test
/// output (~500 bytes) but bounded enough that a runaway tack that
/// never produces the anchor cannot consume unbounded memory.
const PHASE_MAX_BYTES: usize = 64 * 1024;

/// Tight phase-capture loop. Drains the PTY byte-by-byte through
/// [`PtySession::drain_until`] and returns the captured grid the
/// instant `phase_anchor` first appears.
///
/// **Why byte-by-byte (not [`crate::session::poll_until`]).** See
/// the module-level rustdoc above. The short version: tack writes
/// the entire modes-test sweep into one PTY buffer chunk on a fast
/// host; chunk-at-a-time drain processes it atomically and the
/// grid jumps from "no output" to "post-test (os) Done" in one
/// step. Byte-by-byte drain stops feeding the moment the anchor
/// appears so the captured grid reflects the intermediate state
/// BEFORE later cap lines scroll the anchor off the viewport.
///
/// **No post-match quiesce.** [`PtySession::drain_until`] returns
/// the moment the needle appears. There is NO `wait(200)` call
/// after — that would defeat the entire point of phase capture.
///
/// **Bounded by [`PHASE_MAX_BYTES`].** A runaway tack that never
/// produces the anchor cannot consume unbounded memory: the loop
/// caps at 64 KiB before returning `None`. The bound is well above
/// tack's actual output (~500 bytes for the full modes sweep) but
/// catches infinite loops cleanly.
pub(crate) fn phase_capture_loop(
    session: &mut PtySession,
    phase_anchor: &str,
    timeout_ms: u64,
) -> Option<String> {
    session.drain_until(phase_anchor, PHASE_MAX_BYTES, timeout_ms)
}

impl ScenarioRunner {
    /// Run a phase-capture scenario at the standard 80x24 size.
    ///
    /// Phase capture differs from [`Self::run`] in that the
    /// post-navigation step uses
    /// [`PtySession::send_raw`] (no built-in 300 ms quiesce)
    /// followed by [`phase_capture_loop`] which returns the moment
    /// `phase_anchor` appears (no 200 ms post-match quiesce). This
    /// is the only way to catch mid-flow content that scrolls off
    /// in under ~500 ms.
    ///
    /// The same pre-existing-anchor guard applies: if the phase
    /// anchor is already present in the grid BEFORE the phase
    /// trigger fires, the runner panics with a "phase anchor
    /// pre-existing" message.
    ///
    /// Panics on:
    /// - sentinel detection (any `unverified_menu_key()` /
    ///   `unverified_anchor()` placeholder, BEFORE PTY spawn)
    /// - navigation timeout (delegates to
    ///   [`TackNavigator::navigate`])
    /// - phase pre-existing-anchor violation
    /// - phase timeout (anchor never appeared)
    /// - non-success exit status from tack
    #[must_use]
    pub fn run_phase(spec: &PhaseSpec) -> ScenarioOutcome {
        Self::run_phase_at(spec, 80, 24)
    }

    /// Run a phase-capture scenario at a specific grid size.
    ///
    /// See [`Self::run_phase`] for the full panic contract. Sizes
    /// other than 80x24 are valid but rare for phase scenarios:
    /// phase capture timing is the dominant variable, not viewport
    /// size, so most phase scenarios pin a single 80x24 size and
    /// use stable-screen scenarios for the size matrix.
    #[must_use]
    pub fn run_phase_at(spec: &PhaseSpec, cols: u16, rows: u16) -> ScenarioOutcome {
        // Sentinel gate FIRST — fires before any I/O so misconfigured
        // consts cannot leak garbage bytes to tack and corrupt the
        // shared CONPTY/PTY state for subsequent tests in the same
        // process. Mirrors `prepare_and_navigate`'s gate so the stable
        // and phase paths share one detection algorithm.
        assert_no_unverified_sentinels(
            spec.id,
            spec.menu_path,
            Some(spec.phase_trigger),
            &[spec.phase_setup_anchor, spec.phase_anchor],
        );

        let env = TerminfoEnv::compile();
        let mut session = PtySession::spawn_tack(&env, cols, rows);
        session.wait_for(TACK_MAIN_MENU_PROMPT, MAIN_MENU_READY_TIMEOUT_MS);
        TackNavigator::navigate(&mut session, spec.menu_path);
        session.wait_for(spec.phase_setup_anchor, READY_ANCHOR_TIMEOUT_MS);

        // Pre-existing-anchor guard for the phase anchor.
        let pre_grid = session.grid_text();
        assert!(
            !pre_grid.contains(spec.phase_anchor),
            "scenario {id} ({cols}x{rows}): phase_anchor {anchor:?} already \
             present BEFORE phase_trigger fires; pick a phase_anchor unique \
             to the post-trigger viewport.\nGrid:\n{pre_grid}",
            id = spec.id,
            anchor = spec.phase_anchor,
        );

        // Trigger the phase WITHOUT the 300 ms quiesce so the poll
        // loop can begin immediately. Capture the wall-clock instant
        // for the timeout panic message.
        let trigger_started_at = Instant::now();
        session.send_raw(spec.phase_trigger);

        // Tight poll for the phase anchor with NO post-match quiesce.
        let timeout_ms = if spec.phase_timeout_ms > 0 {
            spec.phase_timeout_ms
        } else {
            PHASE_DEFAULT_TIMEOUT_MS
        };
        let Some(grid_text) = phase_capture_loop(&mut session, spec.phase_anchor, timeout_ms)
        else {
            let elapsed = trigger_started_at.elapsed();
            // Diagnostic must include EVERY input the loop knew about
            // so a future failure can be reproduced from the panic
            // message alone — no log scraping. Print the phase anchor
            // we were waiting for, the setup anchor that proved we
            // were on the right pre-trigger screen, the literal
            // trigger bytes, the per-scenario timeout, and the full
            // captured grid at the moment of failure.
            panic!(
                "scenario {id} ({cols}x{rows}): phase_anchor {anchor:?} \
                 did not appear within {timeout_ms} ms (elapsed: {elapsed:?}).\n\
                 phase_setup_anchor: {setup:?}\n\
                 phase_trigger: {trigger:?}\n\
                 menu_path steps: {steps}\n\
                 Grid at timeout:\n{grid}",
                id = spec.id,
                anchor = spec.phase_anchor,
                setup = spec.phase_setup_anchor,
                trigger = spec.phase_trigger,
                steps = spec.menu_path.len(),
                grid = session.grid_text(),
            );
        };

        let parsed = (spec.parser)(&grid_text);
        let _exit = finish_and_assert(&mut session, spec.quit_path, spec.id, cols, rows);
        // _env held until end-of-scope so tack's lazy terminfo reads
        // always see a live temp dir.
        let _ = env;

        ScenarioOutcome {
            scenario_id: spec.id,
            screen_id: spec.screen_id,
            cols,
            rows,
            grid_text,
            parsed,
        }
    }
}
