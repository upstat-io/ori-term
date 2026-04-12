//! Stable-screen `ScenarioRunner` API: `run`, `run_at`,
//! `run_with_session_at`.
//!
//! These methods drive tack scenarios where the captured screen is
//! quiescent — tack draws the screen in response to the final
//! navigation keystroke and then waits for input. The 500 ms
//! post-write quiesce baked into [`crate::session::PtySession::send`]
//! and [`crate::session::PtySession::wait_for`] is the right primitive
//! here: it gives tack time to finish painting before
//! [`super::prepare_and_navigate`] reads `grid_text()`.
//!
//! Mid-flow scrolling content (the modes test sweep, the SGR table
//! that scrolls past) is the domain of `runner/phase.rs` (added in
//! 05.0.b's feat commit). Phase capture cannot reuse this path
//! because the post-write quiesce lets the captured line scroll off
//! before the runner returns control.
//!
//! This file is the SOLE owner of `ScenarioRunner::run`, `run_at`,
//! and `run_with_session_at`. The split from the pre-05.0.b
//! single-file `runner/mod.rs` is documented in the parent module's
//! rustdoc.

use super::super::spec::ScenarioSpec;
use super::{
    LiveSession, ScenarioOutcome, ScenarioRunner, finish_and_assert, prepare_and_navigate,
};

impl ScenarioRunner {
    /// Run a single scenario at the standard 80x24 size.
    ///
    /// Spawns tack via [`crate::session::PtySession::spawn_tack`]
    /// against a fresh [`crate::terminfo::TerminfoEnv`], navigates
    /// the `menu_path`, calls the parser, quits tack cleanly via
    /// [`crate::session::PtySession::quit_tack`] (or
    /// `spec.quit_path` if set), and asserts the child exited with
    /// `success()`.
    ///
    /// Panics on navigation timeout (via
    /// [`super::super::navigator::TackNavigator::navigate`],
    /// pre-existing-anchor guard, or step timeout) and on
    /// non-success exit. The panic message includes the captured
    /// grid AND the exit status.
    #[must_use]
    pub fn run(spec: &ScenarioSpec) -> ScenarioOutcome {
        Self::run_at(spec, 80, 24)
    }

    /// Run a scenario at a specific grid size. Used by Sections
    /// 05-08 for size-matrix tests.
    ///
    /// Delegates the spawn → navigate → capture → parse pipeline to
    /// [`prepare_and_navigate`] and the quit + assert tail to
    /// [`finish_and_assert`] so this method and
    /// [`Self::run_with_session_at`] cannot drift, and so the C2
    /// (state-aware quit) and C3 (exit-success assertion) fixes
    /// have a single canonical home.
    ///
    /// Panics on navigation timeout (via the
    /// [`super::super::navigator::TackNavigator::navigate`]
    /// pre-existing-anchor guard or step timeout) and on
    /// non-success exit. The panic message includes the captured
    /// grid AND the exit status.
    #[must_use]
    pub fn run_at(spec: &ScenarioSpec, cols: u16, rows: u16) -> ScenarioOutcome {
        let (mut session, _env, grid_text, parsed) = prepare_and_navigate(spec, cols, rows);
        let _exit = finish_and_assert(&mut session, spec.quit_path, spec.id, cols, rows);
        // _env held until end-of-scope so tack's lazy terminfo
        // reads always see a live temp dir.
        ScenarioOutcome {
            scenario_id: spec.id,
            screen_id: spec.screen_id,
            cols,
            rows,
            grid_text,
            parsed,
        }
    }

    /// Like [`Self::run_at`] but returns the live
    /// [`crate::session::PtySession`] so GPU callers can render it
    /// through the pipeline before quitting.
    ///
    /// Caller MUST call [`LiveSession::finish`] after rendering.
    /// Dropping `LiveSession` without calling `finish` reaps the
    /// child via `Drop` (see
    /// [`crate::session::PtySession::drop`]) but loses the
    /// exit-status assertion — that's a regression risk Section 07's
    /// checklist guards against.
    #[must_use]
    pub fn run_with_session_at(spec: &ScenarioSpec, cols: u16, rows: u16) -> LiveSession {
        let (session, env, _grid_text, facts) = prepare_and_navigate(spec, cols, rows);
        LiveSession {
            session,
            facts,
            scenario_id: spec.id,
            screen_id: spec.screen_id,
            cols,
            rows,
            _terminfo: env,
            quit_path: spec.quit_path,
        }
    }
}
