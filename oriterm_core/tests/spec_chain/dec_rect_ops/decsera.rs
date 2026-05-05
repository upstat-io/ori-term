//! DECSERA (CSI Pt;Pl;Pb;Pr $ {) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECSERA`
//! Apex: state-snapshot
//!
//! Selective erase — skips cells carrying `CellFlags::PROTECTED`.

use oriterm_core::CellFlags;
use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decsera_preserves_protected_cells() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"\x1b[1;1H");
    h.feed(b"\x1b[1\"q"); // PROTECTED on
    h.feed(b"AB");
    h.feed(b"\x1b[0\"q"); // PROTECTED off
    h.feed(b"CDE");
    h.feed(b"\x1b[1;1;1;5${"); // DECSERA row 1, cols 1..=5

    let row = &h.term().grid()[Line(0)];
    // Protected cells survive with content + flag.
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert!(row[Column(0)].flags.contains(CellFlags::PROTECTED));
    // Unprotected cells erased.
    assert_eq!(row[Column(2)].ch, ' ');
    assert_eq!(row[Column(4)].ch, ' ');
}

#[test]
fn decsera_zero_area_noop() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"ABC");
    h.feed(b"\x1b[1;5;1;1${"); // left > right
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
}

#[test]
fn decsera_negative_pin_cells_outside_rect_unchanged() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"ABCDE\r\nFGHIJ");
    h.feed(b"\x1b[1;2;1;3${"); // row 1, cols 2..=3
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
    assert_eq!(h.term().grid()[Line(0)][Column(1)].ch, ' '); // erased
    assert_eq!(h.term().grid()[Line(0)][Column(2)].ch, ' '); // erased
    assert_eq!(h.term().grid()[Line(0)][Column(3)].ch, 'D'); // outside
    assert_eq!(h.term().grid()[Line(1)][Column(1)].ch, 'G'); // row outside
}

// ── DECSCA × DECSERA cross-subsection behavioral pins (§09A.N) ────────

/// Cross-subsection pin: `DECSCA Ps=1` (protect) → write → `DECSERA` MUST
/// NOT erase the protected cell. Pairs with `decera_decsca_unprotected_is_erased`
/// in `decera.rs` to clamp the DECSCA/DECSERA/DECERA interaction from both
/// sides.
#[test]
fn decsera_decsca_protected_cell_not_erased() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"\x1b[1;1H\x1b[1\"qX\x1b[1;1;1;5${"); // protect, write X, DECSERA whole row
    let row = &h.term().grid()[Line(0)];
    // Positive pin: protected X survives.
    assert_eq!(row[Column(0)].ch, 'X');
    assert!(row[Column(0)].flags.contains(CellFlags::PROTECTED));
    // Regression guard: adjacent un-written cell still blank (and unprotected).
    assert_eq!(row[Column(4)].ch, ' ');
}

/// Cross-subsection mixed-matrix pin: alternating protected / unprotected
/// cells in one row; DECSERA must erase ONLY the unprotected subset. Proves
/// per-cell granularity of `CellFlags::PROTECTED`, not a row-level flag.
#[test]
fn decsera_decsca_mixed_protection_only_unprotected_erased() {
    let mut h = SpecHarness::with_size(3, 5);
    // Pattern: P U P U P  (P=protected, U=unprotected).
    h.feed(b"\x1b[1;1H\x1b[1\"qA\x1b[0\"qB\x1b[1\"qC\x1b[0\"qD\x1b[1\"qE");
    h.feed(b"\x1b[1;1;1;5${"); // DECSERA whole row
    let row = &h.term().grid()[Line(0)];
    // Protected cells preserved.
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(row[Column(4)].ch, 'E');
    assert!(row[Column(0)].flags.contains(CellFlags::PROTECTED));
    assert!(row[Column(2)].flags.contains(CellFlags::PROTECTED));
    assert!(row[Column(4)].flags.contains(CellFlags::PROTECTED));
    // Unprotected cells erased.
    assert_eq!(row[Column(1)].ch, ' ');
    assert_eq!(row[Column(3)].ch, ' ');
}
