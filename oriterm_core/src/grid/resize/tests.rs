use crate::cell::{Cell, CellFlags};
use crate::grid::Grid;
use crate::index::{Column, Line};

/// Helper: create a cell with the given character.
fn cell(ch: char) -> Cell {
    Cell {
        ch,
        ..Cell::default()
    }
}

/// Helper: write text into a grid row.
fn write_row(grid: &mut Grid, line: usize, text: &str) {
    for (col, ch) in text.chars().enumerate() {
        grid[Line(line as i32)][Column(col)] = cell(ch);
    }
}

/// Helper: read text from a grid row (trimming trailing spaces).
fn read_row(grid: &Grid, line: usize) -> String {
    let row = &grid[Line(line as i32)];
    let mut s: String = (0..row.cols()).map(|c| row[Column(c)].ch).collect();
    let trimmed = s.trim_end().len();
    s.truncate(trimmed);
    s
}

// ── Zero-size guards ────────────────────────────────────────────────

#[test]
fn resize_zero_cols_is_noop() {
    let mut grid = Grid::new(24, 80);
    grid.resize(0, 24, false);
    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.lines(), 24);
}

#[test]
fn resize_zero_lines_is_noop() {
    let mut grid = Grid::new(24, 80);
    grid.resize(80, 0, false);
    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.lines(), 24);
}

#[test]
fn resize_same_dimensions_is_noop() {
    let mut grid = Grid::new(24, 80);
    write_row(&mut grid, 0, "hello");
    grid.resize(80, 24, true);
    assert_eq!(read_row(&grid, 0), "hello");
}

// ── Row resize (vertical) ───────────────────────────────────────────

#[test]
fn shrink_rows_trims_trailing_blanks_first() {
    let mut grid = Grid::new(10, 80);
    // Write content in the first 3 rows, leave rest blank.
    write_row(&mut grid, 0, "line0");
    write_row(&mut grid, 1, "line1");
    write_row(&mut grid, 2, "line2");
    grid.cursor_mut().set_line(2);

    grid.resize(80, 5, false);

    assert_eq!(grid.lines(), 5);
    assert_eq!(read_row(&grid, 0), "line0");
    assert_eq!(read_row(&grid, 1), "line1");
    assert_eq!(read_row(&grid, 2), "line2");
    // No rows pushed to scrollback — blanks were trimmed.
    assert_eq!(grid.scrollback().len(), 0);
}

#[test]
fn shrink_rows_pushes_excess_to_scrollback() {
    let mut grid = Grid::new(5, 80);
    write_row(&mut grid, 0, "line0");
    write_row(&mut grid, 1, "line1");
    write_row(&mut grid, 2, "line2");
    write_row(&mut grid, 3, "line3");
    write_row(&mut grid, 4, "line4");
    grid.cursor_mut().set_line(4);

    grid.resize(80, 3, false);

    assert_eq!(grid.lines(), 3);
    // Top 2 rows pushed to scrollback.
    assert_eq!(grid.scrollback().len(), 2);
    // Visible rows are the last 3.
    assert_eq!(read_row(&grid, 0), "line2");
    assert_eq!(read_row(&grid, 1), "line3");
    assert_eq!(read_row(&grid, 2), "line4");
    // Cursor adjusted.
    assert_eq!(grid.cursor().line(), 2);
}

#[test]
fn shrink_rows_cursor_adjusted_for_scrollback_push() {
    let mut grid = Grid::new(5, 80);
    write_row(&mut grid, 0, "a");
    write_row(&mut grid, 1, "b");
    write_row(&mut grid, 2, "c");
    grid.cursor_mut().set_line(2);

    // Shrink by 1: trailing blanks trimmed (rows 3,4 blank), none pushed.
    grid.resize(80, 4, false);
    assert_eq!(grid.cursor().line(), 2);
    assert_eq!(grid.scrollback().len(), 0);
}

