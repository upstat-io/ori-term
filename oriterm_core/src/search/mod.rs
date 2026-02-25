//! Terminal search: state, algorithm, and text extraction.
//!
//! Provides plain-text and regex search across the terminal grid
//! (viewport + scrollback) with match navigation and O(log n)
//! per-cell match classification for rendering.

pub mod find;
pub mod text;

use crate::grid::{Grid, StableRowIndex};

/// Per-cell match classification for rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchType {
    /// Cell is not part of any match.
    None,
    /// Cell is part of a non-focused match.
    Match,
    /// Cell is part of the currently focused match.
    FocusedMatch,
}

/// A single match span in stable grid coordinates.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// Stable row of match start.
    pub start_row: StableRowIndex,
    /// Column of match start.
    pub start_col: usize,
    /// Stable row of match end (same row for single-line matches).
    pub end_row: StableRowIndex,
    /// Column of match end (inclusive).
    pub end_col: usize,
}

/// Search session state: query, matches, focused index, navigation.
pub struct SearchState {
    /// Current search query text.
    query: String,
    /// All matches, sorted by position (earliest first).
    matches: Vec<SearchMatch>,
    /// Index of the currently focused match.
    focused: usize,
    /// Case sensitivity toggle.
    case_sensitive: bool,
    /// Regex mode toggle.
    use_regex: bool,
}

impl SearchState {
    /// Create a new empty search state.
    pub fn new() -> Self {
        Self {
            query: String::new(),
            matches: Vec::new(),
            focused: 0,
            case_sensitive: false,
            use_regex: false,
        }
    }

    /// The current search query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// All current matches.
    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Index of the focused match.
    pub fn focused_index(&self) -> usize {
        self.focused
    }

    /// Whether case-sensitive mode is enabled.
    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Whether regex mode is enabled.
    pub fn use_regex(&self) -> bool {
        self.use_regex
    }

    /// Toggle case sensitivity and re-run the search.
    pub fn toggle_case_sensitive(&mut self, grid: &Grid) {
        self.case_sensitive = !self.case_sensitive;
        self.update_query(grid);
    }

    /// Toggle regex mode and re-run the search.
    pub fn toggle_regex(&mut self, grid: &Grid) {
        self.use_regex = !self.use_regex;
        self.update_query(grid);
    }

    /// Advance to the next match, wrapping from last to first.
    pub fn next_match(&mut self) {
        if !self.matches.is_empty() {
            self.focused = (self.focused + 1) % self.matches.len();
        }
    }

    /// Go to the previous match, wrapping from first to last.
    pub fn prev_match(&mut self) {
        if !self.matches.is_empty() {
            self.focused = if self.focused == 0 {
                self.matches.len() - 1
            } else {
                self.focused - 1
            };
        }
    }

    /// Set the query text and re-run the search.
    pub fn set_query(&mut self, query: String, grid: &Grid) {
        self.query = query;
        self.update_query(grid);
    }

    /// Re-run search with current query and settings.
    pub fn update_query(&mut self, grid: &Grid) {
        if self.query.is_empty() {
            self.matches.clear();
            self.focused = 0;
            return;
        }
        self.matches = find::find_matches(grid, &self.query, self.case_sensitive, self.use_regex);
        // Clamp focused index to valid range.
        if self.matches.is_empty() {
            self.focused = 0;
        } else {
            self.focused = self.focused.min(self.matches.len() - 1);
        }
    }

    /// The currently focused match, if any.
    pub fn focused_match(&self) -> Option<&SearchMatch> {
        self.matches.get(self.focused)
    }

    /// Classify a cell's match status for rendering.
    ///
    /// Uses binary search via `partition_point` for O(log n) lookup.
    pub fn cell_match_type(&self, stable_row: StableRowIndex, col: usize) -> MatchType {
        if self.matches.is_empty() {
            return MatchType::None;
        }

        // Binary search: find the first match whose start is beyond (row, col).
        let idx = self
            .matches
            .partition_point(|m| (m.start_row, m.start_col) <= (stable_row, col));

        // Check a small window around the found index for containment.
        let start = idx.saturating_sub(1);
        let end = (idx + 1).min(self.matches.len());

        for i in start..end {
            if cell_in_match(&self.matches[i], stable_row, col) {
                return if i == self.focused {
                    MatchType::FocusedMatch
                } else {
                    MatchType::Match
                };
            }
        }

        MatchType::None
    }
}

impl Default for SearchState {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if `(stable_row, col)` falls within a match span.
fn cell_in_match(m: &SearchMatch, stable_row: StableRowIndex, col: usize) -> bool {
    if stable_row < m.start_row || stable_row > m.end_row {
        return false;
    }
    if m.start_row == m.end_row {
        // Single-row match.
        return col >= m.start_col && col <= m.end_col;
    }
    // Multi-row match.
    if stable_row == m.start_row {
        col >= m.start_col
    } else if stable_row == m.end_row {
        col <= m.end_col
    } else {
        // Middle row: entirely contained.
        true
    }
}

#[cfg(test)]
mod tests;
