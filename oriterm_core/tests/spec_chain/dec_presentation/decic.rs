//! DECIC (CSI Ps ' }) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECIC`
//! Apex: state-snapshot (grid cells mutated in place)
//!
//! Inserts `Ps` blank columns at the cursor column on every row in
//! the active scroll region (xterm `util.c:819 xtermColScroll`),
//! respecting DECLRMM horizontal clamping. Default `Ps = 1`.

use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

/// Seed five lines with "ABCDEF" at columns 0..=5, cursor home.
fn seed_abcdef(h: &mut SpecHarness) {
    for line in 0..5 {
        h.feed(format!("\x1b[{};1H", line + 1).as_bytes());
        h.feed(b"ABCDEF");
    }
    h.feed(b"\x1b[1;1H");
}

/// Pins: DECIC Ps=2 at column 3 inserts 2 blanks on EVERY row of the
/// scroll region; content from the cursor column shifts right.
/// Anchor: catalog row `DECPRES-DECIC` (basic multi-row insert).
#[test]
fn decic_inserts_columns_at_cursor_on_every_row() {
    let mut h = SpecHarness::with_size(5, 10);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;3H"); // cursor at col 3 (0-based 2)
    h.feed(b"\x1b[2'}"); // DECIC count=2
    for line in 0..5 {
        let row = &h.term().grid()[Line(line)];
        assert_eq!(row[Column(0)].ch, 'A', "line={line} col=0");
        assert_eq!(row[Column(1)].ch, 'B', "line={line} col=1");
        assert_eq!(row[Column(2)].ch, ' ', "line={line} col=2");
        assert_eq!(row[Column(3)].ch, ' ', "line={line} col=3");
        assert_eq!(row[Column(4)].ch, 'C', "line={line} col=4");
        assert_eq!(row[Column(5)].ch, 'D', "line={line} col=5");
    }
}

/// Pins: DECIC Ps=1 at column 0 inserts a single blank at the left edge
/// — proves the left-edge boundary is handled correctly rather than
/// short-circuiting to no-op. Anchor: catalog row `DECPRES-DECIC`
/// (column-zero boundary).
#[test]
fn decic_at_column_zero_inserts_at_left_edge() {
    let mut h = SpecHarness::with_size(3, 8);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;1H");
    h.feed(b"\x1b[1'}"); // DECIC count=1
    for line in 0..3 {
        let row = &h.term().grid()[Line(line)];
        assert_eq!(row[Column(0)].ch, ' ');
        assert_eq!(row[Column(1)].ch, 'A');
        assert_eq!(row[Column(2)].ch, 'B');
    }
}

/// Pins: DECIC with omitted Ps defaults to 1 — `CSI ' }` at col 1
/// inserts a single blank, shifting 'A' right. Anchor: catalog row
/// `DECPRES-DECIC` (default-count).
#[test]
fn decic_default_count_is_one() {
    let mut h = SpecHarness::with_size(3, 8);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;1H");
    h.feed(b"\x1b['}"); // DECIC no explicit count
    let row = &h.term().grid()[Line(0)];
    assert_eq!(row[Column(0)].ch, ' ');
    assert_eq!(row[Column(1)].ch, 'A');
}

/// Pins: DECIC under DECLRMM inserts only within the margin band —
/// cells outside the band are untouched, the rightmost band cell is
/// pushed off. Anchor: catalog row `DECPRES-DECIC` (DECLRMM
/// band-constrained).
#[test]
fn decic_declrmm_constrains_to_margin_band() {
    let mut h = SpecHarness::with_size(3, 10);
    // Fill row 0 with "ABCDEFGHIJ" (10 cols).
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    // Enable DECLRMM (mode 69), set margins cols 3..=7 (1-based).
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s");
    // DECOM moves cursor after DECSLRM? No — DECSLRM doesn't; move
    // explicitly to col 4 (1-based) within the band.
    h.feed(b"\x1b[1;4H");
    h.feed(b"\x1b[1'}"); // DECIC count=1
    let row = &h.term().grid()[Line(0)];
    // Cols 0..=1 outside band → untouched.
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    // Band is cols 2..=6. DECIC at col 3 inserts one blank; content
    // from col 3 onward shifts right; `G` at col 6 falls off the
    // right margin.
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(row[Column(3)].ch, ' ');
    assert_eq!(row[Column(4)].ch, 'D');
    assert_eq!(row[Column(5)].ch, 'E');
    assert_eq!(row[Column(6)].ch, 'F');
    // Cols 7..=9 outside band → untouched (`G` was discarded).
    assert_eq!(row[Column(7)].ch, 'H');
    assert_eq!(row[Column(8)].ch, 'I');
    assert_eq!(row[Column(9)].ch, 'J');
}

/// Negative pin: DECIC with the cursor OUTSIDE the DECLRMM band is a
/// no-op — every cell is byte-identical after the sequence.
/// Anchor: catalog row `DECPRES-DECIC` (cursor-outside-band no-op).
#[test]
fn decic_cursor_outside_declrmm_band_is_noop() {
    let mut h = SpecHarness::with_size(3, 10);
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s"); // margins cols 2..=6 (0-based)
    // Move cursor to col 1 (0-based 0), outside the band.
    h.feed(b"\x1b[1;1H");
    h.feed(b"\x1b[2'}"); // DECIC count=2 — must be no-op
    let row = &h.term().grid()[Line(0)];
    for (col, ch) in "ABCDEFGHIJ".chars().enumerate() {
        assert_eq!(row[Column(col)].ch, ch, "col={col}");
    }
}

/// Negative pin: rows outside the active scroll region (DECSTBM) are
/// NOT touched by DECIC — only rows inside the vertical region are
/// affected. Guards against inserting on all rows regardless of
/// DECSTBM. Anchor: catalog row `DECPRES-DECIC` (DECSTBM vertical-bound
/// respected).
#[test]
fn decic_negative_pin_leaves_other_rows_alone() {
    // Constrain the scroll region to a single line and verify DECIC
    // does NOT touch lines outside the region.
    let mut h = SpecHarness::with_size(4, 6);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[2;3r"); // DECSTBM: top=2 bot=3 (0-based 1..=2)
    h.feed(b"\x1b[2;3H"); // cursor row 2 col 3 (inside region)
    h.feed(b"\x1b[1'}"); // DECIC count=1
    // Lines 0 and 3 unchanged.
    let row0 = &h.term().grid()[Line(0)];
    assert_eq!(row0[Column(0)].ch, 'A');
    assert_eq!(row0[Column(1)].ch, 'B');
    assert_eq!(row0[Column(2)].ch, 'C');
    let row3 = &h.term().grid()[Line(3)];
    assert_eq!(row3[Column(0)].ch, 'A');
    assert_eq!(row3[Column(1)].ch, 'B');
    assert_eq!(row3[Column(2)].ch, 'C');
    // Lines 1 and 2 shifted right at col 2 by 1.
    let row1 = &h.term().grid()[Line(1)];
    assert_eq!(row1[Column(0)].ch, 'A');
    assert_eq!(row1[Column(1)].ch, 'B');
    assert_eq!(row1[Column(2)].ch, ' ');
    assert_eq!(row1[Column(3)].ch, 'C');
}
