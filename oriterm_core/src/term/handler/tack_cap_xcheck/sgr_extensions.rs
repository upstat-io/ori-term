//! Section 06.5 direct-VTE cap xcheck — SGR extensions.
//!
//! Covers `Smulx` (kitty colon underline style) and `Setulc`
//! (truecolor underline color). Both caps are exercised via SGR
//! sub-parameter forms (`CSI 4:N m` and `CSI 58:2::r:g:b m`) that
//! tack v1.08 has no tool to probe.

use vte::ansi::{Color, Rgb};

use crate::cell::CellFlags;

use super::super::test_helpers::{feed, term_with_recorder};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["Smulx", "Setulc"];

#[test]
fn tack_cap_xcheck_smulx_off_4_0() {
    assert_cap_declared("Smulx");
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[4:0m");
    assert!(
        !term
            .grid()
            .cursor()
            .template
            .flags
            .intersects(CellFlags::ALL_UNDERLINES),
        "CSI 4:0m must clear all underline flags"
    );
}

#[test]
fn tack_cap_xcheck_smulx_straight_4_1() {
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[4:1m");
    let flags = term.grid().cursor().template.flags;
    assert!(flags.contains(CellFlags::UNDERLINE));
    assert!(!flags.contains(CellFlags::DOUBLE_UNDERLINE));
    assert!(!flags.contains(CellFlags::CURLY_UNDERLINE));
    assert!(!flags.contains(CellFlags::DOTTED_UNDERLINE));
    assert!(!flags.contains(CellFlags::DASHED_UNDERLINE));
}

#[test]
fn tack_cap_xcheck_smulx_double_4_2() {
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[4:2m");
    let flags = term.grid().cursor().template.flags;
    assert!(flags.contains(CellFlags::DOUBLE_UNDERLINE));
    assert!(!flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn tack_cap_xcheck_smulx_curly_4_3() {
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[4:3m");
    let flags = term.grid().cursor().template.flags;
    assert!(flags.contains(CellFlags::CURLY_UNDERLINE));
    assert!(!flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn tack_cap_xcheck_smulx_dotted_4_4() {
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[4:4m");
    let flags = term.grid().cursor().template.flags;
    assert!(flags.contains(CellFlags::DOTTED_UNDERLINE));
    assert!(!flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn tack_cap_xcheck_smulx_dashed_4_5() {
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[4:5m");
    let flags = term.grid().cursor().template.flags;
    assert!(flags.contains(CellFlags::DASHED_UNDERLINE));
    assert!(!flags.contains(CellFlags::UNDERLINE));
}

#[test]
fn tack_cap_xcheck_smulx_transitions_clear_previous() {
    // SEMANTIC PIN — feeding curly-then-dotted must leave ONLY
    // dotted set, not both. Catches the "bitflag-or instead of
    // replace" regression.
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[4:3m\x1b[4:4m");
    let flags = term.grid().cursor().template.flags;
    assert!(flags.contains(CellFlags::DOTTED_UNDERLINE));
    assert!(!flags.contains(CellFlags::CURLY_UNDERLINE));
    assert!(!flags.contains(CellFlags::UNDERLINE));
    assert!(!flags.contains(CellFlags::DOUBLE_UNDERLINE));
    assert!(!flags.contains(CellFlags::DASHED_UNDERLINE));
}

#[test]
fn tack_cap_xcheck_setulc_truecolor_sets_underline_color() {
    assert_cap_declared("Setulc");
    let (mut term, _l) = term_with_recorder();
    // CSI 58 ; 2 ; r ; g ; b m  (truecolor underline color)
    feed(&mut term, b"\x1b[58;2;255;100;50m");
    let extra = term
        .grid()
        .cursor()
        .template
        .extra
        .as_ref()
        .expect("CellExtra must be allocated for SGR 58 underline color");
    assert_eq!(
        extra.underline_color,
        Some(Color::Spec(Rgb {
            r: 255,
            g: 100,
            b: 50
        })),
    );
}

#[test]
fn tack_cap_xcheck_setulc_reset_clears_underline_color() {
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[58;2;255;100;50m");
    feed(&mut term, b"\x1b[59m");
    // CSI 59m clears the underline color; with no other extra
    // data set, CellExtra is dropped entirely.
    assert!(
        term.grid().cursor().template.extra.is_none(),
        "CSI 59m must clear underline color and drop CellExtra"
    );
}

#[test]
fn tack_cap_xcheck_smulx_cap_value_matches() {
    // Pin the literal terminfo declaration. A future edit that
    // changes the cap value forces a re-validation of the
    // sub-parameter forms above.
    assert_cap_value_matches("Smulx", "\\E[4\\:%p1%dm");
}

#[test]
fn tack_cap_xcheck_setulc_cap_value_matches() {
    // fix: pin the literal Setulc declaration so a
    // terminfo edit that changes the SGR 58 cap value triggers a
    // test failure BEFORE the round-trip tests above are exercised.
    assert_cap_value_matches(
        "Setulc",
        "\\E[58:2::%p1%{65536}%/%d:%p1%{256}%/%{255}%&%d:%p1%{255}%&%d%;m",
    );
}
