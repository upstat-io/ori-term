//! 05.6 SGR edge case scenarios (dim+bold, inverse, DECSCNM).

use oriterm_core::cell::CellFlags;
use oriterm_core::color::{Palette, Rgb};
use vte::ansi::{Color, NamedColor};

use crate::harness::{assert_cell_flags_contain, cell_bg_at, cell_fg_at};

use super::run_scenario;

#[test]
fn edge_dim_bold_named() {
    let Some(outcome) = run_scenario("edge_dim_bold_named") else {
        return;
    };
    let palette = Palette::default();
    // DIM + BOLD + red = DimRed (Named::DimRed from palette). Both flags set.
    let dim_red = palette.resolve(Color::Named(NamedColor::DimRed));
    assert_eq!(cell_fg_at(&outcome, 0, 0), dim_red);
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::DIM);
}

#[test]
fn edge_dim_bold_indexed() {
    let Some(outcome) = run_scenario("edge_dim_bold_indexed") else {
        return;
    };
    let palette = Palette::default();
    // DIM + BOLD + indexed(2) = dimmed green (2/3 of each channel).
    let green = palette.resolve(Color::Indexed(2));
    let dimmed = Rgb {
        r: (green.r as u16 * 2 / 3) as u8,
        g: (green.g as u16 * 2 / 3) as u8,
        b: (green.b as u16 * 2 / 3) as u8,
    };
    assert_eq!(cell_fg_at(&outcome, 0, 0), dimmed);
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::DIM);
}

#[test]
fn edge_dim_bold_truecolor() {
    let Some(outcome) = run_scenario("edge_dim_bold_truecolor") else {
        return;
    };
    // DIM on Rgb{150,120,90} = Rgb{100,80,60}.
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        Rgb {
            r: 100,
            g: 80,
            b: 60
        }
    );
}

#[test]
fn edge_inverse_colors() {
    let Some(outcome) = run_scenario("edge_inverse_colors") else {
        return;
    };
    let palette = Palette::default();
    let red = palette.resolve(Color::Indexed(1));
    let green = palette.resolve(Color::Indexed(2));

    // "Red on green" at col 0: fg=red, bg=green.
    assert_eq!(cell_fg_at(&outcome, 0, 0), red);
    assert_eq!(cell_bg_at(&outcome, 0, 0), green);

    // "Inverted" at col 12: fg=green, bg=red.
    assert_eq!(cell_fg_at(&outcome, 0, 12), green);
    assert_eq!(cell_bg_at(&outcome, 0, 12), red);
}

#[test]
fn edge_decscnm_basic() {
    let Some(outcome) = run_scenario("edge_decscnm_basic") else {
        return;
    };
    let palette = Palette::default();
    // pre_feed enables mode 5. Default cells: fg=original_bg, bg=original_fg.
    let original_fg = palette.resolve(Color::Named(NamedColor::Foreground));
    let original_bg = palette.resolve(Color::Named(NamedColor::Background));
    assert_eq!(cell_fg_at(&outcome, 0, 0), original_bg);
    assert_eq!(cell_bg_at(&outcome, 0, 0), original_fg);
}

#[test]
fn edge_decscnm_inverse() {
    let Some(outcome) = run_scenario("edge_decscnm_inverse") else {
        return;
    };
    let palette = Palette::default();
    // pre_feed mode 5 + SGR 7. Double swap = normal appearance.
    let original_fg = palette.resolve(Color::Named(NamedColor::Foreground));
    let original_bg = palette.resolve(Color::Named(NamedColor::Background));
    assert_eq!(cell_fg_at(&outcome, 0, 0), original_fg);
    assert_eq!(cell_bg_at(&outcome, 0, 0), original_bg);
}

#[test]
fn edge_decscnm_explicit_color() {
    let Some(outcome) = run_scenario("edge_decscnm_explicit_color") else {
        return;
    };
    // pre_feed mode 5 + explicit TrueColor. fg=Rgb{100,200,50} unaffected by palette swap.
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        Rgb {
            r: 100,
            g: 200,
            b: 50
        }
    );
}
