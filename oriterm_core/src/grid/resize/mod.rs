//! Grid resize and text reflow.
//!
//! `Grid::resize` is the main entry point: it handles dimension changes,
//! scroll region reset, cursor clamping, and optional text reflow on
//! column changes. Row resize manages scrollback interaction (shrink
//! pushes rows to scrollback, grow pulls them back). Column reflow rewrites
//! cell-by-cell to wrap/unwrap soft-wrapped lines (matches Ghostty
//! `src/terminal/Screen.zig` reflow); `reflow_cells` and `reflow_row_cells`
//! live in the sibling `reflow` submodule.

mod reflow;

use crate::index::Column;

use super::Grid;
use super::row::Row;

use reflow::{ReflowOutcome, ReflowParams, reflow_cells};

/// Maps old absolute row indices to result row indices after reflow.
///
/// Built during `reflow_cells` with O(1) per-row overhead. Consumed by
/// `ImageCache::remap_placements` to translate cache-coordinate
/// placement `StableRowIndex` values across a reflow operation.
///
/// For each source row, `first_output_row[src_idx]` gives the output
/// row index where that source row's first cell landed. Wrapped source
/// rows may share the same output row as their neighbors (unwrap case),
/// and a single source row may span multiple output rows (wrap case) —
/// the mapping always records the FIRST landing row for consistent
/// placement remapping.
///
/// `old_total_evicted` is captured BEFORE `scrollback.clear()` so
/// consumers can convert `StableRowIndex(X)` → old absolute row via
/// `X.checked_sub(old_total_evicted)`.
#[derive(Debug, Clone)]
pub struct ReflowMapping {
    /// Per source row: the output row index where that row's first cell
    /// landed. Always `all_rows.len()` long after reflow — every source
    /// row maps to exactly one output row.
    pub first_output_row: Vec<usize>,
    /// Old `total_evicted` value (pre-reflow). Subtract from
    /// `StableRowIndex.0` to get the pre-reflow absolute row index.
    pub old_total_evicted: u64,
}

impl Grid {
    /// Resize the grid to new dimensions.
    ///
    /// When `reflow` is true, soft-wrapped lines are re-wrapped to fit the
    /// new column width (cell-by-cell rewriting). When false, rows are simply
    /// truncated or extended (for alternate screen).
    ///
    /// Returns `Some(ReflowMapping)` when reflow actually occurred (column
    /// count changed AND `reflow` was true) so consumers (e.g.
    /// `ImageCache::remap_placements`) can translate row-indexed state
    /// through the reflow. Returns `None` when no reflow occurred.
    ///
    /// Resets scroll region, clamps cursor, and marks everything dirty.
    pub fn resize(
        &mut self,
        new_lines: usize,
        new_cols: usize,
        reflow: bool,
    ) -> Option<ReflowMapping> {
        if new_cols == 0 || new_lines == 0 {
            return None;
        }
        if new_cols == self.cols && new_lines == self.lines {
            return None;
        }

        let mapping = if reflow && new_cols != self.cols {
            if new_cols > self.cols {
                // Growing cols: reflow first (unwrap), then adjust rows.
                let m = self.reflow_cols(new_cols);
                self.cols = new_cols;
                Self::reset_tab_stops(&mut self.tab_stops, new_cols);
                self.resize_rows(new_lines);
                m
            } else {
                // Shrinking cols: adjust rows first, then reflow (wrap).
                self.resize_rows(new_lines);
                let m = self.reflow_cols(new_cols);
                self.cols = new_cols;
                Self::reset_tab_stops(&mut self.tab_stops, new_cols);
                m
            }
        } else {
            self.resize_no_reflow(new_cols, new_lines);
            None
        };

        // Reset scroll region, clamp cursor, mark dirty.
        self.finalize_resize();
        mapping
    }

    /// Resize without text reflow (for alt screen or same-width changes).
    fn resize_no_reflow(&mut self, new_cols: usize, new_lines: usize) {
        self.resize_rows(new_lines);
        if new_cols != self.cols {
            for row in &mut self.rows {
                row.resize(new_cols);
            }
            self.cols = new_cols;
            Self::reset_tab_stops(&mut self.tab_stops, new_cols);
        }
    }

