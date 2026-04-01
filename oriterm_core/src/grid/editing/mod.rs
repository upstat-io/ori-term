//! Grid editing operations.
//!
//! Character insertion, deletion, and erase operations. These are the
//! primitives the VTE handler calls for writing text and manipulating
//! grid content.

mod erase;
mod wide_char;

use unicode_width::UnicodeWidthChar;

use crate::cell::{Cell, CellFlags};
use crate::index::Column;

use super::Grid;

/// Erase mode for display erase operations (ED / CSI Ps J).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayEraseMode {
    /// Erase from cursor to end of display.
    Below,
    /// Erase from start of display to cursor.
    Above,
    /// Erase entire display.
    All,
    /// Erase scrollback buffer only (CSI 3 J).
    Scrollback,
}

/// Erase mode for line erase operations (EL / CSI Ps K).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEraseMode {
    /// Erase from cursor to end of line.
    Right,
    /// Erase from start of line to cursor.
    Left,
    /// Erase entire line.
    All,
}

impl Grid {
    /// Write a character at the cursor position.
    ///
    /// Handles wide characters (writes cell + spacer), wrap at end of line,
    /// and clearing overwritten wide char pairs.
    #[inline]
    pub fn put_char(&mut self, ch: char) {
        debug_assert!(
            self.cursor.line() < self.lines,
            "cursor line {} out of bounds (lines={})",
            self.cursor.line(),
            self.lines,
        );

        self.put_char_slow(ch);
    }

    /// Fast path for ASCII printable characters (0x20–0x7E, width 1).
    ///
    /// Caller guarantees `ch` is in the ASCII printable range. Skips
    /// `UnicodeWidthChar::width()`, the wrap loop, and wide char cleanup.
    /// Returns `true` if the write succeeded; `false` if the caller must
    /// fall through to the full `put_char` path (wrap pending, or target
    /// cell has wide char flags requiring cleanup).
    ///
    /// Called from `Term::input()`'s fast path which already verified the
    /// ASCII range, so this avoids double-checking.
    #[inline]
    pub fn put_char_ascii(&mut self, ch: char) -> bool {
        let line = self.cursor.line();
        let col = self.cursor.col().0;

        // Wrap pending or at end of line — fall through to slow path which
        // handles WRAP flag + linefeed.
        if col >= self.cols {
            return false;
        }

        // Target cell is part of a wide char pair — needs cleanup via
        // clear_wide_char_at() in the slow path.
        let flags = self.rows[line][Column(col)].flags;
        if flags.intersects(CellFlags::WIDE_CHAR | CellFlags::WIDE_CHAR_SPACER) {
            return false;
        }

        // Direct cell write — no width lookup, no wide char handling.
        let cell = &mut self.rows[line][Column(col)];
        cell.ch = ch;
        cell.fg = self.cursor.template.fg;
        cell.bg = self.cursor.template.bg;
        cell.flags = self.cursor.template.flags;
        cell.extra.clone_from(&self.cursor.template.extra);

        self.cursor.set_col(Column(col + 1));
        self.dirty.mark_cols(line, col, col);
        true
    }

    /// Slow path for `put_char`: full width lookup, wide char handling,
    /// and wrap logic.
    fn put_char_slow(&mut self, ch: char) {
        let width = UnicodeWidthChar::width(ch).unwrap_or(1);
        let cols = self.cols;

        // Wide char can never fit in this terminal width — skip it.
        // Without this guard, a width-2 char on a 1-column grid would
        // loop forever: wrap → col 0 → can't fit → wrap → col 0 → …
        if width > cols {
            return;
        }

        loop {
            let line = self.cursor.line();
            let col = self.cursor.col().0;

            // If a pending wrap is active and we're at the last column, wrap now.
            if col >= cols {
                self.rows[line][Column(cols - 1)].flags |= CellFlags::WRAP;
                self.linefeed();
                self.cursor.set_col(Column(0));
                continue;
            }

            // For wide chars at the last column, wrap instead of splitting.
            // Mark the boundary cell as LEADING_WIDE_CHAR_SPACER so reflow,
            // selection, and search skip it (avoids spurious spaces).
            if width == 2 && col + 1 >= cols {
                let boundary = &mut self.rows[line][Column(col)];
                boundary.ch = ' ';
                boundary.flags = CellFlags::LEADING_WIDE_CHAR_SPACER | CellFlags::WRAP;
                self.linefeed();
                self.cursor.set_col(Column(0));
                continue;
            }

            // Clear any wide char pair that we're overwriting.
            self.clear_wide_char_at(line, col);

            // Extract template fields before mutable row borrow. `rows` and
            // `cursor` are disjoint Grid fields, so this avoids a full Cell clone.
            let tmpl_fg = self.cursor.template.fg;
            let tmpl_bg = self.cursor.template.bg;
            let tmpl_flags = self.cursor.template.flags;
            let tmpl_extra = self.cursor.template.extra.clone();
            let cell = &mut self.rows[line][Column(col)];
            cell.ch = ch;
            cell.fg = tmpl_fg;
            cell.bg = tmpl_bg;
            cell.flags = tmpl_flags;
            cell.extra = tmpl_extra;

            if width == 2 {
                cell.flags |= CellFlags::WIDE_CHAR;

                // Write the spacer in the next column.
                if col + 1 < cols {
                    self.clear_wide_char_at(line, col + 1);
                    let spacer = &mut self.rows[line][Column(col + 1)];
                    spacer.ch = ' ';
                    spacer.fg = tmpl_fg;
                    spacer.bg = tmpl_bg;
                    spacer.flags = CellFlags::WIDE_CHAR_SPACER;
                    spacer.extra = None;
                }
            }

            // Advance cursor by character width.
            self.cursor.set_col(Column(col + width));

            // Mark only the affected columns dirty.
            let right = if width == 2 { col + 1 } else { col };
            self.dirty.mark_cols(line, col, right);
            break;
        }
    }

