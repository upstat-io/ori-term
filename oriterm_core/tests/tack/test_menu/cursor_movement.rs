//! Test wrappers for the tack cursor movement scenario at three sizes.
//!
//! Const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::cursor_movement`.
//! This file defines `#[test] fn` wrappers that invoke
//! `ScenarioRunner::run_at` against the const at 80x24,
//! 97x33, and 120x40 (the canonical size matrix from the
//! existing `vttest_menu1` pattern).
//!
//! # Empirical reality (tack v1.08)
//!
//! Tack v1.08's cursor movement test does NOT emit any of the
//! 8 cursor cap labels (`cup`, `hpa`, `vpa`, `csr`, `cuu`, `cud`,
//! `cub`, `cuf`). The captured output is only:
//!
//! ```text
//! \x1B[H\x1B[2JThis line should start in the home position.
//! The rest of the screen should be clear. (clear) Done
//! ```
//!
//! See `crates/oriterm_test_support/src/tack_framework/scenarios/cursor_movement/mod.rs`
//! rustdoc for the full empirical evidence and the hybrid coverage
//! strategy: this file asserts the grid contains `Done` plus the
//! testable semantic facts (`This line should start in the home
//! position`, `(clear)`) and snapshots the captured grid for
//! visual regression. The parser's `capability_labels` field is
//! always empty against tack v1.08 so the wrapper does NOT assert
//! on it — same hybrid pattern as 05.2 / 05.3 wrappers.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::cursor_movement::TACK_CURSOR_MOVEMENT;

fn run_cursor_movement_at(cols: u16, rows: u16) {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic unavailable, skipping tack_cursor_movement_{cols}x{rows}");
        return;
    }
    let outcome = ScenarioRunner::run_at(&TACK_CURSOR_MOVEMENT, cols, rows);

    // End-to-end pin: tack ran the standard cursor movement test
    // and reported its terminator. Catches regressions in spawn /
    // navigate / capture / quit pipeline at this size.
    assert!(
        outcome.grid_text.contains("Done"),
        "expected captured grid at {cols}x{rows} to contain 'Done' terminator, got:\n{}",
        outcome.grid_text
    );

    // SEMANTIC PINS: tack v1.08's cursor movement test only
    // surfaces the (clear) cap name plus a single descriptive
    // header line. These two assertions are the canonical
    // semantic claims for 05.4 cap-coverage:
    // 1. "This line should start in the home position" — proves
    // tack entered the cursor movement test code path. Note
    // this does NOT independently prove `cup` was exercised:
    // `clear` in `extra/ori_term.info` is defined as
    // `\E[H\E[2J`, which already homes the cursor via a
    // literal escape (NOT an invocation of the parameterized
    // `cup` capability). The "home position" behavior is
    // therefore explained entirely by `clear` itself; a `cup`
    // regression would not be caught here. (fix.)
    // 2. "(clear)" — proves tack referenced the clear cap by its
    // terminfo short name (the canonical tack output format
    // matching the (am)/(os)/(bel)/(colors)/(pairs) pattern
    // from prior screens). This is the cap-coverage pin for
    // `clear` in 05.5.
    // Per the empirical-finding block in 05.4, only `clear` is
    // honestly covered by 05.4. Coverage for `cup`, `csr`, `hpa`,
    // `vpa`, `cuu`, `cud`, `cub`, `cuf` must come from Section
    // 07's GPU goldens or vttest — `cup` was previously claimed
    // as transitively covered, but (Codex
    // review-work) correctly identified that the home behavior
    // is explained by `clear`'s literal escape and does not
    // independently exercise `cup`.
    assert!(
        outcome
            .grid_text
            .contains("This line should start in the home position"),
        "expected captured grid at {cols}x{rows} to contain 'This line should start in the home position' header, got:\n{}",
        outcome.grid_text
    );
    assert!(
        outcome.grid_text.contains("(clear)"),
        "expected captured grid at {cols}x{rows} to contain '(clear)' parenthesized cap, got:\n{}",
        outcome.grid_text
    );

    // Insta snapshot of the full grid for visual regression.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
fn tack_cursor_movement_80x24() {
    run_cursor_movement_at(80, 24);
}

#[test]
fn tack_cursor_movement_97x33() {
    run_cursor_movement_at(97, 33);
}

#[test]
fn tack_cursor_movement_120x40() {
    run_cursor_movement_at(120, 40);
}
