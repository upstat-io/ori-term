use super::{DisplayEraseMode, LineEraseMode};
use crate::grid::Grid;
use crate::index::Column;

/// Helper: create a grid and write a string of ASCII chars.
fn grid_with_text(lines: usize, cols: usize, text: &str) -> Grid {
    let mut grid = Grid::new(lines, cols);
    for ch in text.chars() {
        grid.put_char(ch);
    }
    grid
}

#[test]
fn put_char_writes_and_advances() {
    let mut grid = Grid::new(24, 80);
    grid.put_char('A');
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn put_char_wide_writes_pair() {
    let mut grid = Grid::new(24, 80);
    grid.put_char('\u{597d}');
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, '\u{597d}');
    assert!(grid[line][Column(0)].flags.contains(CellFlags::WIDE_CHAR));
    assert!(
        grid[line][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
    assert_eq!(grid.cursor().col(), Column(2));
}

#[test]
fn wide_char_at_last_column_wraps() {
    let mut grid = Grid::new(24, 5);
    // Fill columns 0..4 with 'A', cursor at col 4.
    for _ in 0..4 {
        grid.put_char('A');
    }
    assert_eq!(grid.cursor().col(), Column(4));
    // Writing a wide char at col 4 should wrap to next line.
    grid.put_char('\u{597d}');
    assert_eq!(grid.cursor().line(), 1);
    assert_eq!(grid.cursor().col(), Column(2));
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, '\u{597d}');
}

#[test]
fn overwrite_spacer_clears_wide_char() {
    let mut grid = Grid::new(24, 80);
    grid.put_char('\u{597d}');
    // Now cursor is at col 2. Move cursor to col 1 (the spacer).
    grid.cursor_mut().set_col(Column(1));
    grid.put_char('X');
    let line = crate::index::Line(0);
    // The wide char at col 0 should be cleared.
    assert_eq!(grid[line][Column(0)].ch, ' ');
    assert!(!grid[line][Column(0)].flags.contains(CellFlags::WIDE_CHAR));
    assert_eq!(grid[line][Column(1)].ch, 'X');
}

#[test]
fn overwrite_wide_char_clears_spacer() {
    let mut grid = Grid::new(24, 80);
    grid.put_char('\u{597d}');
    // Move cursor back to col 0 (the wide char).
    grid.cursor_mut().set_col(Column(0));
    grid.put_char('Y');
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, 'Y');
    // The spacer at col 1 should be cleared.
    assert_eq!(grid[line][Column(1)].ch, ' ');
    assert!(
        !grid[line][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

/// Regression: (notcurses-demo mojibake) — after a span
/// covered by a wide-char (emoji) is rewritten with narrow chars, every
/// narrow-char cell must carry its own character. Simulates a stdplane
/// row getting covered by an emoji-plane composite then re-rendered
/// back to stdplane text after the emoji plane scrolls away.
///
/// The notcurses compositor emits: (1) row-wide narrow text, (2)
/// emoji-overlay row (wide chars + spacers + narrow text fill), (3)
/// row-wide narrow text again. ori_term MUST leave no "holes" in the
/// final re-render.
#[test]
fn mojibake_emoji_overlay_then_redraw_leaves_no_holes() {
    let mut grid = Grid::new(24, 20);

    // Phase 1: fill row 0 with "relative to the ....." (stdplane content).
    for ch in "relative to the xxxx".chars() {
        grid.put_char(ch);
    }
    let line = crate::index::Line(0);
    for col in 0..20 {
        let expected = "relative to the xxxx".chars().nth(col).unwrap();
        assert_eq!(grid[line][Column(col)].ch, expected, "phase 1 col {col}");
    }

    // Phase 2: emoji-plane overlay — cursor back to col 0, write 4 wide
    // emojis (cols 0..8) + " - food" narrow label filling cols 8..15 +
    // no-op on cols 15..20 (plane ends at col 15, stdplane continues).
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(0));
    grid.put_char('\u{1F347}'); // 🍇 grape, width 2
    grid.put_char('\u{1F34F}'); // 🍏 green apple, width 2
    grid.put_char('\u{1F34A}'); // 🍊 orange, width 2
    grid.put_char('\u{1F34B}'); // 🍋 lemon, width 2
    for ch in " - food".chars() {
        grid.put_char(ch);
    }
    // Cols 15..20 still hold "xxxx" (unchanged by overlay).
    assert_eq!(grid[line][Column(15)].ch, ' '); // space between "the" and "xxxx" actually
    // Actually: "relative to the xxxx" → cols 0='r', ..., 12='e', 13=' ',
    // 14='x', 15='x', 16='x', 17='x', 18=..., wait let me recount.

    // Phase 3: plane scrolls away — stdplane re-emits full row.
    // Cursor to (0, 0), write "relative to the xxxx" again.
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(0));
    for ch in "relative to the xxxx".chars() {
        grid.put_char(ch);
    }

    // Every cell must hold its expected narrow char — no leftover spacer
    // flags, no stuck emojis, no empty "holes."
    let expected: Vec<char> = "relative to the xxxx".chars().collect();
    for col in 0..expected.len() {
        let cell = &grid[line][Column(col)];
        assert_eq!(
            cell.ch, expected[col],
            "phase 3 col {col}: expected {:?} got {:?} (flags {:?})",
            expected[col], cell.ch, cell.flags
        );
        assert!(
            !cell.flags.contains(CellFlags::WIDE_CHAR),
            "col {col} still flagged WIDE_CHAR after narrow redraw"
        );
        assert!(
            !cell.flags.contains(CellFlags::WIDE_CHAR_SPACER),
            "col {col} still flagged WIDE_CHAR_SPACER after narrow redraw"
        );
    }
}

#[test]
fn insert_blank_shifts_right() {
    let mut grid = grid_with_text(24, 80, "ABCDE");
    grid.cursor_mut().set_col(Column(1));
    grid.insert_blank(3);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, 'A');
    assert_eq!(grid[line][Column(1)].ch, ' ');
    assert_eq!(grid[line][Column(2)].ch, ' ');
    assert_eq!(grid[line][Column(3)].ch, ' ');
    assert_eq!(grid[line][Column(4)].ch, 'B');
    assert_eq!(grid[line][Column(5)].ch, 'C');
}

#[test]
fn delete_chars_shifts_left() {
    let mut grid = grid_with_text(24, 80, "ABCDE");
    grid.cursor_mut().set_col(Column(1));
    grid.delete_chars(2);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, 'A');
    assert_eq!(grid[line][Column(1)].ch, 'D');
    assert_eq!(grid[line][Column(2)].ch, 'E');
    // Cells at right are blank.
    assert!(grid[line][Column(3)].is_empty());
}

#[test]
fn erase_display_below() {
    let mut grid = Grid::new(3, 10);
    // Fill all 3 lines with 'X'.
    for line in 0..3 {
        grid.cursor_mut().set_line(line);
        grid.cursor_mut().set_col(Column(0));
        for _ in 0..10 {
            grid.put_char('X');
        }
    }
    // Position cursor at line 1, col 5 and erase below.
    grid.cursor_mut().set_line(1);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_display(DisplayEraseMode::Below);
    let line0 = crate::index::Line(0);
    let line1 = crate::index::Line(1);
    let line2 = crate::index::Line(2);
    // Line 0 untouched.
    assert_eq!(grid[line0][Column(0)].ch, 'X');
    // Line 1: cols 0-4 untouched, 5+ erased.
    assert_eq!(grid[line1][Column(4)].ch, 'X');
    assert!(grid[line1][Column(5)].is_empty());
    // Line 2 fully erased.
    assert!(grid[line2][Column(0)].is_empty());
}