#[test]
fn grow_rows_appends_blanks_when_cursor_in_middle() {
    let mut grid = Grid::new(5, 80);
    write_row(&mut grid, 0, "line0");
    write_row(&mut grid, 1, "line1");
    grid.cursor_mut().set_line(1);

    grid.resize(80, 8, false);

    assert_eq!(grid.lines(), 8);
    assert_eq!(read_row(&grid, 0), "line0");
    assert_eq!(read_row(&grid, 1), "line1");
    assert_eq!(grid.cursor().line(), 1);
    assert_eq!(grid.scrollback().len(), 0);
}

#[test]
fn grow_rows_pulls_from_scrollback_when_cursor_at_bottom() {
    let mut grid = Grid::new(3, 80);
    write_row(&mut grid, 0, "line0");
    write_row(&mut grid, 1, "line1");
    write_row(&mut grid, 2, "line2");
    grid.cursor_mut().set_line(2);

    // Shrink to push rows to scrollback.
    grid.resize(80, 2, false);
    assert_eq!(grid.scrollback().len(), 1);
    assert_eq!(grid.cursor().line(), 1);

    // Grow back — should pull from scrollback.
    grid.resize(80, 3, false);
    assert_eq!(grid.lines(), 3);
    assert_eq!(grid.scrollback().len(), 0);
    assert_eq!(read_row(&grid, 0), "line0");
    assert_eq!(read_row(&grid, 1), "line1");
    assert_eq!(read_row(&grid, 2), "line2");
    assert_eq!(grid.cursor().line(), 2);
}

// ── Column resize (no reflow) ───────────────────────────────────────

#[test]
fn grow_cols_no_reflow_pads_with_blanks() {
    let mut grid = Grid::new(3, 10);
    write_row(&mut grid, 0, "hello");

    grid.resize(20, 3, false);

    assert_eq!(grid.cols(), 20);
    assert_eq!(read_row(&grid, 0), "hello");
    assert_eq!(grid[Line(0)].cols(), 20);
}

#[test]
fn shrink_cols_no_reflow_truncates() {
    let mut grid = Grid::new(3, 20);
    write_row(&mut grid, 0, "hello world here");

    grid.resize(5, 3, false);

    assert_eq!(grid.cols(), 5);
    assert_eq!(read_row(&grid, 0), "hello");
}

// ── Scroll region and cursor clamping ───────────────────────────────

#[test]
fn resize_resets_scroll_region() {
    let mut grid = Grid::new(24, 80);
    grid.set_scroll_region(5, Some(20));
    assert_eq!(*grid.scroll_region(), 4..20);

    grid.resize(80, 10, false);

    assert_eq!(*grid.scroll_region(), 0..10);
}

#[test]
fn resize_clamps_cursor_to_new_bounds() {
    let mut grid = Grid::new(24, 80);
    grid.cursor_mut().set_line(23);
    grid.cursor_mut().set_col(Column(79));

    grid.resize(40, 10, false);

    assert_eq!(grid.cursor().line(), 9);
    assert_eq!(grid.cursor().col(), Column(39));
}

#[test]
fn resize_clamps_display_offset() {
    let mut grid = Grid::with_scrollback(5, 80, 100);
    // Push some content to scrollback.
    for i in 0..10 {
        write_row(&mut grid, 0, &format!("line{i}"));
        grid.scroll_up(1);
    }
    // Scroll back into history.
    grid.scroll_display(5);
    assert!(grid.display_offset() > 0);

    grid.resize(80, 5, false);

    assert!(grid.display_offset() <= grid.scrollback().len());
}

// ── Tab stops ───────────────────────────────────────────────────────

#[test]
fn resize_resets_tab_stops_for_new_width() {
    let mut grid = Grid::new(24, 80);
    grid.resize(40, 24, false);

    // Tab stops should be reset for new column count.
    let stops = grid.tab_stops();
    assert_eq!(stops.len(), 40);
    assert!(stops[0]);
    assert!(stops[8]);
    assert!(stops[16]);
    assert!(stops[24]);
    assert!(stops[32]);
    assert!(!stops[39]);
}

