use crate::index::Column;
use crate::term::Term;
use crate::theme::Theme;

use super::super::test_helpers::feed;

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- SGR (Select Graphic Rendition) tests ---

#[test]
fn sgr_bold_sets_flag_on_cursor_template() {
    let mut t = term();
    // ESC[1m — set bold.
    feed(&mut t, b"\x1b[1m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::BOLD));
}

#[test]
fn sgr_fg_red_sets_ansi_color() {
    let mut t = term();
    // ESC[31m — set fg to red (ANSI 1).
    feed(&mut t, b"\x1b[31m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
}

#[test]
fn sgr_256color_fg() {
    let mut t = term();
    // ESC[38;5;196m — set fg to 256-color index 196.
    feed(&mut t, b"\x1b[38;5;196m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(fg, vte::ansi::Color::Indexed(196));
}

#[test]
fn sgr_truecolor_fg() {
    let mut t = term();
    // ESC[38;2;255;128;0m — set fg to RGB(255, 128, 0).
    feed(&mut t, b"\x1b[38;2;255;128;0m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(
        fg,
        vte::ansi::Color::Spec(vte::ansi::Rgb {
            r: 255,
            g: 128,
            b: 0
        })
    );
}

#[test]
fn sgr_reset_clears_all_attributes() {
    let mut t = term();
    // Set bold + red fg + green bg, then reset.
    feed(&mut t, b"\x1b[1;31;42m");
    feed(&mut t, b"\x1b[0m");

    let template = &t.grid().cursor().template;
    assert_eq!(template.flags, crate::cell::CellFlags::empty());
    assert_eq!(
        template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Foreground)
    );
    assert_eq!(
        template.bg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Background)
    );
}

#[test]
fn sgr_compound_bold_red_fg_green_bg() {
    let mut t = term();
    // ESC[1;31;42m — bold + red fg + green bg in one sequence.
    feed(&mut t, b"\x1b[1;31;42m");

    let template = &t.grid().cursor().template;
    assert!(template.flags.contains(crate::cell::CellFlags::BOLD));
    assert_eq!(
        template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
    assert_eq!(
        template.bg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Green)
    );
}

#[test]
fn sgr_curly_underline() {
    let mut t = term();
    // ESC[4:3m — curly underline (sub-param style).
    feed(&mut t, b"\x1b[4:3m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::CURLY_UNDERLINE));
    // Should not have regular underline.
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_underline_color_truecolor() {
    let mut t = term();
    // ESC[58;2;255;0;0m — set underline color to red (CellExtra).
    feed(&mut t, b"\x1b[58;2;255;0;0m");

    let template = &t.grid().cursor().template;
    let extra = template
        .extra
        .as_ref()
        .expect("CellExtra should be allocated");
    assert_eq!(
        extra.underline_color,
        Some(vte::ansi::Color::Spec(vte::ansi::Rgb {
            r: 255,
            g: 0,
            b: 0
        }))
    );
}

#[test]
fn sgr_59_clears_underline_color() {
    let mut t = term();
    // Set underline color, then clear it.
    feed(&mut t, b"\x1b[58;2;255;0;0m");
    feed(&mut t, b"\x1b[59m");

    let template = &t.grid().cursor().template;
    // CellExtra should be dropped (no other extra data).
    assert!(template.extra.is_none());
}

// --- SGR individual attribute flag tests ---

#[test]
fn sgr_dim_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[2m");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::DIM)
    );
}

#[test]
fn sgr_italic_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[3m");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::ITALIC)
    );
}

#[test]
fn sgr_blink_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[5m");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BLINK)
    );
}

#[test]
fn sgr_inverse_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[7m");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::INVERSE)
    );
}

#[test]
fn sgr_hidden_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[8m");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::HIDDEN)
    );
}

#[test]
fn sgr_strikethrough_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[9m");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::STRIKETHROUGH)
    );
}

// --- SGR cancel attribute tests ---

