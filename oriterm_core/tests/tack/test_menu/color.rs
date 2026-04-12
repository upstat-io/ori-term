//! Test wrappers for the tack color scenario at three sizes.
//!
//! Const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::color`.
//! This file defines `#[test] fn` wrappers that invoke
//! `ScenarioRunner::run_at` against the const at 80x24,
//! 97x33, and 120x40 (the canonical size matrix from the
//! existing `vttest_menu1` pattern).
//!
//! # Empirical reality (tack v1.08)
//!
//! Tack v1.08's color test does NOT emit any of the 8 ANSI
//! named colors. The captured output is only:
//!
//! ```text
//! \x1B[H\x1B[2JThis terminal can display 256 colors and 32767 color pairs.  (colors) (pairs)
//! (colors) (pairs) Done
//! ```
//!
//! See `crates/oriterm_test_support/src/tack_framework/scenarios/color/mod.rs`
//! rustdoc for the full empirical evidence and the hybrid coverage
//! strategy: this file asserts the grid contains `Done` plus the
//! testable semantic facts (`This terminal can display`,
//! `(colors)`, `(pairs)`) and snapshots the captured grid for
//! visual regression. The parser's `capability_labels` field is
//! always empty against tack v1.08 so the wrapper does NOT assert
//! on it — same hybrid pattern as 05.2's ACS / graphic-rendition
//! wrappers pinning `(bel)`.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::color::TACK_COLOR;

fn run_color_at(cols: u16, rows: u16) {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic unavailable, skipping tack_color_{cols}x{rows}");
        return;
    }
    let outcome = ScenarioRunner::run_at(&TACK_COLOR, cols, rows);

    // End-to-end pin: tack ran the standard color test and
    // reported its terminator. Catches regressions in spawn /
    // navigate / capture / quit pipeline at this size.
    assert!(
        outcome.grid_text.contains("Done"),
        "expected captured grid at {cols}x{rows} to contain 'Done' terminator, got:\n{}",
        outcome.grid_text
    );

    // SEMANTIC PINS: tack v1.08's color test only surfaces the
    // (colors) and (pairs) cap names plus a single descriptive
    // header line. These three assertions are the canonical
    // semantic claims for 05.3 cap-coverage:
    //
    // 1. "This terminal can display" — proves tack entered the
    //    color test code path (the test description header).
    // 2. "(colors)" — proves tack referenced the colors cap by
    //    its terminfo short name (the canonical tack output
    //    format — matches the (am)/(os)/(bel) pattern from
    //    prior screens). This is the cap-coverage pin for
    //    `colors` in 05.5.
    // 3. "(pairs)" — same for the `pairs` cap.
    //
    // Per the empirical-finding block in 05.3, only `colors`
    // and `pairs` are honestly covered by 05.3. Coverage for
    // `setaf`/`setab`/`op` and the named-color caps must come
    // from Section 07's GPU goldens or vttest.
    assert!(
        outcome.grid_text.contains("This terminal can display"),
        "expected captured grid at {cols}x{rows} to contain 'This terminal can display' header, got:\n{}",
        outcome.grid_text
    );
    assert!(
        outcome.grid_text.contains("(colors)"),
        "expected captured grid at {cols}x{rows} to contain '(colors)' parenthesized cap, got:\n{}",
        outcome.grid_text
    );
    assert!(
        outcome.grid_text.contains("(pairs)"),
        "expected captured grid at {cols}x{rows} to contain '(pairs)' parenthesized cap, got:\n{}",
        outcome.grid_text
    );

    // Insta snapshot of the full grid for visual regression.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
fn tack_color_80x24() {
    run_color_at(80, 24);
}

#[test]
fn tack_color_97x33() {
    run_color_at(97, 33);
}

#[test]
fn tack_color_120x40() {
    run_color_at(120, 40);
}
