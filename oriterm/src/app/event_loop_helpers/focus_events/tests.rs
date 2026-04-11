//! Sibling tests for `focus_events.rs` — Section 06.5 Track B
//! cross-crate `kxIN`/`kxOUT` xcheck.
//!
//! These tests pin both:
//!   1. The literal byte values of `FOCUS_IN_SEQ` / `FOCUS_OUT_SEQ`
//!      against the `kxIN`/`kxOUT` cap declarations in
//!      `extra/ori_term.info` (via the canonical
//!      `oriterm_test_support::tack_framework::cap_coverage::declared_cap_value`
//!      helper consolidates the parser into one
//!      canonical home).
//!   2. The actual emission-path decision via
//!      [`super::focus_event_seq_for_mode`] (fix —
//!      tests the load-bearing pure kernel of `send_focus_event`,
//!      not just the constants).

use oriterm_core::TermMode;
use oriterm_test_support::tack_framework::cap_coverage::declared_cap_value;

use super::{FOCUS_IN_SEQ, FOCUS_OUT_SEQ, focus_event_seq_for_mode};

// ----- Constant value pins (cross-reference against terminfo) -----

#[test]
fn focus_in_seq_matches_kxin_terminfo_declaration() {
    // Pin the literal byte value.
    assert_eq!(FOCUS_IN_SEQ, b"\x1b[I");
    // Cross-reference the terminfo declaration via the canonical
    // helper from `oriterm_test_support`.
    let kxin = declared_cap_value("kxIN").expect("kxIN must be declared in extra/ori_term.info");
    assert_eq!(
        kxin,
        "\\E[I",
        "FOCUS_IN_SEQ ({:?}) must encode the same bytes as the \
         kxIN cap declaration ({}). Update both sites in lockstep \
         if focus-event encoding changes.",
        std::str::from_utf8(FOCUS_IN_SEQ).unwrap_or("<invalid utf8>"),
        kxin,
    );
}

#[test]
fn focus_out_seq_matches_kxout_terminfo_declaration() {
    assert_eq!(FOCUS_OUT_SEQ, b"\x1b[O");
    let kxout = declared_cap_value("kxOUT").expect("kxOUT must be declared in extra/ori_term.info");
    assert_eq!(
        kxout,
        "\\E[O",
        "FOCUS_OUT_SEQ ({:?}) must encode the same bytes as the \
         kxOUT cap declaration ({}).",
        std::str::from_utf8(FOCUS_OUT_SEQ).unwrap_or("<invalid utf8>"),
        kxout,
    );
}

#[test]
fn focus_in_and_out_seqs_are_distinct() {
    // SSOT pin: a regression that aliased both constants to the
    // same byte sequence would silently break focus event
    // round-tripping. Catch it here.
    assert_ne!(FOCUS_IN_SEQ, FOCUS_OUT_SEQ);
}

// ----- Emission decision pins (exercise the actual
// pure kernel that `send_focus_event` uses, not just the
// constants) -----

#[test]
fn focus_event_seq_for_mode_returns_none_when_focus_in_out_disabled() {
    // NEGATIVE PIN — without DECSET 1004 (FOCUS_IN_OUT), the
    // function MUST return None regardless of the focused flag.
    // This is the load-bearing gate that prevents focus events
    // from leaking to terminals that didn't ask for them.
    let mode = TermMode::empty();
    assert_eq!(focus_event_seq_for_mode(mode, true), None);
    assert_eq!(focus_event_seq_for_mode(mode, false), None);
}

#[test]
fn focus_event_seq_for_mode_returns_focus_in_seq_when_focused_true() {
    // POSITIVE PIN — with FOCUS_IN_OUT enabled and focused=true,
    // the function returns FOCUS_IN_SEQ. This is the kxIN
    // emission path.
    let mode = TermMode::FOCUS_IN_OUT;
    assert_eq!(focus_event_seq_for_mode(mode, true), Some(FOCUS_IN_SEQ));
}

#[test]
fn focus_event_seq_for_mode_returns_focus_out_seq_when_focused_false() {
    // POSITIVE PIN — with FOCUS_IN_OUT enabled and focused=false,
    // the function returns FOCUS_OUT_SEQ. This is the kxOUT
    // emission path.
    let mode = TermMode::FOCUS_IN_OUT;
    assert_eq!(focus_event_seq_for_mode(mode, false), Some(FOCUS_OUT_SEQ));
}

#[test]
fn focus_event_seq_for_mode_only_checks_the_focus_in_out_flag() {
    // SEMANTIC PIN — even with other unrelated mode flags set,
    // the function continues to honor the FOCUS_IN_OUT flag and
    // ignores the others. Catches a regression where a refactor
    // accidentally added an extra mode predicate that gated
    // emission on something else.
    let mode = TermMode::FOCUS_IN_OUT | TermMode::INSERT | TermMode::ALT_SCREEN;
    assert_eq!(focus_event_seq_for_mode(mode, true), Some(FOCUS_IN_SEQ));
    assert_eq!(focus_event_seq_for_mode(mode, false), Some(FOCUS_OUT_SEQ));

    // And the inverse: a mode WITHOUT FOCUS_IN_OUT but with other
    // flags still emits nothing.
    let no_focus = TermMode::INSERT | TermMode::ALT_SCREEN;
    assert_eq!(focus_event_seq_for_mode(no_focus, true), None);
    assert_eq!(focus_event_seq_for_mode(no_focus, false), None);
}