    /// Append a zero-width character (combining mark) to the previous cell.
    ///
    /// Backtracks from the cursor to find the cell that was just written.
    /// If the cursor is at column 0 with no previous cell, the character
    /// is silently discarded. Handles wrap-pending state and wide-char
    /// spacers.
    pub fn push_zerowidth(&mut self, ch: char) {
        debug_assert!(
            self.cursor.line() < self.lines,
            "cursor line {} out of bounds (lines={})",
            self.cursor.line(),
            self.lines,
        );
        let col = self.cursor.col().0;
        let cols = self.cols;

        // Determine the column of the previous cell.
        let prev_col = if col < cols {
            // Normal: cursor hasn't wrapped yet.
            col.checked_sub(1)
        } else {
            // Wrap pending: cursor is past last column; previous cell is
            // the last column.
            Some(cols.saturating_sub(1))
        };

        let Some(mut prev_col) = prev_col else {
            // Column 0 with no previous cell — discard.
            return;
        };

        let line = self.cursor.line();

        // If on a wide-char spacer, step back to the base cell.
        if self.rows[line][Column(prev_col)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
        {
            prev_col = prev_col.saturating_sub(1);
        }

        self.rows[line][Column(prev_col)].push_zerowidth(ch);
        self.dirty.mark_cols(line, prev_col, prev_col);
    }

    /// Insert `count` blank cells at the cursor, shifting existing cells right.
    ///
    /// Cells that shift past the right edge are lost.
    pub fn insert_blank(&mut self, count: usize) {
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

        if col >= cols {
            return;
        }

        let count = count.min(cols - col);

        // Clean the partner of any wide char pair at the insertion point,
        // then strip the cell's own wide flag so the shifted copy doesn't
        // carry a stale WIDE_CHAR or WIDE_CHAR_SPACER to its new position.
        self.clear_wide_char_at(line, col);
        self.rows[line][Column(col)]
            .flags
            .remove(CellFlags::WIDE_CHAR | CellFlags::WIDE_CHAR_SPACER);

        let row = &mut self.rows[line];
        let cells = row.as_mut_slice();

        // Shift cells right by swapping (no allocation).
        for i in (col + count..cols).rev() {
            cells.swap(i, i - count);
        }

        // Reset the gap cells in-place.
        for cell in &mut cells[col..col + count] {
            cell.reset(&template);
        }

        // Fix wide char base pushed to the right edge (spacer fell off-screen).
        if cells[cols - 1].flags.contains(CellFlags::WIDE_CHAR) {
            cells[cols - 1].ch = ' ';
            cells[cols - 1].flags.remove(CellFlags::WIDE_CHAR);
        }

        // Cells shifted right: occ grows by at most `count`, capped at cols.
        row.set_occ((row.occ() + count).min(cols));

        // Content shifts from cursor to right edge.
        self.dirty.mark_cols(line, col, cols.saturating_sub(1));
    }

    /// Delete `count` cells at the cursor, shifting remaining cells left.
    ///
    /// New cells at the right edge are blank.
    pub fn delete_chars(&mut self, count: usize) {
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

        if col >= cols {
            return;
        }

        let count = count.min(cols - col);

        // Clean wide char pair at the cursor so stale flags don't persist.
        self.clear_wide_char_at(line, col);
        // Spacer at first shifted position: its base is in the delete zone.
        if col + count < cols
            && self.rows[line][Column(col + count)]
                .flags
                .contains(CellFlags::WIDE_CHAR_SPACER)
        {
            self.rows[line][Column(col + count)].ch = ' ';
            self.rows[line][Column(col + count)]
                .flags
                .remove(CellFlags::WIDE_CHAR_SPACER);
        }

        let row = &mut self.rows[line];
        let cells = row.as_mut_slice();

        // Shift cells left by swapping (no allocation).
        for i in col..cols - count {
            cells.swap(i, i + count);
        }

        // Reset the vacated right cells in-place.
        for cell in &mut cells[cols - count..cols] {
            cell.reset(&template);
        }

        if !template.is_empty() {
            // BCE: fill cells at [cols-count..cols] are dirty.
            row.set_occ(cols);
        }
        // else: Content shifted left; existing occ remains a valid upper
        // bound. Fill cells are empty and don't extend the dirty range.

        // Content shifts from cursor to right edge.
        self.dirty.mark_cols(line, col, cols.saturating_sub(1));
    }

    // Erase operations (`erase_display`, `erase_line`, `erase_chars`) are
    // in `erase.rs`. Wide character boundary fixup helpers
    // (`fix_wide_boundaries`, `clear_wide_char_at`) are in `wide_char.rs`.
}

#[cfg(test)]
mod tests;