// ── Reflow: column grow (unwrap) ────────────────────────────────────

#[test]
fn reflow_grow_unwraps_soft_wrapped_line() {
    let mut grid = Grid::new(3, 10);

    // Simulate a soft-wrapped line: "helloabcde" + "world" split across two rows.
    // Fill first row fully, then set WRAP on the last cell.
    write_row(&mut grid, 0, "helloabcde");
    grid[Line(0)][Column(9)].flags.insert(CellFlags::WRAP);
    write_row(&mut grid, 1, "world");

    // Grow to 20 cols: the wrapped line should unwrap into one row.
    grid.resize(20, 3, true);

    assert_eq!(grid.cols(), 20);
    let row0 = read_row(&grid, 0);
    assert_eq!(row0, "helloabcdeworld");
}

#[test]
fn reflow_grow_non_wrapped_lines_stay_separate() {
    let mut grid = Grid::new(3, 10);
    write_row(&mut grid, 0, "hello");
    // No WRAP flag — hard newline.
    write_row(&mut grid, 1, "world");

    grid.resize(20, 3, true);

    assert_eq!(read_row(&grid, 0), "hello");
    assert_eq!(read_row(&grid, 1), "world");
}

// ── Reflow: column shrink (wrap) ────────────────────────────────────

#[test]
fn reflow_shrink_wraps_long_line() {
    let mut grid = Grid::new(20, 20);
    write_row(&mut grid, 0, "hello world here!!");

    grid.resize(10, 20, true);

    assert_eq!(grid.cols(), 10);

    // 18 chars wraps to 2 rows at 10 cols. The extra row from wrapping
    // pushes the first half to scrollback (standard terminal behavior).
    assert_eq!(grid.scrollback().len(), 1);

    // Scrollback has the first 10 chars with WRAP flag.
    let sb_row = grid.scrollback().get(0).expect("scrollback row");
    let sb_text: String = (0..10).map(|c| sb_row[Column(c)].ch).collect();
    assert_eq!(sb_text, "hello worl");
    assert!(sb_row[Column(9)].flags.contains(CellFlags::WRAP));

    // Visible row 0 has the remaining 8 chars.
    let r0 = read_row(&grid, 0);
    assert_eq!(r0, "d here!!");
}

#[test]
fn reflow_shrink_preserves_cursor_within_bounds() {
    let mut grid = Grid::new(20, 20);
    write_row(&mut grid, 0, "hello world");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(6)); // On 'w'.

    grid.resize(5, 20, true);

    // "hello world" (11 chars) at 5 cols wraps to 3 rows, pushing 2 to
    // scrollback. Cursor's original content ('w') is in scrollback, so
    // cursor is clamped to visible area.
    assert!(grid.cursor().line() < grid.lines());
    assert!(grid.cursor().col().0 < grid.cols());

    // Content is preserved across scrollback + visible.
    assert!(grid.scrollback().len() >= 2);
}

// ── Reflow: round-trip ──────────────────────────────────────────────

#[test]
fn reflow_shrink_then_grow_preserves_content() {
    // Use enough lines so wrapped content stays visible during shrink.
    let mut grid = Grid::new(10, 20);
    write_row(&mut grid, 0, "hello world here!!");
    write_row(&mut grid, 1, "second line");

    // Shrink to 10 cols (wrap).
    grid.resize(10, 10, true);
    // Grow back to 20 cols (unwrap).
    grid.resize(20, 10, true);

    assert_eq!(read_row(&grid, 0), "hello world here!!");
    assert_eq!(read_row(&grid, 1), "second line");
}

// ── Reflow: empty grid ──────────────────────────────────────────────

