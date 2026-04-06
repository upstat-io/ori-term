//! 05.3 16-color and 05.4 extended color scenarios.

use std::path::Path;

use oriterm_core::cell::CellFlags;
use oriterm_core::color::{Palette, Rgb};
use vte::ansi::Color;

use crate::harness::{
    TeseqHarness, assert_cell_flags_contain, assert_spec, cell_bg_at, cell_fg_at, reseq_available,
};

use super::run_scenario;

#[test]
fn color_16_fg() {
    let Some(outcome) = run_scenario("color_16_fg") else {
        return;
    };
    let palette = Palette::default();

    // Base colors (SGR 30-37): 3 chars each at cols 0,3,6,9,12,15,18,21.
    for (i, col) in (0..8).zip((0..).step_by(3)) {
        let expected = palette.resolve(Color::Indexed(i));
        assert_eq!(
            cell_fg_at(&outcome, 0, col),
            expected,
            "base fg color index {i} at col {col}"
        );
    }

    // Bright colors (SGR 90-97): 4 chars each at cols 24,28,32,36,40,44,48,52.
    for (i, col) in (8..16).zip((24..).step_by(4)) {
        let expected = palette.resolve(Color::Indexed(i));
        assert_eq!(
            cell_fg_at(&outcome, 0, col),
            expected,
            "bright fg color index {i} at col {col}"
        );
    }
}

#[test]
fn color_16_bg() {
    let Some(outcome) = run_scenario("color_16_bg") else {
        return;
    };
    let palette = Palette::default();

    // Base bg colors (SGR 40-47): 3 chars each at cols 0,3,6,9,12,15,18,21.
    for (i, col) in (0..8).zip((0..).step_by(3)) {
        let expected = palette.resolve(Color::Indexed(i));
        assert_eq!(
            cell_bg_at(&outcome, 0, col),
            expected,
            "base bg color index {i} at col {col}"
        );
    }

    // Bright bg colors (SGR 100-107): 4 chars each at cols 24,28,32,36,40,44,48,52.
    for (i, col) in (8..16).zip((24..).step_by(4)) {
        let expected = palette.resolve(Color::Indexed(i));
        assert_eq!(
            cell_bg_at(&outcome, 0, col),
            expected,
            "bright bg color index {i} at col {col}"
        );
    }
}

#[test]
fn color_bold_bright() {
    let Some(outcome) = run_scenario("color_bold_bright") else {
        return;
    };
    let palette = Palette::default();
    // Bold + red (index 1) promotes to bright red (index 9).
    let bright_red = palette.resolve(Color::Indexed(9));
    assert_eq!(cell_fg_at(&outcome, 0, 0), bright_red);
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
}

#[test]
fn color_bold_no_promote_above_7() {
    let Some(outcome) = run_scenario("color_bold_no_promote_above_7") else {
        return;
    };
    let palette = Palette::default();
    // Bold + indexed(100) stays indexed(100), not promoted to 108.
    let expected = palette.resolve(Color::Indexed(100));
    assert_eq!(cell_fg_at(&outcome, 0, 0), expected);
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
}

#[test]
fn color_bold_bright_disabled() {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/sgr")
        .join("color_bold_bright.teseq");
    let mut h = TeseqHarness::from_scenario(&path);
    h.set_bold_is_bright(false);
    let outcome = h.run(&path);
    assert_spec(&outcome, h.spec(), "sgr_color_bold_bright_disabled");

    let palette = Palette::default();
    // With bold-is-bright disabled, bold + red stays normal red (index 1).
    let normal_red = palette.resolve(Color::Indexed(1));
    assert_eq!(cell_fg_at(&outcome, 0, 0), normal_red);
}

#[test]
fn color_256_fg() {
    let Some(outcome) = run_scenario("color_256_fg") else {
        return;
    };
    let palette = Palette::default();
    // "Red 256" at col 0 = index 196.
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(196))
    );
    // "Green 256" at col 7 = index 46.
    assert_eq!(
        cell_fg_at(&outcome, 0, 7),
        palette.resolve(Color::Indexed(46))
    );
    // "Blue 256" at col 16 = index 21.
    assert_eq!(
        cell_fg_at(&outcome, 0, 16),
        palette.resolve(Color::Indexed(21))
    );
}

#[test]
fn color_256_bg() {
    let Some(outcome) = run_scenario("color_256_bg") else {
        return;
    };
    let palette = Palette::default();
    // "Red bg" at col 0 = index 196.
    assert_eq!(
        cell_bg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(196))
    );
    // "Green bg" at col 6 = index 46.
    assert_eq!(
        cell_bg_at(&outcome, 0, 6),
        palette.resolve(Color::Indexed(46))
    );
    // "Blue bg" at col 14 = index 21.
    assert_eq!(
        cell_bg_at(&outcome, 0, 14),
        palette.resolve(Color::Indexed(21))
    );
}

#[test]
fn color_rgb_fg() {
    let Some(outcome) = run_scenario("color_rgb_fg") else {
        return;
    };
    // "Orange" at col 0 = Rgb(255, 128, 0).
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        Rgb {
            r: 255,
            g: 128,
            b: 0
        }
    );
    // "Spring" at col 6 = Rgb(0, 255, 128).
    assert_eq!(
        cell_fg_at(&outcome, 0, 6),
        Rgb {
            r: 0,
            g: 255,
            b: 128
        }
    );
}

#[test]
fn color_rgb_bg() {
    let Some(outcome) = run_scenario("color_rgb_bg") else {
        return;
    };
    // "Orange bg" at col 0 = Rgb(255, 128, 0).
    assert_eq!(
        cell_bg_at(&outcome, 0, 0),
        Rgb {
            r: 255,
            g: 128,
            b: 0
        }
    );
    // "Spring bg" at col 9 = Rgb(0, 255, 128).
    assert_eq!(
        cell_bg_at(&outcome, 0, 9),
        Rgb {
            r: 0,
            g: 255,
            b: 128
        }
    );
}