#[test]
fn sgr_22_cancels_bold_and_dim() {
    let mut t = term();
    // Set both bold and dim, then cancel both with SGR 22.
    feed(&mut t, b"\x1b[1;2m");
    feed(&mut t, b"\x1b[22m");

    let flags = t.grid().cursor().template.flags;
    assert!(!flags.contains(crate::cell::CellFlags::BOLD));
    assert!(!flags.contains(crate::cell::CellFlags::DIM));
}

#[test]
fn sgr_23_cancels_italic() {
    let mut t = term();
    feed(&mut t, b"\x1b[3m");
    feed(&mut t, b"\x1b[23m");

    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::ITALIC)
    );
}

#[test]
fn sgr_24_cancels_all_underlines() {
    let mut t = term();
    // Set curly underline, then cancel.
    feed(&mut t, b"\x1b[4:3m");
    feed(&mut t, b"\x1b[24m");

    let flags = t.grid().cursor().template.flags;
    assert!(!flags.contains(crate::cell::CellFlags::CURLY_UNDERLINE));
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_25_cancels_blink() {
    let mut t = term();
    feed(&mut t, b"\x1b[5m");
    feed(&mut t, b"\x1b[25m");

    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BLINK)
    );
}

#[test]
fn sgr_27_cancels_inverse() {
    let mut t = term();
    feed(&mut t, b"\x1b[7m");
    feed(&mut t, b"\x1b[27m");

    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::INVERSE)
    );
}

#[test]
fn sgr_28_cancels_hidden() {
    let mut t = term();
    feed(&mut t, b"\x1b[8m");
    feed(&mut t, b"\x1b[28m");

    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::HIDDEN)
    );
}

#[test]
fn sgr_29_cancels_strikethrough() {
    let mut t = term();
    feed(&mut t, b"\x1b[9m");
    feed(&mut t, b"\x1b[29m");

    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::STRIKETHROUGH)
    );
}

// --- SGR underline mutual exclusion tests ---

