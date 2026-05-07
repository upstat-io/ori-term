//! DECBI (ESC 6) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECBI`
//! Apex: state-snapshot (grid cells mutated in place OR cursor moved)
//!
//! ESC 6 — Back Index. Cursor at the left margin (or column 0 when
//! DECLRMM is inactive): insert a blank column at the left margin on
//! every row of the scroll region. Otherwise: move cursor left by
//! one column. Matches xterm `util.c:788 xtermColIndex(toLeft=True)`.

use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

fn seed_abcdef(h: &mut SpecHarness) {
    for line in 0..3 {
        h.feed(format!("\x1b[{};1H", line + 1).as_bytes());
        h.feed(b"ABCDEF");
    }
    h.feed(b"\x1b[1;1H");
}

/// Pins: DECBI with the cursor NOT at the left margin moves the cursor
/// left by one column and leaves every grid cell unchanged — matches
/// the xterm `util.c:788 xtermColIndex(toLeft=True)` cursor-move branch.
/// Anchor: catalog row `DECPRES-DECBI` (move-branch).
#[test]
fn decbi_not_at_margin_moves_cursor_left_only() {
    let mut h = SpecHarness::with_size(3, 10);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;3H"); // col 3 (0-based 2)
    h.feed(b"\x1b6"); // DECBI
    // Content unchanged.
    let row = &h.term().grid()[Line(0)];
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
    // Cursor moved left by 1.
    assert_eq!(h.term().grid().cursor().col().0, 1);
}

/// Pins: DECBI with cursor at column 0 (no DECLRMM) inserts a blank at
/// column 0 on every row of the scroll region, shifting existing content
/// right by one column; cursor stays put (insert branch per xterm
/// `util.c:795`). Anchor: catalog row `DECPRES-DECBI` (insert-branch).
#[test]
fn decbi_at_column_zero_inserts_blank_column_on_every_row() {
    let mut h = SpecHarness::with_size(3, 10);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;1H"); // col 0 — left bound
    h.feed(b"\x1b6"); // DECBI
    for line in 0..3 {
        let row = &h.term().grid()[Line(line)];
        // Column 0 is now blank; original content shifted right.
        assert_eq!(row[Column(0)].ch, ' ', "line={line}");
        assert_eq!(row[Column(1)].ch, 'A', "line={line}");
        assert_eq!(row[Column(2)].ch, 'B', "line={line}");
        assert_eq!(row[Column(3)].ch, 'C', "line={line}");
        assert_eq!(row[Column(4)].ch, 'D', "line={line}");
        assert_eq!(row[Column(5)].ch, 'E', "line={line}");
        assert_eq!(row[Column(6)].ch, 'F', "line={line}");
    }
    // Cursor stays at col 0 (did NOT move — this branch inserts, not
    // moves — per xterm `util.c:795`).
    assert_eq!(h.term().grid().cursor().col().0, 0);
}

/// Pins: with DECLRMM active and cursor at the DECSLRM left margin,
/// DECBI inserts a blank AT THE LEFT MARGIN (not column 0) and shifts
/// cells right within the margin band only — cells outside the band are
/// untouched, and the rightmost cell of the band is pushed off.
/// Anchor: catalog row `DECPRES-DECBI` (insert-at-DECSLRM-margin branch).
#[test]
fn decbi_at_left_margin_under_declrmm_inserts_at_margin() {
    let mut h = SpecHarness::with_size(3, 10);
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s"); // margins cols 2..=6 (0-based)
    h.feed(b"\x1b[1;3H"); // cursor at col 3 (1-based) == col 2 (0-based) = left margin
    h.feed(b"\x1b6"); // DECBI
    let row = &h.term().grid()[Line(0)];
    // Cols 0..=1 untouched (outside band).
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    // Col 2 (left margin) now blank; content shifts right inside band.
    assert_eq!(row[Column(2)].ch, ' ');
    assert_eq!(row[Column(3)].ch, 'C');
    assert_eq!(row[Column(4)].ch, 'D');
    assert_eq!(row[Column(5)].ch, 'E');
    assert_eq!(row[Column(6)].ch, 'F');
    // Cols 7..=9 untouched (outside band); 'G' was pushed off the
    // right margin.
    assert_eq!(row[Column(7)].ch, 'H');
    assert_eq!(row[Column(8)].ch, 'I');
    assert_eq!(row[Column(9)].ch, 'J');
}

/// Pins: with DECLRMM active and cursor inside the margin band but not
/// at the left margin, DECBI takes the cursor-move branch (not the
/// insert branch) — proves the decision is "at left margin?" not
/// "inside band?". Anchor: catalog row `DECPRES-DECBI` (inside-band-not-at-margin).
#[test]
fn decbi_inside_declrmm_band_but_not_at_left_margin_moves_cursor() {
    let mut h = SpecHarness::with_size(3, 10);
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s"); // margins 2..=6
    h.feed(b"\x1b[1;5H"); // col 5 (1-based) == col 4 (0-based) — inside, not at left margin
    h.feed(b"\x1b6"); // DECBI
    // Content unchanged.
    let row = &h.term().grid()[Line(0)];
    for (col, ch) in "ABCDEFGHIJ".chars().enumerate() {
        assert_eq!(row[Column(col)].ch, ch);
    }
    // Cursor moved left by 1.
    assert_eq!(h.term().grid().cursor().col().0, 3);
}

/// Regression guard: the cursor-move branch must NOT touch any grid cell —
/// a DECBI that takes the move path leaves every row's content byte-
/// identical. Guards against a regression where the move branch
/// accidentally calls the insert path on one row.
/// Anchor: catalog row `DECPRES-DECBI` (move-branch grid-immutable).
#[test]
fn decbi_negative_pin_other_rows_only_for_insert_branch() {
    // Non-insert branch (cursor move) must NOT touch any cell.
    let mut h = SpecHarness::with_size(3, 6);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;4H"); // col 4 (1-based) == col 3 (0-based)
    h.feed(b"\x1b6"); // DECBI
    for line in 0..3 {
        let row = &h.term().grid()[Line(line)];
        for (col, ch) in "ABCDEF".chars().enumerate() {
            assert_eq!(row[Column(col)].ch, ch, "line={line} col={col}");
        }
    }
}
