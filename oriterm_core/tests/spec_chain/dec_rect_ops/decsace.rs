//! DECSACE (CSI Ps * x) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECSACE`
//! Apex: effect-mode-state (Term::ace_mode)

use oriterm_core::AceMode;
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decsace_ps2_enters_rectangle_mode() {
    let mut h = SpecHarness::new();
    assert_eq!(h.term().ace_mode(), AceMode::Stream);
    h.feed(b"\x1b[2*x");
    assert_eq!(h.term().ace_mode(), AceMode::Rectangle);
}

#[test]
fn decsace_ps0_and_ps1_select_stream_mode() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[2*x");
    assert_eq!(h.term().ace_mode(), AceMode::Rectangle);
    h.feed(b"\x1b[0*x");
    assert_eq!(h.term().ace_mode(), AceMode::Stream);
    h.feed(b"\x1b[2*x");
    h.feed(b"\x1b[1*x");
    assert_eq!(h.term().ace_mode(), AceMode::Stream);
}

#[test]
fn decsace_default_is_stream_and_ris_resets() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[2*x");
    assert_eq!(h.term().ace_mode(), AceMode::Rectangle);
    // RIS (ESC c) resets to stream.
    h.feed(b"\x1bc");
    assert_eq!(h.term().ace_mode(), AceMode::Stream);
}
