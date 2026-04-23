//! DECSCL (CSI Pl;Pc " p) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECSCL`
//! Apex: EffectModeState
//!
//! DECSCL stores the conformance level + C1 mode as observable state
//! then triggers a DECSTR soft reset.

use oriterm_core::term::TermMode;
use oriterm_test_support::spec_chain::SpecHarness;

/// Pins: DECSCL stores the conformance level (Pl) and C1 7-bit/8-bit
/// mode (Pc) as observable `Term` state. Matrix covers VT200/7-bit and
/// VT300/8-bit so neither field aliases to a hardcoded constant.
/// Anchor: catalog row `DECPRES-DECSCL` (level + C1 mode).
#[test]
fn decscl_stores_level_and_c1_mode() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[62;1\"p"); // VT200, 7-bit C1
    assert_eq!(h.term().conformance_level(), 62);
    assert!(h.term().c1_7bit());
    h.feed(b"\x1b[63;0\"p"); // VT300, 8-bit C1
    assert_eq!(h.term().conformance_level(), 63);
    assert!(!h.term().c1_7bit());
}

/// Pins: DECSCL triggers a DECSTR soft reset — previously-set INSERT
/// mode is cleared after DECSCL. Guards against the
/// store-without-soft-reset regression. Anchor: catalog row
/// `DECPRES-DECSCL` (soft-reset side effect).
#[test]
fn decscl_triggers_soft_reset_clears_insert_mode() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[4h"); // SM: insert
    assert!(h.term().mode().contains(TermMode::INSERT));
    h.feed(b"\x1b[64;1\"p"); // DECSCL
    assert!(
        !h.term().mode().contains(TermMode::INSERT),
        "DECSCL must clear INSERT via DECSTR soft reset"
    );
}

/// Pins: DECSCL's soft reset also resets the DECSTBM scroll region to
/// full-screen — matrix companion to the INSERT-mode pin covering a
/// second piece of state DECSTR must clear. Anchor: catalog row
/// `DECPRES-DECSCL` (DECSTBM reset via soft-reset).
#[test]
fn decscl_resets_scroll_region() {
    let mut h = SpecHarness::with_size(24, 80);
    h.feed(b"\x1b[3;20r"); // DECSTBM
    h.feed(b"\x1b[64;1\"p"); // DECSCL — soft reset
    let region = h.term().grid().scroll_region();
    assert_eq!(region.start, 0);
    assert_eq!(region.end, h.term().grid().lines());
}
