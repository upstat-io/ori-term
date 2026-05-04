use super::Row;
use crate::cell::{Cell, CellFlags};
use crate::index::Column;

#[test]
fn new_row_has_correct_length_and_defaults() {
    let row = Row::new(80);
    assert_eq!(row.cols(), 80);
    assert_eq!(row.occ(), 0);
    assert!(row[Column(0)].is_empty());
    assert!(row[Column(79)].is_empty());
}

#[test]
fn writing_cell_updates_occ() {
    let mut row = Row::new(80);
    let mut cell = Cell::default();
    cell.ch = 'A';
    row.append(Column(5), &cell);
    assert_eq!(row.occ(), 6);
    assert_eq!(row[Column(5)].ch, 'A');
}

#[test]
fn reset_clears_and_resets_occ() {
    let mut row = Row::new(80);
    let mut cell = Cell::default();
    cell.ch = 'X';
    row.append(Column(10), &cell);
    assert_eq!(row.occ(), 11);

    row.reset(80, &Cell::default());
    assert_eq!(row.occ(), 0);
    assert!(row[Column(10)].is_empty());
}

#[test]
fn index_returns_correct_cell() {
    let mut row = Row::new(80);
    let mut cell = Cell::default();
    cell.ch = 'B';
    cell.flags = CellFlags::BOLD;
    row.append(Column(3), &cell);

    assert_eq!(row[Column(3)].ch, 'B');
    assert!(row[Column(3)].flags.contains(CellFlags::BOLD));
}

#[test]
fn index_mut_updates_occ() {
    let mut row = Row::new(80);
    row[Column(20)].ch = 'Z';
    // IndexMut bumps occ as an upper bound — it does not check emptiness.
    assert_eq!(row.occ(), 21);
}

#[test]
fn clear_range_resets_columns() {
    let mut row = Row::new(80);
    let mut cell = Cell::default();
    cell.ch = 'X';
    for i in 0..10 {
        row.append(Column(i), &cell);
    }
    assert_eq!(row.occ(), 10);

    row.clear_range(Column(3)..Column(7), &Cell::default());
    assert!(row[Column(3)].is_empty());
    assert!(row[Column(6)].is_empty());
    assert_eq!(row[Column(2)].ch, 'X');
    assert_eq!(row[Column(7)].ch, 'X');
}

#[test]
fn truncate_clears_from_column_to_end() {
    let mut row = Row::new(80);
    let mut cell = Cell::default();
    cell.ch = 'A';
    for i in 0..20 {
        row.append(Column(i), &cell);
    }
    assert_eq!(row.occ(), 20);

    row.truncate(Column(10), &Cell::default());
    assert_eq!(row.occ(), 10);
    assert_eq!(row[Column(9)].ch, 'A');
    assert!(row[Column(10)].is_empty());
}

#[test]
fn reset_bce_across_consecutive_resets() {
    use vte::ansi::Color;

    let color1 = Color::Indexed(1);
    let color2 = Color::Indexed(2);
    let tmpl1 = Cell::from(color1);
    let tmpl2 = Cell::from(color2);

    let mut row = Row::new(10);

    // First reset: bg=color1 -> all cells get color1, occ drops to 0.
    row.reset(10, &tmpl1);
    assert_eq!(row.occ(), 0);
    assert_eq!(row[Column(0)].bg, color1);
    assert_eq!(row[Column(9)].bg, color1);

    // Second reset with different bg: even though occ is 0, the BCE
    // guard must detect the bg mismatch and repaint all cells.
    row.reset(10, &tmpl2);
    assert_eq!(row.occ(), 0);
    assert_eq!(row[Column(0)].bg, color2);
    assert_eq!(row[Column(9)].bg, color2);
}

// --- Additional tests from reference repo gap analysis ---

#[test]
fn reset_resizes_row_larger() {
    let mut row = Row::new(10);
    assert_eq!(row.cols(), 10);
    row.reset(20, &Cell::default());
    assert_eq!(row.cols(), 20);
    assert_eq!(row.occ(), 0);
}

#[test]
fn reset_shrinks_row() {
    let mut row = Row::new(20);
    let mut cell = Cell::default();
    cell.ch = 'A';
    row.append(Column(15), &cell);
    row.reset(10, &Cell::default());
    assert_eq!(row.cols(), 10);
    assert_eq!(row.occ(), 0);
}

#[test]
fn clear_range_full_row() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'X';
    for i in 0..10 {
        row.append(Column(i), &cell);
    }
    row.clear_range(Column(0)..Column(10), &Cell::default());
    for i in 0..10 {
        assert!(row[Column(i)].is_empty(), "Column {i} not empty");
    }
}

