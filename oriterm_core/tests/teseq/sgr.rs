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

// 05.4 256-color & TrueColor scenarios

#[test]
fn color_256_fg() {
    let Some(outcome) = run_scenario("color_256_fg") else {
        return;
    };
    let palette = Palette::default();
    // "Red 256" at col 0: SGR 38;5;196.
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(196))
    );
    // "Green 256" at col 7: SGR 38;5;46.
    assert_eq!(
        cell_fg_at(&outcome, 0, 7),
        palette.resolve(Color::Indexed(46))
    );
    // "Blue 256" at col 16: SGR 38;5;21.
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
    // "Red bg" at col 0: SGR 48;5;196.
    assert_eq!(
        cell_bg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(196))
    );
    // "Green bg" at col 6: SGR 48;5;46.
    assert_eq!(
        cell_bg_at(&outcome, 0, 6),
        palette.resolve(Color::Indexed(46))
    );
    // "Blue bg" at col 14: SGR 48;5;21.
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
    // "Orange" at col 0: SGR 38;2;255;128;0.
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        Rgb {
            r: 255,
            g: 128,
            b: 0
        }
    );
    // "Spring" at col 6: SGR 38;2;0;255;128.
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
    // "Orange bg" at col 0: SGR 48;2;255;128;0.
    assert_eq!(
        cell_bg_at(&outcome, 0, 0),
        Rgb {
            r: 255,
            g: 128,
            b: 0
        }
    );
    // "Spring bg" at col 9: SGR 48;2;0;255;128.
    assert_eq!(
        cell_bg_at(&outcome, 0, 9),
        Rgb {
            r: 0,
            g: 255,
            b: 128
        }
    );
}

// 05.5a Selective attribute resets (SGR 21-29)

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
    // "Curly" at col 0: CURLY_UNDERLINE set.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::CURLY_UNDERLINE);
    // "None" at col 5: ALL_UNDERLINES cleared.
    assert_cell_flags_not_contain(&outcome, 0, 5, CellFlags::ALL_UNDERLINES);
}

#[test]
fn reset_25_cancel_blink() {
    let Some(outcome) = run_scenario("reset_25_cancel_blink") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BLINK | CellFlags::BOLD);
    // "Bold" at col 9: BOLD only, BLINK cancelled.
    assert_cell_flags_contain(&outcome, 0, 9, CellFlags::BOLD);
    assert_cell_flags_not_contain(&outcome, 0, 9, CellFlags::BLINK);
}

#[test]
fn reset_27_cancel_inverse() {
    let Some(outcome) = run_scenario("reset_27_cancel_inverse") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::INVERSE | CellFlags::ITALIC);
    // "Italic" at col 9: ITALIC only, INVERSE cancelled.
    assert_cell_flags_contain(&outcome, 0, 9, CellFlags::ITALIC);
    assert_cell_flags_not_contain(&outcome, 0, 9, CellFlags::INVERSE);
}

#[test]
fn reset_28_cancel_hidden() {
    let Some(outcome) = run_scenario("reset_28_cancel_hidden") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::HIDDEN | CellFlags::BOLD);
    // "Bold" at col 7: BOLD only, HIDDEN cancelled.
    assert_cell_flags_contain(&outcome, 0, 7, CellFlags::BOLD);
    assert_cell_flags_not_contain(&outcome, 0, 7, CellFlags::HIDDEN);
}

#[test]
fn reset_29_cancel_strike() {
    let Some(outcome) = run_scenario("reset_29_cancel_strike") else {
        return;
    };
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::STRIKETHROUGH | CellFlags::ITALIC);
    // "Italic" at col 12: ITALIC only, STRIKETHROUGH cancelled.
    assert_cell_flags_contain(&outcome, 0, 12, CellFlags::ITALIC);
    assert_cell_flags_not_contain(&outcome, 0, 12, CellFlags::STRIKETHROUGH);
}

#[test]
fn reset_selective_preserves_others() {
    let Some(outcome) = run_scenario("reset_selective_preserves_others") else {
        return;
    };
    let all = CellFlags::BOLD
        | CellFlags::ITALIC
        | CellFlags::UNDERLINE
        | CellFlags::BLINK
        | CellFlags::INVERSE
        | CellFlags::STRIKETHROUGH;
    // Line 0 "All": all 6 flags set.
    assert_cell_flags_contain(&outcome, 0, 0, all);
    // Line 1 "NoBold": BOLD cleared, rest remain.
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
    // Line 2 "NoItalic": ITALIC cleared.
    assert_cell_flags_not_contain(&outcome, 2, 0, CellFlags::BOLD | CellFlags::ITALIC);
    assert_cell_flags_contain(
        &outcome,
        2,
        0,
        CellFlags::UNDERLINE | CellFlags::BLINK | CellFlags::INVERSE | CellFlags::STRIKETHROUGH,
    );
    // Line 3 "NoUL": UNDERLINE cleared.
    assert_cell_flags_not_contain(&outcome, 3, 0, CellFlags::ALL_UNDERLINES);
    assert_cell_flags_contain(
        &outcome,
        3,
        0,
        CellFlags::BLINK | CellFlags::INVERSE | CellFlags::STRIKETHROUGH,
    );
    // Line 4 "NoBlink": BLINK cleared.
    assert_cell_flags_not_contain(&outcome, 4, 0, CellFlags::BLINK);
    assert_cell_flags_contain(
        &outcome,
        4,
        0,
        CellFlags::INVERSE | CellFlags::STRIKETHROUGH,
    );
    // Line 5 "NoInv": INVERSE cleared.
    assert_cell_flags_not_contain(&outcome, 5, 0, CellFlags::INVERSE);
    assert_cell_flags_contain(&outcome, 5, 0, CellFlags::STRIKETHROUGH);
    // Line 6 "NoStrike": STRIKETHROUGH cleared, no SGR flags remain.
    assert_cell_flags_not_contain(&outcome, 6, 0, all);
}

