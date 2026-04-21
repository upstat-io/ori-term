//! DECERA (CSI Pt;Pl;Pb;Pr $ z) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECERA`
//! Apex: state-snapshot
//!
//! Erases every cell in the rectangle unconditionally. Does NOT honor
//! `CellFlags::PROTECTED` — that is DECSERA's job.

use oriterm_core::CellFlags;
use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decera_erases_all_cells_including_protected() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"\x1b[1;1H"); // home
    h.feed(b"\x1b[1\"q"); // PROTECTED on
    h.feed(b"AB");
    h.feed(b"\x1b[0\"q"); // PROTECTED off
    h.feed(b"C");
    h.feed(b"\x1b[1;1;1;5$z"); // DECERA row 1, cols 1..=5
    let row = &h.term().grid()[Line(0)];
    assert_eq!(row[Column(0)].ch, ' ');
    assert_eq!(row[Column(1)].ch, ' ');
    assert_eq!(row[Column(2)].ch, ' ');
    // DECERA erases even the protected flag (cells are reset).
    assert!(!row[Column(0)].flags.contains(CellFlags::PROTECTED));
}

#[test]
fn decera_zero_area_noop() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"ABCDE");
    h.feed(b"\x1b[3;5;1;1$z"); // top > bot
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
    assert_eq!(h.term().grid()[Line(0)][Column(4)].ch, 'E');
}

#[test]
fn decera_negative_pin_cells_outside_rect_unchanged() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"ABCDE\r\nFGHIJ\r\nKLMNO");
    h.feed(b"\x1b[1;2;2;3$z"); // rows 1..=2, cols 2..=3
    // Inside rect: erased.
    assert_eq!(h.term().grid()[Line(0)][Column(1)].ch, ' ');
    assert_eq!(h.term().grid()[Line(0)][Column(2)].ch, ' ');
    assert_eq!(h.term().grid()[Line(1)][Column(1)].ch, ' ');
    // Outside rect: preserved.
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
    assert_eq!(h.term().grid()[Line(0)][Column(3)].ch, 'D');
    assert_eq!(h.term().grid()[Line(2)][Column(0)].ch, 'K');
}

// ── DECSCA × DECERA cross-subsection behavioral pins (§09A.N) ─────────

/// Cross-subsection pin: `DECSCA Ps=2` (unprotect) → write → `DECERA`
/// erases the cell. Complements `decsera_decsca_protected_cell_not_erased`
/// to clamp the DECERA path — DECERA ignores protection entirely, so the
/// unprotected cell IS erased.
#[test]
fn decera_decsca_unprotected_is_erased() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"\x1b[1;1H\x1b[2\"qX\x1b[1;1;1;5$z"); // unprotect, write X, DECERA
    let row = &h.term().grid()[Line(0)];
    assert_eq!(row[Column(0)].ch, ' ');
    assert!(!row[Column(0)].flags.contains(CellFlags::PROTECTED));
}

/// Cross-subsection mixed-matrix pin: DECERA over mixed protected /
/// unprotected cells erases EVERY cell unconditionally. Pins the DECERA /
/// DECSERA semantic split — DECERA ignores protection, DECSERA honors it.
#[test]
fn decera_decsca_mixed_protection_all_erased() {
    let mut h = SpecHarness::with_size(3, 5);
    // Pattern: P U P U P.
    h.feed(b"\x1b[1;1H\x1b[1\"qA\x1b[0\"qB\x1b[1\"qC\x1b[0\"qD\x1b[1\"qE");
    h.feed(b"\x1b[1;1;1;5$z"); // DECERA whole row
    let row = &h.term().grid()[Line(0)];
    // All cells erased; DECERA does not honor PROTECTED.
    for col in 0..5 {
        assert_eq!(row[Column(col)].ch, ' ', "col {col} not erased");
        assert!(
            !row[Column(col)].flags.contains(CellFlags::PROTECTED),
            "col {col} still carries PROTECTED after DECERA"
        );
    }
}
