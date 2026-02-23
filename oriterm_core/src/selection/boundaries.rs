//! Word and line boundary detection for selection.
//!
//! `word_boundaries` finds contiguous same-class character regions around
//! a click position. `logical_line_start`/`logical_line_end` walk the WRAP
//! flag chain to find the extent of soft-wrapped lines.

use crate::cell::CellFlags;
use crate::grid::Grid;
use crate::index::Column;

/// Default word delimiter characters.
///
/// Matches Alacritty's `semantic_escape_chars`. Characters in this set stop
/// word expansion during double-click selection. Everything not in this set
/// is treated as a word character. For example, `-` is not in the default
/// set, so `hello-world` selects as one word.
pub const DEFAULT_WORD_DELIMITERS: &str = ",│`|:\"' ()[]{}<>\t";

/// Character classification for word boundary detection.
///
/// Returns 0 for word characters, 1 for whitespace delimiters (space, null,
/// tab), 2 for non-whitespace delimiters. The `word_delimiters` string is
/// the authoritative set of boundary characters — anything not in it is a
/// word character.
pub(crate) fn delimiter_class(c: char, word_delimiters: &str) -> u8 {
    if c == '\0' {
        1
    } else if c == ' ' || c == '\t' {
        // Whitespace always gets its own class so spaces group together.
        1
    } else if word_delimiters.contains(c) {
        2
    } else {
        0
    }
}

/// Find word boundaries around (`abs_row`, `col`) in the grid.
///
/// Returns (`start_col`, `end_col`) inclusive. Wide-char spacers are
/// redirected to their base cell and skipped during scanning so that
/// double-clicking a CJK character selects the full character.
///
/// `word_delimiters` controls which characters act as word boundaries.
/// Pass [`DEFAULT_WORD_DELIMITERS`] for standard behavior.
pub fn word_boundaries(
    grid: &Grid,
    abs_row: usize,
    col: usize,
    word_delimiters: &str,
) -> (usize, usize) {
    let row = match grid.absolute_row(abs_row) {
        Some(r) => r,
        None => return (col, col),
    };

    let cols = row.cols();
    if cols == 0 || col >= cols {
        return (col, col);
    }

    // If clicked on a wide-char spacer, redirect to the base cell.
    let click_col = if row[Column(col)].flags.contains(CellFlags::WIDE_CHAR_SPACER) && col > 0 {
        col - 1
    } else {
        col
    };

    let ch = row[Column(click_col)].ch;
    let class = delimiter_class(ch, word_delimiters);

    // Scan left, skipping wide-char spacers.
    let mut start = click_col;
    while start > 0 {
        let prev = start - 1;
        if row[Column(prev)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
            && prev > 0
        {
            // Spacer: check the base cell before it.
            if delimiter_class(row[Column(prev - 1)].ch, word_delimiters) == class {
                start = prev - 1;
            } else {
                break;
            }
        } else if delimiter_class(row[Column(prev)].ch, word_delimiters) == class {
            start = prev;
        } else {
            break;
        }
    }

    // Scan right, skipping wide-char spacers.
    let mut end = click_col;
    while end + 1 < cols {
        let next = end + 1;
        if row[Column(next)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
        {
            // Spacer belongs to the wide char at `end` — include it.
            end = next;
            continue;
        }
        if delimiter_class(row[Column(next)].ch, word_delimiters) == class {
            end = next;
        } else {
            break;
        }
    }

    (start, end)
}

/// Walk backwards to find the start of a logical (soft-wrapped) line.
///
/// Returns the absolute row index of the first row in the logical line.
pub fn logical_line_start(grid: &Grid, abs_row: usize) -> usize {
    let mut current = abs_row;
    while current > 0 {
        let prev = current - 1;
        let Some(row) = grid.absolute_row(prev) else {
            break;
        };
        // The WRAP flag on a row means it continues onto the next row.
        let last_col = row.cols().saturating_sub(1);
        if row[Column(last_col)].flags.contains(CellFlags::WRAP) {
            current = prev;
        } else {
            break;
        }
    }
    current
}

/// Walk forwards to find the end of a logical (soft-wrapped) line.
///
/// Returns the absolute row index of the last row in the logical line.
pub fn logical_line_end(grid: &Grid, abs_row: usize) -> usize {
    let mut current = abs_row;
    loop {
        let Some(row) = grid.absolute_row(current) else {
            break;
        };
        let last_col = row.cols().saturating_sub(1);
        if row[Column(last_col)].flags.contains(CellFlags::WRAP) {
            current += 1;
        } else {
            break;
        }
    }
    current
}