#[test]
fn reflow_empty_grid_produces_valid_state() {
    let mut grid = Grid::new(3, 10);
    grid.resize(20, 3, true);

    assert_eq!(grid.cols(), 20);
    assert_eq!(grid.lines(), 3);
    assert!(grid[Line(0)][Column(0)].is_empty());
}

// ── Reflow: wide characters ─────────────────────────────────────────

#[test]
fn reflow_wide_char_at_boundary_wraps_correctly() {
    let mut grid = Grid::new(10, 10);

    // Write "abcd" then a wide CJK char at cols 4-5.
    for (col, ch) in "abcd".chars().enumerate() {
        grid[Line(0)][Column(col)] = cell(ch);
    }
    let mut wide = cell('\u{4e16}'); // CJK char (width=2).
    wide.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(4)] = wide;
    let mut spacer = Cell::default();
    spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(5)] = spacer;

    // Shrink to 5 cols: wide char at cols 4-5 can't fit (only col 4 left).
    grid.resize(5, 10, true);

    assert_eq!(grid.cols(), 5);
    // Wrapping pushes "abcd" to scrollback, wide char starts visible row 0.
    assert_eq!(grid.scrollback().len(), 1);
    let sb = grid.scrollback().get(0).expect("scrollback");
    let sb_text: String = (0..4).map(|c| sb[Column(c)].ch).collect();
    assert_eq!(sb_text, "abcd");

    // Visible row 0 has the wide char.
    assert!(
        grid[Line(0)][Column(0)]
            .flags
            .contains(CellFlags::WIDE_CHAR)
    );
    assert!(
        grid[Line(0)][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

#[test]
fn reflow_wide_char_boundary_sets_leading_spacer() {
    let mut grid = Grid::new(10, 10);

    // "abcd" (4 cols) + wide CJK (2 cols) = 6 cols.
    for (col, ch) in "abcd".chars().enumerate() {
        grid[Line(0)][Column(col)] = cell(ch);
    }
    let mut wide = cell('\u{4e16}');
    wide.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(4)] = wide;
    let mut spacer = Cell::default();
    spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(5)] = spacer;

    // Shrink to 5 cols: wide char can't fit at col 4 (needs 2 cells).
    // Cell at col 4 should become LEADING_WIDE_CHAR_SPACER.
    grid.resize(5, 10, true);

    let sb = grid.scrollback().get(0).expect("scrollback");
    assert!(
        sb[Column(4)]
            .flags
            .contains(CellFlags::LEADING_WIDE_CHAR_SPACER),
        "boundary cell should be LEADING_WIDE_CHAR_SPACER"
    );
    assert!(
        sb[Column(4)].flags.contains(CellFlags::WRAP),
        "boundary cell should also have WRAP"
    );
}

#[test]
fn reflow_wide_char_round_trip_preserves_content() {
    let mut grid = Grid::new(10, 10);

    // "abcd" + wide CJK char at cols 4-5.
    for (col, ch) in "abcd".chars().enumerate() {
        grid[Line(0)][Column(col)] = cell(ch);
    }
    let mut wide = cell('\u{4e16}');
    wide.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(4)] = wide;
    let mut spacer = Cell::default();
    spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(5)] = spacer;

    // Shrink to 5 cols, then grow back to 10 cols.
    grid.resize(5, 10, true);
    grid.resize(10, 10, true);

    // Content should be preserved without spurious spaces.
    let r: String = (0..6).map(|c| grid[Line(0)][Column(c)].ch).collect();
    assert_eq!(r, "abcd\u{4e16} ");
    assert!(
        grid[Line(0)][Column(4)]
            .flags
            .contains(CellFlags::WIDE_CHAR)
    );
    assert!(
        grid[Line(0)][Column(5)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

#[test]
fn reflow_leading_spacer_skipped_during_reflow() {
    let mut grid = Grid::new(10, 6);

    // "abc" + wide CJK at cols 3-4 in a 6-col grid.
    for (col, ch) in "abc".chars().enumerate() {
        grid[Line(0)][Column(col)] = cell(ch);
    }
    let mut wide = cell('\u{4e16}');
    wide.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(3)] = wide;
    let mut spacer = Cell::default();
    spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(4)] = spacer;

    // Shrink to 4 cols: wide char at col 3 can't fit (needs col 3+4, only 4 available).
    grid.resize(4, 10, true);

    // Boundary cell at col 3 should be LEADING_WIDE_CHAR_SPACER.
    let sb = grid.scrollback().get(0).expect("scrollback");
    assert!(
        sb[Column(3)]
            .flags
            .contains(CellFlags::LEADING_WIDE_CHAR_SPACER)
    );

    // Grow back: leading spacer should be skipped, no extra space.
    grid.resize(6, 10, true);
    let r: String = (0..5).map(|c| grid[Line(0)][Column(c)].ch).collect();
    assert_eq!(r, "abc\u{4e16} ");
}

