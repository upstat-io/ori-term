//! DECCRA (CSI Pts;Pls;Pbs;Prs;Pps;Ptd;Pld;Ppd $ v) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECCRA`
//! Apex: state-snapshot
//!
//! Copies a source rectangle to a destination. Source and destination
//! may overlap; implementation uses a scratch buffer to guarantee
//! copy-before-overwrite semantics.

use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn deccra_copies_non_overlapping_rect() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"ABCDE\r\nFGHIJ\r\nKLMNO");
    // Copy (1,1)-(1,3) ("ABC") to (3,3). `src_page`/`dst_page` = 1.
    h.feed(b"\x1b[1;1;1;3;1;3;3;1$v");
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
    assert_eq!(h.term().grid()[Line(2)][Column(2)].ch, 'A');
    assert_eq!(h.term().grid()[Line(2)][Column(3)].ch, 'B');
    assert_eq!(h.term().grid()[Line(2)][Column(4)].ch, 'C');
    // Regression guard: row 2 untouched.
    assert_eq!(h.term().grid()[Line(1)][Column(0)].ch, 'F');
}

#[test]
fn deccra_overlap_uses_copy_before_overwrite() {
    let mut h = SpecHarness::with_size(1, 5);
    h.feed(b"ABCDE");
    // Copy (1,2)-(1,4) = "BCD" to (1,1) — overlaps with source.
    h.feed(b"\x1b[1;2;1;4;1;1;1;1$v");
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'B');
    assert_eq!(h.term().grid()[Line(0)][Column(1)].ch, 'C');
    assert_eq!(h.term().grid()[Line(0)][Column(2)].ch, 'D');
    // Cols 3-4 stay untouched (fell outside destination).
    assert_eq!(h.term().grid()[Line(0)][Column(3)].ch, 'D');
    assert_eq!(h.term().grid()[Line(0)][Column(4)].ch, 'E');
}

#[test]
fn deccra_zero_area_source_noop() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"ABCDE");
    // src top=5, bot=1 → zero area.
    h.feed(b"\x1b[5;1;1;3;1;2;1;1$v");
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
    assert_eq!(h.term().grid()[Line(1)][Column(0)].ch, ' ');
}

#[test]
fn deccra_clips_dest_to_grid_bounds() {
    let mut h = SpecHarness::with_size(3, 5);
    h.feed(b"ABCDE\r\nFGHIJ\r\nKLMNO");
    // Copy (1,1)-(1,5) = "ABCDE" (5 cells wide) to (3,4). Destination
    // starts at col 4 (1-based) = col 3 (0-based); the source width of
    // 5 would extend to col 7 but the grid only has 5 cols, so cols
    // 4..=7 beyond the edge are dropped, leaving only "AB" at (3,4)-(3,5).
    h.feed(b"\x1b[1;1;1;5;1;3;4;1$v");
    assert_eq!(h.term().grid()[Line(2)][Column(3)].ch, 'A');
    assert_eq!(h.term().grid()[Line(2)][Column(4)].ch, 'B');
    // Cols 0..=2 of line 2 still "KLM".
    assert_eq!(h.term().grid()[Line(2)][Column(0)].ch, 'K');
    assert_eq!(h.term().grid()[Line(2)][Column(2)].ch, 'M');
}
