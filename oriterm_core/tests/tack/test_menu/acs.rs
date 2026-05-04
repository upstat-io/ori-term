//! Test wrapper for the ACS / graphic-character-set scenario.
//!
//! Const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::acs`. This
//! file just defines a `#[test] fn` wrapper that invokes
//! `ScenarioRunner::run` against the const.
//!
//! # Empirical reality (tack v1.08)
//!
//! Tack v1.08's "alternate character set" test does NOT draw the
//! DEC line-drawing block (U+2500..=U+257F) that the spec parser
//! was designed for. The captured output is only:
//!
//! ```text
//! \x1B[H\x1B[2JTesting bell (bel)
//! If you did not hear the Bell then (bel) has failed. (bel) Done
//! ```
//!
//! See `crates/oriterm_test_support/src/tack_framework/scenarios/acs/mod.rs`
//! rustdoc for the full empirical evidence and the hybrid coverage
//! strategy: this test asserts the grid contains `Done` (proving
//! the test ran end-to-end) and snapshots the captured grid for
//! visual regression. The parser's
//! `distinct_line_drawing_chars=N` count is preserved as
//! forward-compatible infrastructure but not asserted because it
//! is always 0 against tack v1.08.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::acs::TACK_ACS_GRAPHIC_CHARS;

#[test]
fn tack_acs_graphic_chars() {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic unavailable, skipping tack_acs_graphic_chars");
        return;
    }
    let outcome = ScenarioRunner::run(&TACK_ACS_GRAPHIC_CHARS);

    // End-to-end pin: tack ran the standard ACS test and reported
    // its terminator. Catches regressions in spawn / navigate /
    // capture / quit pipeline (the run_phase_at_returns_grid_containing_anchor
    // analog at the stable-screen path).
    assert!(
        outcome.grid_text.contains("Done"),
        "expected captured grid to contain 'Done' terminator, got:\n{}",
        outcome.grid_text
    );

    // Verifies for assert the (bel) capability is
    // actually exercised. Tack v1.08's "alternate character set and
    // graphic rendition" test runs only the bell probe — it does
    // NOT emit DEC line-drawing chars or SGR sample text — so the
    // ONLY honest semantic claim this wrapper can make is that
    // (bel) was invoked. The "Testing bell" header pins that tack
    // entered the bell probe code path; the "(bel)" parenthesized
    // token pins that tack referenced the capability by its
    // terminfo short name (the canonical tack output format —
    // matches the (am)/(os) pattern from the modes test). A
    // regression that breaks the bell test or rerouted the menu
    // key would surface here even though the existing `Done` pin
    // would still pass.
    assert!(
        outcome.grid_text.contains("Testing bell"),
        "expected captured grid to contain 'Testing bell' header (bel cap pin), got:\n{}",
        outcome.grid_text
    );
    assert!(
        outcome.grid_text.contains("(bel)"),
        "expected captured grid to contain '(bel)' parenthesized cap (bel cap pin), got:\n{}",
        outcome.grid_text
    );

    // Insta snapshot of the full grid for visual regression.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
