//! DECFI (ESC 9) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECFI`
//! Apex: state-snapshot (grid cells mutated in place OR cursor moved)
//!
//! ESC 9 — Forward Index. Cursor at the right margin (or last column
//! when DECLRMM is inactive): delete a column at the left margin on
//! every row of the scroll region (scrolling the band left by one).
//! Otherwise: move cursor right by one column. Mirrors xterm
//! `util.c:803 xtermColIndex(toLeft=False)`.

use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

fn seed_abcdef(h: &mut SpecHarness) {
    for line in 0..3 {
        h.feed(format!("\x1b[{};1H", line + 1).as_bytes());
        h.feed(b"ABCDEF");
    }
    h.feed(b"\x1b[1;1H");
}

#[test]
fn decfi_not_at_margin_moves_cursor_right_only() {
    let mut h = SpecHarness::with_size(3, 10);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;3H"); // col 3 (0-based 2)
    h.feed(b"\x1b9"); // DECFI
    let row = &h.term().grid()[Line(0)];
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(h.term().grid().cursor().col().0, 3);
}

#[test]
fn decfi_at_last_column_scrolls_band_left() {
    let mut h = SpecHarness::with_size(3, 10);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;10H"); // col 10 (1-based) == col 9 (0-based) = last
    h.feed(b"\x1b9"); // DECFI
    for line in 0..3 {
        let row = &h.term().grid()[Line(line)];
        // Entire band shifts left by 1. 'A' was at col 0 — deleted.
        assert_eq!(row[Column(0)].ch, 'B', "line={line}");
        assert_eq!(row[Column(1)].ch, 'C', "line={line}");
        assert_eq!(row[Column(2)].ch, 'D', "line={line}");
        assert_eq!(row[Column(3)].ch, 'E', "line={line}");
        assert_eq!(row[Column(4)].ch, 'F', "line={line}");
        // Rest of the row is blank (cols 5..=9).
        assert_eq!(row[Column(5)].ch, ' ', "line={line}");
        assert_eq!(row[Column(9)].ch, ' ', "line={line}");
    }
    // Cursor stays at last column after the scroll branch (matches
    // xterm: `xtermColScroll` does not move the cursor).
    assert_eq!(h.term().grid().cursor().col().0, 9);
}

#[test]
fn decfi_at_right_margin_under_declrmm_scrolls_band_left() {
    let mut h = SpecHarness::with_size(3, 10);
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s"); // margins cols 2..=6 (0-based)
    h.feed(b"\x1b[1;7H"); // col 7 (1-based) == col 6 (0-based) = right margin
    h.feed(b"\x1b9"); // DECFI
    let row = &h.term().grid()[Line(0)];
    // Cols 0..=1 untouched (outside band).
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    // Band (cols 2..=6) scrolls left: 'C' drops off; 'D' 'E' 'F' 'G'
    // shift to cols 2..=5; col 6 blank.
    assert_eq!(row[Column(2)].ch, 'D');
    assert_eq!(row[Column(3)].ch, 'E');
    assert_eq!(row[Column(4)].ch, 'F');
    assert_eq!(row[Column(5)].ch, 'G');
    assert_eq!(row[Column(6)].ch, ' ');
    // Cols 7..=9 untouched.
    assert_eq!(row[Column(7)].ch, 'H');
    assert_eq!(row[Column(8)].ch, 'I');
    assert_eq!(row[Column(9)].ch, 'J');
}

#[test]
fn decfi_inside_declrmm_band_but_not_at_right_margin_moves_cursor() {
    let mut h = SpecHarness::with_size(3, 10);
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s");
    h.feed(b"\x1b[1;5H"); // col 5 (1-based) == col 4 (0-based) — inside, not at right margin
    h.feed(b"\x1b9"); // DECFI
    let row = &h.term().grid()[Line(0)];
    for (col, ch) in "ABCDEFGHIJ".chars().enumerate() {
        assert_eq!(row[Column(col)].ch, ch);
    }
    assert_eq!(h.term().grid().cursor().col().0, 5);
}

#[test]
fn decfi_negative_pin_other_rows_for_cursor_move_branch() {
    let mut h = SpecHarness::with_size(3, 6);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;3H"); // col 3 (1-based) == col 2 (0-based) — not at margin
    h.feed(b"\x1b9"); // DECFI
    for line in 0..3 {
        let row = &h.term().grid()[Line(line)];
        for (col, ch) in "ABCDEF".chars().enumerate() {
            assert_eq!(row[Column(col)].ch, ch, "line={line} col={col}");
        }
    }
}
