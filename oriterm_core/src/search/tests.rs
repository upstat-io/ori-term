//! Tests for search state, algorithm, and text extraction.

use std::sync::Arc;

use crate::cell::{CellExtra, CellFlags};
use crate::grid::{Grid, Row, StableRowIndex};
use crate::index::{Column, Line};

use super::find::find_matches;
use super::text::{byte_span_to_cols, extract_row_text};
use super::{MatchType, SearchState};

// Helpers

/// Build a row from an ASCII string (one char per cell).
fn row_from_str(s: &str) -> Row {
    let mut row = Row::new(s.len());
    for (i, ch) in s.chars().enumerate() {
        row[Column(i)].ch = ch;
    }
    row
}

/// Build a grid with the given row strings (visible rows, no scrollback).
fn grid_with_rows(rows: &[&str]) -> Grid {
    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(80);
    let lines = rows.len();
    let mut grid = Grid::with_scrollback(lines, cols, 0);
    for (line, text) in rows.iter().enumerate() {
        for (col, ch) in text.chars().enumerate() {
            grid[Line(line as i32)][Column(col)].ch = ch;
        }
    }
    grid
}

/// Create a `StableRowIndex` from a raw value.
fn sri(n: u64) -> StableRowIndex {
    StableRowIndex(n)
}

// Text Extraction

#[test]
fn extract_ascii_row() {
    let row = row_from_str("hello");
    let (text, col_map) = extract_row_text(&row);
    assert_eq!(text, "hello");
    assert_eq!(col_map, vec![0, 1, 2, 3, 4]);
}

#[test]
fn extract_null_cells_replaced_with_spaces() {
    let mut row = Row::new(3);
    // Leave cells as default (null char '\0').
    row[Column(1)].ch = 'x';
    let (text, col_map) = extract_row_text(&row);
    assert_eq!(text, " x ");
    assert_eq!(col_map, vec![0, 1, 2]);
}

#[test]
fn extract_wide_char_skips_spacers() {
    let mut row = Row::new(4);
    row[Column(0)].ch = '\u{597D}'; // 好 (wide)
    row[Column(0)].flags = CellFlags::WIDE_CHAR;
    row[Column(1)].flags = CellFlags::WIDE_CHAR_SPACER;
    row[Column(2)].ch = 'a';
    row[Column(3)].ch = 'b';
    let (text, col_map) = extract_row_text(&row);
    assert_eq!(text, "\u{597D}ab");
    assert_eq!(col_map, vec![0, 2, 3]);
}

#[test]
fn extract_combining_marks_share_column() {
    let mut row = Row::new(2);
    row[Column(0)].ch = 'e';
    row[Column(0)].extra = Some(Arc::new(CellExtra {
        zerowidth: vec!['\u{0301}'], // combining acute accent
        underline_color: None,
        hyperlink: None,
    }));
    row[Column(1)].ch = 'x';
    let (text, col_map) = extract_row_text(&row);
    assert_eq!(text, "e\u{0301}x");
    // Both 'e' and the combining mark map to column 0.
    assert_eq!(col_map, vec![0, 0, 1]);
}

// Byte Span to Cols

#[test]
fn byte_span_ascii_identity() {
    let text = "hello";
    let col_map = vec![0, 1, 2, 3, 4];
    assert_eq!(byte_span_to_cols(text, &col_map, 1, 4), Some((1, 3)));
}

#[test]
fn byte_span_multibyte_utf8() {
    // "好ab" — 好 is 3 bytes (E5 A5 BD), a=1 byte, b=1 byte.
    let text = "\u{597D}ab";
    let col_map = vec![0, 2, 3]; // 好 at col 0, 'a' at col 2, 'b' at col 3
    // Byte span 0..3 = 好 → col (0, 0)
    assert_eq!(byte_span_to_cols(text, &col_map, 0, 3), Some((0, 0)));
    // Byte span 3..4 = 'a' → col (2, 2)
    assert_eq!(byte_span_to_cols(text, &col_map, 3, 4), Some((2, 2)));
}

#[test]
fn byte_span_empty_returns_none() {
    let text = "abc";
    let col_map = vec![0, 1, 2];
    assert_eq!(byte_span_to_cols(text, &col_map, 2, 2), None);
}

// Find Matches (Plain Text)

#[test]
fn plain_text_finds_in_two_rows() {
    let grid = grid_with_rows(&["hello world", "hello again"]);
    let matches = find_matches(&grid, "hello", true, false);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].start_col, 0);
    assert_eq!(matches[0].end_col, 4);
    assert_eq!(matches[1].start_col, 0);
    assert_eq!(matches[1].end_col, 4);
}

#[test]
fn plain_text_case_insensitive() {
    let grid = grid_with_rows(&["Hello HELLO hello"]);
    let matches = find_matches(&grid, "hello", false, false);
    assert_eq!(matches.len(), 3);
}

#[test]
fn plain_text_case_sensitive() {
    let grid = grid_with_rows(&["Hello HELLO hello"]);
    let matches = find_matches(&grid, "hello", true, false);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start_col, 12);
}

#[test]
fn plain_text_empty_query() {
    let grid = grid_with_rows(&["some text"]);
    let matches = find_matches(&grid, "", true, false);
    assert!(matches.is_empty());
}

// Find Matches (Regex)

#[test]
fn regex_digits() {
    let grid = grid_with_rows(&["abc 123 def 456"]);
    let matches = find_matches(&grid, r"\d+", true, true);
    assert_eq!(matches.len(), 2);
    assert_eq!(matches[0].start_col, 4);
    assert_eq!(matches[0].end_col, 6);
    assert_eq!(matches[1].start_col, 12);
    assert_eq!(matches[1].end_col, 14);
}

