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
//! # Module layout (split in 05.0.b)
//!
//! Section 04 had `runner/mod.rs` as a single 375-line file. 05.0.b
//! adds `ScenarioRunner::run_phase[_at]` and a sentinel-detection
//! helper, which would push the file past the 500-line code-hygiene
//! limit. The mandatory split (per 05.0.b's "first commit" task):
//!
//! - `runner/mod.rs` (this file) — type defs ([`ScenarioRunner`],
//!   [`ScenarioOutcome`], [`LiveSession`]), shared private helpers
//!   ([`prepare_and_navigate`], [`finish_and_assert`],
//!   [`scenario_name`]), and the canonical timing constants
//!   ([`MAIN_MENU_READY_TIMEOUT_MS`], [`READY_ANCHOR_TIMEOUT_MS`],
//!   [`TACK_MAIN_MENU_PROMPT`], [`TACK_QUIT_MAX_ITERATIONS`]).
//! - `runner/stable.rs` — the stable-screen API
//!   (`ScenarioRunner::run`, `run_at`, `run_with_session_at`). All
//!   pre-05.0.b methods live here so the diff for the phase-capture
//!   feat commit is reviewable on its own.
//!
//! 05.0.b's feat commit adds `runner/phase.rs` with `run_phase[_at]`
//! plus the sentinel-detection helper they share with the stable
//! path through `assert_no_unverified_sentinels`.
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

pub mod stable;

/// Canonical name format for snapshot/golden files: `<screen_id>_<cols>x<rows>`.
///
/// SINGLE SOURCE OF TRUTH for snapshot/golden naming. Both
/// [`ScenarioOutcome::snapshot_name`] / [`ScenarioOutcome::golden_name`]
/// and [`LiveSession::snapshot_name`] / [`LiveSession::golden_name`]
/// delegate here. Section 07's GPU bridge MUST also call this (or
/// equivalently `live.golden_name()`) instead of rebuilding the
/// format literal — rebuilding at any second site is
/// `LEAK:scattered-knowledge`.
///
/// A future change to the naming convention (e.g., adding a theme
/// suffix) propagates automatically because every consumer routes
/// through this one function.
fn scenario_name(screen_id: &str, cols: u16, rows: u16) -> String {
    format!("{screen_id}_{cols}x{rows}")
}

/// CI-safe wait for tack's main-menu prompt after spawning. Tack
/// initializes ncurses and writes its first menu within ~100 ms in
/// practice; 5 s is the conservative budget for slow hosts. The
/// constant lives at module scope so all spawn paths use the same
/// value (`run_at`, `run_with_session_at`, and any future variants).
pub(super) const MAIN_MENU_READY_TIMEOUT_MS: u64 = 5_000;

/// CI-safe wait for the post-navigation `ready_anchor` to land in
/// the grid. Tack's screen redraws are usually instant after the
/// last navigation keystroke, but tests for slow caps may need
/// headroom; 5 s is the conservative shared budget.
pub(super) const READY_ANCHOR_TIMEOUT_MS: u64 = 5_000;

/// Tack's canonical main-menu prompt. Pinned by Section 03's smoke
/// test snapshot — see section-03-tack-smoke-test.md "Section 04
/// handoff contract" item 2. Centralized here so a tack version
/// upgrade that changes the prompt only requires editing one site.
pub(super) const TACK_MAIN_MENU_PROMPT: &str = "tack [n] >";

/// Default `quit_tack` send budget for tack's submenu nesting:
/// main-menu → submenu → sub-submenu plus one extra for safety.
/// See [`PtySession::quit_tack`] for the two-phase contract.
pub(super) const TACK_QUIT_MAX_ITERATIONS: u32 = 5;

