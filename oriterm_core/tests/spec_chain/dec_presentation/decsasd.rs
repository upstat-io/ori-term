//! DECSASD (CSI Ps $ }) spec-chain scenario.
//!
//! Catalog row: `DECPRES-DECSASD`
//! Apex: EffectModeState
//!
//! `verified-with-deviation`: status line target is stored but ori_term
//! does not render a DEC status line.

use oriterm_test_support::spec_chain::SpecHarness;

/// Pins: DECSASD cycles `Term::active_status_display` between 0 (main)
/// and 1 (status line) per the Ps parameter — ori_term does not render
/// a status line, but the target is stored verbatim per
/// verified-with-deviation semantics. Covers 0 → 1 → 0 round-trip.
/// Anchor: catalog row `DECPRES-DECSASD`.
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
