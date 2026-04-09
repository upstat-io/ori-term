//! Section 06.3 test wrapper for tack's `c) ANSI character sets`
//! tool screen.
//!
//! The const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::character_sets`.
//! This file is a thin wrapper that invokes `ScenarioRunner::run(..)`
//! and pins the captured grid via insta.
//!
//! See the `character_sets/mod.rs` module rustdoc for the full
//! empirical record (DEC special graphics → Unicode box-drawing
//! translation, the combined `)0` send-step rationale, and the
//! `TACK_TOOLS_G0_DEC_GRAPHICS` const-name historical note).

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::character_sets::{
    MIN_DEC_GRAPHICS_THRESHOLD, TACK_TOOLS_G0_DEC_GRAPHICS,
};

#[test]
fn tack_tools_g0_dec_graphics_80x24() {
    if !ScenarioRunner::available() {
        eprintln!(
            "tack, tic, or a supported tack version unavailable, \
             skipping tack_tools_g0_dec_graphics_80x24"
        );
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_TOOLS_G0_DEC_GRAPHICS);

    // NEGATIVE PIN: empty grid means tack navigation failed before
    // the SCS render landed. The threshold comparison below would
    // pass on an empty grid (count == 0 < threshold == 4 → fail,
    // but the panic message would not name the navigation failure).
    // This explicit assertion fires first with a precise diagnostic.
    assert!(
        !outcome.grid_text.is_empty(),
        "TACK_TOOLS_G0_DEC_GRAPHICS returned empty grid — tack \
         navigation failed before render"
    );

    // SEMANTIC PIN: the parser found at least the minimum number of
    // distinct Unicode box-drawing chars. Uses the canonical
    // `MIN_DEC_GRAPHICS_THRESHOLD` constant so a regression that
    // lowers the threshold in `character_sets/mod.rs` flips both
    // this and the sibling parser tests simultaneously.
    let count_note = outcome
        .parsed
        .notes
        .iter()
        .find(|n| n.starts_with("dec_graphics_distinct_chars="))
        .expect("parse_character_sets_screen must record a dec_graphics_distinct_chars note");
    let count: usize = count_note
        .trim_start_matches("dec_graphics_distinct_chars=")
        .parse()
        .expect("dec_graphics_distinct_chars note must carry an integer count");
    assert!(
        count >= MIN_DEC_GRAPHICS_THRESHOLD,
        "expected >= {MIN_DEC_GRAPHICS_THRESHOLD} distinct \
         Unicode box-drawing chars on tack's tools/character_sets \
         screen, got {count}.\nGrid:\n{}",
        outcome.grid_text
    );

    // Insta snapshot of the full grid for visual regression — pins
    // the empirical screen layout produced by tack v1.08 + ori_term's
    // pinned terminfo.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