#[test]
fn clear_range_with_bce() {
    use vte::ansi::Color;
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'X';
    for i in 0..10 {
        row.append(Column(i), &cell);
    }
    let template = Cell::from(Color::Indexed(1));
    row.clear_range(Column(3)..Column(7), &template);
    assert_eq!(row[Column(3)].bg, Color::Indexed(1));
    assert_eq!(row[Column(6)].bg, Color::Indexed(1));
    assert_eq!(row[Column(3)].ch, ' ');
    // Cells outside range untouched.
    assert_eq!(row[Column(2)].ch, 'X');
    assert_eq!(row[Column(7)].ch, 'X');
}

#[test]
fn truncate_at_col_zero_clears_entire_row() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'X';
    for i in 0..10 {
        row.append(Column(i), &cell);
    }
    row.truncate(Column(0), &Cell::default());
    assert_eq!(row.occ(), 0);
    for i in 0..10 {
        assert!(row[Column(i)].is_empty());
    }
}

#[test]
fn append_empty_cell_does_not_bump_occ() {
    let mut row = Row::new(10);
    row.append(Column(5), &Cell::default());
    assert_eq!(row.occ(), 0);
}

#[test]
fn row_equality() {
    let row1 = Row::new(10);
    let row2 = Row::new(10);
    assert_eq!(row1, row2);

    let mut row3 = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'A';
    row3.append(Column(0), &cell);
    assert_ne!(row1, row3);
}

#[test]
fn clear_range_bce_updates_occ() {
    use vte::ansi::Color;
    let mut row = Row::new(10);
    assert_eq!(row.occ(), 0);
    let template = Cell::from(Color::Indexed(1));
    row.clear_range(Column(3)..Column(7), &template);
    // BCE clear must bump occ to cover the dirty cells.
    assert!(
        row.occ() >= 7,
        "occ should cover BCE cells, got {}",
        row.occ()
    );
}

#[test]
fn clear_range_bce_survives_reset() {
    use vte::ansi::Color;
    let mut row = Row::new(10);
    let bce = Cell::from(Color::Indexed(1));
    row.clear_range(Column(3)..Column(7), &bce);
    // Reset with default template must clear the BCE cells.
    row.reset(10, &Cell::default());
    for i in 0..10 {
        assert!(
            row[Column(i)].is_empty(),
            "Column {i} not empty after reset"
        );
    }
}

#[test]
fn truncate_bce_updates_occ() {
    use vte::ansi::Color;
    let mut row = Row::new(10);
    let bce = Cell::from(Color::Indexed(1));
    row.truncate(Column(5), &bce);
    // BCE truncate should set occ to cover all dirty cells.
    assert_eq!(row.occ(), 10);
}

#[test]
fn clear_range_inverted_is_noop() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'A';
    row.append(Column(0), &cell);
    // Inverted range (start > end) should not panic or modify cells.
    row.clear_range(Column(7)..Column(3), &Cell::default());
    assert_eq!(row[Column(0)].ch, 'A');
}

#[test]
fn clear_range_start_beyond_row_is_noop() {
    let mut row = Row::new(10);
    // Start beyond row length should not panic.
    row.clear_range(Column(20)..Column(30), &Cell::default());
    assert_eq!(row.occ(), 0);
}

#[test]
fn truncate_beyond_row_is_noop() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'A';
    row.append(Column(0), &cell);
    // Column beyond row length should not panic.
    row.truncate(Column(20), &Cell::default());
    assert_eq!(row[Column(0)].ch, 'A');
}

#[test]
fn is_blank_true_for_default_row() {
    let row = Row::new(10);
    assert!(row.is_blank());
}

#[test]
fn is_blank_false_after_write() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'A';
    row.append(Column(0), &cell);
    assert!(!row.is_blank());
}

/// Regression: review round-1 F1. A row of cells with
/// ONLY `CellFlags::DRAWN` set (no visible content beyond the DRAWN
/// write-history bit) MUST still be `is_blank()` — DRAWN is the xterm
/// CHARDRAWN analog consumed by DECRQCRA, but `is_blank`/`content_len`
/// are visual-empty queries consumed by reflow at
/// `oriterm_core/src/grid/resize/mod.rs:222` (`count_trimmable_rows`)
/// and `:407` (content_len for reflow). Reflow must NOT treat app-
/// written plain-space rows differently from pristine rows.
#[test]
fn is_blank_true_for_drawn_only_cells() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.flags = CellFlags::DRAWN;
    for col in 0..10 {
        row.append(Column(col), &cell);
    }
    assert!(
        row.is_blank(),
        "DRAWN-only row MUST be is_blank — DRAWN is orthogonal to visual emptiness"
    );
}

/// Regression: review round-1 F1. `content_len` is
/// visual-empty only. A row of DRAWN-only cells has no visual content,
/// so `content_len() == 0`. Reflow relies on this to decide effective
/// row lengths (`resize/mod.rs:407`).
#[test]
fn content_len_zero_for_drawn_only_row() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.flags = CellFlags::DRAWN;
    for col in 0..10 {
        row.append(Column(col), &cell);
    }
    assert_eq!(
        row.content_len(),
        0,
        "DRAWN-only row has no visual content; content_len must be 0"
    );
}