#[test]
fn erase_display_above() {
    let mut grid = Grid::new(3, 10);
    for line in 0..3 {
        grid.cursor_mut().set_line(line);
        grid.cursor_mut().set_col(Column(0));
        for _ in 0..10 {
            grid.put_char('X');
        }
    }
    grid.cursor_mut().set_line(1);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_display(DisplayEraseMode::Above);
    let line0 = crate::index::Line(0);
    let line1 = crate::index::Line(1);
    let line2 = crate::index::Line(2);
    // Line 0 fully erased.
    assert!(grid[line0][Column(0)].is_empty());
    // Line 1: 0-5 erased, 6+ untouched.
    assert!(grid[line1][Column(5)].is_empty());
    assert_eq!(grid[line1][Column(6)].ch, 'X');
    // Line 2 untouched.
    assert_eq!(grid[line2][Column(0)].ch, 'X');
}

#[test]
fn erase_display_all() {
    let mut grid = grid_with_text(3, 10, "AAAAAAAAAA");
    grid.erase_display(DisplayEraseMode::All);
    for line in 0..3 {
        for col in 0..10 {
            assert!(
                grid[crate::index::Line(line as i32)][Column(col)].is_empty(),
                "Cell ({line}, {col}) not empty"
            );
        }
    }
}

#[test]
fn erase_line_below() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_line(LineEraseMode::Right);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(4)].ch, 'E');
    assert!(grid[line][Column(5)].is_empty());
    assert!(grid[line][Column(9)].is_empty());
}

#[test]
fn erase_line_all() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_line(LineEraseMode::All);
    let line = crate::index::Line(0);
    for col in 0..10 {
        assert!(grid[line][Column(col)].is_empty());
    }
}

#[test]
fn erase_chars_no_shift() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(2));
    grid.erase_chars(5);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, 'A');
    assert_eq!(grid[line][Column(1)].ch, 'B');
    assert!(grid[line][Column(2)].is_empty());
    assert!(grid[line][Column(6)].is_empty());
    assert_eq!(grid[line][Column(7)].ch, 'H');
}

#[test]
fn erase_chars_default_bg_does_not_inflate_occ() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    // occ is 10 after writing 10 chars.
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_chars(3);
    // Erased [5..8) with default bg. occ should stay at 10
    // (cells beyond 8 are still dirty from the original write).
    let line = crate::index::Line(0);
    assert!(grid[line][Column(5)].is_empty());
    assert!(grid[line][Column(7)].is_empty());
    assert_eq!(grid[line][Column(8)].ch, 'I');
}

#[test]
fn wide_char_on_single_column_grid_does_not_hang() {
    let mut grid = Grid::new(3, 1);
    // Width-2 char can never fit in a 1-column grid. Must return
    // immediately without writing or looping.
    grid.put_char('\u{597d}');
    assert_eq!(grid.cursor().col(), Column(0));
    assert!(grid[crate::index::Line(0)][Column(0)].is_empty());
}

// --- Additional tests from reference repo gap analysis ---

#[test]
fn put_char_inherits_template_attributes() {
    use vte::ansi::Color;
    let mut grid = Grid::new(24, 80);
    grid.cursor_mut().template.fg = Color::Indexed(1);
    grid.cursor_mut().template.bg = Color::Indexed(2);
    grid.cursor_mut().template.flags = CellFlags::BOLD;
    grid.put_char('A');

    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'A');
    assert_eq!(cell.fg, Color::Indexed(1));
    assert_eq!(cell.bg, Color::Indexed(2));
    assert!(cell.flags.contains(CellFlags::BOLD));
}

#[test]
fn put_char_fills_row_and_wraps_to_next_line() {
    let mut grid = Grid::new(3, 5);
    for ch in "ABCDE".chars() {
        grid.put_char(ch);
    }
    // After filling row, cursor is at col 5 (pending wrap).
    assert_eq!(grid.cursor().col(), Column(5));
    assert_eq!(grid.cursor().line(), 0);

    // Writing another char triggers wrap to next line.
    grid.put_char('F');
    assert_eq!(grid.cursor().line(), 1);
    assert_eq!(grid.cursor().col(), Column(1));
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'F');
}

#[test]
fn put_char_sequence_fills_correctly() {
    let mut grid = Grid::new(24, 10);
    for ch in "ABCDEFGHIJ".chars() {
        grid.put_char(ch);
    }
    let line = crate::index::Line(0);
    for (i, ch) in "ABCDEFGHIJ".chars().enumerate() {
        assert_eq!(grid[line][Column(i)].ch, ch, "Column {i} mismatch");
    }
}

#[test]
fn insert_blank_at_end_of_line() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(9));
    grid.insert_blank(1);
    let line = crate::index::Line(0);
    // Last cell should be blank, 'J' shifted off the edge.
    assert!(grid[line][Column(9)].is_empty());
    assert_eq!(grid[line][Column(8)].ch, 'I');
}

#[test]
fn insert_blank_count_exceeds_remaining() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(5));
    grid.insert_blank(100);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, 'A');
    assert_eq!(grid[line][Column(4)].ch, 'E');
    for col in 5..10 {
        assert!(grid[line][Column(col)].is_empty(), "Column {col} not empty");
    }
}

#[test]
fn insert_blank_with_bce() {
    use vte::ansi::Color;
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(2));
    grid.cursor_mut().template.bg = Color::Indexed(3);
    grid.insert_blank(2);
    let line = crate::index::Line(0);
    // Inserted blanks should have the BCE background.
    assert_eq!(grid[line][Column(2)].bg, Color::Indexed(3));
    assert_eq!(grid[line][Column(3)].bg, Color::Indexed(3));
    assert_eq!(grid[line][Column(2)].ch, ' ');
}

#[test]
fn insert_blank_cursor_past_end_is_noop() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(10));
    grid.insert_blank(5);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(9)].ch, 'J');
}

#[test]
fn delete_chars_at_end_of_line() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(9));
    grid.delete_chars(1);
    let line = crate::index::Line(0);
    assert!(grid[line][Column(9)].is_empty());
    assert_eq!(grid[line][Column(8)].ch, 'I');
}

#[test]
fn delete_chars_count_exceeds_remaining() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(5));
    grid.delete_chars(100);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(4)].ch, 'E');
    for col in 5..10 {
        assert!(grid[line][Column(col)].is_empty(), "Column {col} not empty");
    }
}

#[test]
fn delete_chars_with_bce() {
    use vte::ansi::Color;
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(2));
    grid.cursor_mut().template.bg = Color::Indexed(5);
    grid.delete_chars(3);
    let line = crate::index::Line(0);
    // Shifted: col 2 now has 'F', col 3 has 'G', etc.
    assert_eq!(grid[line][Column(2)].ch, 'F');
    // Right edge filled with BCE cells.
    assert_eq!(grid[line][Column(7)].bg, Color::Indexed(5));
    assert_eq!(grid[line][Column(8)].bg, Color::Indexed(5));
    assert_eq!(grid[line][Column(9)].bg, Color::Indexed(5));
}

#[test]
fn delete_chars_cursor_past_end_is_noop() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(10));
    grid.delete_chars(5);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(9)].ch, 'J');
}

#[test]
fn erase_line_above() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_line(LineEraseMode::Left);
    let line = crate::index::Line(0);
    // Cols 0..=5 should be erased.
    for col in 0..=5 {
        assert!(grid[line][Column(col)].is_empty(), "Column {col} not empty");
    }
    // Cols 6..9 untouched.
    assert_eq!(grid[line][Column(6)].ch, 'G');
    assert_eq!(grid[line][Column(9)].ch, 'J');
}

#[test]
fn erase_chars_past_end_of_line() {
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(7));
    grid.erase_chars(100);
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(6)].ch, 'G');
    for col in 7..10 {
        assert!(grid[line][Column(col)].is_empty(), "Column {col} not empty");
    }
}

#[test]
fn erase_display_with_bce_background() {
    use vte::ansi::Color;
    let mut grid = Grid::new(3, 10);
    for line in 0..3 {
        grid.cursor_mut().set_line(line);
        grid.cursor_mut().set_col(Column(0));
        for _ in 0..10 {
            grid.put_char('X');
        }
    }
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(0));
    grid.cursor_mut().template.bg = Color::Indexed(6);
    grid.erase_display(DisplayEraseMode::All);
    // All cells should have the BCE background.
    for line in 0..3 {
        for col in 0..10 {
            assert_eq!(
                grid[crate::index::Line(line as i32)][Column(col)].bg,
                Color::Indexed(6),
                "Cell ({line}, {col}) bg mismatch"
            );
        }
    }
}

