//! Search state snapshotted for one frame.
//!
//! [`FrameSearch`] captures the search matches, focused match index, and query
//! from a pane snapshot. The Prepare phase uses [`cell_match_type`](FrameSearch::cell_match_type)
//! to classify each cell for match/focused-match highlighting.

use oriterm_core::SearchMatch;
use oriterm_core::grid::StableRowIndex;
use oriterm_core::search::MatchType;
use oriterm_mux::PaneSnapshot;

use super::search_match::cell_in_search_match;

/// Search state for one frame.
///
/// Built from a [`PaneSnapshot`] when search is active. The Prepare phase
/// queries [`cell_match_type`](Self::cell_match_type) per cell to classify
/// it as `None`, `Match`, or `FocusedMatch`.
#[derive(Debug)]
pub struct FrameSearch {
    /// Matches from the search state (copied per frame).
    matches: Vec<SearchMatch>,
    /// Index of the focused match.
    focused: usize,
    /// Stable row index of viewport line 0.
    base_stable: u64,
    /// Total match count (for search bar "N of M" display).
    match_count: usize,
    /// Query string (for search bar display).
    query: String,
}

impl FrameSearch {
    /// Build from snapshot search data.
    ///
    /// Converts wire-format search matches into `SearchMatch` values
    /// for client-side highlight rendering. Used in daemon mode where
    /// search state lives on the server.
    pub fn from_snapshot(snapshot: &PaneSnapshot) -> Option<Self> {
        if !snapshot.search_active {
            return None;
        }
        let matches: Vec<SearchMatch> = snapshot
            .search_matches
            .iter()
            .map(|m| SearchMatch {
                start_row: StableRowIndex(m.start_row),
                start_col: m.start_col as usize,
                end_row: StableRowIndex(m.end_row),
                end_col: m.end_col as usize,
            })
            .collect();
        let match_count = matches.len();
        let focused = snapshot.search_focused.map_or(0, |f| f as usize);
        Some(Self {
            matches,
            focused,
            base_stable: snapshot.stable_row_base,
            match_count,
            query: snapshot.search_query.clone(),
        })
    }

    /// Refill this `FrameSearch` from a snapshot, reusing allocations.
    ///
    /// Returns `false` if search is not active (caller should set field to `None`).
    #[allow(
        dead_code,
        reason = "infrastructure for allocation-reusing extract path"
    )]
    pub fn update_from_snapshot(&mut self, snapshot: &PaneSnapshot) -> bool {
        if !snapshot.search_active {
            return false;
        }
        self.matches.clear();
        self.matches
            .extend(snapshot.search_matches.iter().map(|m| SearchMatch {
                start_row: StableRowIndex(m.start_row),
                start_col: m.start_col as usize,
                end_row: StableRowIndex(m.end_row),
                end_col: m.end_col as usize,
            }));
        self.match_count = self.matches.len();
        self.focused = snapshot.search_focused.map_or(0, |f| f as usize);
        self.base_stable = snapshot.stable_row_base;
        self.query.clear();
        self.query.push_str(&snapshot.search_query);
        true
    }

    /// Classify a visible cell for search match highlighting.
    pub fn cell_match_type(&self, viewport_line: usize, col: usize) -> MatchType {
        if self.matches.is_empty() {
            return MatchType::None;
        }
        let stable = StableRowIndex(self.base_stable + viewport_line as u64);

        // Binary search: find first match whose start is beyond (row, col).
        let idx = self
            .matches
            .partition_point(|m| (m.start_row, m.start_col) <= (stable, col));

        let start = idx.saturating_sub(1);
        let end = (idx + 1).min(self.matches.len());

        for i in start..end {
            if cell_in_search_match(&self.matches[i], stable, col) {
                return if i == self.focused {
                    MatchType::FocusedMatch
                } else {
                    MatchType::Match
                };
            }
        }
        MatchType::None
    }

    /// Total number of matches.
    pub fn match_count(&self) -> usize {
        self.match_count
    }

    /// 1-based focused match index (for "N of M" display).
    pub fn focused_display(&self) -> usize {
        if self.match_count == 0 {
            0
        } else {
            self.focused + 1
        }
    }

    /// The current query string.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Build a test search snapshot from manually constructed matches.
    ///
    /// `focused` is the index into `matches` of the focused match.
    /// `stable_row_base` maps viewport line 0 to stable row coordinates.
    #[cfg(test)]
    pub fn for_test(matches: Vec<SearchMatch>, focused: usize, stable_row_base: u64) -> Self {
        Self {
            match_count: matches.len(),
            matches,
            focused,
            base_stable: stable_row_base,
            query: String::from("test"),
        }
    }
}