#[test]
fn sgr_underline_replaces_curly() {
    let mut t = term();
    // Set curly, then single — single should replace curly.
    feed(&mut t, b"\x1b[4:3m");
    feed(&mut t, b"\x1b[4m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::UNDERLINE));
    assert!(!flags.contains(crate::cell::CellFlags::CURLY_UNDERLINE));
}

#[test]
fn sgr_double_underline_replaces_single() {
    let mut t = term();
    // Single underline, then double via sub-param ESC[4:2m.
    feed(&mut t, b"\x1b[4m");
    feed(&mut t, b"\x1b[4:2m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::DOUBLE_UNDERLINE));
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_dotted_underline() {
    let mut t = term();
    feed(&mut t, b"\x1b[4:4m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::DOTTED_UNDERLINE));
}

#[test]
fn sgr_dashed_underline() {
    let mut t = term();
    feed(&mut t, b"\x1b[4:5m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::DASHED_UNDERLINE));
}

// --- SGR cancel preserves unrelated attributes ---

#[test]
fn sgr_cancel_underline_preserves_bold() {
    let mut t = term();
    // Bold + underline, then cancel underline — bold should remain.
    feed(&mut t, b"\x1b[1;4m");
    feed(&mut t, b"\x1b[24m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::BOLD));
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_cancel_bold_preserves_italic_and_color() {
    let mut t = term();
    // Bold + italic + red fg, then cancel bold.
    feed(&mut t, b"\x1b[1;3;31m");
    feed(&mut t, b"\x1b[22m");

    let template = &t.grid().cursor().template;
    assert!(!template.flags.contains(crate::cell::CellFlags::BOLD));
    assert!(template.flags.contains(crate::cell::CellFlags::ITALIC));
    assert_eq!(
        template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
}

// --- SGR color tests ---

#[test]
fn sgr_bg_256color() {
    let mut t = term();
    feed(&mut t, b"\x1b[48;5;42m");

    assert_eq!(t.grid().cursor().template.bg, vte::ansi::Color::Indexed(42));
}

#[test]
fn sgr_bg_truecolor() {
    let mut t = term();
    feed(&mut t, b"\x1b[48;2;0;128;255m");

    assert_eq!(
        t.grid().cursor().template.bg,
        vte::ansi::Color::Spec(vte::ansi::Rgb {
            r: 0,
            g: 128,
            b: 255
        })
    );
}

#[test]
fn sgr_bright_fg() {
    let mut t = term();
    // ESC[91m — bright red foreground (ANSI 8–15 range).
    feed(&mut t, b"\x1b[91m");

    assert_eq!(
        t.grid().cursor().template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::BrightRed)
    );
}

#[test]
fn sgr_bright_bg() {
    let mut t = term();
    // ESC[102m — bright green background.
    feed(&mut t, b"\x1b[102m");

    assert_eq!(
        t.grid().cursor().template.bg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::BrightGreen)
    );
}

#[test]
fn sgr_39_resets_fg_only() {
    let mut t = term();
    // Red fg + green bg, then reset fg only.
    feed(&mut t, b"\x1b[31;42m");
    feed(&mut t, b"\x1b[39m");

    let template = &t.grid().cursor().template;
    assert_eq!(
        template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Foreground)
    );
    assert_eq!(
        template.bg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Green)
    );
}

#[test]
fn sgr_49_resets_bg_only() {
    let mut t = term();
    // Red fg + green bg, then reset bg only.
    feed(&mut t, b"\x1b[31;42m");
    feed(&mut t, b"\x1b[49m");

    let template = &t.grid().cursor().template;
    assert_eq!(
        template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
    assert_eq!(
        template.bg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Background)
    );
}

// --- SGR character inheritance tests ---

#[test]
fn printed_char_inherits_bold() {
    let mut t = term();
    // Bold, then print 'A'.
    feed(&mut t, b"\x1b[1mA");

    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'A');
    assert!(cell.flags.contains(crate::cell::CellFlags::BOLD));
}

#[test]
fn printed_char_inherits_fg_color() {
    let mut t = term();
    feed(&mut t, b"\x1b[31mA");

    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
}

#[test]
fn reset_between_chars_gives_different_attrs() {
    let mut t = term();
    // Bold 'A', then reset + 'B'.
    feed(&mut t, b"\x1b[1mA\x1b[0mB");

    let a = &t.grid()[crate::index::Line(0)][Column(0)];
    let b = &t.grid()[crate::index::Line(0)][Column(1)];
    assert!(a.flags.contains(crate::cell::CellFlags::BOLD));
    assert!(!b.flags.contains(crate::cell::CellFlags::BOLD));
}

// --- SGR persistence tests ---

#[test]
fn sgr_persists_across_cursor_movement() {
    let mut t = term();
    // Set bold, then move cursor down 5.
    feed(&mut t, b"\x1b[1m");
    feed(&mut t, b"\x1b[5B");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD)
    );
}

#[test]
fn sgr_stacks_across_separate_sequences() {
    let mut t = term();
    // Bold in one sequence, underline in another, color in a third.
    feed(&mut t, b"\x1b[1m");
    feed(&mut t, b"\x1b[4m");
    feed(&mut t, b"\x1b[31m");

    let template = &t.grid().cursor().template;
    assert!(template.flags.contains(crate::cell::CellFlags::BOLD));
    assert!(template.flags.contains(crate::cell::CellFlags::UNDERLINE));
    assert_eq!(
        template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
}

// --- SGR edge case tests ---

#[test]
fn sgr_empty_params_resets() {
    let mut t = term();
    // Set bold, then ESC[m (no params) should reset like SGR 0.
    feed(&mut t, b"\x1b[1m");
    feed(&mut t, b"\x1b[m");

    assert_eq!(
        t.grid().cursor().template.flags,
        crate::cell::CellFlags::empty()
    );
}

#[test]
fn sgr_last_color_wins() {
    let mut t = term();
    // ESC[30;31m — black then red in same sequence; red should win.
    feed(&mut t, b"\x1b[30;31m");

    assert_eq!(
        t.grid().cursor().template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
}

#[test]
fn sgr_fast_blink_uses_blink_flag() {
    let mut t = term();
    // SGR 6 (fast blink) — mapped to same BLINK flag as slow blink.
    feed(&mut t, b"\x1b[6m");

    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BLINK)
    );
}

#[test]
fn sgr_underline_color_survives_underline_type_change() {
    let mut t = term();
    // Set underline color to red, then switch from single to curly.
    feed(&mut t, b"\x1b[4m");
    feed(&mut t, b"\x1b[58;2;255;0;0m");
    feed(&mut t, b"\x1b[4:3m");

    let template = &t.grid().cursor().template;
    assert!(
        template
            .flags
            .contains(crate::cell::CellFlags::CURLY_UNDERLINE)
    );
    let extra = template
        .extra
        .as_ref()
        .expect("underline color should survive");
    assert_eq!(
        extra.underline_color,
        Some(vte::ansi::Color::Spec(vte::ansi::Rgb {
            r: 255,
            g: 0,
            b: 0
        }))
    );
}

#[test]
fn sgr_underline_color_256() {
    let mut t = term();
    // ESC[58;5;196m — set underline color to 256-color index 196.
    feed(&mut t, b"\x1b[58;5;196m");

    let extra = t
        .grid()
        .cursor()
        .template
        .extra
        .as_ref()
        .expect("CellExtra should be allocated");
    assert_eq!(extra.underline_color, Some(vte::ansi::Color::Indexed(196)));
}

#[test]
fn sgr_reset_clears_underline_color() {
    let mut t = term();
    // Set underline color, then full reset.
    feed(&mut t, b"\x1b[58;2;255;0;0m");
    feed(&mut t, b"\x1b[0m");

    assert!(t.grid().cursor().template.extra.is_none());
}

// --- SGR colon-separated color parameters (ISO 8613-6) ---
//
// Modern terminals accept both semicolon and colon as sub-parameter
// separators for extended color sequences. The VTE crate parses both.

/// `ESC[38:5:196m` — colon-separated 256-color foreground.
#[test]
fn sgr_256color_fg_colon_separator() {
    let mut t = term();
    feed(&mut t, b"\x1b[38:5:196m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(fg, vte::ansi::Color::Indexed(196));
}

/// `ESC[48:5:42m` — colon-separated 256-color background.
#[test]
fn sgr_256color_bg_colon_separator() {
    let mut t = term();
    feed(&mut t, b"\x1b[48:5:42m");

    assert_eq!(t.grid().cursor().template.bg, vte::ansi::Color::Indexed(42));
}

/// `ESC[38:2::255:128:0m` — colon-separated truecolor foreground.
///
/// Per ISO 8613-6, the format is `38:2:<color-space>:R:G:B`. The color
/// space parameter is optional (empty = default RGB). The double colon
/// after `2` represents the empty color space ID.
#[test]
fn sgr_truecolor_fg_colon_separator() {
    let mut t = term();
    feed(&mut t, b"\x1b[38:2::255:128:0m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(
        fg,
        vte::ansi::Color::Spec(vte::ansi::Rgb {
            r: 255,
            g: 128,
            b: 0
        })
    );
}

/// `ESC[48:2::0:128:255m` — colon-separated truecolor background.
#[test]
fn sgr_truecolor_bg_colon_separator() {
    let mut t = term();
    feed(&mut t, b"\x1b[48:2::0:128:255m");

    assert_eq!(
        t.grid().cursor().template.bg,
        vte::ansi::Color::Spec(vte::ansi::Rgb {
            r: 0,
            g: 128,
            b: 255
        })
    );
}

/// Semicolon and colon SGR produce identical results.
#[test]
fn sgr_colon_and_semicolon_equivalent_256() {
    let mut semi = term();
    feed(&mut semi, b"\x1b[38;5;100m");

    let mut colon = term();
    feed(&mut colon, b"\x1b[38:5:100m");

    assert_eq!(
        semi.grid().cursor().template.fg,
        colon.grid().cursor().template.fg,
    );
}

/// Semicolon and colon SGR produce identical truecolor results.
#[test]
fn sgr_colon_and_semicolon_equivalent_truecolor() {
    let mut semi = term();
    feed(&mut semi, b"\x1b[38;2;10;20;30m");

    let mut colon = term();
    feed(&mut colon, b"\x1b[38:2::10:20:30m");

    assert_eq!(
        semi.grid().cursor().template.fg,
        colon.grid().cursor().template.fg,
    );
}
