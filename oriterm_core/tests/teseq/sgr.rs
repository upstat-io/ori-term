//! SGR & color scenarios (attributes, underline styles, colors, selective resets).

use std::path::Path;

use vte::ansi::Color;

use oriterm_core::cell::CellFlags;
use oriterm_core::color::{Palette, Rgb};

use super::harness::{
    self, ScenarioOutcome, TeseqHarness, assert_cell_flags_contain, assert_cell_flags_not_contain,
    cell_bg_at, cell_fg_at, cell_underline_color_at, reseq_available,
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

// 05.2 Underline style & color scenarios

#[test]
fn underline_single() {
    let Some(outcome) = run_scenario("underline_single") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    let others = CellFlags::DOUBLE_UNDERLINE
        | CellFlags::CURLY_UNDERLINE
        | CellFlags::DOTTED_UNDERLINE
        | CellFlags::DASHED_UNDERLINE;
    assert_cell_flags_not_contain(&outcome, 0, 0, others);
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
    // 'S' at col 0: UNDERLINE only.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 0, CellFlags::CURLY_UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 0, CellFlags::DOUBLE_UNDERLINE);
    // 'C' at col 1: CURLY_UNDERLINE only.
    assert_cell_flags_contain(&outcome, 0, 1, CellFlags::CURLY_UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 1, CellFlags::UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 1, CellFlags::DOUBLE_UNDERLINE);
    // 'D' at col 2: DOUBLE_UNDERLINE only.
    assert_cell_flags_contain(&outcome, 0, 2, CellFlags::DOUBLE_UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 2, CellFlags::UNDERLINE);
    assert_cell_flags_not_contain(&outcome, 0, 2, CellFlags::CURLY_UNDERLINE);
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
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    let expected = Palette::default().resolve(Color::Indexed(196));
    assert_eq!(cell_underline_color_at(&outcome, 0, 0), Some(expected));
}

#[test]
fn underline_color_reset() {
    let Some(outcome) = run_scenario("underline_color_reset") else {
        return;
    };
    // "Red UL" at line 0: underline color set.
    assert_eq!(
        cell_underline_color_at(&outcome, 0, 0),
        Some(Rgb { r: 255, g: 0, b: 0 })
    );
    // "Default UL" at line 0 after "Red UL" (col 6): underline color cleared.
    assert_eq!(cell_underline_color_at(&outcome, 0, 6), None);
}

#[test]
fn underline_cancel_subparam() {
    let Some(outcome) = run_scenario("underline_cancel_subparam") else {
        return;
    };
    // 'C' at col 0: CURLY_UNDERLINE set.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::CURLY_UNDERLINE);
    // 'N' at col 5 (after "Curly"): all underlines cleared by SGR 4:0.
    assert_cell_flags_not_contain(&outcome, 0, 5, CellFlags::ALL_UNDERLINES);
}

#[test]
fn underline_color_survives_style_change() {
    let Some(outcome) = run_scenario("underline_color_survives_style_change") else {
        return;
    };
    let green = Rgb { r: 0, g: 255, b: 0 };
    // 'A' at col 0: UNDERLINE + green underline color.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_eq!(cell_underline_color_at(&outcome, 0, 0), Some(green));
    // 'B' at col 1: CURLY_UNDERLINE + green underline color (survived style switch).
    assert_cell_flags_contain(&outcome, 0, 1, CellFlags::CURLY_UNDERLINE);
    assert_eq!(cell_underline_color_at(&outcome, 0, 1), Some(green));
}

// 05.3 16-color & bold-as-bright scenarios

#[test]
fn color_16_fg() {
    let Some(outcome) = run_scenario("color_16_fg") else {
        return;
    };
    let palette = Palette::default();
    // "blk" at col 0-2: SGR 30 = black (index 0).
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(0))
    );
    // "red" at col 3-5: SGR 31 = red (index 1).
    assert_eq!(
        cell_fg_at(&outcome, 0, 3),
        palette.resolve(Color::Indexed(1))
    );
    // "grn" at col 6-8: SGR 32 = green (index 2).
    assert_eq!(
        cell_fg_at(&outcome, 0, 6),
        palette.resolve(Color::Indexed(2))
    );
    // "Bblk" at col 24-27: SGR 90 = bright black (index 8).
    assert_eq!(
        cell_fg_at(&outcome, 0, 24),
        palette.resolve(Color::Indexed(8))
    );
    // "Bred" at col 28-31: SGR 91 = bright red (index 9).
    assert_eq!(
        cell_fg_at(&outcome, 0, 28),
        palette.resolve(Color::Indexed(9))
    );
}

#[test]
fn color_16_bg() {
    let Some(outcome) = run_scenario("color_16_bg") else {
        return;
    };
    let palette = Palette::default();
    // "blk" at col 0-2: SGR 40 = black bg (index 0).
    assert_eq!(
        cell_bg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(0))
    );
    // "red" at col 3-5: SGR 41 = red bg (index 1).
    assert_eq!(
        cell_bg_at(&outcome, 0, 3),
        palette.resolve(Color::Indexed(1))
    );
    // "Bblk" at col 24-27: SGR 100 = bright black bg (index 8).
    assert_eq!(
        cell_bg_at(&outcome, 0, 24),
        palette.resolve(Color::Indexed(8))
    );
}

#[test]
fn color_bold_bright() {
    let Some(outcome) = run_scenario("color_bold_bright") else {
        return;
    };
    let palette = Palette::default();
    // Bold + red (SGR 1;31) with bold_is_bright=true → bright red (index 9).
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
    // Bold + indexed 100 does NOT promote to 108.
    let idx100 = palette.resolve(Color::Indexed(100));
    assert_eq!(cell_fg_at(&outcome, 0, 0), idx100);
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
}

#[test]
fn color_bold_bright_disabled() {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/sgr/color_bold_bright.teseq");
    let mut h = TeseqHarness::from_scenario(&path);
    h.set_bold_is_bright(false);
    let outcome = h.run(&path);
    // Bold + red with bold_is_bright=false → normal red (index 1), not bright.
    let palette = Palette::default();
    let normal_red = palette.resolve(Color::Indexed(1));
    assert_eq!(cell_fg_at(&outcome, 0, 0), normal_red);
}