#[test]
fn erase_display_below_at_last_line() {
    let mut grid = grid_with_text(3, 10, "AAAAAAAAAA");
    grid.cursor_mut().set_line(2);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_display(DisplayEraseMode::Below);
    // Only line 2 from col 5 should be erased (line 2 was empty anyway).
    let line2 = crate::index::Line(2);
    assert!(grid[line2][Column(5)].is_empty());
    // Line 0 untouched.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
}

#[test]
fn erase_display_above_at_first_line() {
    let mut grid = grid_with_text(3, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_display(DisplayEraseMode::Above);
    let line0 = crate::index::Line(0);
    // Cols 0..=5 erased on line 0.
    assert!(grid[line0][Column(0)].is_empty());
    assert!(grid[line0][Column(5)].is_empty());
    // Cols 6+ untouched.
    assert_eq!(grid[line0][Column(6)].ch, 'G');
}

#[test]
fn wrap_flag_set_on_wrapped_line() {
    let mut grid = Grid::new(3, 5);
    for ch in "ABCDEF".chars() {
        grid.put_char(ch);
    }
    // The last cell of line 0 should have the WRAP flag.
    let line0 = crate::index::Line(0);
    assert!(grid[line0][Column(4)].flags.contains(CellFlags::WRAP));
}

#[test]
fn put_char_wide_spacer_inherits_template_bg() {
    use vte::ansi::Color;
    let mut grid = Grid::new(24, 80);
    grid.cursor_mut().template.bg = Color::Indexed(3);
    grid.put_char('\u{597d}');
    let line = crate::index::Line(0);
    // Wide char cell gets template bg.
    assert_eq!(grid[line][Column(0)].bg, Color::Indexed(3));
    // Spacer also gets template bg.
    assert_eq!(grid[line][Column(1)].bg, Color::Indexed(3));
}

#[test]
fn erase_chars_with_bce_background() {
    use vte::ansi::Color;
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(3));
    grid.cursor_mut().template.bg = Color::Indexed(7);
    grid.erase_chars(4);
    let line = crate::index::Line(0);
    // Erased cells [3..7) get BCE background.
    for col in 3..7 {
        assert_eq!(grid[line][Column(col)].bg, Color::Indexed(7));
        assert_eq!(grid[line][Column(col)].ch, ' ');
    }
    // Surrounding cells untouched.
    assert_eq!(grid[line][Column(2)].ch, 'C');
    assert_eq!(grid[line][Column(7)].ch, 'H');
}

#[test]
fn erase_line_below_with_bce() {
    use vte::ansi::Color;
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(5));
    grid.cursor_mut().template.bg = Color::Indexed(2);
    grid.erase_line(LineEraseMode::Right);
    let line = crate::index::Line(0);
    for col in 5..10 {
        assert_eq!(grid[line][Column(col)].bg, Color::Indexed(2));
    }
    // Cols before cursor untouched.
    assert_eq!(grid[line][Column(4)].ch, 'E');
}

// --- dirty tracking ---

/// Helper: create a grid and drain its dirty state so tests start clean.
fn clean_grid(lines: usize, cols: usize) -> Grid {
    let mut grid = Grid::new(lines, cols);
    grid.dirty_mut().drain().for_each(drop);
    grid
}

#[test]
fn put_char_marks_cursor_line_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(2);
    grid.cursor_mut().set_col(Column(0));
    grid.put_char('A');

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![2]);
}

#[test]
fn put_char_wraparound_marks_new_line_dirty() {
    let mut grid = clean_grid(5, 5);
    // Fill line 0 to trigger pending wrap.
    for ch in "ABCDE".chars() {
        grid.put_char(ch);
    }
    grid.dirty_mut().drain().for_each(drop);

    // This put_char triggers wrap to line 1.
    grid.put_char('F');
    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert!(dirty.contains(&1), "new line should be dirty: {dirty:?}");
}

#[test]
fn insert_blank_marks_cursor_line_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(3);
    grid.cursor_mut().set_col(Column(2));
    grid.insert_blank(3);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![3]);
}

#[test]
fn delete_chars_marks_cursor_line_dirty() {
    let mut grid = clean_grid(5, 10);
    // Write some content first.
    grid.put_char('A');
    grid.put_char('B');
    grid.put_char('C');
    grid.dirty_mut().drain().for_each(drop);

    grid.cursor_mut().set_col(Column(0));
    grid.delete_chars(1);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![0]);
}

#[test]
fn erase_chars_marks_cursor_line_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.put_char('A');
    grid.dirty_mut().drain().for_each(drop);

    grid.cursor_mut().set_col(Column(0));
    grid.erase_chars(5);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![0]);
}

#[test]
fn erase_line_below_marks_cursor_line_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(2);
    grid.cursor_mut().set_col(Column(3));
    grid.erase_line(LineEraseMode::Right);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![2]);
}

#[test]
fn erase_line_above_marks_cursor_line_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(2);
    grid.cursor_mut().set_col(Column(3));
    grid.erase_line(LineEraseMode::Left);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![2]);
}

#[test]
fn erase_line_all_marks_cursor_line_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(2);
    grid.erase_line(LineEraseMode::All);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![2]);
}

#[test]
fn erase_display_below_marks_cursor_and_below_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(2);
    grid.cursor_mut().set_col(Column(3));
    grid.erase_display(DisplayEraseMode::Below);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    // Cursor line (2) + lines below (3, 4).
    assert_eq!(dirty, vec![2, 3, 4]);
}

#[test]
fn erase_display_above_marks_above_and_cursor_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(2);
    grid.cursor_mut().set_col(Column(3));
    grid.erase_display(DisplayEraseMode::Above);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    // Lines above (0, 1) + cursor line (2).
    assert_eq!(dirty, vec![0, 1, 2]);
}

#[test]
fn erase_display_all_marks_all_dirty() {
    let mut grid = clean_grid(5, 10);
    grid.erase_display(DisplayEraseMode::All);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    assert_eq!(dirty, vec![0, 1, 2, 3, 4]);
}

#[test]
fn erase_display_below_does_not_dirty_lines_above() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(3);
    grid.cursor_mut().set_col(Column(0));
    grid.erase_display(DisplayEraseMode::Below);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    // Only lines 3 and 4.
    assert_eq!(dirty, vec![3, 4]);
}

#[test]
fn erase_display_above_does_not_dirty_lines_below() {
    let mut grid = clean_grid(5, 10);
    grid.cursor_mut().set_line(1);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_display(DisplayEraseMode::Above);

    let dirty: Vec<usize> = grid.dirty_mut().drain().map(|d| d.line).collect();
    // Only lines 0 and 1.
    assert_eq!(dirty, vec![0, 1]);
}

// --- Wide char boundary edge cases (tmux audit) ---

/// Helper: place a wide char at `(line, col)` in a grid with existing content.
fn grid_with_wide_at(lines: usize, cols: usize, fill: &str, wide_col: usize) -> Grid {
    let mut grid = grid_with_text(lines, cols, fill);
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(wide_col));
    grid.put_char('\u{597d}'); // CJK char, width 2
    grid
}

#[test]
fn insert_blank_splits_wide_char_at_cursor() {
    // Wide char at cols 4-5. Insert at col 5 (the spacer). The base
    // at col 4 should be cleared because its spacer is being shifted.
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(4));
    grid.put_char('\u{597d}'); // Wide char at cols 4-5
    grid.cursor_mut().set_col(Column(5));
    grid.insert_blank(1);

    let line = crate::index::Line(0);
    // The wide char at col 4 should be cleared (spacer was displaced).
    assert_eq!(grid[line][Column(4)].ch, ' ');
    assert!(!grid[line][Column(4)].flags.contains(CellFlags::WIDE_CHAR));
}

