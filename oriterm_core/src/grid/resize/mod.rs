//! Grid resize and text reflow.
//!
//! `Grid::resize` is the main entry point: it handles dimension changes,
//! scroll region reset, cursor clamping, and optional text reflow on
//! column changes. Row resize manages scrollback interaction (shrink
//! pushes rows to scrollback, grow pulls them back). Column reflow uses
//! Ghostty-style cell-by-cell rewriting to wrap/unwrap soft-wrapped lines.

use crate::cell::{Cell, CellFlags};
use crate::index::Column;

use super::Grid;
use super::row::Row;

impl Grid {
    /// Resize the grid to new dimensions.
    ///
    /// When `reflow` is true, soft-wrapped lines are re-wrapped to fit the
    /// new column width (cell-by-cell rewriting). When false, rows are simply
    /// truncated or extended (for alternate screen).
    ///
    /// Resets scroll region, clamps cursor, and marks everything dirty.
    pub fn resize(&mut self, new_cols: usize, new_lines: usize, reflow: bool) {
        if new_cols == 0 || new_lines == 0 {
            return;
        }
        if new_cols == self.cols && new_lines == self.lines {
            return;
        }

        if reflow && new_cols != self.cols {
            if new_cols > self.cols {
                // Growing cols: reflow first (unwrap), then adjust rows.
                self.reflow_cols(new_cols);
                self.cols = new_cols;
                Self::reset_tab_stops(&mut self.tab_stops, new_cols);
                self.resize_rows(new_lines);
            } else {
                // Shrinking cols: adjust rows first, then reflow (wrap).
                self.resize_rows(new_lines);
                self.reflow_cols(new_cols);
                self.cols = new_cols;
                Self::reset_tab_stops(&mut self.tab_stops, new_cols);
            }
        } else {
            self.resize_no_reflow(new_cols, new_lines);
        }

        // Reset scroll region, clamp cursor, mark dirty.
        self.finalize_resize();
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
    fn finalize_resize(&mut self) {
        self.scroll_region = 0..self.lines;

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

        self.display_offset = self.display_offset.min(self.scrollback.len());
        self.dirty.resize(self.lines);
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
        self.dirty.resize(new_lines);
    }

    /// Shrink visible rows: trim trailing blanks, push excess to scrollback.
    fn shrink_rows(&mut self, new_lines: usize) {
        let to_remove = self.lines - new_lines;
        let trimmed = self.count_trailing_blank_rows(to_remove);
        for _ in 0..trimmed {
            self.rows.pop();
        }
        let push_count = to_remove - trimmed;
        for _ in 0..push_count {
            if self.rows.is_empty() {
                break;
            }
            let row = self.rows.remove(0);
            if self.scrollback.push(row).is_some() {
                self.total_evicted += 1;
            }
            let line = self.cursor.line();
            self.cursor.set_line(line.saturating_sub(1));
        }
        self.rows.truncate(new_lines);
        while self.rows.len() < new_lines {
            self.rows.push(Row::new(self.cols));
        }
    }

    /// Grow visible rows: pull from scrollback or append blanks.
    fn grow_rows(&mut self, new_lines: usize) {
        let delta = new_lines - self.lines;
        if self.cursor.line() >= self.lines.saturating_sub(1) {
            let from_sb = delta.min(self.scrollback.len());
            for _ in 0..from_sb {
                if let Some(row) = self.scrollback.pop_newest() {
                    self.rows.insert(0, row);
                    let line = self.cursor.line();
                    self.cursor.set_line(line + 1);
                }
            }
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
    fn reflow_cols(&mut self, new_cols: usize) {
        let old_cols = self.cols;
        if old_cols == new_cols || new_cols == 0 {
            return;
        }

        // Collect all rows: scrollback (oldest first) then visible.
        let (all_rows, visible_start) = self.collect_all_rows();
        let cursor_abs = visible_start + self.cursor.line();
        let cursor_col = self.cursor.col().0;

        // Reflow cells into new-width rows.
        let (result, new_cursor_abs, new_cursor_col) =
            reflow_cells(&all_rows, old_cols, new_cols, cursor_abs, cursor_col);

        // Distribute into scrollback + visible, update cursor.
        self.apply_reflow_result(result, new_cols, new_cursor_abs, new_cursor_col);
    }

    /// Collect all rows (scrollback oldest-first + visible) for reflow.
    fn collect_all_rows(&mut self) -> (Vec<Row>, usize) {
        let mut all_rows: Vec<Row> = Vec::with_capacity(self.scrollback.len() + self.rows.len());
        let sb_rows: Vec<Row> = self.scrollback.iter().cloned().collect();
        for row in sb_rows.into_iter().rev() {
            all_rows.push(row);
        }
        let visible_start = all_rows.len();
        all_rows.append(&mut self.rows);
        (all_rows, visible_start)
    }

    /// Apply reflow result: split into scrollback + visible, update cursor.
    fn apply_reflow_result(
        &mut self,
        mut result: Vec<Row>,
        new_cols: usize,
        new_cursor_abs: usize,
        new_cursor_col: usize,
    ) {
        for row in &mut result {
            row.resize(new_cols);
        }
        if result.is_empty() {
            result.push(Row::new(new_cols));
        }

        let total = result.len();
        self.scrollback.clear();
        if total > self.lines {
            let sb_count = total - self.lines;
            for row in result.drain(..sb_count) {
                self.scrollback.push(row);
            }
        } else {
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
    }
}

/// Reflow all rows from old column width to new column width.
///
/// Returns the reflowed rows, new cursor absolute position, and new cursor column.
fn reflow_cells(
    all_rows: &[Row],
    old_cols: usize,
    new_cols: usize,
    cursor_abs: usize,
    cursor_col: usize,
) -> (Vec<Row>, usize, usize) {
    let mut new_cursor_abs = 0usize;
    let mut new_cursor_col = 0usize;
    let mut result: Vec<Row> = Vec::with_capacity(all_rows.len());
    let mut out_row = Row::new(new_cols);
    let mut out_col = 0usize;

    for (src_idx, src_row) in all_rows.iter().enumerate() {
        let wrapped = old_cols > 0
            && src_row.cols() >= old_cols
            && src_row[Column(old_cols - 1)]
                .flags
                .contains(CellFlags::WRAP);

        let content_len = if wrapped {
            old_cols
        } else {
            src_row.content_len()
        };

        reflow_row_cells(
            src_row,
            src_idx,
            content_len,
            new_cols,
            cursor_abs,
            cursor_col,
            &mut result,
            &mut out_row,
            &mut out_col,
            &mut new_cursor_abs,
            &mut new_cursor_col,
        );

        // Track cursor when it's past content on this source row.
        if src_idx == cursor_abs && cursor_col >= content_len {
            new_cursor_abs = result.len();
            new_cursor_col = if wrapped {
                out_col.min(new_cols.saturating_sub(1))
            } else {
                cursor_col.min(new_cols.saturating_sub(1))
            };
        }

        // End of source row: finalize if not wrapped.
        if !wrapped {
            result.push(out_row);
            out_row = Row::new(new_cols);
            out_col = 0;
        }
    }

    if out_col > 0 {
        result.push(out_row);
    }

    (result, new_cursor_abs, new_cursor_col)
}

/// Reflow cells from a single source row into the output.
#[expect(
    clippy::too_many_arguments,
    reason = "cell-by-cell reflow: source context, output state, cursor tracking"
)]
fn reflow_row_cells(
    src_row: &Row,
    src_idx: usize,
    content_len: usize,
    new_cols: usize,
    cursor_abs: usize,
    cursor_col: usize,
    result: &mut Vec<Row>,
    out_row: &mut Row,
    out_col: &mut usize,
    new_cursor_abs: &mut usize,
    new_cursor_col: &mut usize,
) {
    for src_col in 0..content_len {
        let cell = &src_row[Column(src_col)];

        // Skip spacer cells (regenerated at new positions).
        if cell.flags.contains(CellFlags::WIDE_CHAR_SPACER) {
            if src_idx == cursor_abs && src_col == cursor_col {
                *new_cursor_abs = result.len();
                *new_cursor_col = out_col.saturating_sub(1);
            }
            continue;
        }

        let is_wide = cell.flags.contains(CellFlags::WIDE_CHAR) && new_cols >= 2;
        let cell_width = if is_wide { 2 } else { 1 };

        // Wrap to next output row if cell doesn't fit.
        if *out_col + cell_width > new_cols {
            if *out_col > 0 {
                out_row[Column(new_cols - 1)].flags.insert(CellFlags::WRAP);
            }
            out_row.set_occ(new_cols);
            result.push(std::mem::replace(out_row, Row::new(new_cols)));
            *out_col = 0;
        }

        // Track cursor position.
        if src_idx == cursor_abs && src_col == cursor_col {
            *new_cursor_abs = result.len();
            *new_cursor_col = *out_col;
        }

        // Write cell (strip old WRAP flag).
        let mut new_cell = cell.clone();
        new_cell.flags.remove(CellFlags::WRAP);
        if !is_wide && cell.flags.contains(CellFlags::WIDE_CHAR) {
            new_cell.flags.remove(CellFlags::WIDE_CHAR);
        }
        out_row[Column(*out_col)] = new_cell;
        *out_col += 1;

        // Write wide char spacer in next column.
        if is_wide {
            let mut spacer = Cell::default();
            spacer.flags.insert(CellFlags::WIDE_CHAR_SPACER);
            spacer.fg = cell.fg;
            spacer.bg = cell.bg;
            out_row[Column(*out_col)] = spacer;
            *out_col += 1;
        }
        out_row.set_occ(*out_col);
    }
}

#[cfg(test)]
mod tests;