// ── Combined row + col resize ───────────────────────────────────────

#[test]
fn resize_both_dimensions_simultaneously() {
    let mut grid = Grid::new(10, 80);
    write_row(&mut grid, 0, "hello");
    write_row(&mut grid, 1, "world");
    grid.cursor_mut().set_line(1);

    grid.resize(40, 5, false);

    assert_eq!(grid.cols(), 40);
    assert_eq!(grid.lines(), 5);
    assert_eq!(read_row(&grid, 0), "hello");
    assert_eq!(read_row(&grid, 1), "world");
}

// ── Rapid resize sequences ──────────────────────────────────────────

#[test]
fn rapid_resize_sequence_does_not_panic() {
    let mut grid = Grid::new(24, 80);
    write_row(&mut grid, 0, "hello world");
    grid.cursor_mut().set_line(5);

    // Simulate rapid resize events.
    grid.resize(40, 12, true);
    grid.resize(120, 30, true);
    grid.resize(80, 24, true);
    grid.resize(10, 5, true);
    grid.resize(80, 24, true);

    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.lines(), 24);
    // Content should survive.
    assert_eq!(read_row(&grid, 0), "hello world");
}

#[test]
fn resize_to_minimum_1x1() {
    let mut grid = Grid::new(24, 80);
    write_row(&mut grid, 0, "hello");

    grid.resize(1, 1, true);

    assert_eq!(grid.cols(), 1);
    assert_eq!(grid.lines(), 1);
    assert_eq!(grid.cursor().line(), 0);
    assert_eq!(grid.cursor().col(), Column(0));
}

// ── Sparse content reflow ────────────────────────────────────────────

#[test]
fn reflow_sparse_cells_preserves_interior_blanks() {
    // "a  b  c" with interior spaces — reflow must not collapse them.
    let mut grid = Grid::new(10, 10);
    grid[Line(0)][Column(0)] = cell('a');
    grid[Line(0)][Column(3)] = cell('b');
    grid[Line(0)][Column(6)] = cell('c');

    // Shrink to 4 cols: wraps at col 4.
    grid.resize(4, 10, true);
    // Grow back: should recover exact positions.
    grid.resize(10, 10, true);

    assert_eq!(grid[Line(0)][Column(0)].ch, 'a');
    assert_eq!(grid[Line(0)][Column(3)].ch, 'b');
    assert_eq!(grid[Line(0)][Column(6)].ch, 'c');
}

// ── Multi-line wide char unwrap ─────────────────────────────────────

