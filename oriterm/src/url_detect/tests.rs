//! Unit tests for URL detection engine.

use oriterm_core::cell::CellFlags;
use oriterm_core::{Column, Grid, Line};

use super::{
    DetectedUrl, UrlDetectCache, detect_urls_in_logical_line, row_continues_for_url,
    trim_url_trailing,
};

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

#[test]
fn detect_simple_url() {
    let grid = grid_with_rows(&["Visit https://example.com for info"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com");
    assert_eq!(urls[0].segments.len(), 1);
    assert_eq!(urls[0].segments[0], (0, 6, 24));
}

#[test]
fn detect_multiple_urls() {
    let grid = grid_with_rows(&["see https://a.com and http://b.com/x ok"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0].url, "https://a.com");
    assert_eq!(urls[1].url, "http://b.com/x");
}

#[test]
fn detect_url_with_balanced_parens() {
    let grid = grid_with_rows(&["see https://en.wikipedia.org/wiki/Rust_(language) ok"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://en.wikipedia.org/wiki/Rust_(language)");
}

#[test]
fn no_urls_in_plain_text() {
    let grid = grid_with_rows(&["just plain text here"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert!(urls.is_empty());
}

#[test]
fn detect_wrapped_url() {
    // 20-col grid: URL wraps to second row.
    let mut grid = Grid::with_scrollback(2, 20, 0);
    let text = "go https://example.com/long/path ok";
    // Write first 20 chars to row 0, remaining to row 1.
    for (i, ch) in text.chars().take(20).enumerate() {
        grid[Line(0)][Column(i)].ch = ch;
    }
    // Mark row 0 as wrapped (last cell has WRAP flag).
    grid[Line(0)][Column(19)].flags.insert(CellFlags::WRAP);
    for (i, ch) in text.chars().skip(20).enumerate() {
        grid[Line(1)][Column(i)].ch = ch;
    }

    let urls = detect_urls_in_logical_line(&grid, 0, 1);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com/long/path");
    assert_eq!(urls[0].segments.len(), 2);
    // First segment: starts at col 3 on row 0, goes to col 19.
    assert_eq!(urls[0].segments[0].0, 0);
    assert_eq!(urls[0].segments[0].1, 3);
    assert_eq!(urls[0].segments[0].2, 19);
    // Second segment: continues on row 1.
    assert_eq!(urls[0].segments[1].0, 1);
}

#[test]
fn url_contains() {
    let url = DetectedUrl {
        segments: vec![(5, 3, 19), (6, 0, 10)],
        url: "https://example.com/long/path".to_string(),
    };
    assert!(url.contains(5, 3));
    assert!(url.contains(5, 19));
    assert!(url.contains(6, 0));
    assert!(url.contains(6, 10));
    assert!(!url.contains(5, 2));
    assert!(!url.contains(5, 20));
    assert!(!url.contains(6, 11));
    assert!(!url.contains(7, 0));
}

#[test]
fn trim_trailing_punctuation() {
    assert_eq!(
        trim_url_trailing("https://example.com."),
        "https://example.com"
    );
    assert_eq!(
        trim_url_trailing("https://example.com,"),
        "https://example.com"
    );
    assert_eq!(
        trim_url_trailing("https://example.com;"),
        "https://example.com"
    );
    assert_eq!(
        trim_url_trailing("https://example.com:"),
        "https://example.com"
    );
    assert_eq!(
        trim_url_trailing("https://example.com!"),
        "https://example.com"
    );
    assert_eq!(
        trim_url_trailing("https://example.com?"),
        "https://example.com"
    );
}

#[test]
fn trim_preserves_balanced_parens() {
    assert_eq!(
        trim_url_trailing("https://en.wikipedia.org/wiki/Rust_(language)"),
        "https://en.wikipedia.org/wiki/Rust_(language)"
    );
}

#[test]
fn trim_strips_unbalanced_parens() {
    assert_eq!(
        trim_url_trailing("https://example.com)"),
        "https://example.com"
    );
}

#[test]
fn no_false_positive_bare_scheme() {
    let grid = grid_with_rows(&["the word https is not a url"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert!(urls.is_empty());
}

#[test]
fn ftp_and_file_schemes() {
    let grid = grid_with_rows(&["ftp://files.example.com/pub file://localhost/tmp"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 2);
    assert_eq!(urls[0].url, "ftp://files.example.com/pub");
    assert_eq!(urls[1].url, "file://localhost/tmp");
}

// --- High priority: scrollback and boundary tests ---

#[test]
fn detect_url_in_scrollback() {
    // URL in the visible viewport gets scrolled into scrollback.
    let mut grid = Grid::with_scrollback(3, 40, 100);
    let text = "Visit https://example.com for info";
    for (col, ch) in text.chars().enumerate() {
        grid[Line(0)][Column(col)].ch = ch;
    }

    // Push all visible rows into scrollback.
    grid.scroll_up(3);
    assert_eq!(grid.scrollback().len(), 3);

    // abs_row 0 is the oldest scrollback row (our URL row).
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com");
}

#[test]
fn detect_wrapped_url_across_scrollback_boundary() {
    // URL spans 2 rows. Row 0 gets scrolled into scrollback, row 1 stays visible.
    let mut grid = Grid::with_scrollback(3, 20, 100);
    let text = "go https://example.com/long/path ok";

    for (i, ch) in text.chars().take(20).enumerate() {
        grid[Line(0)][Column(i)].ch = ch;
    }
    grid[Line(0)][Column(19)].flags.insert(CellFlags::WRAP);
    for (i, ch) in text.chars().skip(20).enumerate() {
        grid[Line(1)][Column(i)].ch = ch;
    }

    // Push row 0 to scrollback. Now row 1 becomes visible row 0.
    grid.scroll_up(1);
    assert_eq!(grid.scrollback().len(), 1);

    // The URL spans abs_row 0 (scrollback) and abs_row 1 (visible row 0).
    let urls = detect_urls_in_logical_line(&grid, 0, 1);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com/long/path");
    assert_eq!(urls[0].segments.len(), 2);
    assert_eq!(urls[0].segments[0].0, 0); // scrollback row
    assert_eq!(urls[0].segments[1].0, 1); // visible row
}

#[test]
fn cache_invalidation_clears_stale_urls() {
    let mut grid = Grid::with_scrollback(1, 40, 0);
    let text = "Visit https://example.com ok";
    for (col, ch) in text.chars().enumerate() {
        grid[Line(0)][Column(col)].ch = ch;
    }

    let mut cache = UrlDetectCache::default();

    // First lookup — should detect and cache the URL.
    let hit = cache.url_at(&grid, 0, 10);
    assert!(hit.is_some());
    assert_eq!(hit.unwrap().url, "https://example.com");

    // Modify the grid content — replace URL with plain text.
    let replacement = "Visit some-plain-text-here ok";
    for (col, ch) in replacement.chars().enumerate() {
        grid[Line(0)][Column(col)].ch = ch;
    }
    // Clear remaining columns.
    for col in replacement.len()..40 {
        grid[Line(0)][Column(col)].ch = '\0';
    }

    // Without invalidation, cache returns stale data.
    let stale = cache.url_at(&grid, 0, 10);
    assert!(
        stale.is_some(),
        "cache still has stale URL before invalidation"
    );

    // Invalidate and re-query — no URL should be found.
    cache.invalidate();
    let fresh = cache.url_at(&grid, 0, 10);
    assert!(
        fresh.is_none(),
        "after invalidation, stale URL should be gone"
    );
}

#[test]
fn out_of_bounds_row_returns_empty() {
    let grid = Grid::with_scrollback(2, 40, 0);

    // abs_row 100 is way beyond the grid — should not panic.
    let urls = detect_urls_in_logical_line(&grid, 100, 100);
    assert!(urls.is_empty());
}

// --- Medium priority: edge cases and robustness ---

#[test]
fn url_adjacent_to_wide_chars() {
    // Wide chars (CJK) around URL: 漢 https://ex.com 漢
    // 漢 takes 2 columns: col 0 = base (WIDE_CHAR), col 1 = spacer (WIDE_CHAR_SPACER).
    let mut grid = Grid::with_scrollback(1, 30, 0);

    // Col 0-1: wide char 漢.
    grid[Line(0)][Column(0)].ch = '漢';
    grid[Line(0)][Column(0)].flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(1)].ch = ' ';
    grid[Line(0)][Column(1)]
        .flags
        .insert(CellFlags::WIDE_CHAR_SPACER);

    // Col 2: space.
    grid[Line(0)][Column(2)].ch = ' ';

    // Col 3-16: "https://ex.com" (14 chars).
    let url = "https://ex.com";
    for (i, ch) in url.chars().enumerate() {
        grid[Line(0)][Column(3 + i)].ch = ch;
    }

    // Col 17: space.
    grid[Line(0)][Column(17)].ch = ' ';

    // Col 18-19: wide char 漢.
    grid[Line(0)][Column(18)].ch = '漢';
    grid[Line(0)][Column(18)].flags.insert(CellFlags::WIDE_CHAR);
    grid[Line(0)][Column(19)].ch = ' ';
    grid[Line(0)][Column(19)]
        .flags
        .insert(CellFlags::WIDE_CHAR_SPACER);

    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://ex.com");
    // Verify the segment column range maps to the correct grid columns.
    assert_eq!(urls[0].segments[0].1, 3); // start col
    assert_eq!(urls[0].segments[0].2, 16); // end col (inclusive)
}

#[test]
fn hit_test_at_wrapped_segment_boundaries() {
    // URL wraps: last cell of first segment and first cell of second segment.
    let url = DetectedUrl {
        segments: vec![(0, 5, 19), (1, 0, 8)],
        url: "https://example.com/path".to_string(),
    };

    // Last cell of first segment.
    assert!(url.contains(0, 19));
    // First cell of second segment.
    assert!(url.contains(1, 0));
    // One column past end of first segment (no wrap gap).
    assert!(!url.contains(0, 20));
    // One column before start of first segment.
    assert!(!url.contains(0, 4));
    // One column past end of second segment.
    assert!(!url.contains(1, 9));
}

#[test]
fn gap_between_urls_returns_no_hit() {
    let grid = grid_with_rows(&["https://a.com XXX https://b.com"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 2);

    // Hit first URL.
    assert!(urls[0].contains(0, 0));
    assert!(urls[0].contains(0, 12));

    // Gap: "XXX" between col 14 and col 17.
    assert!(!urls[0].contains(0, 14));
    assert!(!urls[0].contains(0, 15));
    assert!(!urls[1].contains(0, 14));
    assert!(!urls[1].contains(0, 15));

    // Hit second URL.
    assert!(urls[1].contains(0, 18));
}

#[test]
fn url_with_query_string_and_fragment() {
    let grid = grid_with_rows(&["see https://example.com/path?q=a&b=c#section ok"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com/path?q=a&b=c#section");
}

// --- Low priority: edge cases ---

#[test]
fn nested_balanced_parentheses() {
    let grid = grid_with_rows(&["see https://example.com/wiki/A_(B_(C)) ok"]);
    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com/wiki/A_(B_(C))");
}

#[test]
fn url_ending_at_last_column() {
    // URL ends exactly at the last column (col 29 on a 30-col grid).
    let mut grid = Grid::with_scrollback(1, 30, 0);
    //              0         1         2
    //              0123456789012345678901234567890
    let text = "go https://example.com/abcdef";
    assert_eq!(text.len(), 29);
    for (col, ch) in text.chars().enumerate() {
        grid[Line(0)][Column(col)].ch = ch;
    }
    // Last cell (col 29) is the last char of the URL, no trailing space.

    let urls = detect_urls_in_logical_line(&grid, 0, 0);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, "https://example.com/abcdef");
    assert_eq!(urls[0].segments[0].2, 28); // end col (inclusive, 'f')
}

#[test]
fn row_continues_heuristic() {
    // WRAP flag → row continues.
    let mut grid = Grid::with_scrollback(2, 10, 0);
    grid[Line(0)][Column(9)].flags.insert(CellFlags::WRAP);
    assert!(row_continues_for_url(&grid, 0));

    // Non-empty last cell without WRAP → row continues (filled heuristic).
    let mut grid = Grid::with_scrollback(2, 10, 0);
    grid[Line(0)][Column(9)].ch = 'x';
    assert!(row_continues_for_url(&grid, 0));

    // Empty/space last cell without WRAP → row does not continue.
    let mut grid = Grid::with_scrollback(2, 10, 0);
    grid[Line(0)][Column(9)].ch = ' ';
    assert!(!row_continues_for_url(&grid, 0));

    // Null last cell → row does not continue.
    let grid = Grid::with_scrollback(2, 10, 0);
    assert!(!row_continues_for_url(&grid, 0));
}

#[test]
fn url_spanning_three_rows() {
    // A long URL wrapping across 3 rows on a 20-col grid.
    let mut grid = Grid::with_scrollback(3, 20, 0);
    let url_text = "https://example.com/very/long/path/that/wraps";
    let prefix = "go ";
    let full = format!("{prefix}{url_text}");

    // Distribute chars across rows.
    for (i, ch) in full.chars().enumerate() {
        let row = i / 20;
        let col = i % 20;
        if row < 3 {
            grid[Line(row as i32)][Column(col)].ch = ch;
        }
    }
    // Mark row 0 and row 1 as wrapped.
    grid[Line(0)][Column(19)].flags.insert(CellFlags::WRAP);
    grid[Line(1)][Column(19)].flags.insert(CellFlags::WRAP);

    let urls = detect_urls_in_logical_line(&grid, 0, 2);
    assert_eq!(urls.len(), 1);
    assert_eq!(urls[0].url, url_text);
    assert_eq!(urls[0].segments.len(), 3, "URL should span 3 row segments");
    assert_eq!(urls[0].segments[0].0, 0);
    assert_eq!(urls[0].segments[1].0, 1);
    assert_eq!(urls[0].segments[2].0, 2);
}