// 05.5b Default color & template resets (SGR 0/39/49/59)

#[test]
fn reset_sgr0() {
    let Some(outcome) = run_scenario("reset_sgr0") else {
        return;
    };
    let palette = Palette::default();
    // "All on" at col 0: BOLD+ITALIC+UNDERLINE, fg=red, bg=green.
    assert_cell_flags_contain(
        &outcome,
        0,
        0,
        CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE,
    );
    // Bold-as-bright: red (1) → bright red (9).
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(9))
    );
    assert_eq!(
        cell_bg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(2))
    );
    // "Clean" at col 6: no SGR flags, default colors.
    let sgr_flags = CellFlags::BOLD
        | CellFlags::DIM
        | CellFlags::ITALIC
        | CellFlags::UNDERLINE
        | CellFlags::BLINK
        | CellFlags::INVERSE
        | CellFlags::HIDDEN
        | CellFlags::STRIKETHROUGH;
    assert_cell_flags_not_contain(&outcome, 0, 6, sgr_flags);
}

#[test]
fn reset_39_default_fg() {
    let Some(outcome) = run_scenario("reset_39_default_fg") else {
        return;
    };
    let palette = Palette::default();
    // "Red bold" at col 0: BOLD + red fg.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(9))
    ); // bold-as-bright: red→bright red
    // "Default fg bold" at col 8: BOLD preserved, fg reset to default.
    assert_cell_flags_contain(&outcome, 0, 8, CellFlags::BOLD);
    let default_fg = palette.resolve(Color::Named(vte::ansi::NamedColor::Foreground));
    assert_eq!(cell_fg_at(&outcome, 0, 8), default_fg);
}

#[test]
fn reset_49_default_bg() {
    let Some(outcome) = run_scenario("reset_49_default_bg") else {
        return;
    };
    let palette = Palette::default();
    // "Green bg italic" at col 0: ITALIC + green bg.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::ITALIC);
    assert_eq!(
        cell_bg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(2))
    );
    // "Default bg italic" at col 15: ITALIC preserved, bg reset to default.
    assert_cell_flags_contain(&outcome, 0, 15, CellFlags::ITALIC);
    let default_bg = palette.resolve(Color::Named(vte::ansi::NamedColor::Background));
    assert_eq!(cell_bg_at(&outcome, 0, 15), default_bg);
}

#[test]
fn reset_59_underline_color() {
    let Some(outcome) = run_scenario("reset_59_underline_color") else {
        return;
    };
    // "Colored" at col 0: UNDERLINE + custom underline color.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::UNDERLINE);
    assert_eq!(
        cell_underline_color_at(&outcome, 0, 0),
        Some(Rgb {
            r: 200,
            g: 100,
            b: 50
        })
    );
    // "Default" at col 7: UNDERLINE preserved, underline color cleared.
    assert_cell_flags_contain(&outcome, 0, 7, CellFlags::UNDERLINE);
    assert_eq!(cell_underline_color_at(&outcome, 0, 7), None);
}

// 05.6 Color resolution edge cases

#[test]
fn edge_dim_bold_named() {
    let Some(outcome) = run_scenario("edge_dim_bold_named") else {
        return;
    };
    let palette = Palette::default();
    // DIM + BOLD + red: DIM takes priority, resolves to DimRed, NOT BrightRed.
    let dim_red = palette.resolve(Color::Named(vte::ansi::NamedColor::DimRed));
    assert_eq!(cell_fg_at(&outcome, 0, 0), dim_red);
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::DIM);
}

#[test]
fn edge_dim_bold_indexed() {
    let Some(outcome) = run_scenario("edge_dim_bold_indexed") else {
        return;
    };
    let palette = Palette::default();
    // DIM + BOLD + indexed 2: dimmed green, not bright green.
    let base = palette.resolve(Color::Indexed(2));
    let expected = Rgb {
        r: (base.r as u16 * 2 / 3) as u8,
        g: (base.g as u16 * 2 / 3) as u8,
        b: (base.b as u16 * 2 / 3) as u8,
    };
    assert_eq!(cell_fg_at(&outcome, 0, 0), expected);
}