#[test]
fn reflow_multiline_wide_spacer_head_unwrap() {
    // 3-line scenario: "abcde" wrapped at 3 cols with a wide char that
    // splits across line 2→3. Growing should reconstruct all content.
    let mut grid = Grid::new(10, 6);

    // Row 0: "ab" + wide char at cols 2-3 = 4 display cols.
    grid[Line(0)][Column(0)] = cell('a');
    grid[Line(0)][Column(1)] = cell('b');
    let mut w1 = cell('\u{4e16}');
    w1.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(2)] = w1;
    let mut s1 = Cell::default();
    s1.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(3)] = s1;
    // Row 1: another wide char at cols 0-1 + "cd".
    let mut w2 = cell('\u{4e16}');
    w2.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(1)][Column(0)] = w2;
    let mut s2 = Cell::default();
    s2.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(1)][Column(1)] = s2;
    grid[Line(1)][Column(2)] = cell('c');
    grid[Line(1)][Column(3)] = cell('d');

    // Set WRAP to form a logical line.
    grid[Line(0)][Column(5)].flags.insert(CellFlags::WRAP);

    // Shrink to 3 cols, then grow back to 6.
    grid.resize(3, 10, true);
    grid.resize(6, 10, true);

    // Content should survive: "ab" + wide + wide + "cd".
    assert_eq!(grid[Line(0)][Column(0)].ch, 'a');
    assert_eq!(grid[Line(0)][Column(1)].ch, 'b');
    assert!(
        grid[Line(0)][Column(2)]
            .flags
            .contains(CellFlags::WIDE_CHAR)
    );
}

// ── Cursor tracking across multi-step reflows ───────────────────────

#[test]
fn cursor_tracks_through_narrow_grow_narrow_grow() {
    let mut grid = Grid::new(10, 20);
    write_row(&mut grid, 0, "hello world!!");
    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(5)); // On ' '.

    // Narrow → grow → narrow → grow.
    grid.resize(5, 10, true);
    grid.resize(20, 10, true);
    grid.resize(7, 10, true);
    grid.resize(20, 10, true);

    // Cursor should remain within bounds.
    assert!(grid.cursor().line() < grid.lines());
    assert!(grid.cursor().col().0 < grid.cols());

    // Content should survive.
    assert_eq!(read_row(&grid, 0), "hello world!!");
}

#[test]
fn cursor_on_wide_char_tracks_through_reflow() {
    let mut grid = Grid::new(10, 10);
    // "ab" + wide char at cols 2-3.
    grid[Line(0)][Column(0)] = cell('a');
    grid[Line(0)][Column(1)] = cell('b');
    let mut wide = cell('\u{4e16}');
    wide.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(2)] = wide;
    let mut spacer = Cell::default();
    spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(3)] = spacer;

    grid.cursor_mut().set_line(0);
    grid.cursor_mut().set_col(Column(2)); // On the wide char.

    // Shrink to 3 cols: wide char wraps.
    grid.resize(3, 10, true);
    // Grow back.
    grid.resize(10, 10, true);

    // Cursor should be on or near the wide char.
    assert!(grid.cursor().line() < grid.lines());
    assert!(grid.cursor().col().0 < grid.cols());
}

// ── Exact-fit boundary ──────────────────────────────────────────────

#[test]
fn reflow_content_fits_exactly_in_new_width() {
    let mut grid = Grid::new(10, 10);
    // 10 chars fills the row exactly.
    write_row(&mut grid, 0, "abcdefghij");
    grid[Line(0)][Column(9)].flags.insert(CellFlags::WRAP);
    write_row(&mut grid, 1, "klmno");

    // Grow to 15: "abcdefghij" + "klmno" = 15 chars, fits exactly.
    grid.resize(15, 10, true);

    assert_eq!(read_row(&grid, 0), "abcdefghijklmno");
    // No WRAP should remain since content fits the width exactly.
    assert!(!grid[Line(0)][Column(14)].flags.contains(CellFlags::WRAP));
}

#[test]
fn reflow_shrink_to_exact_content_length() {
    let mut grid = Grid::new(10, 20);
    write_row(&mut grid, 0, "hello");

    // Shrink cols to exactly match content length (5).
    grid.resize(5, 10, true);

    assert_eq!(grid.cols(), 5);
    assert_eq!(read_row(&grid, 0), "hello");
    // Content fits exactly — no wrapping should occur.
    assert!(!grid[Line(0)][Column(4)].flags.contains(CellFlags::WRAP));
    assert_eq!(grid.scrollback().len(), 0);
}

