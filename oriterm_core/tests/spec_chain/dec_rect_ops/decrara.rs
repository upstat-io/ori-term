//! DECRARA (CSI Pt;Pl;Pb;Pr;Pm $ t) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECRARA`
//! Apex: state-snapshot
//!
//! Toggles SGR attributes on every cell in the rectangle.

use oriterm_core::CellFlags;
use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decrara_toggles_bold_flag() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"\x1b[2*x"); // rectangle mode
    h.feed(b"\x1b[1;1;2;3;1$t"); // toggle BOLD rows 1..=2, cols 1..=3
    assert!(
        h.term().grid()[Line(0)][Column(0)]
            .flags
            .contains(CellFlags::BOLD)
    );
    assert!(
        h.term().grid()[Line(1)][Column(2)]
            .flags
            .contains(CellFlags::BOLD)
    );
    // Outside rect untouched.
    assert!(
        !h.term().grid()[Line(0)][Column(3)]
            .flags
            .contains(CellFlags::BOLD)
    );

    // Second DECRARA on same rect: toggle back.
    h.feed(b"\x1b[1;1;2;3;1$t");
    assert!(
        !h.term().grid()[Line(0)][Column(0)]
            .flags
            .contains(CellFlags::BOLD)
    );
}

#[test]
fn decrara_ps0_toggles_all_reversible() {
    let mut h = SpecHarness::with_size(2, 3);
    h.feed(b"\x1b[2*x");
    // Ps=0 toggles every reversible bit (BOLD | UNDERLINE | BLINK |
    // INVERSE | HIDDEN).
    h.feed(b"\x1b[1;1;2;3;0$t");
    let cell = &h.term().grid()[Line(0)][Column(0)];
    assert!(cell.flags.contains(CellFlags::BOLD));
    assert!(cell.flags.contains(CellFlags::UNDERLINE));
    assert!(cell.flags.contains(CellFlags::BLINK));
    assert!(cell.flags.contains(CellFlags::INVERSE));
    assert!(cell.flags.contains(CellFlags::HIDDEN));
}

#[test]
fn decrara_zero_area_noop() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"\x1b[5;5;1;1;1$t");
    for line in 0..3 {
        for col in 0..5 {
            assert!(
                !h.term().grid()[Line(line)][Column(col)]
                    .flags
                    .contains(CellFlags::BOLD)
            );
        }
    }
}