    /// Common post-resize cleanup: scroll region, cursor clamping, dirty.
    ///
    /// `dirty.resize()` marks all dirty only when the line/column count
    /// actually changed. Callers that reflow content should call
    /// `dirty.mark_all()` explicitly if needed.
    fn finalize_resize(&mut self) {
        self.scroll_region = 0..self.lines;
        self.left_margin = 0;
        self.right_margin = self.cols.saturating_sub(1);

        let max_line = self.lines.saturating_sub(1);
        let max_col = self.cols.saturating_sub(1);
        if self.cursor.line() > max_line {
            self.cursor.set_line(max_line);
        }
        if self.cursor.col().0 > max_col {
            self.cursor.set_col(Column(max_col));
        }
        if let Some(saved) = &mut self.saved_cursor {
            if saved.line() > max_line {
                saved.set_line(max_line);
            }
            if saved.col().0 > max_col {
                saved.set_col(Column(max_col));
            }
        }

        // Reset to live view. Reflow rewrites scrollback entirely, so the
        // old display_offset no longer points at the same content. Keeping a
        // stale offset causes the renderer to show corrupted/duplicated
        // scrollback instead of the live cursor position.
        self.display_offset = 0;
        self.dirty.resize(self.lines, self.cols);
    }

    /// Resize the number of visible lines.
    fn resize_rows(&mut self, new_lines: usize) {
        if new_lines == self.lines {
            return;
        }
        if new_lines < self.lines {
            self.shrink_rows(new_lines);
        } else {
            self.grow_rows(new_lines);
        }
        self.lines = new_lines;
        self.dirty.resize(new_lines, self.cols);
    }

    /// Shrink visible rows: trim trailing blanks, push excess to scrollback.
    fn shrink_rows(&mut self, new_lines: usize) {
        let to_remove = self.lines - new_lines;
        let trimmed = self.count_trailing_blank_rows(to_remove);
        for _ in 0..trimmed {
            self.rows.pop();
        }
        let push_count = (to_remove - trimmed).min(self.rows.len());
        for row in self.rows.drain(..push_count) {
            if self.scrollback.push(row).is_some() {
                self.total_evicted += 1;
            }
        }
        self.cursor
            .set_line(self.cursor.line().saturating_sub(push_count));
        self.rows.truncate(new_lines);
        while self.rows.len() < new_lines {
            self.rows.push(Row::new(self.cols));
        }
    }

    /// Grow visible rows: restore scrollback content and add blank rows.
    ///
    /// When the cursor is at the bottom, rows that were pushed to scrollback
    /// by a previous shrink are pulled back and inserted at the top of the
    /// visible area. This preserves terminal output across shrink/grow cycles
    /// (e.g. window minimize → restore). Rows are resized to the current
    /// column width if they were pushed at a different width.
    fn grow_rows(&mut self, new_lines: usize) {
        let delta = new_lines - self.lines;
        if self.cursor.line() >= self.lines.saturating_sub(1) {
            let from_sb = delta.min(self.scrollback.len());
            // Restore scrollback rows (newest = most recently pushed = bottom
            // of the restored block). Pop newest-first, then reverse so the
            // oldest restored row is at the top.
            let cols = self.cols;
            let mut restored: Vec<Row> = Vec::with_capacity(from_sb);
            for _ in 0..from_sb {
                if let Some(mut row) = self.scrollback.pop_newest() {
                    row.resize(cols);
                    restored.push(row);
                }
            }
            restored.reverse();
            self.resize_pushed = self.resize_pushed.saturating_sub(from_sb);
            // Insert restored rows at top, shifting cursor down.
            self.rows.splice(0..0, restored);
            self.cursor.set_line(self.cursor.line() + from_sb);
            for _ in 0..(delta - from_sb) {
                self.rows.push(Row::new(self.cols));
            }
        } else {
            for _ in 0..delta {
                self.rows.push(Row::new(self.cols));
            }
        }
    }

    /// Count trailing blank rows from the bottom, below the cursor.
    fn count_trailing_blank_rows(&self, max: usize) -> usize {
        let len = self.rows.len();
        let mut count = 0;
        while count < max && len > count + 1 {
            let idx = len - 1 - count;
            if idx <= self.cursor.line() {
                break;
            }
            if !self.rows[idx].is_blank() {
                break;
            }
            count += 1;
        }
        count
    }