// ── Wide char multi-size round-trip ─────────────────────────────────

#[test]
fn wide_char_survives_multiple_intermediate_sizes() {
    let mut grid = Grid::new(10, 10);
    // "a" + wide at cols 1-2.
    grid[Line(0)][Column(0)] = cell('a');
    let mut wide = cell('\u{4e16}');
    wide.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(1)] = wide;
    let mut spacer = Cell::default();
    spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(2)] = spacer;

    // Cycle through multiple sizes.
    grid.resize(2, 10, true); // Wide char wraps.
    grid.resize(5, 10, true); // Unwrap.
    grid.resize(3, 10, true); // Wrap again.
    grid.resize(10, 10, true); // Back to original.

    assert_eq!(grid[Line(0)][Column(0)].ch, 'a');
    assert!(
        grid[Line(0)][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR)
    );
    assert!(
        grid[Line(0)][Column(2)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );
}

// ── Attribute preservation ──────────────────────────────────────────

#[test]
fn reflow_preserves_cell_attributes() {
    use vte::ansi::Color;

    let mut grid = Grid::new(10, 10);
    let mut c = cell('X');
    c.flags = CellFlags::BOLD | CellFlags::ITALIC;
    c.fg = Color::Indexed(1); // red
    grid[Line(0)][Column(0)] = c;
    write_row(&mut grid, 0, "Xbcdefghij");
    // Restore the styled cell after write_row.
    let mut styled = cell('X');
    styled.flags = CellFlags::BOLD | CellFlags::ITALIC;
    styled.fg = Color::Indexed(1);
    grid[Line(0)][Column(0)] = styled;
    grid[Line(0)][Column(9)].flags.insert(CellFlags::WRAP);
    write_row(&mut grid, 1, "klmno");

    // Shrink to 5, then grow back.
    grid.resize(5, 10, true);
    grid.resize(10, 10, true);

    // The styled cell should retain its attributes.
    let recovered = &grid[Line(0)][Column(0)];
    assert!(recovered.flags.contains(CellFlags::BOLD));
    assert!(recovered.flags.contains(CellFlags::ITALIC));
    assert_eq!(recovered.fg, Color::Indexed(1));
}

// ── Scrollback overflow during reflow ───────────────────────────────

#[test]
fn reflow_scrollback_overflow_evicts_oldest() {
    // Small scrollback capacity. Wrapping should evict oldest rows.
    let mut grid = Grid::with_scrollback(5, 10, 3);
    for i in 0..5 {
        write_row(&mut grid, i, &format!("line{i}____")); // Fill 10 cols.
    }
    grid.cursor_mut().set_line(4);

    // Shrink to 5 cols: each 10-char row wraps into 2 rows.
    // 5 rows × 2 = 10 rows. 5 visible, 5 to scrollback.
    // But scrollback capacity is only 3, so oldest 2 are evicted.
    grid.resize(5, 5, true);

    assert!(grid.scrollback().len() <= 3);
    // Grid should still be valid.
    assert_eq!(grid.lines(), 5);
    assert_eq!(grid.cols(), 5);
}

// ── Saved cursor tracking ───────────────────────────────────────────

#[test]
fn resize_clamps_saved_cursor() {
    let mut grid = Grid::new(24, 80);
    grid.cursor_mut().set_line(20);
    grid.cursor_mut().set_col(Column(70));
    grid.save_cursor();

    // Shrink well below saved cursor position.
    grid.resize(40, 10, false);

    // Restore and verify it was clamped.
    grid.restore_cursor();
    assert!(grid.cursor().line() < grid.lines());
    assert!(grid.cursor().col().0 < grid.cols());
}

// ── Display offset edge cases ───────────────────────────────────────

