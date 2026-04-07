//! Public entry point for running tack scenarios.
//!
//! [`ScenarioRunner`] is what Sections 05-08 of the tack-conformance
//! plan call from individual `#[test] fn`s. Given a `&ScenarioSpec`,
//! it spawns tack against a freshly compiled [`TerminfoEnv`],
//! navigates the menu path via [`super::navigator::TackNavigator`],
//! captures `grid_text`, runs the per-scenario parser, observes a
//! CLEAN child exit via [`PtySession::quit_tack`], and returns a
//! size-aware [`ScenarioOutcome`].
//!
//! GPU goldens (Section 07) consume [`LiveSession`] via
//! [`ScenarioRunner::run_with_session_at`] — same spawn+navigate+
//! parse pipeline, but the live `PtySession` is returned to the
//! caller for rendering through the GPU pipeline. Section 07 callers
//! MUST call [`LiveSession::finish`] after rendering so the
//! exit-success assertion still runs (M5 cleanup contract).
//!
//! # `PtySession::send` quiesce dependency (Mi1)
//!
//! [`PtySession::send`] calls `wait(300)` internally to drain output
//! before returning. The framework's "no fixed sleeps in the
//! navigator loop" claim refers to the navigator's poll loop, NOT the
//! post-write quiesce inside `send`. The 300 ms inside `send` is
//! canonical behavior pinned by the existing 198 vttest tests; the
//! framework consumes it as-is. The [`PtySession::send_raw`] lever
//! (introduced alongside `quit_tack` in 04.0.b.3) is consumed by
//! `quit_tack` for the teardown path; navigation code continues to
//! use `send()` for its quiesce. If observed flakes ever require a
//! tighter quiesce for navigation too, [`super::navigator::TackNavigator`]
//! can switch to `send_raw` and add its own explicit drain between
//! steps — but that is not done in Section 04.
//!
//! # Per-scenario terminfo compile cost (Mi2)
//!
//! Each [`ScenarioRunner::run_at`] call invokes
//! [`TerminfoEnv::compile`] which shells out to `tic`. With ~30
//! scenarios × 3 sizes that's ~90 `tic` invocations per test run.
//! Section 09 of the tack-conformance plan measures the wall-clock
//! cost via `/usr/bin/time -v` after Sections 05-08 land and adds an
//! `OnceLock` cache (compiling each `TerminfoVariant` exactly once
//! per process) if the regression exceeds 10 s. The lever is
//! documented here so the future maintainer knows it exists.

use portable_pty::ExitStatus;

use crate::session::{PtySession, tack_available, tic_available};
use crate::terminfo::TerminfoEnv;

use super::navigator::TackNavigator;
use super::parser::ScreenFacts;
use super::spec::ScenarioSpec;

/// The result of running one scenario: the captured grid text and
/// the per-scenario parser's typed extraction.
///
/// Carries SIZE-AWARE identity: `scenario_id` is the test name,
/// `screen_id` is the dedupable screen identity. [`Self::snapshot_name`]
/// and [`Self::golden_name`] build the insta/PNG file names from
/// `screen_id` + `cols` + `rows` so size-matrix runs share goldens
/// when navigation produces the same screen.
#[derive(Clone, Debug)]
pub struct ScenarioOutcome {
    /// The scenario's semantic test name (e.g. `"tack_modes_am"`).
    pub scenario_id: &'static str,
    /// Dedupable screen identity for snapshot naming
    /// (e.g. `"tack_modes"` for every modes scenario).
    pub screen_id: &'static str,
    /// Grid columns the scenario ran at.
    pub cols: u16,
    /// Grid rows the scenario ran at.
    pub rows: u16,
    /// Captured grid text at the moment of parsing.
    pub grid_text: String,
    /// Parser-extracted typed facts about the screen.
    pub parsed: ScreenFacts,
}

impl ScenarioOutcome {
    /// Insta snapshot name: `<screen_id>_<cols>x<rows>`. Multiple
    /// scenarios that share `screen_id` AND size will share an insta
    /// `.snap` file.
    #[must_use]
    pub fn snapshot_name(&self) -> String {
        format!("{}_{}x{}", self.screen_id, self.cols, self.rows)
    }

    /// PNG golden name: same convention as [`Self::snapshot_name`].
    /// Used by Section 07's GPU bridge as the SSOT for golden
    /// filenames.
    #[must_use]
    pub fn golden_name(&self) -> String {
        self.snapshot_name()
    }
}

