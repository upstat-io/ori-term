//! Section 06.5 direct-VTE cap xcheck — bracketed paste.
//!
//! `BD`/`BE` toggle `TermMode::BRACKETED_PASTE` via DECRST/DECSET
//! 2004 (`oriterm_core/src/term/handler/helpers.rs:47,76`).
//! `PS`/`PE` are the OUTBOUND markers `\x1b[200~` / `\x1b[201~`
//! produced by `oriterm_core::paste::prepare_paste` at
//! `oriterm_core/src/paste/mod.rs:11-14` — this submodule pins
//! the outbound bytes against the cap declaration.

use crate::paste::prepare_paste;
use crate::term::TermMode;

use super::super::test_helpers::{feed, term_with_recorder};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["BD", "BE", "PS", "PE"];

#[test]
fn tack_cap_xcheck_be_enters_bracketed_paste() {
    assert_cap_declared("BE");
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[?2004h");
    assert!(
        term.mode().contains(TermMode::BRACKETED_PASTE),
        "DECSET 2004 must set TermMode::BRACKETED_PASTE"
    );
}

#[test]
fn tack_cap_xcheck_bd_exits_bracketed_paste() {
    assert_cap_declared("BD");
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[?2004h");
    feed(&mut term, b"\x1b[?2004l");
    assert!(
        !term.mode().contains(TermMode::BRACKETED_PASTE),
        "DECRST 2004 must clear TermMode::BRACKETED_PASTE"
    );
}

#[test]
fn tack_cap_xcheck_bracketed_paste_idempotent_on() {
    let (mut term, _l) = term_with_recorder();
    feed(&mut term, b"\x1b[?2004h\x1b[?2004h");
    assert!(term.mode().contains(TermMode::BRACKETED_PASTE));
    // Single DECRST clears it cleanly — no "double-set" state.
    feed(&mut term, b"\x1b[?2004l");
    assert!(!term.mode().contains(TermMode::BRACKETED_PASTE));
}

#[test]
fn tack_cap_xcheck_be_cap_value_matches() {
    assert_cap_value_matches("BE", "\\E[?2004h");
}

#[test]
fn tack_cap_xcheck_bd_cap_value_matches() {
    assert_cap_value_matches("BD", "\\E[?2004l");
}

#[test]
fn tack_cap_xcheck_ps_outbound_marker_matches_terminfo() {
    assert_cap_declared("PS");
    // PS is an OUTBOUND marker produced by prepare_paste, NOT
    // an inbound escape sequence. Cross-reference the cap value
    // and assert prepare_paste emits the same byte sequence.
    assert_cap_value_matches("PS", "\\E[200~");
    let bytes = prepare_paste("marker", true, false);
    assert!(
        bytes.starts_with(b"\x1b[200~"),
        "prepare_paste(bracketed=true) must start with PS marker \
         \\x1b[200~; got {:?}",
        std::str::from_utf8(&bytes).unwrap_or("<invalid utf8>"),
    );
    assert!(
        bytes.ends_with(b"\x1b[201~"),
        "prepare_paste(bracketed=true) must end with PE marker \
         \\x1b[201~"
    );
}

#[test]
fn tack_cap_xcheck_pe_outbound_marker_matches_terminfo() {
    assert_cap_declared("PE");
    assert_cap_value_matches("PE", "\\E[201~");
    // Empty input → bytes = PS + PE only.
    let bytes = prepare_paste("", true, false);
    assert_eq!(bytes, b"\x1b[200~\x1b[201~");
}
