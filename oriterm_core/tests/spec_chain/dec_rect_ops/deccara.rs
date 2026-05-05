//! DECCARA (CSI Pt;Pl;Pb;Pr;Pm $ r) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECCARA`
//! Apex: state-snapshot
//!
//! Applies SGR `Pm` params to every cell in the rect. DECSACE governs
//! whether the extent is rectangular (Ps=2) or stream (Ps=0/1, default).

use oriterm_core::CellFlags;
use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn deccara_applies_bold_to_rectangle() {
    let mut h = SpecHarness::with_size(5, 10);
    h.feed(b"\x1b[2*x"); // DECSACE rectangle
    h.feed(b"\x1b[1;1;3;5;1$r"); // DECCARA rows 1..=3, cols 1..=5, SGR 1 (bold)
    for line in 0..3 {
        for col in 0..5 {
            assert!(
                h.term().grid()[Line(line)][Column(col)]
                    .flags
                    .contains(CellFlags::BOLD),
                "line={line} col={col}"
            );
        }
    }
    // Regression guard: cell outside rect not bold.
    assert!(
        !h.term().grid()[Line(0)][Column(5)]
            .flags
            .contains(CellFlags::BOLD)
    );
    assert!(
        !h.term().grid()[Line(3)][Column(0)]
            .flags
            .contains(CellFlags::BOLD)
    );
}

#[test]
fn deccara_stream_mode_wraps_across_rows() {
    let mut h = SpecHarness::with_size(5, 5);
    h.feed(b"\x1b[0*x"); // DECSACE stream (default)
    // Rect (1,2)-(3,3) in stream mode: row 1 cols 2..=4, row 2 cols
    // 0..=4, row 3 cols 0..=3.
    h.feed(b"\x1b[1;2;3;3;1$r");
    // Row 1: col 0 not touched, col 1 not touched, col 2 onward bold.
    assert!(
        !h.term().grid()[Line(0)][Column(0)]
            .flags
            .contains(CellFlags::BOLD)
    );
    assert!(
        h.term().grid()[Line(0)][Column(1)]
            .flags
            .contains(CellFlags::BOLD)
    );
    assert!(
        h.term().grid()[Line(0)][Column(4)]
            .flags
            .contains(CellFlags::BOLD)
    );
    // Row 2 (middle): full-row bold.
    assert!(
        h.term().grid()[Line(1)][Column(0)]
            .flags
            .contains(CellFlags::BOLD)
    );
    assert!(
        h.term().grid()[Line(1)][Column(4)]
            .flags
            .contains(CellFlags::BOLD)
    );
    // Row 3: col 0..=2 bold, col 3 not bold (right = 3 in 1-based → 0-based col 2).
    // Actually right = 3 means cols 0..=2 inclusive in 0-based.
    assert!(
        h.term().grid()[Line(2)][Column(0)]
            .flags
            .contains(CellFlags::BOLD)
    );
    assert!(
        h.term().grid()[Line(2)][Column(2)]
            .flags
            .contains(CellFlags::BOLD)
    );
    assert!(
        !h.term().grid()[Line(2)][Column(3)]
            .flags
            .contains(CellFlags::BOLD)
    );
}

#[test]
fn deccara_zero_area_noop() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"\x1b[5;5;1;1;1$r"); // top>bot
    // No cell gains BOLD.
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
