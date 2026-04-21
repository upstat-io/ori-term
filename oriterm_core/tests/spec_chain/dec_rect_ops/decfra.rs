//! DECFRA (CSI Pc;Pt;Pl;Pb;Pr $ x) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECFRA`
//! Apex: state-snapshot (grid cells mutated in place)
//!
//! Fills the rectangle with character Pc + current SGR. DECLRMM-aware,
//! clamps to [1, rows] × [1, cols], zero-area rectangles no-op.

use oriterm_core::CellFlags;
use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

#[test]
fn decfra_fills_rectangle_with_character() {
    let mut h = SpecHarness::with_size(5, 10);
    h.feed(b"\x1b[88;1;1;3;5$x"); // Pc=88 ('X'), rows 1..=3, cols 1..=5
    for line in 0..3 {
        for col in 0..5 {
            let cell = &h.term().grid()[Line(line)][Column(col)];
            assert_eq!(cell.ch, 'X', "line={line} col={col}");
            assert!(cell.flags.contains(CellFlags::DRAWN));
        }
    }
    // Cell just outside the rect must stay pristine.
    assert_eq!(h.term().grid()[Line(0)][Column(5)].ch, ' ');
    assert_eq!(h.term().grid()[Line(3)][Column(0)].ch, ' ');
}

#[test]
fn decfra_zero_area_rectangle_is_noop() {
    let mut h = SpecHarness::with_size(5, 10);
    h.feed(b"ABC");
    // top > bot → zero area.
    h.feed(b"\x1b[88;5;1;1;5$x");
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
    assert_eq!(h.term().grid()[Line(0)][Column(1)].ch, 'B');
    assert_eq!(h.term().grid()[Line(0)][Column(2)].ch, 'C');
}

#[test]
fn decfra_out_of_bounds_clamps_to_grid() {
    // Ask to fill rows 1..=999, cols 1..=999 on a 3×4 grid. Should
    // clamp to rows 1..=3, cols 1..=4 — equivalent to filling the
    // entire grid.
    let mut h_oob = SpecHarness::with_size(3, 4);
    h_oob.feed(b"\x1b[42;1;1;999;999$x"); // Pc='*'
    let mut h_ref = SpecHarness::with_size(3, 4);
    h_ref.feed(b"\x1b[42;1;1;3;4$x");
    for line in 0..3 {
        for col in 0..4 {
            assert_eq!(
                h_oob.term().grid()[Line(line)][Column(col)].ch,
                h_ref.term().grid()[Line(line)][Column(col)].ch,
            );
        }
    }
}

#[test]
fn decfra_declrmm_constrains_to_margins() {
    let mut h = SpecHarness::with_size(5, 10);
    // Enable DECLRMM (mode 69).
    h.feed(b"\x1b[?69h");
    // Set left/right margins to cols 2..=5 (1-based: 3..=6 in CSI).
    h.feed(b"\x1b[3;6s");
    // Fill rows 1..=3, cols 1..=10 with 'X' — clamps to cols 3..=6.
    h.feed(b"\x1b[88;1;1;3;10$x");
    // Col 0 (outside margin) untouched.
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, ' ');
    assert_eq!(h.term().grid()[Line(0)][Column(1)].ch, ' ');
    // Cols 2..=5 (margin band) filled.
    assert_eq!(h.term().grid()[Line(0)][Column(2)].ch, 'X');
    assert_eq!(h.term().grid()[Line(0)][Column(5)].ch, 'X');
    // Col 6 (outside margin) untouched.
    assert_eq!(h.term().grid()[Line(0)][Column(6)].ch, ' ');
}

#[test]
fn decfra_rejects_c0_control_chars() {
    let mut h = SpecHarness::with_size(3, 3);
    h.feed(b"ABC");
    // Pc=1 is a C0 control char (0x01, not in [0x20..=0x7E] or
    // [0xA0..=0xFF]) — per xterm the fill is silently dropped.
    h.feed(b"\x1b[1;1;1;3;3$x");
    // Grid unchanged.
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, 'A');
    assert_eq!(h.term().grid()[Line(0)][Column(1)].ch, 'B');
    assert_eq!(h.term().grid()[Line(0)][Column(2)].ch, 'C');
}

#[test]
fn decfra_pc_zero_defaults_to_space() {
    // Per xterm `charproc.c:5661`, Pc=0 falls back to space (0x20).
    let mut h = SpecHarness::with_size(3, 3);
    h.feed(b"ABC");
    h.feed(b"\x1b[0;1;1;1;3$x"); // Pc=0 → fill with space
    assert_eq!(h.term().grid()[Line(0)][Column(0)].ch, ' ');
    assert_eq!(h.term().grid()[Line(0)][Column(1)].ch, ' ');
    assert_eq!(h.term().grid()[Line(0)][Column(2)].ch, ' ');
}
