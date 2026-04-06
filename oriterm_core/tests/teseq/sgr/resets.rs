//! 05.5a Selective reset and 05.5b full reset scenarios.

use oriterm_core::cell::CellFlags;
use oriterm_core::color::{Palette, Rgb};
use vte::ansi::{Color, NamedColor};

use crate::harness::{
    assert_cell_flags_contain, assert_cell_flags_not_contain, cell_bg_at, cell_fg_at,
    cell_underline_color_at,
};

use super::run_scenario;

#[test]
fn reset_21_cancel_bold() {
    let Some(outcome) = run_scenario("reset_21_cancel_bold") else {
        return;
    };
    // "BI" at col 0: BOLD+ITALIC.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::ITALIC);
    // "I" at col 2: ITALIC only, BOLD cancelled by SGR 21.
    assert_cell_flags_contain(&outcome, 0, 2, CellFlags::ITALIC);
    assert_cell_flags_not_contain(&outcome, 0, 2, CellFlags::BOLD);
}

#[test]
fn reset_22_cancel_bold_dim() {
    let Some(outcome) = run_scenario("reset_22_cancel_bold_dim") else {
        return;
    };
    // "BD" at col 0: BOLD+DIM.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::DIM);
    // "Neither" at col 2: neither BOLD nor DIM.
    assert_cell_flags_not_contain(&outcome, 0, 2, CellFlags::BOLD | CellFlags::DIM);
}

#[test]
fn reset_23_cancel_italic() {
    let Some(outcome) = run_scenario("reset_23_cancel_italic") else {
        return;
    };
    // "IB" at col 0: ITALIC+BOLD.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::ITALIC | CellFlags::BOLD);
    // "B" at col 2: BOLD only, ITALIC cancelled.
    assert_cell_flags_contain(&outcome, 0, 2, CellFlags::BOLD);
    assert_cell_flags_not_contain(&outcome, 0, 2, CellFlags::ITALIC);
}

#[test]
fn reset_24_cancel_underline() {
    let Some(outcome) = run_scenario("reset_24_cancel_underline") else {
        return;
    };
    // "Curly" at col 0: CURLY_UNDERLINE.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::CURLY_UNDERLINE);
    // "None" at col 5: ALL_UNDERLINES cleared.
    assert_cell_flags_not_contain(&outcome, 0, 5, CellFlags::ALL_UNDERLINES);
}

#[test]
fn reset_25_cancel_blink() {
    let Some(outcome) = run_scenario("reset_25_cancel_blink") else {
        return;
    };
    // Blink + bold at start, then SGR 25 cancels blink at col 9, bold remains.
    assert_cell_flags_not_contain(&outcome, 0, 9, CellFlags::BLINK);
    assert_cell_flags_contain(&outcome, 0, 9, CellFlags::BOLD);
}

#[test]
fn reset_27_cancel_inverse() {
    let Some(outcome) = run_scenario("reset_27_cancel_inverse") else {
        return;
    };
    // Inverse + italic at start, then SGR 27 cancels inverse at col 9, italic remains.
    assert_cell_flags_not_contain(&outcome, 0, 9, CellFlags::INVERSE);
    assert_cell_flags_contain(&outcome, 0, 9, CellFlags::ITALIC);
}

#[test]
fn reset_28_cancel_hidden() {
    let Some(outcome) = run_scenario("reset_28_cancel_hidden") else {
        return;
    };
    // Hidden + bold at start, then SGR 28 cancels hidden at col 7, bold remains.
    assert_cell_flags_not_contain(&outcome, 0, 7, CellFlags::HIDDEN);
    assert_cell_flags_contain(&outcome, 0, 7, CellFlags::BOLD);
}

#[test]
fn reset_29_cancel_strike() {
    let Some(outcome) = run_scenario("reset_29_cancel_strike") else {
        return;
    };
    // Strikethrough + italic at start, then SGR 29 cancels strike at col 12, italic remains.
    assert_cell_flags_not_contain(&outcome, 0, 12, CellFlags::STRIKETHROUGH);
    assert_cell_flags_contain(&outcome, 0, 12, CellFlags::ITALIC);
}