#[test]
fn regex_invalid_returns_empty() {
    let grid = grid_with_rows(&["some text"]);
    let matches = find_matches(&grid, r"[invalid", true, true);
    assert!(matches.is_empty());
}

#[test]
fn regex_case_insensitive() {
    let grid = grid_with_rows(&["Hello World"]);
    let matches = find_matches(&grid, "hello", false, true);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].start_col, 0);
}

// SearchState Navigation

#[test]
fn next_match_wraps_around() {
    let grid = grid_with_rows(&["aaa"]);
    let mut state = SearchState::new();
    state.set_query("a".to_string(), &grid);
    assert_eq!(state.matches().len(), 3);
    assert_eq!(state.focused_index(), 0);

    state.next_match();
    assert_eq!(state.focused_index(), 1);
    state.next_match();
    assert_eq!(state.focused_index(), 2);
    state.next_match();
    assert_eq!(state.focused_index(), 0); // Wrapped.
}

#[test]
fn prev_match_wraps_around() {
    let grid = grid_with_rows(&["aaa"]);
    let mut state = SearchState::new();
    state.set_query("a".to_string(), &grid);
    assert_eq!(state.focused_index(), 0);

    state.prev_match();
    assert_eq!(state.focused_index(), 2); // Wrapped to last.
    state.prev_match();
    assert_eq!(state.focused_index(), 1);
}

#[test]
fn focused_match_returns_correct_match() {
    let grid = grid_with_rows(&["ab ab"]);
    let mut state = SearchState::new();
    state.set_query("ab".to_string(), &grid);
    assert_eq!(state.matches().len(), 2);

    let m = state.focused_match().unwrap();
    assert_eq!(m.start_col, 0);

    state.next_match();
    let m = state.focused_match().unwrap();
    assert_eq!(m.start_col, 3);
}

// Cell Match Type (Binary Search)

#[test]
fn cell_match_type_binary_search() {
    let grid = grid_with_rows(&["hello world"]);
    let mut state = SearchState::new();
    state.set_query("world".to_string(), &grid);
    assert_eq!(state.matches().len(), 1);

    let row = StableRowIndex::from_absolute(&grid, 0);
    // 'w' at col 6.
    assert_eq!(state.cell_match_type(row, 6), MatchType::FocusedMatch);
    assert_eq!(state.cell_match_type(row, 10), MatchType::FocusedMatch);
    // Outside match.
    assert_eq!(state.cell_match_type(row, 0), MatchType::None);
    assert_eq!(state.cell_match_type(row, 5), MatchType::None);
}

#[test]
fn cell_match_type_distinguishes_focused() {
    let grid = grid_with_rows(&["aa aa"]);
    let mut state = SearchState::new();
    state.set_query("aa".to_string(), &grid);
    assert_eq!(state.matches().len(), 2);

    let row = StableRowIndex::from_absolute(&grid, 0);
    // Focused is match 0 (cols 0-1).
    assert_eq!(state.cell_match_type(row, 0), MatchType::FocusedMatch);
    assert_eq!(state.cell_match_type(row, 3), MatchType::Match);

    state.next_match();
    // Now focused is match 1 (cols 3-4).
    assert_eq!(state.cell_match_type(row, 0), MatchType::Match);
    assert_eq!(state.cell_match_type(row, 3), MatchType::FocusedMatch);
}

#[test]
fn cell_match_type_empty_matches() {
    let state = SearchState::new();
    assert_eq!(state.cell_match_type(sri(0), 0), MatchType::None);
}

// Update Query

#[test]
fn update_query_clears_on_empty() {
    let grid = grid_with_rows(&["hello"]);
    let mut state = SearchState::new();
    state.set_query("hello".to_string(), &grid);
    assert_eq!(state.matches().len(), 1);

    state.set_query(String::new(), &grid);
    assert!(state.matches().is_empty());
    assert_eq!(state.focused_index(), 0);
}

#[test]
fn update_query_clamps_focused() {
    let grid = grid_with_rows(&["aaa bbb aaa"]);
    let mut state = SearchState::new();
    state.set_query("aaa".to_string(), &grid);
    assert_eq!(state.matches().len(), 2);

    state.next_match(); // focused = 1
    assert_eq!(state.focused_index(), 1);

    // Change query so only 1 match remains.
    state.set_query("bbb".to_string(), &grid);
    assert_eq!(state.matches().len(), 1);
    assert_eq!(state.focused_index(), 0); // Clamped.
}

// Toggle Modes

#[test]
fn toggle_case_sensitive_reruns_search() {
    let grid = grid_with_rows(&["Hello hello"]);
    let mut state = SearchState::new();
    state.set_query("hello".to_string(), &grid);
    // Default case-insensitive: finds both.
    assert_eq!(state.matches().len(), 2);

    state.toggle_case_sensitive(&grid);
    // Now case-sensitive: only lowercase.
    assert_eq!(state.matches().len(), 1);
    assert!(state.case_sensitive());
}

#[test]
fn toggle_regex_reruns_search() {
    let grid = grid_with_rows(&["abc 123 def"]);
    let mut state = SearchState::new();
    state.set_query(r"\d+".to_string(), &grid);
    // Default plain text: searches for literal "\d+".
    assert_eq!(state.matches().len(), 0);

    state.toggle_regex(&grid);
    // Now regex: finds "123".
    assert_eq!(state.matches().len(), 1);
    assert!(state.use_regex());
}