    /// Reflow content to fit new column width using cell-by-cell rewriting.
    ///
    /// Handles both growing (unwrapping) and shrinking (re-wrapping).
    /// Cursor position is tracked through the reflow.
    ///
    /// Returns `Some(ReflowMapping)` with the per-source-row output-row
    /// mapping when reflow actually runs. Returns `None` only when
    /// `old_cols == new_cols` or `new_cols == 0` (no-op guards).
    fn reflow_cols(&mut self, new_cols: usize) -> Option<ReflowMapping> {
        let old_cols = self.cols;
        if old_cols == new_cols || new_cols == 0 {
            return None;
        }

        // Capture BEFORE collect_all_rows + apply_reflow_result — the
        // mapping's old_total_evicted must reflect the state that
        // pre-reflow StableRowIndex values were computed against.
        let old_total_evicted = self.total_evicted as u64;

        // Collect all rows: scrollback (oldest first) then visible.
        let (all_rows, visible_start) = self.collect_all_rows();
        let cursor_abs = visible_start + self.cursor.line();
        let cursor_col = self.cursor.col().0;

        // Real history ends where previous reflow overflow begins.
        // In `all_rows`: [0..history_end) = real history,
        // [history_end..visible_start) = reflow overflow from last resize,
        // [visible_start..) = visible rows.
        let history_boundary = visible_start.saturating_sub(self.resize_pushed);

        // Reflow cells into new-width rows.
        let outcome = reflow_cells(
            &all_rows,
            ReflowParams {
                old_cols,
                new_cols,
                cursor_abs,
                cursor_col,
                history_boundary,
            },
        );

        // Distribute into scrollback + visible, update cursor.
        let first_output_row = self.apply_reflow_result(outcome, new_cols);

        Some(ReflowMapping {
            first_output_row,
            old_total_evicted,
        })
    }

    /// Collect all rows (scrollback oldest-first + visible) for reflow.
    fn collect_all_rows(&mut self) -> (Vec<Row>, usize) {
        let mut all_rows = self.scrollback.drain_oldest_first();
        let visible_start = all_rows.len();
        all_rows.append(&mut self.rows);
        (all_rows, visible_start)
    }

    /// Apply reflow result: split into scrollback + visible, update cursor.
    ///
    /// `outcome.new_history_boundary` is the output row index where real
    /// scrollback history ends. Rows beyond that in the scrollback portion
    /// are reflow overflow (stale copies of visible content that wrapped).
    /// Returns `outcome.first_output_row` for `ReflowMapping` construction.
    fn apply_reflow_result(&mut self, outcome: ReflowOutcome, new_cols: usize) -> Vec<usize> {
        let ReflowOutcome {
            rows: mut result,
            new_cursor_abs,
            new_cursor_col,
            new_history_boundary,
            first_output_row,
        } = outcome;
        // All rows in `result` are already `new_cols` wide (created by
        // `Row::new(new_cols)` in `reflow_cells`), so no resize needed.
        if result.is_empty() {
            result.push(Row::new(new_cols));
        }

        // Trim trailing blank rows so they don't push real content into
        // scrollback. Keep at least `self.lines` rows (visible area) and
        // enough to include the cursor position.
        let min_rows = self.lines.max(new_cursor_abs + 1);
        while result.len() > min_rows && result.last().is_some_and(Row::is_blank) {
            result.pop();
        }

        let total = result.len();
        self.scrollback.clear();
        if total > self.lines {
            let sb_count = total - self.lines;
            for row in result.drain(..sb_count) {
                // Track evictions so StableRowIndex values produced after
                // reflow remain valid (mirrors `shrink_rows`). Without
                // this, a ring-buffer eviction silently drops a row but
                // leaves `total_evicted` unchanged, shifting all future
                // StableRowIndex computations by the dropped row count —
                // breaking image placements, selection anchors, and any
                // other eviction-stable state that survives reflow.
                if self.scrollback.push(row).is_some() {
                    self.total_evicted += 1;
                }
            }
            // Overflow = scrollback rows beyond the real history boundary.
            self.resize_pushed = sb_count.saturating_sub(new_history_boundary);
        } else {
            self.resize_pushed = 0;
            while result.len() < self.lines {
                result.push(Row::new(new_cols));
            }
        }
        self.rows = result;

        let sb_len = self.scrollback.len();
        self.cursor.set_line(if new_cursor_abs >= sb_len {
            (new_cursor_abs - sb_len).min(self.lines.saturating_sub(1))
        } else {
            0
        });
        self.cursor
            .set_col(Column(new_cursor_col.min(new_cols.saturating_sub(1))));
        first_output_row
    }
}

#[cfg(test)]
mod tests;