/// # Snapshot policy for duplicate-screen scenarios
///
/// When multiple scenarios visit the SAME tack screen (e.g. seven
/// `tack_modes_*` variants that all navigate `[n] [m]`), they all
/// produce the same `screen_id` and the same `grid_text`. Tests
/// snapshot via `insta::assert_snapshot!(outcome.snapshot_name(),
/// &outcome.grid_text)` — insta dedupes on the name, so seven
/// scenarios sharing one `screen_id` write ONE `.snap` file. The
/// individual scenarios still differ on the `parsed` field (which
/// cap they assert).
///
/// Convention: the FIRST scenario in alphabetical order
/// (e.g. `tack_modes_am`) is documented as the snapshot owner so the
/// test that "owns" the insta golden is unambiguous. The rest of the
/// same-screen scenarios assert on `parsed` only.
pub struct ScenarioRunner;

impl ScenarioRunner {
    /// Returns true iff both `tack` and `tic` are available — call
    /// at the top of every test that runs scenarios so the test
    /// skips cleanly when the tools are missing.
    #[must_use]
    pub fn available() -> bool {
        tack_available() && tic_available()
    }

    /// Run a single scenario at the standard 80x24 size.
    ///
    /// Spawns tack via [`PtySession::spawn_tack`] against a fresh
    /// [`TerminfoEnv`], navigates the `menu_path`, calls the parser,
    /// quits tack cleanly via [`PtySession::quit_tack`] (or
    /// `spec.quit_path` if set), and asserts the child exited with
    /// `success()`.
    ///
    /// Panics on navigation timeout (via
    /// [`super::navigator::TackNavigator::navigate`],
    /// pre-existing-anchor guard, or step timeout) and on
    /// non-success exit. The panic message includes the captured
    /// grid AND the exit status.
    #[must_use]
    pub fn run(spec: &ScenarioSpec) -> ScenarioOutcome {
        Self::run_at(spec, 80, 24)
    }

    /// Run a scenario at a specific grid size. Used by Sections
    /// 05-08 for size-matrix tests.
    #[must_use]
    pub fn run_at(spec: &ScenarioSpec, cols: u16, rows: u16) -> ScenarioOutcome {
        let env = TerminfoEnv::compile();
        let mut session = PtySession::spawn_tack(&env, cols, rows);

        // Wait for the main menu prompt before navigating. The
        // `tack [n] >` prompt is the canonical readiness anchor
        // pinned by Section 03's smoke test snapshot — see
        // section-03-tack-smoke-test.md "Section 04 handoff
        // contract" item 2.
        session.wait_for("tack [n] >", 5_000);

        TackNavigator::navigate(&mut session, spec.menu_path);
        session.wait_for(spec.ready_anchor, 5_000);

        let grid_text = session.grid_text();
        let parsed = (spec.parser)(&grid_text);

        // State-aware clean quit. `quit_tack(5)` (introduced in
        // 04.0.b.4) sends one `q\n` via `send_raw` (no 300 ms
        // quiesce per iteration), observes `try_wait()`, and stops
        // the moment the child exits — no fixed-count guesswork.
        // The C2 fix replaces the previous
        // `send(b"q\n") × 3 + wait_for_child_exit(2_000)`
        // antipattern.
        let exit = match spec.quit_path {
            Some(quit) => quit(&mut session),
            None => session.quit_tack(5),
        };

        // C3 fix: assert exit success. The bare `let _exit = ...`
        // throws away the exit status and silently passes when
        // tack aborts with an error code.
        assert!(
            exit.success(),
            "scenario {scenario_id} ({cols}x{rows}): tack exited \
             non-zero: {exit:?}\nGrid:\n{grid_text}",
            scenario_id = spec.id,
        );

        ScenarioOutcome {
            scenario_id: spec.id,
            screen_id: spec.screen_id,
            cols,
            rows,
            grid_text,
            parsed,
        }
    }

