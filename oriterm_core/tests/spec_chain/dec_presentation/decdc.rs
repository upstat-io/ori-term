//! DECDC (CSI Ps ' ~) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECDC`
//! Apex: state-snapshot (grid cells mutated in place)
//!
//! Deletes `Ps` columns at the cursor column on every row in the
//! active scroll region; content shifts left, with blanks filling
//! the right edge of the DECLRMM band. Companion to DECIC.

use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::SpecHarness;

fn seed_abcdef(h: &mut SpecHarness) {
    for line in 0..5 {
        h.feed(format!("\x1b[{};1H", line + 1).as_bytes());
        h.feed(b"ABCDEF");
    }
    h.feed(b"\x1b[1;1H");
}

#[test]
fn decdc_deletes_columns_at_cursor_on_every_row() {
    let mut h = SpecHarness::with_size(5, 10);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;3H"); // cursor col 3 (0-based 2)
    h.feed(b"\x1b[2'~"); // DECDC count=2
    for line in 0..5 {
        let row = &h.term().grid()[Line(line)];
        assert_eq!(row[Column(0)].ch, 'A', "line={line}");
        assert_eq!(row[Column(1)].ch, 'B', "line={line}");
        // 'C' and 'D' deleted; 'E' and 'F' shift left.
        assert_eq!(row[Column(2)].ch, 'E', "line={line}");
        assert_eq!(row[Column(3)].ch, 'F', "line={line}");
        // Tail is blank (no content past col 5 before the shift).
        assert_eq!(row[Column(4)].ch, ' ', "line={line}");
    }
}

#[test]
fn decdc_at_last_column_erases_only_that_column() {
    let mut h = SpecHarness::with_size(3, 6);
    seed_abcdef(&mut h);
    // Cursor at col 6 (last column on a 6-col grid → 0-based 5).
    h.feed(b"\x1b[1;6H");
    h.feed(b"\x1b[1'~"); // DECDC count=1
    let row = &h.term().grid()[Line(0)];
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(row[Column(3)].ch, 'D');
    assert_eq!(row[Column(4)].ch, 'E');
    assert_eq!(row[Column(5)].ch, ' ');
}

#[test]
fn decdc_default_count_is_one() {
    let mut h = SpecHarness::with_size(3, 6);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[1;1H");
    h.feed(b"\x1b['~"); // DECDC no explicit count
    let row = &h.term().grid()[Line(0)];
    assert_eq!(row[Column(0)].ch, 'B');
    assert_eq!(row[Column(1)].ch, 'C');
}

#[test]
fn decdc_declrmm_constrains_to_margin_band() {
    let mut h = SpecHarness::with_size(3, 10);
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s"); // margins cols 2..=6 (0-based)
    h.feed(b"\x1b[1;4H"); // cursor col 3 (0-based)
    h.feed(b"\x1b[1'~"); // DECDC count=1
    let row = &h.term().grid()[Line(0)];
    // Cols 0..=1 outside band.
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    // Col 2 outside the delete (col index 2 < cursor col 3, but still
    // inside band — untouched because deletion is at cursor).
    assert_eq!(row[Column(2)].ch, 'C');
    // Delete at col 3: 'D' is removed; 'E' 'F' 'G' shift left by 1;
    // blank fills col 6 (right margin).
    assert_eq!(row[Column(3)].ch, 'E');
    assert_eq!(row[Column(4)].ch, 'F');
    assert_eq!(row[Column(5)].ch, 'G');
    assert_eq!(row[Column(6)].ch, ' ');
    // Cols 7..=9 outside band.
    assert_eq!(row[Column(7)].ch, 'H');
    assert_eq!(row[Column(8)].ch, 'I');
    assert_eq!(row[Column(9)].ch, 'J');
}

#[test]
fn decdc_cursor_outside_declrmm_band_is_noop() {
    let mut h = SpecHarness::with_size(3, 10);
    h.feed(b"\x1b[1;1H");
    h.feed(b"ABCDEFGHIJ");
    h.feed(b"\x1b[?69h");
    h.feed(b"\x1b[3;7s");
    h.feed(b"\x1b[1;1H"); // cursor outside band
    h.feed(b"\x1b[2'~"); // DECDC count=2
    let row = &h.term().grid()[Line(0)];
    for (col, ch) in "ABCDEFGHIJ".chars().enumerate() {
        assert_eq!(row[Column(col)].ch, ch, "col={col}");
    }
}

#[test]
fn decdc_negative_pin_leaves_other_rows_alone() {
    let mut h = SpecHarness::with_size(4, 6);
    seed_abcdef(&mut h);
    h.feed(b"\x1b[2;3r"); // scroll region 1..=2 (0-based)
    h.feed(b"\x1b[2;3H"); // cursor inside region
    h.feed(b"\x1b[1'~"); // DECDC count=1
    // Lines 0 and 3 unchanged.
    let row0 = &h.term().grid()[Line(0)];
    assert_eq!(row0[Column(0)].ch, 'A');
    assert_eq!(row0[Column(1)].ch, 'B');
    assert_eq!(row0[Column(2)].ch, 'C');
    let row3 = &h.term().grid()[Line(3)];
    assert_eq!(row3[Column(0)].ch, 'A');
    assert_eq!(row3[Column(1)].ch, 'B');
    assert_eq!(row3[Column(2)].ch, 'C');
    // Lines 1 and 2 deleted at col 2.
    let row1 = &h.term().grid()[Line(1)];
    assert_eq!(row1[Column(0)].ch, 'A');
    assert_eq!(row1[Column(1)].ch, 'B');
    assert_eq!(row1[Column(2)].ch, 'D');
    assert_eq!(row1[Column(3)].ch, 'E');
    assert_eq!(row1[Column(4)].ch, 'F');
    assert_eq!(row1[Column(5)].ch, ' ');
}