#[test]
fn edge_dim_bold_truecolor() {
    let Some(outcome) = run_scenario("edge_dim_bold_truecolor") else {
        return;
    };
    // DIM on TrueColor 150,120,90 → 2/3 of each = 100,80,60.
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
    // "Inverted" at col 12: fg=green, bg=red (swapped).
    assert_eq!(cell_fg_at(&outcome, 0, 12), green);
    assert_eq!(cell_bg_at(&outcome, 0, 12), red);
}

#[test]
fn edge_decscnm_basic() {
    let Some(outcome) = run_scenario("edge_decscnm_basic") else {
        return;
    };
    let palette = Palette::default();
    let default_fg = palette.resolve(Color::Named(vte::ansi::NamedColor::Foreground));
    let default_bg = palette.resolve(Color::Named(vte::ansi::NamedColor::Background));
    // DECSCNM swaps default palette: text fg becomes original bg, text bg becomes original fg.
    assert_eq!(cell_fg_at(&outcome, 0, 0), default_bg);
    assert_eq!(cell_bg_at(&outcome, 0, 0), default_fg);
}

#[test]
fn edge_decscnm_inverse() {
    let Some(outcome) = run_scenario("edge_decscnm_inverse") else {
        return;
    };
    let palette = Palette::default();
    let default_fg = palette.resolve(Color::Named(vte::ansi::NamedColor::Foreground));
    let default_bg = palette.resolve(Color::Named(vte::ansi::NamedColor::Background));
    // DECSCNM + SGR 7 = double swap → back to normal appearance.
    assert_eq!(cell_fg_at(&outcome, 0, 0), default_fg);
    assert_eq!(cell_bg_at(&outcome, 0, 0), default_bg);
}

#[test]
fn edge_decscnm_explicit_color() {
    let Some(outcome) = run_scenario("edge_decscnm_explicit_color") else {
        return;
    };
    // DECSCNM doesn't affect explicit TrueColor — Color::Spec resolves directly.
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        Rgb {
            r: 100,
            g: 200,
            b: 50
        }
    );
}

// 05.7 Attribute stacking & combination scenarios

#[test]
fn combo_stack() {
    let Some(outcome) = run_scenario("combo_stack") else {
        return;
    };
    let palette = Palette::default();
    // "Bold italic underline red" at col 0.
    assert_cell_flags_contain(
        &outcome,
        0,
        0,
        CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE,
    );
    // Bold-as-bright: red → bright red (index 9).
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(9))
    );
    // "Normal" at col 25: no SGR flags.
    let sgr = CellFlags::BOLD | CellFlags::ITALIC | CellFlags::UNDERLINE;
    assert_cell_flags_not_contain(&outcome, 0, 25, sgr);
}

#[test]
fn combo_separate_sequences() {
    let Some(outcome) = run_scenario("combo_separate_sequences") else {
        return;
    };
    // Three separate SGR sequences accumulate.
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
    // SGR 31;32;33 → yellow wins (SGR 33 = index 3).
    assert_eq!(
        cell_fg_at(&outcome, 0, 0),
        palette.resolve(Color::Indexed(3))
    );
}

#[test]
fn combo_dim_then_bold() {
    let Some(outcome) = run_scenario("combo_dim_then_bold") else {
        return;
    };
    let palette = Palette::default();
    // DIM + BOLD + red: both flags set, DIM takes priority for color.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::DIM);
    let dim_red = palette.resolve(Color::Named(vte::ansi::NamedColor::DimRed));
    assert_eq!(cell_fg_at(&outcome, 0, 0), dim_red);
}

#[test]
fn combo_empty_sgr_resets() {
    let Some(outcome) = run_scenario("combo_empty_sgr_resets") else {
        return;
    };
    // "Styled" at col 0: BOLD+ITALIC.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD | CellFlags::ITALIC);
    // "Plain" at col 6: no flags (CSI m = SGR 0).
    assert_cell_flags_not_contain(&outcome, 0, 6, CellFlags::BOLD | CellFlags::ITALIC);
}

#[test]
fn combo_sgr_persists_cursor_move() {
    let Some(outcome) = run_scenario("combo_sgr_persists_cursor_move") else {
        return;
    };
    let palette = Palette::default();
    let bright_red = palette.resolve(Color::Indexed(9));
    // 'A' at col 0: BOLD + red.
    assert_cell_flags_contain(&outcome, 0, 0, CellFlags::BOLD);
    assert_eq!(cell_fg_at(&outcome, 0, 0), bright_red);
    // 'B' at col 4 (CHA 5 = 1-based col 5 = 0-based col 4): BOLD + red survives cursor move.
    assert_cell_flags_contain(&outcome, 0, 4, CellFlags::BOLD);
    assert_eq!(cell_fg_at(&outcome, 0, 4), bright_red);
}