/// Spawn tack, wait for the main menu, walk the `menu_path`, wait
/// for the ready anchor, capture the grid, and run the parser. The
/// shared pipeline that both [`ScenarioRunner::run_at`] and
/// [`ScenarioRunner::run_with_session_at`] consume.
///
/// Returns the live `(session, env, grid_text, parsed)` tuple. The
/// `env` MUST be held by the caller (see `LiveSession::_terminfo`
/// and the `tic` lazy-read race rationale).
pub(super) fn prepare_and_navigate(
    spec: &ScenarioSpec,
    cols: u16,
    rows: u16,
) -> (PtySession, TerminfoEnv, String, ScreenFacts) {
    let env = TerminfoEnv::compile();
    let mut session = PtySession::spawn_tack(&env, cols, rows);
    session.wait_for(TACK_MAIN_MENU_PROMPT, MAIN_MENU_READY_TIMEOUT_MS);
    TackNavigator::navigate(&mut session, spec.menu_path);
    session.wait_for(spec.ready_anchor, READY_ANCHOR_TIMEOUT_MS);
    let grid_text = session.grid_text();
    let parsed = (spec.parser)(&grid_text);
    (session, env, grid_text, parsed)
}

/// Quit tack via `quit_tack` (or `quit_path` override) and assert
/// `exit.success()` with a panic message that includes the scenario
/// identity and the captured grid.
///
/// SHARED IMPLEMENTATION between [`ScenarioRunner::run_at`] and
/// [`LiveSession::finish`] — both flows must produce the same
/// post-quit assertion semantics so the C2/C3 fixes (state-aware
/// quit + exit-status assertion) cannot drift between the two
/// public entry points.
pub(super) fn finish_and_assert(
    session: &mut PtySession,
    quit_path: Option<fn(&mut PtySession) -> ExitStatus>,
    scenario_id: &str,
    cols: u16,
    rows: u16,
) -> ExitStatus {
    let exit = match quit_path {
        Some(quit) => quit(session),
        None => session.quit_tack(TACK_QUIT_MAX_ITERATIONS),
    };
    assert!(
        exit.success(),
        "scenario {scenario_id} ({cols}x{rows}): tack exited non-zero: \
         {exit:?}\nGrid:\n{grid}",
        grid = session.grid_text(),
    );
    exit
}

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
    /// `.snap` file. Delegates to the canonical [`scenario_name`]
    /// SSOT helper so [`LiveSession::snapshot_name`] and Section 07
    /// stay in lockstep automatically.
    #[must_use]
    pub fn snapshot_name(&self) -> String {
        scenario_name(self.screen_id, self.cols, self.rows)
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
    pub(super) _terminfo: TerminfoEnv,
    /// Per-scenario quit override propagated from `ScenarioSpec`.
    pub(super) quit_path: Option<fn(&mut PtySession) -> ExitStatus>,
}

impl LiveSession {
    /// Snapshot/golden name for this live session: same convention
    /// as [`ScenarioOutcome::snapshot_name`] —
    /// `"<screen_id>_<cols>x<rows>"`.
    ///
    /// Delegates to the canonical [`scenario_name`] SSOT helper.
    /// Section 07's GPU bridge MUST call this (or its
    /// [`Self::golden_name`] alias) instead of rebuilding the
    /// format literal at the call site — rebuilding at any second
    /// site is `LEAK:scattered-knowledge`. A future change to the
    /// naming convention propagates automatically.
    #[must_use]
    pub fn snapshot_name(&self) -> String {
        scenario_name(self.screen_id, self.cols, self.rows)
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
    /// `exit.success()`. Consumes `self` so the caller can't use
    /// the session after `finish` — `Drop` runs on the held fields
    /// the moment `finish` returns and the temp terminfo + child
    /// are reaped together.
    ///
    /// Delegates the actual quit + assertion to
    /// [`finish_and_assert`] so this method and
    /// [`ScenarioRunner::run_at`] cannot drift on the C2/C3 fixes.
    ///
    /// Section 07 GPU goldens call this AFTER `render_to_pixels`.
    pub fn finish(mut self) -> ExitStatus {
        finish_and_assert(
            &mut self.session,
            self.quit_path,
            self.scenario_id,
            self.cols,
            self.rows,
        )
    }
}

#[cfg(test)]
mod tests;
