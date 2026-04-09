//! Section 06.5 direct-VTE cap xcheck — DECSCUSR cursor style.
//!
//! `Se` resets cursor style; `Ss` sets cursor style with a
//! parameter. The DECSCUSR handler lives at
//! `oriterm_core/src/term/handler/dcs.rs:18 dcs_set_cursor_style`.
//! `ori_term` stores the resulting state in `term.cursor_shape`
//! (the shape) and `TermMode::CURSOR_BLINKING` (the blink flag).

use crate::grid::CursorShape;
use crate::term::TermMode;

use super::super::test_helpers::{feed, term_with_recorder};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["Se", "Ss"];

#[test]
fn tack_cap_xcheck_se_ss_cap_values_match() {
    // TPR-06-001 fix: pin the literal Se/Ss declarations so a
    // terminfo edit that changes the DECSCUSR cap values triggers
    // a test failure BEFORE the round-trip tests below.
    assert_cap_value_matches("Se", "\\E[2 q");
    assert_cap_value_matches("Ss", "\\E[%p1%d q");
}

#[test]
fn tack_cap_xcheck_ss_sets_cursor_style_blinking_bar() {
    assert_cap_declared("Ss");
    let (mut term, _l) = term_with_recorder();
    // CSI 5 SP q — set cursor to blinking bar (DECSCUSR 5).
    feed(&mut term, b"\x1b[5 q");
    assert_eq!(
        term.cursor_shape(),
        CursorShape::Bar,
        "DECSCUSR 5 must set cursor shape to Beam (blinking bar)"
    );
    assert!(
        term.mode().contains(TermMode::CURSOR_BLINKING),
        "DECSCUSR 5 must enable cursor blinking"
    );
}

#[test]
fn tack_cap_xcheck_ss_sets_cursor_style_steady_underline() {
    let (mut term, _l) = term_with_recorder();
    // CSI 4 SP q — set cursor to steady underline (DECSCUSR 4).
    feed(&mut term, b"\x1b[4 q");
    assert_eq!(term.cursor_shape(), CursorShape::Underline);
    assert!(!term.mode().contains(TermMode::CURSOR_BLINKING));
}

#[test]
fn tack_cap_xcheck_se_resets_cursor_style() {
    assert_cap_declared("Se");
    let (mut term, _l) = term_with_recorder();
    // First set a non-default cursor style, then reset via
    // CSI 0 SP q (DECSCUSR 0 = reset).
    feed(&mut term, b"\x1b[4 q");
    feed(&mut term, b"\x1b[0 q");
    // After reset: block cursor, default blinking re-enabled.
    assert_eq!(term.cursor_shape(), CursorShape::Block);
    assert!(term.mode().contains(TermMode::CURSOR_BLINKING));
}