#[test]
fn reset_selective_preserves_others() {
    let Some(outcome) = run_scenario("reset_selective_preserves_others") else {
        return;
    };
    // Lines 0-6: progressive removal of all 6 flags.
    // Line 0: all 6 flags set.
    assert_cell_flags_contain(
        &outcome,
        0,
        0,
        CellFlags::BOLD
            | CellFlags::ITALIC
            | CellFlags::UNDERLINE
            | CellFlags::BLINK
            | CellFlags::INVERSE
            | CellFlags::STRIKETHROUGH,
    );
    // Line 1: bold removed (SGR 22 removes bold+dim).
    assert_cell_flags_not_contain(&outcome, 1, 0, CellFlags::BOLD);
    assert_cell_flags_contain(
        &outcome,
        1,
        0,
        CellFlags::ITALIC
            | CellFlags::UNDERLINE
            | CellFlags::BLINK
            | CellFlags::INVERSE
            | CellFlags::STRIKETHROUGH,
    );
    // Line 2: italic removed.
    assert_cell_flags_not_contain(&outcome, 2, 0, CellFlags::BOLD | CellFlags::ITALIC);
    assert_cell_flags_contain(
        &outcome,
        2,
        0,
        CellFlags::UNDERLINE | CellFlags::BLINK | CellFlags::INVERSE | CellFlags::STRIKETHROUGH,
    );
    // Line 3: underline removed.
    assert_cell_flags_not_contain(&outcome, 3, 0, CellFlags::ALL_UNDERLINES);
    assert_cell_flags_contain(
        &outcome,
        3,
        0,
        CellFlags::BLINK | CellFlags::INVERSE | CellFlags::STRIKETHROUGH,
    );
    // Line 4: blink removed.
    assert_cell_flags_not_contain(&outcome, 4, 0, CellFlags::BLINK);
    assert_cell_flags_contain(
        &outcome,
        4,
        0,
        CellFlags::INVERSE | CellFlags::STRIKETHROUGH,
    );
    // Line 5: inverse removed.
    assert_cell_flags_not_contain(&outcome, 5, 0, CellFlags::INVERSE);
    assert_cell_flags_contain(&outcome, 5, 0, CellFlags::STRIKETHROUGH);
    // Line 6: strikethrough removed — no SGR flags remain.
    let all = CellFlags::BOLD
        | CellFlags::ITALIC
        | CellFlags::UNDERLINE
        | CellFlags::BLINK
        | CellFlags::INVERSE
        | CellFlags::STRIKETHROUGH;
    assert_cell_flags_not_contain(&outcome, 6, 0, all);
}

#[test]
fn reset_sgr0() {
    let Some(outcome) = run_scenario("reset_sgr0") else {
        return;
    };
    let palette = Palette::default();
    // "All on" at col 0: BOLD + ITALIC + UNDERLINE + bright red fg (idx 9) + green bg (idx 2).
    assert_cell_flags_contain(
        &outcome,
        0,
        0,
        CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE,
    );
    let bright_red = palette.resolve(Color::Indexed(9));
    assert_eq!(cell_fg_at(&outcome, 0, 0), bright_red);
    let green_bg = palette.resolve(Color::Indexed(2));
    assert_eq!(cell_bg_at(&outcome, 0, 0), green_bg);

    // "Clean" at col 6: no SGR flags, default colors restored.
    assert_cell_flags_not_contain(
        &outcome,
        0,
        6,
        CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE,
    );
    let default_fg = palette.resolve(Color::Named(NamedColor::Foreground));
    let default_bg = palette.resolve(Color::Named(NamedColor::Background));
    assert_eq!(cell_fg_at(&outcome, 0, 6), default_fg);
    assert_eq!(cell_bg_at(&outcome, 0, 6), default_bg);
}

#[test]
fn reset_39_default_fg() {
    let Some(outcome) = run_scenario("reset_39_default_fg") else {
        return;
    };
    let palette = Palette::default();
    // "Red bold" at col 0: BOLD + bright red (idx 9).
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
    let bright_red = palette.resolve(Color::Indexed(9));
    assert_eq!(cell_fg_at(&outcome, 0, 0), bright_red);

    // "Default fg bold" at col 8: BOLD + default fg.
    assert_cell_flags_contain(&outcome, 0, 8, CellFlags::BOLD);
    let default_fg = palette.resolve(Color::Named(NamedColor::Foreground));
    assert_eq!(cell_fg_at(&outcome, 0, 8), default_fg);
}

#[test]
fn reset_49_default_bg() {
    let Some(outcome) = run_scenario("reset_49_default_bg") else {
        return;
    };
    let palette = Palette::default();
    // "Green bg italic" at col 0: ITALIC + green bg (idx 2).
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::ITALIC);
    let green_bg = palette.resolve(Color::Indexed(2));
    assert_eq!(cell_bg_at(&outcome, 0, 0), green_bg);

    // "Default bg italic" at col 15: ITALIC + default bg.
    assert_cell_flags_contain(&outcome, 0, 15, CellFlags::ITALIC);
    let default_bg = palette.resolve(Color::Named(NamedColor::Background));
    assert_eq!(cell_bg_at(&outcome, 0, 15), default_bg);
}

#[test]
fn reset_59_underline_color() {
    let Some(outcome) = run_scenario("reset_59_underline_color") else {
        return;
    };
    // "Colored" at col 0: UNDERLINE + underline_color Some(Rgb{200,100,50}).
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_eq!(
        cell_underline_color_at(&outcome, 0, 0),
        Some(Rgb {
            r: 200,
            g: 100,
            b: 50
        })
    );

    // "Default" at col 7: UNDERLINE + underline_color None.
    assert_cell_flags_contain(&outcome, 0, 7, CellFlags::UNDERLINE);
    assert_eq!(cell_underline_color_at(&outcome, 0, 7), None);
}
