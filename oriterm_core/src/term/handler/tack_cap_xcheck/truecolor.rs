//! Section 06.5 direct-VTE cap xcheck — truecolor / RGB advertisement.
//!
//! `Tc` and `RGB` are bool/string caps advertising direct-color
//! support. Both are validated by:
//!   1. Confirming the cap is declared in `extra/ori_term.info`
//!      (the static advertisement is the entire surface for
//!      `Tc`/`RGB`/bool markers).
//!   2. Round-tripping a direct-color SGR sequence
//!      (`CSI 38 ; 2 ; r ; g ; b m`) through the SGR handler and
//!      asserting the cell template's foreground color is the
//!      expected RGB triple.

use vte::ansi::{Color, Rgb};

use super::super::test_helpers::{feed, term_with_recorder};
use super::assert_cap_declared;

pub(super) const REGISTERED: &[&str] = &["Tc", "RGB"];

#[test]
fn tack_cap_xcheck_tc_bool_declared() {
    assert_cap_declared("Tc");
}

#[test]
fn tack_cap_xcheck_rgb_declared() {
    assert_cap_declared("RGB");
}

#[test]
fn tack_cap_xcheck_truecolor_sgr_38_2_sets_fg_rgb() {
    // Round-trip pin: feed CSI 38 ; 2 ; r ; g ; b m and assert
    // the cell template's fg field is the RGB triple. Catches a
    // regression in the SGR direct-color sub-parameter parser.
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[38;2;255;100;50m");
    assert_eq!(
        term.grid().cursor().template.fg,
        Color::Spec(Rgb {
            r: 255,
            g: 100,
            b: 50
        }),
    );
}

#[test]
fn tack_cap_xcheck_truecolor_sgr_48_2_sets_bg_rgb() {
    // Same round-trip for background.
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[48;2;10;20;30m");
    assert_eq!(
        term.grid().cursor().template.bg,
        Color::Spec(Rgb {
            r: 10,
            g: 20,
            b: 30
        }),
    );
}