#[test]
fn resize_clamps_display_offset_when_scrollback_shrinks() {
    let mut grid = Grid::with_scrollback(5, 10, 100);
    // Fill grid and push content to scrollback.
    for i in 0..15 {
        write_row(&mut grid, 0, &format!("line{i:02}___"));
        grid.scroll_up(1);
    }
    grid.cursor_mut().set_line(4);

    // Scroll back into history.
    let sb_len = grid.scrollback().len();
    grid.scroll_display(sb_len as isize);
    assert_eq!(grid.display_offset(), sb_len);

    // Grow: pulls from scrollback, reducing its length.
    grid.resize(10, 8, false);

    // Display offset must be clamped to new scrollback length.
    assert!(grid.display_offset() <= grid.scrollback().len());
}

#[test]
fn resize_display_offset_zero_stays_zero() {
    let mut grid = Grid::with_scrollback(5, 10, 100);
    write_row(&mut grid, 0, "hello");
    // display_offset starts at 0 (live view).
    assert_eq!(grid.display_offset(), 0);

    grid.resize(20, 10, true);

    assert_eq!(grid.display_offset(), 0);
}

// ── Reflow with only wide chars ─────────────────────────────────────

#[test]
fn reflow_grid_of_only_wide_chars() {
    let mut grid = Grid::new(10, 6);
    // Fill row 0 with 3 wide chars (6 display cols).
    for i in 0..3 {
        let col = i * 2;
        let mut wide = cell('\u{4e16}');
        wide.flags.insert(CellFlags::WIDE_CHAR);
        grid[Line(0)][Column(col)] = wide;
        let mut spacer = Cell::default();
        spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
        grid[Line(0)][Column(col + 1)] = spacer;
    }

    // Shrink to 4 cols: 3rd wide char wraps.
    grid.resize(4, 10, true);
    assert_eq!(grid.cols(), 4);

    // Grow back to 6.
    grid.resize(6, 10, true);

    // All 3 wide chars should survive.
    for i in 0..3 {
        let col = i * 2;
        assert!(
            grid[Line(0)][Column(col)]
                .flags
                .contains(CellFlags::WIDE_CHAR),
            "wide char {i} missing at col {col}"
        );
        assert!(
            grid[Line(0)][Column(col + 1)]
                .flags
                .contains(CellFlags::WIDE_CHAR_SPACER),
            "spacer for wide char {i} missing at col {}",
            col + 1
        );
    }
}

// ── Mixed wide + narrow across scrollback boundary ──────────────────

#[test]
fn reflow_mixed_wide_narrow_across_scrollback() {
    let mut grid = Grid::new(5, 10);
    // Row 0: "ab" + wide + "c" = 5 display cols.
    grid[Line(0)][Column(0)] = cell('a');
    grid[Line(0)][Column(1)] = cell('b');
    let mut wide = cell('\u{4e16}');
    wide.flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(2)] = wide;
    let mut spacer = Cell::default();
    spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
    grid[Line(0)][Column(3)] = spacer;
    grid[Line(0)][Column(4)] = cell('c');
    // Row 1-4: other content.
    for i in 1..5 {
        write_row(&mut grid, i, &format!("row{i}______"));
    }
    grid.cursor_mut().set_line(4);

    // Shrink to 3 cols: forces wrapping + scrollback interaction.
    grid.resize(3, 5, true);
    // Grow back.
    grid.resize(10, 5, true);

    // First row content should be recoverable.
    assert_eq!(grid[Line(0)][Column(0)].ch, 'a');
    assert_eq!(grid[Line(0)][Column(1)].ch, 'b');
}

// ── Dirty tracking ──────────────────────────────────────────────────

#[test]
fn resize_marks_all_dirty() {
    let mut grid = Grid::new(10, 80);
    // Drain dirty state.
    grid.dirty_mut().drain().for_each(drop);

    grid.resize(40, 5, false);

    assert!(grid.dirty().is_all_dirty());
}
