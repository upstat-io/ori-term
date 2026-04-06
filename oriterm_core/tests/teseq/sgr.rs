//! SGR & color scenarios (attributes, underline styles, colors, selective resets).

use std::path::Path;

use oriterm_core::cell::CellFlags;

use super::harness::{
    self, ScenarioOutcome, TeseqHarness, assert_cell_flags_contain, assert_cell_flags_not_contain,
    reseq_available,
};

/// Run an SGR scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform cell attribute assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/sgr")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("sgr_{name}"));
    Some(outcome)
}

// 05.1 Text attribute scenarios

#[test]
fn attr_bold() {
    let Some(outcome) = run_scenario("attr_bold") else {
        return;
    };
    // "Bold text" at line 0, col 0-8.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
    // "Normal" after reset, starts at col 9.
    assert_cell_flags_not_contain(&outcome, 0, 9, CellFlags::BOLD);
}

#[test]
fn attr_dim() {
    let Some(outcome) = run_scenario("attr_dim") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::DIM);
    assert_cell_flags_not_contain(&outcome, 0, 8, CellFlags::DIM);
}

#[test]
fn attr_italic() {
    let Some(outcome) = run_scenario("attr_italic") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::ITALIC);
    assert_cell_flags_not_contain(&outcome, 0, 11, CellFlags::ITALIC);
}

#[test]
fn attr_underline() {
    let Some(outcome) = run_scenario("attr_underline") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 14, CellFlags::UNDERLINE);
}

#[test]
fn attr_blink() {
    let Some(outcome) = run_scenario("attr_blink") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BLINK);
    assert_cell_flags_not_contain(&outcome, 0, 10, CellFlags::BLINK);
}

#[test]
fn attr_inverse() {
    let Some(outcome) = run_scenario("attr_inverse") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::INVERSE);
    assert_cell_flags_not_contain(&outcome, 0, 12, CellFlags::INVERSE);
}

#[test]
fn attr_hidden() {
    let Some(outcome) = run_scenario("attr_hidden") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::HIDDEN);
    assert_cell_flags_not_contain(&outcome, 0, 11, CellFlags::HIDDEN);
}

#[test]
fn attr_strikethrough() {
    let Some(outcome) = run_scenario("attr_strikethrough") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::STRIKETHROUGH);
    assert_cell_flags_not_contain(&outcome, 0, 11, CellFlags::STRIKETHROUGH);
}

#[test]
fn attr_blink_fast() {
    let Some(outcome) = run_scenario("attr_blink_fast") else {
        return;
    };
    // SGR 6 (BlinkFast) sets the same BLINK flag as SGR 5 (BlinkSlow).
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BLINK);
    assert_cell_flags_not_contain(&outcome, 0, 10, CellFlags::BLINK);
}