/// Regression: review round-1 F1. A row mixing DRAWN-only
/// cells with a single visible char still reports content_len that
/// ignores the DRAWN-only cells. "A<space><space>" where the spaces
/// carry DRAWN: content_len should be 1 (just 'A'), NOT 3.
#[test]
fn content_len_ignores_drawn_only_trailing_cells() {
    let mut row = Row::new(10);
    let mut drawn_blank = Cell::default();
    drawn_blank.flags = CellFlags::DRAWN;
    let mut a_cell = Cell::default();
    a_cell.ch = 'A';
    a_cell.flags = CellFlags::DRAWN;
    row.append(Column(0), &a_cell);
    row.append(Column(1), &drawn_blank);
    row.append(Column(2), &drawn_blank);
    assert_eq!(
        row.content_len(),
        1,
        "content_len ignores DRAWN-only trailing blanks (they're visually empty)"
    );
}

#[test]
fn is_blank_true_after_reset() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'B';
    row.append(Column(3), &cell);
    row.reset(10, &Cell::default());
    assert!(row.is_blank());
}

#[test]
fn content_len_zero_for_empty_row() {
    let row = Row::new(10);
    assert_eq!(row.content_len(), 0);
}

#[test]
fn content_len_tracks_rightmost_nonempty_cell() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'A';
    row.append(Column(0), &cell);
    assert_eq!(row.content_len(), 1);

    cell.ch = 'Z';
    row.append(Column(7), &cell);
    assert_eq!(row.content_len(), 8);
}

#[test]
fn content_len_shrinks_after_clear() {
    let mut row = Row::new(10);
    let mut cell = Cell::default();
    cell.ch = 'X';
    row.append(Column(5), &cell);
    assert_eq!(row.content_len(), 6);

    row.clear_range(Column(5)..Column(6), &Cell::default());
    assert_eq!(row.content_len(), 0);
}

// ---- regression tests: DRAWN lifecycle on Row ----

/// `Row::reset` MUST copy DRAWN state from the template. A DRAWN-clear
/// template (the normal case — `Cell::default()` or `Cell::from(bg)`)
/// therefore wipes DRAWN from every cell in the row.
#[test]
fn row_reset_from_default_template_clears_drawn() {
    let mut row = Row::new(5);
    let mut drawn_cell = Cell::default();
    drawn_cell.ch = 'A';
    drawn_cell.flags = CellFlags::DRAWN | CellFlags::BOLD;
    row.append(Column(0), &drawn_cell);
    row.append(Column(1), &drawn_cell);
    row.append(Column(2), &drawn_cell);
    assert!(row[Column(0)].flags.contains(CellFlags::DRAWN));

    row.reset(5, &Cell::default());

    for col in 0..5 {
        assert!(
            !row[Column(col)].flags.contains(CellFlags::DRAWN),
            "row.reset(default) must clear DRAWN on col {col}"
        );
    }
}

/// `Row::clear_range` clears DRAWN on the cleared range when the
/// template is DRAWN-clear. Outside the range, DRAWN is preserved.
#[test]
fn row_clear_range_clears_drawn_on_range_only() {
    let mut row = Row::new(5);
    let mut drawn_cell = Cell::default();
    drawn_cell.ch = 'X';
    drawn_cell.flags = CellFlags::DRAWN;
    for col in 0..5 {
        row.append(Column(col), &drawn_cell);
    }

    row.clear_range(Column(1)..Column(4), &Cell::default());

    assert!(row[Column(0)].flags.contains(CellFlags::DRAWN));
    assert!(!row[Column(1)].flags.contains(CellFlags::DRAWN));
    assert!(!row[Column(2)].flags.contains(CellFlags::DRAWN));
    assert!(!row[Column(3)].flags.contains(CellFlags::DRAWN));
    assert!(row[Column(4)].flags.contains(CellFlags::DRAWN));
}

/// `Row::truncate` clears DRAWN from the cursor column onward.
#[test]
fn row_truncate_clears_drawn_from_col_onward() {
    let mut row = Row::new(5);
    let mut drawn_cell = Cell::default();
    drawn_cell.ch = 'Y';
    drawn_cell.flags = CellFlags::DRAWN;
    for col in 0..5 {
        row.append(Column(col), &drawn_cell);
    }

    row.truncate(Column(2), &Cell::default());

    assert!(row[Column(0)].flags.contains(CellFlags::DRAWN));
    assert!(row[Column(1)].flags.contains(CellFlags::DRAWN));
    assert!(!row[Column(2)].flags.contains(CellFlags::DRAWN));
    assert!(!row[Column(3)].flags.contains(CellFlags::DRAWN));
    assert!(!row[Column(4)].flags.contains(CellFlags::DRAWN));
}
