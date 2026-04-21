//! DECSSDT (CSI Ps $ ~) spec-chain scenario.
//!
//! Catalog row: `DECPRES-DECSSDT`
//! Apex: EffectModeState
//!
//! `verified-with-deviation`: status line type is stored but ori_term
//! does not render a DEC status line.

use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decssdt_stores_line_type() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[0$~"); // off
    assert_eq!(h.term().status_line_type(), 0);
    h.feed(b"\x1b[1$~"); // indicator
    assert_eq!(h.term().status_line_type(), 1);
    h.feed(b"\x1b[2$~"); // host-writable
    assert_eq!(h.term().status_line_type(), 2);
}