    /// Like [`Self::run_at`] but returns the live [`PtySession`] so
    /// GPU callers can render it through the pipeline before
    /// quitting.
    ///
    /// Caller MUST call [`LiveSession::finish`] after rendering.
    /// Dropping `LiveSession` without calling `finish` reaps the
    /// child via `Drop` (see [`PtySession::drop`]) but loses the
    /// exit-status assertion — that's a regression risk Section 07's
    /// checklist guards against.
    #[must_use]
    pub fn run_with_session_at(spec: &ScenarioSpec, cols: u16, rows: u16) -> LiveSession {
        let env = TerminfoEnv::compile();
        let mut session = PtySession::spawn_tack(&env, cols, rows);
        session.wait_for("tack [n] >", 5_000);
        TackNavigator::navigate(&mut session, spec.menu_path);
        session.wait_for(spec.ready_anchor, 5_000);
        // Capture the grid ONCE here so the parser runs against the
        // same bytes Section 07 will render through the GPU.
        let grid_text = session.grid_text();
        let facts = (spec.parser)(&grid_text);
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

/// Wrapper that returns a LIVE [`PtySession`] instead of just text.
/// Used by Section 07 GPU goldens to render the live session through
/// the GPU pipeline before quitting.
///
/// The `_terminfo` field is intentionally unused at the call site —
/// its only job is to outlive the session, because tack reads
/// terminfo lazily during screen redraws and dropping the
/// [`TerminfoEnv`] before the session would race with tack's reads.
///
/// **Cleanup contract:** GPU callers MUST call [`Self::finish`]
/// after rendering. Relying on `Drop` works for FD cleanup but loses
/// the exit-status assertion that catches tack regressions.
/// [`Self::finish`] shares the SAME [`PtySession::quit_tack`] helper
/// as [`ScenarioRunner::run_at`], so both flows have identical exit
/// semantics. See M5 in the Codex review at the top of section-04
/// for the rationale.
pub struct LiveSession {
    /// The live PTY session running tack.
    pub session: PtySession,
    /// Parser-extracted facts captured at the moment of navigation.
    pub facts: ScreenFacts,
    /// The scenario's semantic test name.
    pub scenario_id: &'static str,
    /// Dedupable screen identity for golden naming.
    pub screen_id: &'static str,
    /// Grid columns the session was opened at.
    pub cols: u16,
    /// Grid rows the session was opened at.
    pub rows: u16,
    /// Held to keep the temp terminfo dir alive for tack's lazy reads.
    _terminfo: TerminfoEnv,
    /// Per-scenario quit override propagated from `ScenarioSpec`.
    quit_path: Option<fn(&mut PtySession) -> ExitStatus>,
}

impl LiveSession {
    /// Snapshot/golden name for this live session: same convention
    /// as [`ScenarioOutcome::snapshot_name`] —
    /// `"<screen_id>_<cols>x<rows>"`.
    ///
    /// SINGLE SOURCE OF TRUTH for the naming convention so Section
    /// 07's GPU bridge does NOT rebuild the string from
    /// `live.screen_id` + `cols` + `rows` at the call site.
    /// Rebuilding the format string at two sites is
    /// `LEAK:scattered-knowledge`; both [`ScenarioOutcome`] and
    /// `LiveSession` delegate to this same format literal so a
    /// future change to the naming convention propagates
    /// automatically.
    #[must_use]
    pub fn snapshot_name(&self) -> String {
        format!("{}_{}x{}", self.screen_id, self.cols, self.rows)
    }

    /// PNG golden name: identical to [`Self::snapshot_name`]. Used
    /// by Section 07's `run_tack_scenario_golden` as the SSOT golden
    /// filename. Section 07 MUST call `live.golden_name()` instead
    /// of rebuilding `format!("{}_{}x{}", ...)` at the call site.
    #[must_use]
    pub fn golden_name(&self) -> String {
        self.snapshot_name()
    }

    /// Quit tack cleanly via the same [`PtySession::quit_tack`]
    /// helper as [`ScenarioRunner::run_at`], asserting
    /// `exit.success()`. Consumes `self` so the caller can't use the
    /// session after `finish` — `Drop` runs on the held fields the
    /// moment `finish` returns and the temp terminfo + child are
    /// reaped together.
    ///
    /// Section 07 GPU goldens call this AFTER `render_to_pixels`.
    pub fn finish(mut self) -> ExitStatus {
        let exit = match self.quit_path {
            Some(quit) => quit(&mut self.session),
            None => self.session.quit_tack(5),
        };
        assert!(
            exit.success(),
            "LiveSession {scenario_id} ({cols}x{rows}): tack exited \
             non-zero: {exit:?}\nGrid:\n{grid}",
            scenario_id = self.scenario_id,
            cols = self.cols,
            rows = self.rows,
            grid = self.session.grid_text(),
        );
        exit
    }
}

#[cfg(test)]
mod tests;
