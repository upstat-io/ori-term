//! Section 06.5 direct-VTE cap xcheck — OSC cursor color.
//!
//! `Cs` sets the cursor color via OSC 12. `Cr` resets the cursor
//! color via OSC 112. The OSC handlers live at
//! `oriterm_core/src/term/handler/osc.rs:74 osc_set_color` (set)
//! and `osc.rs:85 osc_reset_color` (reset).
//!
//! Note: `ori_term` routes OSC 12 set/reset through the palette's
//! cursor color slot. The set form changes the palette directly;
//! the query form (OSC 12 ?) emits a synchronous
//! `Effect::Pty(PtyEffect::Write)` reply directly from the palette.

use crate::effect::{Effect, EffectSink, PtyEffect};

use super::super::test_helpers::{feed, term_with_effect_sink};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["Cr", "Cs"];

#[test]
fn tack_cap_xcheck_cr_cs_cap_values_match() {
    // fix: pin the literal Cr/Cs declarations so a
    // terminfo edit that changes the OSC 12/112 cap values
    // triggers a test failure BEFORE the round-trip tests below.
    assert_cap_value_matches("Cr", "\\E]112\\007");
    assert_cap_value_matches("Cs", "\\E]12;%p1%s\\007");
}

#[test]
fn tack_cap_xcheck_cs_sets_cursor_color_via_osc_12() {
    assert_cap_declared("Cs");
    let mut term = term_with_effect_sink();
    // OSC 12 ; rgb:ff/00/00 BEL — set cursor color to red.
    feed(&mut term, b"\x1b]12;rgb:ff/00/00\x07");
    let cursor_color = term.palette().cursor_color();
    assert_eq!(cursor_color.r, 0xff);
    assert_eq!(cursor_color.g, 0x00);
    assert_eq!(cursor_color.b, 0x00);
}

#[test]
fn tack_cap_xcheck_cr_resets_cursor_color_via_osc_112() {
    assert_cap_declared("Cr");
    let mut term = term_with_effect_sink();
    // First set a non-default cursor color.
    feed(&mut term, b"\x1b]12;rgb:ff/00/00\x07");
    let after_set = term.palette().cursor_color();
    assert_eq!(after_set.r, 0xff);

    // OSC 112 BEL — reset cursor color to default.
    feed(&mut term, b"\x1b]112\x07");
    let after_reset = term.palette().cursor_color();
    // The default cursor color comes from the theme (Theme::default()
    // in test_helpers); the test asserts the value CHANGED back from
    // red, not a specific RGB triple, to keep the test theme-agnostic.
    assert!(
        !(after_reset.r == 0xff && after_reset.g == 0x00 && after_reset.b == 0x00),
        "OSC 112 must reset cursor color away from the explicitly \
         set red value; got rgb({:#x}, {:#x}, {:#x})",
        after_reset.r,
        after_reset.g,
        after_reset.b,
    );
}

#[test]
fn tack_cap_xcheck_osc_12_query_emits_sync_pty_write_reply() {
    let mut term = term_with_effect_sink();
    // OSC 12 ; ? BEL — query cursor color. The handler emits a
    // synchronous Effect::Pty(PtyEffect::Write) reply directly from
    // the palette.
    feed(&mut term, b"\x1b]12;?\x07");

    let mut effects = Vec::new();
    term.effect_sink().drain_into(&mut effects);

    let found = effects.iter().any(|e| {
        matches!(
            e,
            Effect::Pty(PtyEffect::Write { bytes, .. })
                if bytes.starts_with(b"\x1b]12;rgb:")
        )
    });
    assert!(
        found,
        "OSC 12 query must emit sync PtyEffect::Write with \\x1b]12;rgb: prefix; \
         got {effects:?}",
    );
}
