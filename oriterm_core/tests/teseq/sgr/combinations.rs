//! 05.7 SGR combination and stacking scenarios.

use oriterm_core::cell::CellFlags;
use oriterm_core::color::Palette;
use vte::ansi::{Color, NamedColor};

use crate::harness::{assert_cell_flags_contain, assert_cell_flags_not_contain, cell_fg_at};

use super::run_scenario;

#[test]
fn combo_stack() {
    let Some(outcome) = run_scenario("combo_stack") else {
        return;
    };
    let palette = Palette::default();
    // "Bold italic underline red" at col 0: BOLD + ITALIC + UNDERLINE + bright red (idx 9).
    assert_cell_flags_contain(
        &outcome,
        0,
        0,
        CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE,
    );
    let bright_red = palette.resolve(Color::Indexed(9));
    assert_eq!(cell_fg_at(&outcome, 0, 0), bright_red);

    // "Normal" at col 25: no SGR flags.
    assert_cell_flags_not_contain(
        &outcome,
        0,
        25,
        CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE,
    );
}

#[test]
fn combo_separate_sequences() {
    let Some(outcome) = run_scenario("combo_separate_sequences") else {
        return;
    };
    // "Stacked" at col 0: BOLD + ITALIC + UNDERLINE (applied via separate CSI sequences).
    assert_cell_flags_contain(
        &outcome,
        0,
        0,
        CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE,
    );
}

#[test]
fn combo_color_last_wins() {
    let Some(outcome) = run_scenario("combo_color_last_wins") else {
        return;
    };
    let palette = Palette::default();
    // "Yellow wins" at col 0: SGR 33 (yellow) wins over prior colors.
    let yellow = palette.resolve(Color::Indexed(3));
    assert_eq!(cell_fg_at(&outcome, 0, 0), yellow);
}

#[test]
fn combo_dim_then_bold() {
    let Some(outcome) = run_scenario("combo_dim_then_bold") else {
        return;
    };
    let palette = Palette::default();
    // "DimBold" at col 0: BOLD + DIM + DimRed fg (Named::DimRed).
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::DIM);
    let dim_red = palette.resolve(Color::Named(NamedColor::DimRed));
    assert_eq!(cell_fg_at(&outcome, 0, 0), dim_red);
}

#[test]
fn combo_empty_sgr_resets() {
    let Some(outcome) = run_scenario("combo_empty_sgr_resets") else {
        return;
    };
    // "Styled" at col 0: BOLD + ITALIC.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::ITALIC);
    // "Plain" at col 6: neither BOLD nor ITALIC (empty CSI m = SGR 0).
    assert_cell_flags_not_contain(&outcome, 0, 6, CellFlags::BOLD | CellFlags::ITALIC);
}

#[test]
fn combo_sgr_persists_cursor_move() {
    let Some(outcome) = run_scenario("combo_sgr_persists_cursor_move") else {
        return;
    };
    let palette = Palette::default();
    let bright_red = palette.resolve(Color::Indexed(9));

    // 'A' at col 0: BOLD + bright red (idx 9).
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
    assert_eq!(cell_fg_at(&outcome, 0, 0), bright_red);

    // 'B' at col 4: same attributes (CHA 5 = 0-based col 4, SGR persists).
    assert_cell_flags_contain(&outcome, 0, 4, CellFlags::BOLD);
    assert_eq!(cell_fg_at(&outcome, 0, 4), bright_red);
}
