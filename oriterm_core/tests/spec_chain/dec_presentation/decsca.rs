//! DECSCA (CSI Ps " q) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECSCA`
//! Apex: EffectModeState
//!
//! DECSCA toggles the `char_protection` flag on Term and mirrors it
//! onto the cursor template so subsequent cell writes carry
//! `CellFlags::PROTECTED`. DECERA / DECSERA consumers land in §09A.6.

use oriterm_core::cell::CellFlags;
use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decsca_ps1_enables_protection() {
    let mut h = SpecHarness::with_size(5, 10);
    h.feed(b"\x1b[1\"q");
    assert!(h.term().char_protection());
}

#[test]
fn decsca_ps0_and_ps2_disable_protection() {
    let mut h = SpecHarness::with_size(5, 10);
    h.feed(b"\x1b[1\"q");
    h.feed(b"\x1b[0\"q");
    assert!(!h.term().char_protection());
    h.feed(b"\x1b[1\"q");
    h.feed(b"\x1b[2\"q");
    assert!(!h.term().char_protection());
}

#[test]
fn decsca_protection_propagates_to_subsequent_writes() {
    let mut h = SpecHarness::with_size(5, 10);
    h.feed(b"\x1b[1;1H"); // home
    h.feed(b"\x1b[1\"q"); // protect ON
    h.feed(b"AB");
    h.feed(b"\x1b[0\"q"); // protect OFF
    h.feed(b"C");

    let row = &h.term().grid()[Line(0)];
    assert!(row[Column(0)].flags.contains(CellFlags::PROTECTED));
    assert!(row[Column(1)].flags.contains(CellFlags::PROTECTED));
    assert!(!row[Column(2)].flags.contains(CellFlags::PROTECTED));
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
}

// NOTE: cross-subsection test — DECERA/DECSERA consumers of the
// PROTECTED flag land in §09A.6. Those tests ("protected cell →
// DECSERA → NOT erased", "unprotected cell → DECERA → IS erased")
// belong with the rect-op scenarios and are not duplicated here.
