//! Complex workflow scenarios (mode combinations, query-response, real-world patterns, edge cases).

use std::path::Path;

use oriterm_core::TermMode;

use super::harness::{
    self, ScenarioOutcome, TeseqHarness, assert_cell_flags_contain, assert_mode_contains,
    assert_mode_not_contains, assert_scrollback_empty, cell_fg_at, reseq_available,
};

/// Run a workflow scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform additional assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/workflows")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("workflows_{name}"));
    Some(outcome)
}

// Mode combination workflows

#[test]
fn mode_scroll_origin_fill() {
    let Some(outcome) = run_scenario("mode_scroll_origin_fill") else {
        return;
    };
    assert_mode_not_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn mode_alt_with_modes() {
    let Some(outcome) = run_scenario("mode_alt_with_modes") else {
        return;
    };
    // Origin mode should survive the alt screen roundtrip.
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    // Primary screen content should be restored.
    assert!(
        outcome.grid_text.contains("Primary with modes"),
        "primary screen content not restored after alt screen exit"
    );
}

#[test]
fn mode_deccolm_full_cycle() {
    let Some(outcome) = run_scenario("mode_deccolm_full_cycle") else {
        return;
    };
    // After DECCOLM 3l (back to 80 cols), grid should be 80 columns.
    assert_eq!(outcome.cols, 80, "expected 80 columns after DECCOLM off");
    // DECCOLM 3l clears the display and homes cursor.
    // "Back to 80" should be visible at row 0.
    assert!(
        outcome.grid_text.contains("Back to 80"),
        "content after DECCOLM off not found"
    );
    // Origin mode should have been preserved across DECCOLM transitions.
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn mode_decsc_attrs() {
    let Some(outcome) = run_scenario("mode_decsc_attrs") else {
        return;
    };
    // After DECRC, cursor should return to saved position (line 4, col 9).
    assert_eq!(outcome.cursor_line, 4, "DECRC should restore cursor line");
    assert_eq!(outcome.cursor_col, 24, "cursor col after 'q after restore'");
    // The 'q' at col 9 should render as a DEC Special Graphics character
    // if charset was properly saved/restored. Under DEC Special Graphics,
    // 'q' maps to U+2500 HORIZONTAL LINE (─).
    let ch = outcome.grid_chars[4][9];
    assert_eq!(
        ch, '\u{2500}',
        "expected DEC Special Graphics horizontal line at (4,9), got {ch:?} — \
         charset was not saved/restored by DECSC/DECRC"
    );
    // Bold flag should be restored on the text after DECRC.
    assert_cell_flags_contain(&outcome, 4, 9, oriterm_core::cell::CellFlags::BOLD);
    // Red foreground should be restored — resolved Rgb red component should be nonzero.
    let fg = cell_fg_at(&outcome, 4, 9);
    assert!(
        fg.r > 100,
        "expected red-ish fg after DECRC restore, got r={} g={} b={}",
        fg.r,
        fg.g,
        fg.b
    );
}

#[test]
fn mode_decsc_origin_flag() {
    let Some(outcome) = run_scenario("mode_decsc_origin_flag") else {
        return;
    };
    // "absolute" was written at row 0 (origin mode off, CUP 1;1).
    assert!(
        outcome.grid_text.starts_with("absolute"),
        "expected 'absolute' at row 0"
    );
    // After DECRC, origin mode should be restored (enabled).
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    // "relative" should appear within the scroll region (row 4, the region top).
    let line4 = &outcome.grid_text.lines().nth(4).unwrap_or("");
    assert!(
        line4.trim_end().starts_with("relative"),
        "expected 'relative' at scroll region top (row 4), got: {line4:?}"
    );
}

// Multi-size variants (97x33)

#[test]
fn mode_scroll_origin_fill_97x33() {
    let Some(outcome) = run_scenario("mode_scroll_origin_fill_97x33") else {
        return;
    };
    assert_mode_not_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn mode_deccolm_full_cycle_97x33() {
    let Some(outcome) = run_scenario("mode_deccolm_full_cycle_97x33") else {
        return;
    };
    // DECCOLM ?3l restores to the original terminal width (97).
    assert_eq!(outcome.cols, 97, "expected 97 columns after DECCOLM off");
    assert!(
        outcome.grid_text.contains("Back to 80"),
        "content after DECCOLM off not found"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn mode_alt_with_modes_97x33() {
    let Some(outcome) = run_scenario("mode_alt_with_modes_97x33") else {
        return;
    };
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert!(
        outcome.grid_text.contains("Primary with modes"),
        "primary screen content not restored after alt screen exit"
    );
}

#[test]
fn mode_decsc_attrs_97x33() {
    let Some(outcome) = run_scenario("mode_decsc_attrs_97x33") else {
        return;
    };
    // After DECRC, cursor should return to saved position (line 9, col 19).
    assert_eq!(outcome.cursor_line, 9, "DECRC should restore cursor line");
    assert_eq!(outcome.cursor_col, 34, "cursor col after 'q after restore'");
    // DEC Special Graphics 'q' -> U+2500 at the save position.
    let ch = outcome.grid_chars[9][19];
    assert_eq!(
        ch, '\u{2500}',
        "expected DEC Special Graphics horizontal line at (9,19), got {ch:?} — \
         charset was not saved/restored by DECSC/DECRC"
    );
    assert_cell_flags_contain(&outcome, 9, 19, oriterm_core::cell::CellFlags::BOLD);
    let fg = cell_fg_at(&outcome, 9, 19);
    assert!(
        fg.r > 100,
        "expected red-ish fg after DECRC restore, got r={} g={} b={}",
        fg.r,
        fg.g,
        fg.b
    );
}

#[test]
fn mode_decsc_origin_flag_97x33() {
    let Some(outcome) = run_scenario("mode_decsc_origin_flag_97x33") else {
        return;
    };
    assert!(
        outcome.grid_text.starts_with("absolute"),
        "expected 'absolute' at row 0"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    let line4 = &outcome.grid_text.lines().nth(4).unwrap_or("");
    assert!(
        line4.trim_end().starts_with("relative"),
        "expected 'relative' at scroll region top (row 4), got: {line4:?}"
    );
}

// Multi-size variants (120x40)

#[test]
fn mode_scroll_origin_fill_120x40() {
    let Some(outcome) = run_scenario("mode_scroll_origin_fill_120x40") else {
        return;
    };
    assert_mode_not_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn mode_deccolm_full_cycle_120x40() {
    let Some(outcome) = run_scenario("mode_deccolm_full_cycle_120x40") else {
        return;
    };
    // DECCOLM ?3l restores to the original terminal width (120).
    assert_eq!(outcome.cols, 120, "expected 120 columns after DECCOLM off");
    assert!(
        outcome.grid_text.contains("Back to 80"),
        "content after DECCOLM off not found"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn mode_alt_with_modes_120x40() {
    let Some(outcome) = run_scenario("mode_alt_with_modes_120x40") else {
        return;
    };
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert!(
        outcome.grid_text.contains("Primary with modes"),
        "primary screen content not restored after alt screen exit"
    );
}

#[test]
fn mode_decsc_attrs_120x40() {
    let Some(outcome) = run_scenario("mode_decsc_attrs_120x40") else {
        return;
    };
    // After DECRC, cursor should return to saved position (line 14, col 29).
    assert_eq!(outcome.cursor_line, 14, "DECRC should restore cursor line");
    assert_eq!(outcome.cursor_col, 44, "cursor col after 'q after restore'");
    // DEC Special Graphics 'q' -> U+2500 at the save position.
    let ch = outcome.grid_chars[14][29];
    assert_eq!(
        ch, '\u{2500}',
        "expected DEC Special Graphics horizontal line at (14,29), got {ch:?} — \
         charset was not saved/restored by DECSC/DECRC"
    );
    assert_cell_flags_contain(&outcome, 14, 29, oriterm_core::cell::CellFlags::BOLD);
    let fg = cell_fg_at(&outcome, 14, 29);
    assert!(
        fg.r > 100,
        "expected red-ish fg after DECRC restore, got r={} g={} b={}",
        fg.r,
        fg.g,
        fg.b
    );
}

#[test]
fn mode_decsc_origin_flag_120x40() {
    let Some(outcome) = run_scenario("mode_decsc_origin_flag_120x40") else {
        return;
    };
    assert!(
        outcome.grid_text.starts_with("absolute"),
        "expected 'absolute' at row 0"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    let line4 = &outcome.grid_text.lines().nth(4).unwrap_or("");
    assert!(
        line4.trim_end().starts_with("relative"),
        "expected 'relative' at scroll region top (row 4), got: {line4:?}"
    );
}
