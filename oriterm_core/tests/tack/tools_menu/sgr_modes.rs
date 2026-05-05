//! Section 06.2 test wrapper for tack's `g) ANSI SGR modes` tool
//! screen.
//!
//! The const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::sgr_modes`. This
//! file is a thin wrapper that invokes `ScenarioRunner::run(..)` and
//! pins the captured grid via insta.
//!
//! See the `sgr_modes/mod.rs` module rustdoc for the full empirical
//! record (menu path derivation, the skipped `?` private-use variant,
//! and the `\r`-step plan deviation).

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::sgr_modes::{
    MIN_EXPECTED_MODES, TACK_TOOLS_SGR,
};

#[test]
fn tack_tools_sgr_80x24() {
    if !ScenarioRunner::available() {
        eprintln!(
            "tack, tic, or a supported tack version unavailable, skipping tack_tools_sgr_80x24"
        );
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_TOOLS_SGR);

    // End-to-end pin: the spawn → navigate → capture pipeline landed
    // us on a grid containing the last mode label. A regression that
    // broke the `t -> g` navigation (or the post-`g` clear+draw race
    // handling in the navigator) would flip this first.
    assert!(
        outcome.grid_text.contains("Mode 79"),
        "expected captured grid to contain 'Mode 79' (last label on \
         tack's SGR table); grid:\n{}",
        outcome.grid_text
    );

    // Verifies: the parser found the full 80-mode grid. Uses the
    // canonical `MIN_EXPECTED_MODES` constant so a regression that
    // lowers the threshold below 80 in `sgr_modes/mod.rs` flips both
    // this and the sibling parser tests simultaneously.
    let count_note = outcome
        .parsed
        .notes
        .iter()
        .find(|n| n.starts_with("found_modes_count="))
        .expect("parse_sgr_modes_screen must record a found_modes_count note");
    let count: usize = count_note
        .trim_start_matches("found_modes_count=")
        .parse()
        .expect("found_modes_count note must carry an integer count");
    assert!(
        count >= MIN_EXPECTED_MODES,
        "expected >= {MIN_EXPECTED_MODES} SGR mode labels on tack's \
         tools/sgr screen, got {count}.\nGrid:\n{}",
        outcome.grid_text
    );

    // Insta snapshot of the full grid for visual regression.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
