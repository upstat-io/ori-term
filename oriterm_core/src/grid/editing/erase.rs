//! Grid erase operations.
//!
//! Display erase (ED), line erase (EL), and character erase (ECH).
//! Extracted from `editing/mod.rs` to keep that file under the
//! 500-line limit.

use crate::cell::Cell;

use super::super::Grid;
use super::{DisplayEraseMode, LineEraseMode};

impl Grid {
    /// Erase part or all of the display.
    pub fn erase_display(&mut self, mode: DisplayEraseMode) {
        debug_assert!(
            self.cursor.line() < self.lines,
            "cursor line {} out of bounds (lines={})",
            self.cursor.line(),
            self.lines,
        );
        // BCE: erased cells get only the current background color.
        let template = Cell::from(self.cursor.template.bg);
        match mode {
            DisplayEraseMode::Below => {
                self.erase_line_with_template(LineEraseMode::Right, &template);
                let cursor_line = self.cursor.line();
                for line in cursor_line + 1..self.lines {
                    self.rows[line].reset(self.cols, &template);
                }
                if cursor_line + 1 < self.lines {
                    self.dirty.mark_range(cursor_line + 1..self.lines);
                }
            }
            DisplayEraseMode::Above => {
                self.erase_line_with_template(LineEraseMode::Left, &template);
                let cursor_line = self.cursor.line();
                for line in 0..cursor_line {
                    self.rows[line].reset(self.cols, &template);
                }
                if cursor_line > 0 {
                    self.dirty.mark_range(0..cursor_line);
                }
            }
            DisplayEraseMode::All => {
                for line in 0..self.lines {
                    self.rows[line].reset(self.cols, &template);
                }
                // Remove reflow overflow from the most recent column resize.
                // These are stale copies of visible content that wrapped
                // into scrollback during reflow.
                for _ in 0..self.resize_pushed {
                    self.scrollback.pop_newest();
                }
                self.resize_pushed = 0;
                self.dirty.mark_all();
            }
            DisplayEraseMode::Scrollback => {
                // ED 3 — clear scrollback buffer only (visible grid untouched).
                // Adjust total_evicted so StableRowIndex values remain valid.
                self.total_evicted += self.scrollback.len();
                self.scrollback.clear();
                self.display_offset = 0;
                self.dirty.mark_all();
            }
        }
    }

    /// Erase part or all of the current line.
    pub fn erase_line(&mut self, mode: LineEraseMode) {
        debug_assert!(
            self.cursor.line() < self.lines,
            "cursor line {} out of bounds (lines={})",
            self.cursor.line(),
            self.lines,
        );
        let template = Cell::from(self.cursor.template.bg);
        self.erase_line_with_template(mode, &template);
    }

    /// Erase part or all of the current line using a pre-built BCE template.
    pub(super) fn erase_line_with_template(&mut self, mode: LineEraseMode, template: &Cell) {
        let line = self.cursor.line();
        let col = self.cursor.col().0;
        let cols = self.cols;

        match mode {
            LineEraseMode::Right => {
                // Fix spacer at cursor whose base is before the erase range.
                self.fix_wide_boundaries(line, col, cols);
                let row = &mut self.rows[line];
                let cells = row.as_mut_slice();
                for cell in &mut cells[col..cols] {
                    cell.reset(template);
                }
                if template.is_empty() {
                    row.clamp_occ(col);
                } else {
                    row.set_occ(cols);
                }
            }
            LineEraseMode::Left => {
                let end = col.min(cols - 1) + 1;
                // Fix base at end-1 whose spacer is after the erase range.
                self.fix_wide_boundaries(line, 0, end);
                let row = &mut self.rows[line];
                let cells = row.as_mut_slice();
                for cell in &mut cells[..end] {
                    cell.reset(template);
                }
                if template.is_empty() {
                    // Cells [0..end] are now empty. Only cells beyond end
                    // may be dirty, so if occ was within the erased range
                    // all dirty cells are gone.
                    if row.occ() <= end {
                        row.set_occ(0);
                    }
                } else {
                    row.set_occ(row.occ().max(end));
                }
            }
            LineEraseMode::All => {
                self.rows[line].reset(cols, template);
            }
        }

        // Mark the affected column range dirty.
        match mode {
            LineEraseMode::Right => {
                self.dirty.mark_cols(line, col, cols.saturating_sub(1));
            }
            LineEraseMode::Left => {
                self.dirty
                    .mark_cols(line, 0, col.min(cols.saturating_sub(1)));
            }
            LineEraseMode::All => {
                self.dirty.mark(line);
            }
        }
    }

    /// Erase `count` cells starting at cursor (replace with template, don't shift).
    pub fn erase_chars(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        debug_assert!(
            self.cursor.line() < self.lines,
            "cursor line {} out of bounds (lines={})",
            self.cursor.line(),
            self.lines,
        );
        let line = self.cursor.line();
        let col = self.cursor.col().0;
        let cols = self.cols;
        // BCE: erased cells get only the current background color.
        let template = Cell::from(self.cursor.template.bg);

        let end = (col + count).min(cols);

        // Fix wide char pairs split by the erase boundary.
        self.fix_wide_boundaries(line, col, end);

        let row = &mut self.rows[line];
        let cells = row.as_mut_slice();
        for cell in &mut cells[col..end] {
            cell.reset(&template);
        }
        // BCE template has a colored bg — the erased cells are dirty.
        // Default template produces truly empty cells, so existing occ
        // remains a valid upper bound (we only cleared, didn't extend).
        if !template.is_empty() {
            row.set_occ(row.occ().max(end));
        }

        self.dirty.mark_cols(line, col, end.saturating_sub(1));
    }
}
