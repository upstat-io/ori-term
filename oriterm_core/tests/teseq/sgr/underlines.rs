//! 05.2 Underline style and color scenarios.

use oriterm_core::cell::CellFlags;
use oriterm_core::color::{Palette, Rgb};
use vte::ansi::Color;

use crate::harness::{
    assert_cell_flags_contain, assert_cell_flags_not_contain, cell_underline_color_at,
};

use super::run_scenario;

#[test]
fn underline_single() {
    let Some(outcome) = run_scenario("underline_single") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_cell_flags_not_contain(
        &outcome,
        0,
        0,
        CellFlags::DOUBLE_UNDERLINE
            | CellFlags::CURLY_UNDERLINE
            | CellFlags::DOTTED_UNDERLINE
            | CellFlags::DASHED_UNDERLINE,
    );
}

#[test]
fn underline_double() {
    let Some(outcome) = run_scenario("underline_double") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::DOUBLE_UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
}

#[test]
fn underline_curly() {
    let Some(outcome) = run_scenario("underline_curly") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::CURLY_UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
}

#[test]
fn underline_dotted() {
    let Some(outcome) = run_scenario("underline_dotted") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::DOTTED_UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
}

#[test]
fn underline_dashed() {
    let Some(outcome) = run_scenario("underline_dashed") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::DASHED_UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
}

#[test]
fn underline_mutual_exclusion() {
    let Some(outcome) = run_scenario("underline_mutual_exclusion") else {
        return;
    };
    // 'S' at col 0 = single underline only.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_cell_flags_not_contain(
        &outcome,
        0,
        0,
        CellFlags::CURLY_UNDERLINE | CellFlags::DOUBLE_UNDERLINE,
    );
    // 'C' at col 1 = curly only.
    assert_cell_flags_contain(&outcome, 0, 1, CellFlags::CURLY_UNDERLINE);
    assert_cell_flags_not_contain(
        &outcome,
        0,
        1,
        CellFlags::UNDERLINE | CellFlags::DOUBLE_UNDERLINE,
    );
    // 'D' at col 2 = double only.
    assert_cell_flags_contain(&outcome, 0, 2, CellFlags::DOUBLE_UNDERLINE);
    assert_cell_flags_not_contain(
        &outcome,
        0,
        2,
        CellFlags::UNDERLINE | CellFlags::CURLY_UNDERLINE,
    );
}

#[test]
fn underline_color_truecolor() {
    let Some(outcome) = run_scenario("underline_color_truecolor") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_eq!(
        cell_underline_color_at(&outcome, 0, 0),
        Some(Rgb {
            r: 255,
            g: 0,
            b: 128
        })
    );
}

#[test]
fn underline_color_256() {
    let Some(outcome) = run_scenario("underline_color_256") else {
        return;
    };
    let palette = Palette::default();
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_eq!(
        cell_underline_color_at(&outcome, 0, 0),
        Some(palette.resolve(Color::Indexed(196)))
    );
}

#[test]
fn underline_color_reset() {
    let Some(outcome) = run_scenario("underline_color_reset") else {
        return;
    };
    // Col 0: custom underline color set.
    assert_eq!(
        cell_underline_color_at(&outcome, 0, 0),
        Some(Rgb { r: 255, g: 0, b: 0 })
    );
    // Col 6 ("Default UL"): underline color reset to None.
    assert_eq!(cell_underline_color_at(&outcome, 0, 6), None);
}

#[test]
fn underline_cancel_subparam() {
    let Some(outcome) = run_scenario("underline_cancel_subparam") else {
        return;
    };
    // Col 0: curly underline active.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::CURLY_UNDERLINE);
    // Col 5: all underlines cancelled.
    assert_cell_flags_not_contain(&outcome, 0, 5, CellFlags::ALL_UNDERLINES);
}

#[test]
fn underline_color_survives_style_change() {
    let Some(outcome) = run_scenario("underline_color_survives_style_change") else {
        return;
    };
    let green = Rgb { r: 0, g: 255, b: 0 };
    // Col 0: single underline with green underline color.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_eq!(cell_underline_color_at(&outcome, 0, 0), Some(green));
    // Col 1: curly underline with green underline color preserved.
    assert_cell_flags_contain(&outcome, 0, 1, CellFlags::CURLY_UNDERLINE);
    assert_eq!(cell_underline_color_at(&outcome, 0, 1), Some(green));
}
