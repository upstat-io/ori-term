//! Unit tests for `Grid::insert_columns` / `Grid::delete_columns`.
//!
//! Every cell in the test grids is seeded via `put_char`, so cells
//! carry `DRAWN` correctly and the post-mutation occ bookkeeping
//! matches the production paths exercised by DECIC / DECDC.

use crate::grid::Grid;
use crate::index::{Column, Line};

/// Build an `lines x cols` grid and fill the first `rows` lines with
/// the given text prefix. Cursor ends at the beginning of each line
/// after every call (home via `move_to`).
fn grid_filled_rows(lines: usize, cols: usize, rows: usize, per_row: &str) -> Grid {
    let mut grid = Grid::new(lines, cols);
    for line in 0..rows {
        grid.move_to(line, Column(0));
        for ch in per_row.chars() {
            grid.put_char(ch);
        }
    }
    grid.move_to(0, Column(0));
    grid
}

/// Snapshot the `chars` on row `line`, trimming trailing blanks so
/// comparisons stay readable when the grid is wider than the seeded
/// text.
fn row_chars(grid: &Grid, line: usize) -> String {
    let row = &grid[Line(line as i32)];
    let mut s = String::new();
    for col in 0..grid.cols() {
        s.push(row[Column(col)].ch);
    }
    s.trim_end_matches(' ').to_string()
}

#[test]
fn insert_columns_shifts_right_on_every_row() {
    // 5-line × 6-column grid seeded with "ABCDEF" on every row.
    let mut grid = grid_filled_rows(5, 6, 5, "ABCDEF");
    // Move cursor into the scroll region (default: full grid) at col 2.
    grid.move_to(2, Column(2));
    grid.insert_columns(2, 2);
    for line in 0..5 {
        assert_eq!(row_chars(&grid, line), "AB  CD", "line={line}");
    }
}

#[test]
fn insert_columns_preserves_cells_outside_scroll_region() {
    let mut grid = grid_filled_rows(4, 6, 4, "ABCDEF");
    // Restrict scroll region to lines 1..=2 (half-open 1..3).
    grid.set_scroll_region(2, Some(3));
    grid.move_to(1, Column(1));
    grid.insert_columns(2, 1);
    // Lines 0 and 3 unchanged.
    assert_eq!(row_chars(&grid, 0), "ABCDEF");
    assert_eq!(row_chars(&grid, 3), "ABCDEF");
    // Lines 1 and 2 shifted right by 2 at col 1.
    assert_eq!(row_chars(&grid, 1), "A  BCD");
    assert_eq!(row_chars(&grid, 2), "A  BCD");
}

#[test]
fn insert_columns_is_noop_when_cursor_outside_scroll_region() {
    let mut grid = grid_filled_rows(4, 6, 4, "ABCDEF");
    grid.set_scroll_region(2, Some(3));
    grid.move_to(0, Column(1)); // cursor on row 0 — outside region
    grid.insert_columns(2, 1);
    for line in 0..4 {
        assert_eq!(row_chars(&grid, line), "ABCDEF", "line={line}");
    }
}

#[test]
fn insert_columns_clamps_count_to_right_edge() {
    // 3-line × 6-column grid. Insert 10 columns at col 2 — only 4 fit
    // before we run out of room (col_end_excl - at_col == 6 - 2 == 4).
    let mut grid = grid_filled_rows(3, 6, 3, "ABCDEF");
    grid.move_to(0, Column(2));
    grid.insert_columns(10, 2);
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "AB");
    }
}

#[test]
fn insert_columns_declrmm_constrains_to_band() {
    let mut grid = grid_filled_rows(3, 10, 3, "ABCDEFGHIJ");
    grid.set_left_right_margins(2, 6); // 0-based inclusive → cols 2..=6
    grid.move_to(0, Column(3));
    grid.insert_columns(2, 3);
    // Cols 0..=1 untouched; cols 2..=6 shifted with blanks at [3..5];
    // cols 7..=9 untouched.
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "ABC  DEHIJ", "line={line}");
    }
}

#[test]
fn insert_columns_noop_when_at_col_outside_band() {
    let mut grid = grid_filled_rows(3, 10, 3, "ABCDEFGHIJ");
    grid.set_left_right_margins(2, 6);
    // cursor inside the band but `at_col` outside.
    grid.move_to(0, Column(3));
    grid.insert_columns(2, 0);
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "ABCDEFGHIJ");
    }
}

#[test]
fn insert_columns_count_zero_is_noop() {
    let mut grid = grid_filled_rows(3, 6, 3, "ABCDEF");
    grid.move_to(0, Column(2));
    grid.insert_columns(0, 2);
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "ABCDEF");
    }
}

#[test]
fn delete_columns_shifts_left_on_every_row() {
    let mut grid = grid_filled_rows(5, 6, 5, "ABCDEF");
    grid.move_to(2, Column(2));
    grid.delete_columns(2, 2);
    for line in 0..5 {
        assert_eq!(row_chars(&grid, line), "ABEF", "line={line}");
    }
}

#[test]
fn delete_columns_declrmm_constrains_to_band() {
    let mut grid = grid_filled_rows(3, 10, 3, "ABCDEFGHIJ");
    grid.set_left_right_margins(2, 6); // cols 2..=6 inclusive
    grid.move_to(0, Column(3));
    grid.delete_columns(2, 3);
    // Cols 0..=1 untouched; cols 2..=6 shifted (C + FG + blanks 5..=6);
    // cols 7..=9 untouched.
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "ABCFG  HIJ", "line={line}");
    }
}

#[test]
fn delete_columns_clamps_count_to_right_edge() {
    let mut grid = grid_filled_rows(3, 6, 3, "ABCDEF");
    grid.move_to(0, Column(2));
    grid.delete_columns(10, 2);
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "AB");
    }
}

#[test]
fn delete_columns_preserves_cells_outside_scroll_region() {
    let mut grid = grid_filled_rows(4, 6, 4, "ABCDEF");
    grid.set_scroll_region(2, Some(3)); // rows 1..=2
    grid.move_to(1, Column(1));
    grid.delete_columns(2, 1);
    assert_eq!(row_chars(&grid, 0), "ABCDEF");
    assert_eq!(row_chars(&grid, 3), "ABCDEF");
    assert_eq!(row_chars(&grid, 1), "ADEF");
    assert_eq!(row_chars(&grid, 2), "ADEF");
}

#[test]
fn delete_columns_count_zero_is_noop() {
    let mut grid = grid_filled_rows(3, 6, 3, "ABCDEF");
    grid.move_to(0, Column(2));
    grid.delete_columns(0, 2);
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "ABCDEF");
    }
}

#[test]
fn insert_then_delete_roundtrip_restores_content() {
    let mut grid = grid_filled_rows(3, 6, 3, "ABCDEF");
    grid.move_to(0, Column(2));
    grid.insert_columns(2, 2);
    grid.move_to(0, Column(2));
    grid.delete_columns(2, 2);
    // Last two columns of the original content fell off the right
    // edge during the insert, so the roundtrip is NOT fully lossless.
    // This pins xterm's actual behavior (column scroll is destructive).
    for line in 0..3 {
        assert_eq!(row_chars(&grid, line), "ABCD", "line={line}");
    }
}
