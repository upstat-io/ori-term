//! Search algorithm: finds all matches in the grid for a given query.
//!
//! Supports both plain text and regex search, row by row across
//! the entire grid (scrollback + viewport).

use regex::RegexBuilder;

use crate::grid::{Grid, StableRowIndex};

use super::SearchMatch;
use super::text::{byte_span_to_cols, extract_row_text};

/// Per-row context for match finding.
struct RowCtx<'a> {
    text: &'a str,
    col_map: &'a [usize],
    stable_row: StableRowIndex,
}

/// Find all matches in the grid, sorted by position (earliest first).
///
/// Searches every row (scrollback + viewport). Invalid regex returns an
/// empty result without panicking.
pub fn find_matches(
    grid: &Grid,
    query: &str,
    case_sensitive: bool,
    use_regex: bool,
) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }

    let total_rows = grid.total_lines();
    let mut matches = Vec::new();

    for abs_row in 0..total_rows {
        let Some(row) = grid.absolute_row(abs_row) else {
            continue;
        };
        let (text, col_map) = extract_row_text(row);
        if text.is_empty() {
            continue;
        }

        let ctx = RowCtx {
            text: &text,
            col_map: &col_map,
            stable_row: StableRowIndex::from_absolute(grid, abs_row),
        };

        if use_regex {
            find_regex_matches(&ctx, query, case_sensitive, &mut matches);
        } else {
            find_plain_matches(&ctx, query, case_sensitive, &mut matches);
        }
    }

    // Invariant relied upon by `FrameSearch::cell_match_type` (oriterm crate):
    // matches are sorted by `(start_row, start_col)` and have unique starts,
    // so a binary-search window of 2 (idx-1, idx, idx+1) is sufficient to
    // classify any visible cell. Row-major iteration above + per-match
    // advancement in `find_plain_matches`/`find_regex_matches` (≥1 char step)
    // produces this shape; assert it explicitly so future refactors that
    // emit duplicates or out-of-order matches break loudly under debug.
    debug_assert!(
        matches
            .windows(2)
            .all(|w| (w[0].start_row, w[0].start_col) < (w[1].start_row, w[1].start_col)),
        "find_matches must produce strictly-increasing (start_row, start_col) keys"
    );

    matches
}

/// Emit a match from a byte span, mapping to column coordinates.
fn emit_match(ctx: &RowCtx<'_>, byte_start: usize, byte_end: usize, out: &mut Vec<SearchMatch>) {
    if let Some((start_col, end_col)) =
        byte_span_to_cols(ctx.text, ctx.col_map, byte_start, byte_end)
    {
        out.push(SearchMatch {
            start_row: ctx.stable_row,
            start_col,
            end_row: ctx.stable_row,
            end_col,
        });
    }
}

/// Find plain text matches in a single row's text.
fn find_plain_matches(
    ctx: &RowCtx<'_>,
    query: &str,
    case_sensitive: bool,
    out: &mut Vec<SearchMatch>,
) {
    let (haystack, needle);
    let (hay_ref, need_ref) = if case_sensitive {
        (ctx.text, query)
    } else {
        haystack = ctx.text.to_lowercase();
        needle = query.to_lowercase();
        (haystack.as_str(), needle.as_str())
    };

    let mut start = 0;
    while start < hay_ref.len() {
        // Safe: `start` always lies on a char boundary because we advance
        // by at least one full char (via `char_len`) after each match.
        let Some(tail) = hay_ref.get(start..) else {
            break;
        };
        let Some(pos) = tail.find(need_ref) else {
            break;
        };
        let byte_start = start + pos;
        let byte_end = byte_start + need_ref.len();

        emit_match(ctx, byte_start, byte_end, out);

        // Advance past the first byte of this match. Step by one full
        // char to stay on a char boundary.
        let char_len = hay_ref
            .get(byte_start..)
            .and_then(|s| s.chars().next())
            .map_or(1, char::len_utf8);
        start = byte_start + char_len;
    }
}

/// Find regex matches in a single row's text.
fn find_regex_matches(
    ctx: &RowCtx<'_>,
    pattern: &str,
    case_sensitive: bool,
    out: &mut Vec<SearchMatch>,
) {
    let Ok(re) = RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive)
        .build()
    else {
        return;
    };

    for m in re.find_iter(ctx.text) {
        if m.start() == m.end() {
            continue; // Skip zero-length matches.
        }
        emit_match(ctx, m.start(), m.end(), out);
    }
}
