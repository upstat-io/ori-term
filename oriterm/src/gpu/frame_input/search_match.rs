//! Search match cell classification helper.

use oriterm_core::grid::StableRowIndex;

use oriterm_core::SearchMatch;

/// Check if `(stable_row, col)` falls within a search match span.
pub(super) fn cell_in_search_match(
    m: &SearchMatch,
    stable_row: StableRowIndex,
    col: usize,
) -> bool {
    if stable_row < m.start_row || stable_row > m.end_row {
        return false;
    }
    if m.start_row == m.end_row {
        return col >= m.start_col && col <= m.end_col;
    }
    if stable_row == m.start_row {
        col >= m.start_col
    } else if stable_row == m.end_row {
        col <= m.end_col
    } else {
        true
    }
}
