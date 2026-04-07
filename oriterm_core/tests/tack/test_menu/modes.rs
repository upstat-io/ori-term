//! Test wrappers for the modes scenarios.
//!
//! Const `ScenarioSpec`s and parsers live in
//! `oriterm_test_support::tack_framework::scenarios::modes`. This
//! file just defines `#[test] fn` wrappers that invoke
//! `ScenarioRunner` against those consts.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::modes::TACK_MODES_AM;

#[test]
fn tack_modes_am() {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic not installed, skipping tack_modes_am");
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_MODES_AM);

    // Programmatic semantic assertion: the parser found `os`
    // (over-strike) in the modes screen capability list. Tack lists
    // `os` last as the test terminator, so it's always visible in
    // the 24-row viewport at the moment the test reports "Done"
    // (earlier caps like `am`, `bce` scrolled off — Section 05
    // adds per-cap scenarios that capture the right viewport for
    // each). Uses the tokenized `grid_has_paren_token` helper
    // indirectly via `parse_modes_screen` — tack tags every modes
    // result with `(cap_name)` and `grid_has_paren_token` matches
    // exactly that form, so substring collisions cannot false-pass.
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "os"),
        "expected `os` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );

    // Insta snapshot of the full grid for visual regression catching.
    // Use the size-aware snapshot name so size-matrix runs in
    // Section 05 share the snapshot file when the screen is the same.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