#[test]
fn insert_blank_wide_char_pushed_to_right_edge_clears() {
    // Wide char at cols 8-9 in a 10-col grid. Insert at col 0 pushes
    // the wide char base to col 9 (spacer falls off). Should clear.
    let mut grid = Grid::new(24, 10);
    grid.cursor_mut().set_col(Column(8));
    grid.put_char('\u{597d}'); // Wide char at cols 8-9
    grid.cursor_mut().set_col(Column(0));
    grid.insert_blank(1);

    let line = crate::index::Line(0);
    // Wide char base pushed to col 9, spacer off-screen. Base should
    // be cleared to a space without WIDE_CHAR flag.
    assert_eq!(grid[line][Column(9)].ch, ' ');
    assert!(!grid[line][Column(9)].flags.contains(CellFlags::WIDE_CHAR));
}

#[test]
fn delete_chars_at_wide_char_spacer_boundary() {
    // Wide char at cols 2-3. Cursor at col 2. Delete 1 char. The
    // spacer at col 3 is the first shifted position — its base (col 2)
    // is in the delete zone.
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(2));
    grid.put_char('\u{597d}'); // Wide char at cols 2-3
    grid.cursor_mut().set_col(Column(2));
    grid.delete_chars(1);

    let line = crate::index::Line(0);
    // The orphaned spacer at col 3 (now shifted to col 2) should be
    // cleaned up — no stale WIDE_CHAR_SPACER flag.
    assert!(
        !grid[line][Column(2)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

#[test]
fn delete_chars_removes_wide_char_leaves_spacer_orphan() {
    // Wide char at cols 4-5. Delete 2 chars at col 4. The delete range
    // covers the base; the spacer at col 5 is the first shifted cell.
    let mut grid = grid_with_text(24, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(4));
    grid.put_char('\u{597d}'); // Wide char at cols 4-5
    grid.cursor_mut().set_col(Column(4));
    grid.delete_chars(1);

    let line = crate::index::Line(0);
    // After delete: former spacer at col 5 shifts to col 4. It should
    // have been cleaned up (no stale spacer flag).
    assert!(
        !grid[line][Column(4)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER),
        "orphaned spacer should be cleaned up"
    );
}

#[test]
fn erase_line_right_splits_wide_char_at_start() {
    // Wide char at cols 4-5. Erase right from col 5 (the spacer).
    // The base at col 4 should be cleared since its spacer is erased.
    let mut grid = grid_with_wide_at(24, 10, "ABCDEFGHIJ", 4);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_line(LineEraseMode::Right);

    let line = crate::index::Line(0);
    // Base at col 4 should be cleared (its spacer was erased).
    assert_eq!(grid[line][Column(4)].ch, ' ');
    assert!(!grid[line][Column(4)].flags.contains(CellFlags::WIDE_CHAR));
    // Col 5 and beyond should be erased.
    assert!(grid[line][Column(5)].is_empty());
}

#[test]
fn erase_line_left_splits_wide_char_at_end() {
    // Wide char at cols 4-5. Erase left through col 4 (the base).
    // The spacer at col 5 should be cleared since its base is erased.
    let mut grid = grid_with_wide_at(24, 10, "ABCDEFGHIJ", 4);
    grid.cursor_mut().set_col(Column(4));
    grid.erase_line(LineEraseMode::Left);

    let line = crate::index::Line(0);
    // Spacer at col 5 should be cleared (base was erased).
    assert_eq!(grid[line][Column(5)].ch, ' ');
    assert!(
        !grid[line][Column(5)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

#[test]
fn erase_chars_splits_wide_char_at_start_boundary() {
    // Wide char at cols 4-5. Erase 3 chars starting at col 5 (the spacer).
    // The base at col 4 should be cleared.
    let mut grid = grid_with_wide_at(24, 10, "ABCDEFGHIJ", 4);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_chars(3);

    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(4)].ch, ' ');
    assert!(!grid[line][Column(4)].flags.contains(CellFlags::WIDE_CHAR));
    // Erased range is clean.
    assert!(grid[line][Column(5)].is_empty());
    assert!(grid[line][Column(6)].is_empty());
    assert!(grid[line][Column(7)].is_empty());
}

#[test]
fn erase_chars_splits_wide_char_at_end_boundary() {
    // Wide char at cols 6-7. Erase 3 chars starting at col 4 (ends at col 6,
    // which is the base of the wide char). The spacer at col 7 should be cleared.
    let mut grid = grid_with_wide_at(24, 10, "ABCDEFGHIJ", 6);
    grid.cursor_mut().set_col(Column(4));
    grid.erase_chars(3);

    let line = crate::index::Line(0);
    // Spacer at col 7 should be cleared (its base at col 6 was erased).
    assert_eq!(grid[line][Column(7)].ch, ' ');
    assert!(
        !grid[line][Column(7)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

#[test]
fn bce_erase_on_wide_char_spacer_inherits_bg() {
    use vte::ansi::Color;
    // Wide char at cols 4-5. Set BCE bg, erase the spacer region.
    let mut grid = grid_with_wide_at(24, 10, "ABCDEFGHIJ", 4);
    grid.cursor_mut().set_col(Column(5));
    grid.cursor_mut().template.bg = Color::Indexed(9);
    grid.erase_chars(1);

    let line = crate::index::Line(0);
    // The cleared base at col 4 should NOT get the BCE bg (it's outside
    // the erase range — fix_wide_boundaries clears orphaned halves).
    // The erased spacer at col 5 should get the BCE bg.
    assert_eq!(grid[line][Column(5)].bg, Color::Indexed(9));
}

#[test]
fn erase_chars_covers_entire_wide_char() {
    // Wide char at cols 4-5. Erase range [4..6) covers both halves.
    // Neither half should be orphaned.
    let mut grid = grid_with_wide_at(24, 10, "ABCDEFGHIJ", 4);
    grid.cursor_mut().set_col(Column(4));
    grid.erase_chars(2);

    let line = crate::index::Line(0);
    assert!(grid[line][Column(4)].is_empty());
    assert!(grid[line][Column(5)].is_empty());
    // No stale flags.
    assert!(!grid[line][Column(4)].flags.contains(CellFlags::WIDE_CHAR));
    assert!(
        !grid[line][Column(5)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

#[test]
fn insert_blank_between_consecutive_wide_chars() {
    // Two wide chars: cols 0-1 and 2-3. Insert 1 blank at col 2
    // (the base of the second wide char).
    let mut grid = Grid::new(24, 10);
    grid.put_char('\u{597d}'); // cols 0-1
    grid.put_char('\u{4f60}'); // cols 2-3
    grid.cursor_mut().set_col(Column(2));
    grid.insert_blank(1);

    let line = crate::index::Line(0);
    // First wide char at cols 0-1 should be untouched.
    assert!(grid[line][Column(0)].flags.contains(CellFlags::WIDE_CHAR));
    assert!(
        grid[line][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
    // Col 2 should be a blank (inserted).
    assert_eq!(grid[line][Column(2)].ch, ' ');
    assert!(!grid[line][Column(2)].flags.contains(CellFlags::WIDE_CHAR));
    assert!(
        !grid[line][Column(2)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

#[test]
fn delete_chars_between_consecutive_wide_chars() {
    // Two wide chars: cols 0-1 and 2-3, then 'E' at col 4.
    // Delete 2 at col 0 (removes first wide char entirely).
    // Second wide char should shift left to cols 0-1.
    let mut grid = Grid::new(24, 10);
    grid.put_char('\u{597d}'); // cols 0-1
    grid.put_char('\u{4f60}'); // cols 2-3
    grid.put_char('E');
    grid.cursor_mut().set_col(Column(0));
    grid.delete_chars(2);

    let line = crate::index::Line(0);
    // Second wide char shifted to cols 0-1.
    assert_eq!(grid[line][Column(0)].ch, '\u{4f60}');
    assert!(grid[line][Column(0)].flags.contains(CellFlags::WIDE_CHAR));
    assert!(
        grid[line][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
    assert_eq!(grid[line][Column(2)].ch, 'E');
}

// ── Snapshot tests (insta) ──────────────────────────────────────────

#[test]
fn snapshot_insert_blank_shifts_content() {
    let mut grid = grid_with_text(3, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(3));
    grid.insert_blank(2);

    insta::assert_snapshot!(grid.snapshot(), @r"
    [Grid 3x10 cursor=(0,3)]
    |ABC  DEFGH|
    |          |
    |          |
    ");
}

#[test]
fn snapshot_delete_chars_removes_content() {
    let mut grid = grid_with_text(3, 10, "ABCDEFGHIJ");
    grid.cursor_mut().set_col(Column(2));
    grid.delete_chars(3);

    insta::assert_snapshot!(grid.snapshot(), @r"
    [Grid 3x10 cursor=(0,2)]
    |ABFGHIJ   |
    |          |
    |          |
    ");
}

#[test]
fn snapshot_erase_display_below() {
    let mut grid = Grid::new(3, 10);
    for line in 0..3 {
        grid.cursor_mut().set_line(line);
        grid.cursor_mut().set_col(Column(0));
        for ch in "XXXXXXXXXX".chars() {
            grid.put_char(ch);
        }
    }
    grid.cursor_mut().set_line(1);
    grid.cursor_mut().set_col(Column(5));
    grid.erase_display(DisplayEraseMode::Below);

    insta::assert_snapshot!(grid.snapshot(), @r"
    [Grid 3x10 cursor=(1,5)]
    |XXXXXXXXXX|
    |XXXXX     |
    |          |
    ");
}

#[test]
fn snapshot_wide_char_put_and_wrap() {
    let mut grid = Grid::new(3, 6);
    grid.put_char('A');
    grid.put_char('B');
    grid.put_char('\u{4e16}'); // Wide: cols 2-3.
    grid.put_char('\u{754c}'); // Wide: cols 4-5 — fits exactly.

    insta::assert_snapshot!(grid.snapshot(), @r"
    [Grid 3x6 cursor=(0,6)]
    |AB世_界_|
    |      |
    |      |
    ");

    // Now write another wide char — wraps to next line.
    grid.put_char('\u{597d}');

    insta::assert_snapshot!(grid.snapshot(), @r"
    [Grid 3x6 cursor=(1,2)]
    |AB世_界_+
    |好_    |
    |      |
    ");
}

// INSERT mode damage tracking tests.

#[test]
fn insert_blank_then_put_char_damages_cursor_to_right_edge() {
    // Simulates INSERT mode: insert_blank shifts cells right, then put_char writes.
    // Combined damage should cover [col, cols-1] (the full shifted region).
    let mut grid = grid_with_text(3, 10, "ABCDEFGHIJ");
    grid.dirty_mut().drain().for_each(drop);

    // Move cursor to col 3 (simulating cursor positioning before INSERT write).
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(3));

    // INSERT mode sequence: insert_blank then put_char.
    grid.insert_blank(1);
    grid.put_char('X');

    let items: Vec<_> = grid.dirty_mut().drain().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].line, 0);
    // Damage must cover from cursor column (3) to right edge (9).
    assert_eq!(items[0].left, 3);
    assert_eq!(items[0].right, 9);
}

#[test]
fn insert_blank_at_col_zero_damages_full_line() {
    // INSERT at column 0 damages [0, cols-1] which is the full line.
    let mut grid = grid_with_text(3, 10, "ABCDEFGHIJ");
    grid.dirty_mut().drain().for_each(drop);

    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(0));

    grid.insert_blank(1);
    grid.put_char('Z');

    let items: Vec<_> = grid.dirty_mut().drain().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].left, 0);
    assert_eq!(items[0].right, 9);
}

// ── put_char_ascii fast path ──

#[test]
fn put_char_ascii_writes_and_advances() {
    let mut grid = Grid::new(24, 80);
    assert!(grid.put_char_ascii('A'));
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn put_char_ascii_applies_cursor_template() {
    use vte::ansi::Color;
    let mut grid = Grid::new(24, 80);
    grid.cursor_mut().template.fg = Color::Indexed(1);
    grid.cursor_mut().template.bg = Color::Indexed(4);
    assert!(grid.put_char_ascii('X'));
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.fg, Color::Indexed(1));
    assert_eq!(cell.bg, Color::Indexed(4));
}

#[test]
fn put_char_ascii_returns_false_at_end_of_line() {
    let mut grid = Grid::new(24, 10);
    grid.cursor_mut().set_col(Column(10));
    assert!(!grid.put_char_ascii('A'));
}

#[test]
fn put_char_ascii_returns_false_on_wide_char_spacer() {
    let mut grid = Grid::new(24, 80);
    // Write a wide character to create base + spacer.
    grid.put_char('\u{597d}');
    // Position cursor on the spacer (col 1).
    grid.cursor_mut().set_col(Column(1));
    assert!(!grid.put_char_ascii('A'));
}

#[test]
fn put_char_ascii_marks_dirty() {
    let mut grid = Grid::new(3, 10);
    grid.dirty_mut().drain().for_each(drop);
    grid.put_char_ascii('Z');
    assert!(grid.dirty().is_any_dirty());
}

// ── push_zerowidth ──

#[test]
fn push_zerowidth_appends_combining_mark() {
    let mut grid = Grid::new(24, 80);
    grid.put_char('e');
    // Combining acute accent (U+0301).
    grid.push_zerowidth('\u{0301}');
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'e');
    let extra = cell.extra.as_ref().expect("should have CellExtra");
    assert_eq!(extra.zerowidth, vec!['\u{0301}']);
}

#[test]
fn push_zerowidth_at_col_zero_is_discarded() {
    let mut grid = Grid::new(24, 80);
    // Cursor at (0,0) — no previous cell exists.
    grid.push_zerowidth('\u{0301}');
    // Should not panic and cell should remain empty.
    assert!(grid[crate::index::Line(0)][Column(0)].is_empty());
}

#[test]
fn push_zerowidth_on_wide_char_spacer_targets_base() {
    let mut grid = Grid::new(24, 80);
    grid.put_char('\u{597d}');
    // Cursor is at col 2 after wide char. Push zerowidth — should attach
    // to the wide char base at col 0, not the spacer at col 1.
    grid.push_zerowidth('\u{0301}');
    let base = &grid[crate::index::Line(0)][Column(0)];
    let extra = base.extra.as_ref().expect("should have CellExtra on base");
    assert_eq!(extra.zerowidth, vec!['\u{0301}']);
}

#[test]
fn push_zerowidth_marks_dirty() {
    let mut grid = Grid::new(3, 10);
    grid.put_char('A');
    grid.dirty_mut().drain().for_each(drop);
    grid.push_zerowidth('\u{0301}');
    assert!(grid.dirty().is_any_dirty());
}

// ── Zero-count operations produce no dirty marks ──

#[test]
fn insert_blank_zero_no_dirty() {
    let mut grid = grid_with_text(3, 10, "ABCDE");
    grid.dirty_mut().drain().for_each(drop);
    grid.cursor_mut().set_col(Column(0));
    grid.insert_blank(0);
    assert!(!grid.dirty().is_any_dirty());
}

#[test]
fn delete_chars_zero_no_dirty() {
    let mut grid = grid_with_text(3, 10, "ABCDE");
    grid.dirty_mut().drain().for_each(drop);
    grid.cursor_mut().set_col(Column(0));
    grid.delete_chars(0);
    assert!(!grid.dirty().is_any_dirty());
}

#[test]
fn erase_chars_zero_no_dirty() {
    let mut grid = grid_with_text(3, 10, "ABCDE");
    grid.dirty_mut().drain().for_each(drop);
    grid.cursor_mut().set_col(Column(0));
    grid.erase_chars(0);
    assert!(!grid.dirty().is_any_dirty());
}

// --- ICH/DCH with DECLRMM horizontal margins ---

/// Helper: fill row 0 with column-index chars ('0'..'9').
fn grid_with_col_index_row(cols: usize) -> Grid {
    let mut grid = Grid::new(3, cols);
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(0));
    for col in 0..cols {
        let ch = (b'0' + col as u8) as char;
        grid.put_char(ch);
    }
    grid.cursor_mut().set_line(0);
    grid
}

#[test]
fn ich_within_margins_shifts_only_margin_band() {
    let mut grid = grid_with_col_index_row(10);
    // Margins [2, 7]. Cursor at col 3.
    grid.set_left_right_margins(2, 7);
    grid.cursor_mut().set_col(Column(3));
    grid.insert_blank(2);

    // Cols 0-2: unchanged.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, '0');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, '1');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, '2');
    // Cols 3-4: blanked (inserted).
    assert!(
        grid[crate::index::Line(0)][Column(3)].is_empty(),
        "col 3 should be blank"
    );
    assert!(
        grid[crate::index::Line(0)][Column(4)].is_empty(),
        "col 4 should be blank"
    );
    // Cols 5-7: old cols 3-5 shifted right ('3', '4', '5').
    assert_eq!(grid[crate::index::Line(0)][Column(5)].ch, '3');
    assert_eq!(grid[crate::index::Line(0)][Column(6)].ch, '4');
    assert_eq!(grid[crate::index::Line(0)][Column(7)].ch, '5');
    // Cols 8-9: unchanged (outside right margin).
    assert_eq!(grid[crate::index::Line(0)][Column(8)].ch, '8');
    assert_eq!(grid[crate::index::Line(0)][Column(9)].ch, '9');
}

#[test]
fn dch_within_margins_shifts_only_margin_band() {
    let mut grid = grid_with_col_index_row(10);
    // Margins [2, 7]. Cursor at col 3.
    grid.set_left_right_margins(2, 7);
    grid.cursor_mut().set_col(Column(3));
    grid.delete_chars(2);

    // Cols 0-2: unchanged.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, '0');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, '1');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, '2');
    // Cols 3-5: old cols 5-7 shifted left ('5', '6', '7').
    assert_eq!(grid[crate::index::Line(0)][Column(3)].ch, '5');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, '6');
    assert_eq!(grid[crate::index::Line(0)][Column(5)].ch, '7');
    // Cols 6-7: blanked (vacated).
    assert!(
        grid[crate::index::Line(0)][Column(6)].is_empty(),
        "col 6 should be blank"
    );
    assert!(
        grid[crate::index::Line(0)][Column(7)].is_empty(),
        "col 7 should be blank"
    );
    // Cols 8-9: unchanged (outside right margin).
    assert_eq!(grid[crate::index::Line(0)][Column(8)].ch, '8');
    assert_eq!(grid[crate::index::Line(0)][Column(9)].ch, '9');
}

/// Regression: ICH with cursor left of `left_margin` must be a no-op.
/// Previously the shift range started at `col` (the cursor) and extended
/// to `right_margin + 1`, mutating cells BEFORE `left_margin` — outside
/// the margin band in violation of ECMA-48/DECLRMM semantics.
#[test]
fn ich_with_cursor_left_of_left_margin_is_noop() {
    let mut grid = grid_with_col_index_row(10);
    grid.set_left_right_margins(5, 8);
    grid.cursor_mut().set_col(Column(2));
    grid.insert_blank(3);
    for col in 0..10 {
        let expected = (b'0' + col as u8) as char;
        assert_eq!(
            grid[crate::index::Line(0)][Column(col)].ch,
            expected,
            "col {col} must be unchanged when cursor is outside band",
        );
    }
}

#[test]
fn ich_with_cursor_right_of_right_margin_is_noop() {
    let mut grid = grid_with_col_index_row(10);
    grid.set_left_right_margins(2, 5);
    grid.cursor_mut().set_col(Column(8));
    grid.insert_blank(3);
    for col in 0..10 {
        let expected = (b'0' + col as u8) as char;
        assert_eq!(grid[crate::index::Line(0)][Column(col)].ch, expected);
    }
}

#[test]
fn dch_with_cursor_left_of_left_margin_is_noop() {
    let mut grid = grid_with_col_index_row(10);
    grid.set_left_right_margins(5, 8);
    grid.cursor_mut().set_col(Column(2));
    grid.delete_chars(3);
    for col in 0..10 {
        let expected = (b'0' + col as u8) as char;
        assert_eq!(
            grid[crate::index::Line(0)][Column(col)].ch,
            expected,
            "col {col} must be unchanged when cursor is outside band",
        );
    }
}

#[test]
fn dch_with_cursor_right_of_right_margin_is_noop() {
    let mut grid = grid_with_col_index_row(10);
    grid.set_left_right_margins(2, 5);
    grid.cursor_mut().set_col(Column(8));
    grid.delete_chars(3);
    for col in 0..10 {
        let expected = (b'0' + col as u8) as char;
        assert_eq!(grid[crate::index::Line(0)][Column(col)].ch, expected);
    }
}

// ---- regression tests: CHARDRAWN via CellFlags::DRAWN ----
//
// xterm's CHARDRAWN bit is set on every application write and cleared
// on every erase/reset. ori_term mirrors this via CellFlags::DRAWN.
// These tests pin the flag's lifecycle across every relevant write
// path, reset path, and structural operation.

use crate::cell::CellFlags;

#[test]
fn put_char_ascii_sets_drawn() {
    let mut grid = Grid::new(1, 3);
    grid.put_char_ascii('A');
    assert!(
        grid[crate::index::Line(0)][Column(0)]
            .flags
            .contains(CellFlags::DRAWN)
    );
}

/// Regression: — the specific repro from the bug entry.
/// Application writes a plain space with default SGR → cell must carry
/// DRAWN so DECRQCRA sees it as a written cell, not pristine.
#[test]
fn put_char_ascii_space_sets_drawn() {
    let mut grid = Grid::new(1, 3);
    grid.put_char_ascii(' ');
    assert!(
        grid[crate::index::Line(0)][Column(0)]
            .flags
            .contains(CellFlags::DRAWN),
        "plain space write MUST set DRAWN"
    );
}

#[test]
fn put_char_slow_wide_sets_drawn_on_both_cells() {
    let mut grid = Grid::new(1, 4);
    grid.put_char('\u{597d}'); // Chinese char — width 2
    let row_line = crate::index::Line(0);
    assert!(grid[row_line][Column(0)].flags.contains(CellFlags::DRAWN));
    assert!(grid[row_line][Column(1)].flags.contains(CellFlags::DRAWN));
    assert!(
        grid[row_line][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

/// Wide char at the last column wraps and inserts a
/// LEADING_WIDE_CHAR_SPACER boundary cell at (col_last). That boundary
/// cell is a synthesized blank but IS drawn per xterm semantics.
#[test]
fn put_char_slow_leading_wide_spacer_sets_drawn() {
    let mut grid = Grid::new(2, 4);
    // Fill cols 0..3 so the wide char at col 3 has to wrap.
    for _ in 0..3 {
        grid.put_char('A');
    }
    assert_eq!(grid.cursor().col(), Column(3));
    grid.put_char('\u{597d}'); // wraps to next line; col 3 becomes leading spacer
    let boundary = &grid[crate::index::Line(0)][Column(3)];
    assert!(boundary.flags.contains(CellFlags::LEADING_WIDE_CHAR_SPACER));
    assert!(
        boundary.flags.contains(CellFlags::DRAWN),
        "LEADING_WIDE_CHAR_SPACER boundary cell is synthesized-but-drawn; must carry DRAWN"
    );
}

/// Regression: F1 — combining-mark modification on a
/// cell IS a draw operation. Even though push_zerowidth mutates `extra`
/// (not `ch` or `flags`), we explicitly set DRAWN on the target cell
/// so it survives future callers that might target an undrawn cell.
#[test]
fn push_zerowidth_sets_drawn_on_target() {
    let mut grid = Grid::new(1, 3);
    grid.put_char('a');
    // After put_char, cell (0,0) already has DRAWN. Push a combining
    // mark; confirm DRAWN still set (not accidentally cleared) AND the
    // combining mark lands in extra.zerowidth.
    grid.push_zerowidth('\u{0301}'); // combining acute
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert!(cell.flags.contains(CellFlags::DRAWN));
    let extra = cell
        .extra
        .as_ref()
        .expect("combining mark must allocate extra");
    assert_eq!(extra.zerowidth, vec!['\u{0301}']);
}

/// insert_blank shifts existing drawn cells rightward and inserts
/// undrawn (DRAWN-clear) blanks at the cursor. Shifted cells must
/// retain DRAWN; inserted blanks must have DRAWN clear.
#[test]
fn insert_blank_preserves_shifted_drawn_and_inserts_undrawn() {
    let mut grid = Grid::new(1, 5);
    grid.put_char('A');
    grid.put_char('B');
    grid.put_char('C');
    grid.cursor_mut().set_col(Column(1));
    grid.insert_blank(2);
    // After insert: "A", _, _, "B", "C" (B and C shifted to cols 3, 4).
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, 'A');
    assert!(grid[line][Column(0)].flags.contains(CellFlags::DRAWN));
    assert_eq!(grid[line][Column(1)].ch, ' ');
    assert!(
        !grid[line][Column(1)].flags.contains(CellFlags::DRAWN),
        "inserted blank MUST be DRAWN-clear"
    );
    assert_eq!(grid[line][Column(2)].ch, ' ');
    assert!(!grid[line][Column(2)].flags.contains(CellFlags::DRAWN));
    assert_eq!(grid[line][Column(3)].ch, 'B');
    assert!(
        grid[line][Column(3)].flags.contains(CellFlags::DRAWN),
        "shifted cell MUST preserve DRAWN"
    );
    assert_eq!(grid[line][Column(4)].ch, 'C');
    assert!(grid[line][Column(4)].flags.contains(CellFlags::DRAWN));
}

/// delete_chars shifts cells left and fills the right edge with blanks.
/// Shifted cells retain DRAWN; new tail blanks have DRAWN clear.
#[test]
fn delete_chars_preserves_shifted_drawn_and_fills_undrawn() {
    let mut grid = Grid::new(1, 5);
    for ch in ['A', 'B', 'C', 'D'] {
        grid.put_char(ch);
    }
    grid.cursor_mut().set_col(Column(1));
    grid.delete_chars(2);
    // After delete: "A", "D", _, _, _
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, 'A');
    assert!(grid[line][Column(0)].flags.contains(CellFlags::DRAWN));
    assert_eq!(grid[line][Column(1)].ch, 'D');
    assert!(
        grid[line][Column(1)].flags.contains(CellFlags::DRAWN),
        "shifted cell MUST preserve DRAWN"
    );
    assert_eq!(grid[line][Column(2)].ch, ' ');
    assert!(
        !grid[line][Column(2)].flags.contains(CellFlags::DRAWN),
        "tail blank MUST be DRAWN-clear"
    );
}

/// erase_chars clears DRAWN on the erased range (BCE-aware; erased
/// cells are restored to a DRAWN-clear template).
#[test]
fn erase_chars_clears_drawn() {
    let mut grid = Grid::new(1, 5);
    for ch in ['A', 'B', 'C', 'D', 'E'] {
        grid.put_char(ch);
    }
    grid.cursor_mut().set_col(Column(1));
    grid.erase_chars(3);
    let line = crate::index::Line(0);
    assert!(grid[line][Column(0)].flags.contains(CellFlags::DRAWN)); // 'A' intact
    assert!(!grid[line][Column(1)].flags.contains(CellFlags::DRAWN));
    assert!(!grid[line][Column(2)].flags.contains(CellFlags::DRAWN));
    assert!(!grid[line][Column(3)].flags.contains(CellFlags::DRAWN));
    assert!(grid[line][Column(4)].flags.contains(CellFlags::DRAWN)); // 'E' intact
}

/// clear_line (EL) clears DRAWN on the erased line range.
#[test]
fn erase_in_line_clears_drawn() {
    let mut grid = Grid::new(1, 5);
    for ch in ['A', 'B', 'C', 'D', 'E'] {
        grid.put_char(ch);
    }
    grid.cursor_mut().set_col(Column(2));
    grid.erase_line(LineEraseMode::Right); // erase from col 2 to end
    let line = crate::index::Line(0);
    assert!(grid[line][Column(0)].flags.contains(CellFlags::DRAWN));
    assert!(grid[line][Column(1)].flags.contains(CellFlags::DRAWN));
    assert!(!grid[line][Column(2)].flags.contains(CellFlags::DRAWN));
    assert!(!grid[line][Column(3)].flags.contains(CellFlags::DRAWN));
    assert!(!grid[line][Column(4)].flags.contains(CellFlags::DRAWN));
}

/// clear_screen (ED) clears DRAWN on the erased display region.
#[test]
fn erase_in_display_clears_drawn() {
    let mut grid = Grid::new(2, 3);
    grid.put_char('A');
    grid.put_char('B');
    grid.linefeed();
    grid.carriage_return();
    grid.put_char('C');
    grid.cursor_mut().set_col(Column(0));
    grid.erase_display(DisplayEraseMode::All);
    for line_idx in 0..2 {
        for col in 0..3 {
            assert!(
                !grid[crate::index::Line(line_idx)][Column(col)]
                    .flags
                    .contains(CellFlags::DRAWN),
                "line {line_idx} col {col} must have DRAWN clear after ED All"
            );
        }
    }
}

// ── §09A.6 rectangular-area primitives ──────────────────────────────

mod rect_tests {
    use vte::ansi::Color;

    use crate::cell::{Cell, CellFlags};
    use crate::grid::Grid;
    use crate::index::{Column, Line};
    use crate::term::AceMode;

    /// Seed a 3×5 grid with "ABCDE" / "FGHIJ" / "KLMNO".
    fn seed_3x5() -> Grid {
        let mut grid = Grid::new(3, 5);
        for ch in "ABCDE".chars() {
            grid.put_char(ch);
        }
        grid.linefeed();
        grid.carriage_return();
        for ch in "FGHIJ".chars() {
            grid.put_char(ch);
        }
        grid.linefeed();
        grid.carriage_return();
        for ch in "KLMNO".chars() {
            grid.put_char(ch);
        }
        grid
    }

    #[test]
    fn fill_rect_writes_template_with_drawn() {
        let mut grid = seed_3x5();
        let mut tmpl = Cell::default();
        tmpl.ch = 'Q';
        grid.fill_rect(0, 1, 1, 3, &tmpl);
        // Inside the rect: 'Q' + DRAWN.
        for (line, col) in [(0, 1), (0, 3), (1, 2)] {
            let cell = &grid[Line(line)][Column(col)];
            assert_eq!(cell.ch, 'Q', "line={line} col={col}");
            assert!(cell.flags.contains(CellFlags::DRAWN));
        }
        // Outside: unchanged.
        assert_eq!(grid[Line(0)][Column(0)].ch, 'A');
        assert_eq!(grid[Line(0)][Column(4)].ch, 'E');
        assert_eq!(grid[Line(2)][Column(0)].ch, 'K');
    }

    #[test]
    fn fill_rect_single_cell_edge_case() {
        let mut grid = Grid::new(2, 2);
        let mut tmpl = Cell::default();
        tmpl.ch = '*';
        grid.fill_rect(0, 0, 0, 0, &tmpl);
        assert_eq!(grid[Line(0)][Column(0)].ch, '*');
        assert_eq!(grid[Line(0)][Column(1)].ch, ' ');
        assert_eq!(grid[Line(1)][Column(0)].ch, ' ');
    }

    #[test]
    fn erase_rect_all_ignores_protected() {
        let mut grid = seed_3x5();
        // Mark A (line 0, col 0) and K (line 2, col 0) PROTECTED.
        grid[Line(0)][Column(0)].flags.insert(CellFlags::PROTECTED);
        grid[Line(2)][Column(0)].flags.insert(CellFlags::PROTECTED);
        grid.erase_rect_all(0, 0, 2, 4, Color::Named(vte::ansi::NamedColor::Background));
        // Every cell is wiped — even the PROTECTED ones.
        for line in 0..3 {
            for col in 0..5 {
                assert_eq!(
                    grid[Line(line)][Column(col)].ch,
                    ' ',
                    "line={line} col={col}"
                );
            }
        }
    }

    #[test]
    fn erase_rect_unprotected_preserves_protected() {
        let mut grid = seed_3x5();
        grid[Line(0)][Column(0)].flags.insert(CellFlags::PROTECTED);
        grid[Line(2)][Column(4)].flags.insert(CellFlags::PROTECTED);
        grid.erase_rect_unprotected(0, 0, 2, 4, Color::Named(vte::ansi::NamedColor::Background));
        // Protected cells survive with original content.
        assert_eq!(grid[Line(0)][Column(0)].ch, 'A');
        assert_eq!(grid[Line(2)][Column(4)].ch, 'O');
        // Unprotected cells wipe.
        assert_eq!(grid[Line(0)][Column(1)].ch, ' ');
        assert_eq!(grid[Line(1)][Column(2)].ch, ' ');
    }

    #[test]
    fn apply_sgr_rect_rectangle_mode_clips_every_row() {
        let mut grid = seed_3x5();
        // Rectangle mode: every row is clipped to cols [1..=2].
        grid.apply_sgr_rect(0, 1, 2, 2, &[1 /* BOLD */], AceMode::Rectangle);
        // Inside rect: BOLD set.
        assert!(grid[Line(0)][Column(1)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(1)][Column(2)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(2)][Column(1)].flags.contains(CellFlags::BOLD));
        // Outside rect cols: BOLD NOT set.
        assert!(!grid[Line(0)][Column(0)].flags.contains(CellFlags::BOLD));
        assert!(!grid[Line(1)][Column(3)].flags.contains(CellFlags::BOLD));
    }

    #[test]
    fn apply_sgr_rect_stream_mode_wraps_rows() {
        let mut grid = seed_3x5();
        // Stream mode: row 0 starts at col 1; row 1 spans full width;
        // row 2 ends at col 2. Col 4 of row 0 and cols 3-4 of row 2
        // lie outside the "stream" and stay untouched.
        grid.apply_sgr_rect(0, 1, 2, 2, &[1 /* BOLD */], AceMode::Stream);
        // Row 0: BOLD set from col 1 to col 4 (right edge).
        assert!(!grid[Line(0)][Column(0)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(0)][Column(1)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(0)][Column(4)].flags.contains(CellFlags::BOLD));
        // Row 1 (middle): BOLD set over the entire row.
        assert!(grid[Line(1)][Column(0)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(1)][Column(4)].flags.contains(CellFlags::BOLD));
        // Row 2: BOLD set from col 0 to col 2.
        assert!(grid[Line(2)][Column(0)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(2)][Column(2)].flags.contains(CellFlags::BOLD));
        assert!(!grid[Line(2)][Column(3)].flags.contains(CellFlags::BOLD));
    }

    #[test]
    fn apply_sgr_rect_sgr_0_resets_flags() {
        let mut grid = seed_3x5();
        // Seed BOLD + UNDERLINE on every cell.
        for line in 0..3 {
            for col in 0..5 {
                grid[Line(line)][Column(col)]
                    .flags
                    .insert(CellFlags::BOLD | CellFlags::UNDERLINE);
            }
        }
        grid.apply_sgr_rect(0, 1, 1, 3, &[0], AceMode::Rectangle);
        // Inside rect: reset (neither BOLD nor UNDERLINE).
        assert!(!grid[Line(0)][Column(1)].flags.contains(CellFlags::BOLD));
        assert!(
            !grid[Line(0)][Column(1)]
                .flags
                .contains(CellFlags::UNDERLINE)
        );
        // Outside rect: flags preserved.
        assert!(grid[Line(0)][Column(0)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(2)][Column(0)].flags.contains(CellFlags::BOLD));
    }

    #[test]
    fn reverse_sgr_rect_toggles_flags() {
        let mut grid = seed_3x5();
        // Pre-set BOLD on half the rect.
        grid[Line(0)][Column(1)].flags.insert(CellFlags::BOLD);
        grid.reverse_sgr_rect(0, 1, 1, 2, &[1 /* BOLD */], AceMode::Rectangle);
        // Cell that had BOLD: toggled off.
        assert!(!grid[Line(0)][Column(1)].flags.contains(CellFlags::BOLD));
        // Cell that did not have BOLD: toggled on.
        assert!(grid[Line(0)][Column(2)].flags.contains(CellFlags::BOLD));
        assert!(grid[Line(1)][Column(1)].flags.contains(CellFlags::BOLD));
        // Outside rect: unchanged.
        assert!(!grid[Line(2)][Column(0)].flags.contains(CellFlags::BOLD));
    }

    #[test]
    fn copy_rect_non_overlapping() {
        let mut grid = seed_3x5();
        // Copy (0, 0)-(0, 2) = "ABC" to (2, 2).
        grid.copy_rect(0, 0, 0, 2, 2, 2);
        assert_eq!(grid[Line(0)][Column(0)].ch, 'A');
        assert_eq!(grid[Line(2)][Column(2)].ch, 'A');
        assert_eq!(grid[Line(2)][Column(3)].ch, 'B');
        assert_eq!(grid[Line(2)][Column(4)].ch, 'C');
        // Destination col 0-1 of line 2 untouched: still "KL".
        assert_eq!(grid[Line(2)][Column(0)].ch, 'K');
        assert_eq!(grid[Line(2)][Column(1)].ch, 'L');
    }

    #[test]
    fn copy_rect_overlapping_dest_before_source() {
        // Overlapping copy: dest col 0, source col 1..=2. Contents at
        // cols 0..=1 must match the ORIGINAL cols 1..=2 after the copy
        // (not a post-write read of already-mutated cells).
        let mut grid = Grid::new(1, 4);
        for ch in "ABCD".chars() {
            grid.put_char(ch);
        }
        // Copy cols 1..=2 ("BC") to col 0.
        grid.copy_rect(0, 1, 0, 2, 0, 0);
        assert_eq!(grid[Line(0)][Column(0)].ch, 'B');
        assert_eq!(grid[Line(0)][Column(1)].ch, 'C');
        // Cols 2..=3 unchanged by the overlap.
        assert_eq!(grid[Line(0)][Column(2)].ch, 'C');
        assert_eq!(grid[Line(0)][Column(3)].ch, 'D');
    }

    #[test]
    fn copy_rect_overlapping_dest_after_source() {
        // Copy cols 0..=1 ("AB") to col 1 — destination overlaps source
        // on col 1. After the copy, cols 1..=2 should hold "AB".
        let mut grid = Grid::new(1, 4);
        for ch in "ABCD".chars() {
            grid.put_char(ch);
        }
        grid.copy_rect(0, 0, 0, 1, 0, 1);
        assert_eq!(grid[Line(0)][Column(0)].ch, 'A');
        assert_eq!(grid[Line(0)][Column(1)].ch, 'A');
        assert_eq!(grid[Line(0)][Column(2)].ch, 'B');
        assert_eq!(grid[Line(0)][Column(3)].ch, 'D');
    }

    #[test]
    fn copy_rect_clips_dest_to_grid() {
        let mut grid = seed_3x5();
        // Copy (0, 0)-(0, 4) = "ABCDE" to (2, 3) — destination would
        // end at col 7 but the grid has only 5 cols, so cols 3-4 hold
        // "AB" and cells (src cols 2..=4) fall off the right edge.
        grid.copy_rect(0, 0, 0, 4, 2, 3);
        assert_eq!(grid[Line(2)][Column(3)].ch, 'A');
        assert_eq!(grid[Line(2)][Column(4)].ch, 'B');
        // Dest cols 0..=2 of line 2 stay "KLM" (outside dest rect).
        assert_eq!(grid[Line(2)][Column(0)].ch, 'K');
    }
}
