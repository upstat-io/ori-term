//! DECSASD (CSI Ps $ }) spec-chain scenario.
//!
//! Catalog row: `DECPRES-DECSASD`
//! Apex: EffectModeState
//!
//! `verified-with-deviation`: status line target is stored but ori_term
//! does not render a DEC status line.

use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decsasd_stores_display_target() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[0$}"); // main display
    assert_eq!(h.term().active_status_display(), 0);
    h.feed(b"\x1b[1$}"); // status line
    assert_eq!(h.term().active_status_display(), 1);
    h.feed(b"\x1b[0$}"); // back to main
    assert_eq!(h.term().active_status_display(), 0);
}
