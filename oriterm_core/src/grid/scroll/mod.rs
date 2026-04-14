//! Scroll region management and scroll operations.
//!
//! Provides `set_scroll_region` (DECSTBM), `scroll_up`, `scroll_down`,
//! `insert_lines`, `delete_lines`, `scroll_left` (SL), and `scroll_right`
//! (SR). Full-row operations use O(1) rotation; partial-width operations
//! (when DECLRMM margins are active) use cell-by-cell copy within the
//! margin band.

use std::mem;
use std::ops::Range;

use crate::cell::Cell;
use crate::index::Column;

use super::Grid;
use super::row::Row;

impl Grid {
    /// DECSTBM: set the scroll region.
    ///
    /// Parameters are 1-based (matching VTE/ECMA-48). `top` is the first
    /// line of the region, `bottom` is the last line (or `None` for the
    /// screen height). Stored internally as a 0-based half-open range.
    ///
    /// Does **not** move the cursor — that's the handler's job (via
    /// `goto(0, 0)` which respects ORIGIN mode).
    pub fn set_scroll_region(&mut self, top: usize, bottom: Option<usize>) {
        // 1-based params: top=0 is invalid, treat as 1.
        let top = top.max(1) - 1;
        let bottom = bottom.map_or(self.lines, |b| b.min(self.lines));

        // Region must span at least 2 lines.
        if top + 1 >= bottom {
            return;
        }

        self.scroll_region = top..bottom;
    }

    /// Scroll the scroll region up by `count` lines.
    ///
    /// When the scroll region covers the full screen, evicted top rows
    /// are pushed to scrollback history. With a sub-region, top rows
    /// are lost. Blank rows appear at the bottom of the region.
    pub fn scroll_up(&mut self, count: usize) {
        let start = self.scroll_region.start;
        let end = self.scroll_region.end;
        let len = end - start;
        if len == 0 {
            return;
        }
        let count = count.min(len);

        // Push evicted rows to scrollback when scrolling the full screen.
        let is_full_screen = start == 0 && end == self.lines;
        if is_full_screen {
            // Remove stale reflow overflow before pushing real content.
            // After a column resize, overflow rows (from wrapping) sit
            // at the newest end of scrollback. Pop them so they don't
            // compound with the real content we're about to push.
            for _ in 0..self.resize_pushed {
                self.scrollback.pop_newest();
            }
            self.resize_pushed = 0;
            // Keep user's scrollback view stable when new content arrives.
            if self.display_offset > 0 {
                let max_after_push =
                    (self.scrollback.len() + count).min(self.scrollback.max_scrollback());
                self.display_offset = (self.display_offset + count).min(max_after_push);
            }

            for i in 0..count {
                // Move the row out, leave a zero-alloc placeholder.
                // The placeholder rotates to the bottom via
                // scroll_range_up, where reset() will resize it to
                // the correct column count.
                let evicted = mem::replace(&mut self.rows[i], Row::new(0));
                if let Some(mut recycled) = self.scrollback.push(evicted) {
                    // Scrollback was full: oldest row evicted. Track
                    // for StableRowIndex stability.
                    self.total_evicted += 1;
                    recycled.reset(self.cols, &Cell::default());
                    self.rows[i] = recycled;
                }
            }
        }

        self.scroll_range_up(start..end, count);
    }

    /// Scroll the scroll region down by `count` lines.
    ///
    /// Bottom rows are lost. Blank rows appear at the top of the region.
    pub fn scroll_down(&mut self, count: usize) {
        let start = self.scroll_region.start;
        let end = self.scroll_region.end;
        self.scroll_range_down(start..end, count);
    }

    /// IL: insert `count` blank lines at the cursor, pushing existing
    /// lines down within the scroll region.
    ///
    /// Only operates if the cursor is inside the scroll region. Lines
    /// pushed past the bottom of the region are lost. When DECLRMM
    /// horizontal margins are active, only cells within the margin band
    /// are scrolled — content outside margins is untouched.
    pub fn insert_lines(&mut self, count: usize) {
        let line = self.cursor.line();
        if line < self.scroll_region.start || line >= self.scroll_region.end {
            return;
        }
        let range = line..self.scroll_region.end;
        if self.has_horizontal_margins() {
            let col_range = self.left_margin..self.right_margin + 1;
            self.scroll_partial_down(range, col_range, count);
        } else {
            self.scroll_range_down(range, count);
        }
    }

    /// DL: delete `count` lines at the cursor, pulling remaining lines
    /// up within the scroll region.
    ///
    /// Only operates if the cursor is inside the scroll region. Blank
    /// lines appear at the bottom of the region. When DECLRMM
    /// horizontal margins are active, only cells within the margin band
    /// are scrolled — content outside margins is untouched.
    pub fn delete_lines(&mut self, count: usize) {
        let line = self.cursor.line();
        if line < self.scroll_region.start || line >= self.scroll_region.end {
            return;
        }
        let range = line..self.scroll_region.end;
        if self.has_horizontal_margins() {
            let col_range = self.left_margin..self.right_margin + 1;
            self.scroll_partial_up(range, col_range, count);
        } else {
            self.scroll_range_up(range, count);
        }
    }

    /// SL: scroll the content within the scroll region left by `count`
    /// columns.
    ///
    /// When DECLRMM margins are active, only cells within the margin
    /// band are shifted. Blank cells fill from the right margin.
    /// Cells outside the margin band are untouched.
    pub fn scroll_left(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let (left, right) = if self.has_horizontal_margins() {
            (self.left_margin, self.right_margin)
        } else {
            (0, self.cols.saturating_sub(1))
        };
        let band_width = right + 1 - left;
        let count = count.min(band_width);
        let template = Cell::from(self.cursor.template.bg);

        for row_idx in self.scroll_region.clone() {
            let row = &mut self.rows[row_idx];
            let cells = row.as_mut_slice();
            // Shift cells left within the band.
            for col in left..=right - count {
                cells.swap(col, col + count);
            }
            // Clear the rightmost `count` cells in the band.
            for cell in &mut cells[right + 1 - count..=right] {
                cell.reset(&template);
            }
            row.set_occ(row.cols());
        }
        self.dirty.mark_range(self.scroll_region.clone());
    }

    /// SR: scroll the content within the scroll region right by `count`
    /// columns.
    ///
    /// When DECLRMM margins are active, only cells within the margin
    /// band are shifted. Blank cells fill from the left margin.
    /// Cells outside the margin band are untouched.
    pub fn scroll_right(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let (left, right) = if self.has_horizontal_margins() {
            (self.left_margin, self.right_margin)
        } else {
            (0, self.cols.saturating_sub(1))
        };
        let band_width = right + 1 - left;
        let count = count.min(band_width);
        let template = Cell::from(self.cursor.template.bg);

        for row_idx in self.scroll_region.clone() {
            let row = &mut self.rows[row_idx];
            let cells = row.as_mut_slice();
            // Shift cells right within the band.
            for col in ((left + count)..=right).rev() {
                cells.swap(col, col - count);
            }
            // Clear the leftmost `count` cells in the band.
            for cell in &mut cells[left..left + count] {
                cell.reset(&template);
            }
            row.set_occ(row.cols());
        }
        self.dirty.mark_range(self.scroll_region.clone());
    }

    /// Scroll cells within a column range UP across a row range.
    ///
    /// For each affected row, cells in `col_range` move up by `count`
    /// rows. The bottom `count` rows' column ranges are cleared with
    /// BCE background. Content outside `col_range` is untouched.
    ///
    /// Used by DL when DECLRMM horizontal margins are active.
    fn scroll_partial_up(
        &mut self,
        row_range: Range<usize>,
        col_range: Range<usize>,
        count: usize,
    ) {
        let len = row_range.end - row_range.start;
        if len == 0 || col_range.is_empty() {
            return;
        }
        let count = count.min(len);
        if count == 0 {
            return;
        }
        let template = Cell::from(self.cursor.template.bg);

        // Copy cells upward within the column range.
        for offset in 0..len - count {
            let dst = row_range.start + offset;
            let src = dst + count;
            let (lo, hi) = self.rows.split_at_mut(src);
            let dst_row = &mut lo[dst];
            let src_row = &hi[0];
            for col in col_range.clone() {
                dst_row[Column(col)] = src_row[Column(col)].clone();
            }
            dst_row.set_occ(dst_row.cols());
        }

        // Clear the bottom `count` rows' column range.
        for row_idx in (row_range.end - count)..row_range.end {
            let row = &mut self.rows[row_idx];
            for col in col_range.clone() {
                row[Column(col)].reset(&template);
            }
            row.set_occ(row.cols());
        }

        self.dirty.mark_range(row_range);
    }

    /// Scroll cells within a column range DOWN across a row range.
    ///
    /// For each affected row, cells in `col_range` move down by `count`
    /// rows. The top `count` rows' column ranges are cleared with BCE
    /// background. Content outside `col_range` is untouched.
    ///
    /// Used by IL when DECLRMM horizontal margins are active.
    fn scroll_partial_down(
        &mut self,
        row_range: Range<usize>,
        col_range: Range<usize>,
        count: usize,
    ) {
        let len = row_range.end - row_range.start;
        if len == 0 || col_range.is_empty() {
            return;
        }
        let count = count.min(len);
        if count == 0 {
            return;
        }
        let template = Cell::from(self.cursor.template.bg);

        // Copy cells downward within the column range (iterate in reverse
        // to avoid overwriting source data).
        for offset in (0..len - count).rev() {
            let src = row_range.start + offset;
            let dst = src + count;
            let (lo, hi) = self.rows.split_at_mut(dst);
            let src_row = &lo[src];
            let dst_row = &mut hi[0];
            for col in col_range.clone() {
                dst_row[Column(col)] = src_row[Column(col)].clone();
            }
            dst_row.set_occ(dst_row.cols());
        }

        // Clear the top `count` rows' column range.
        for row_idx in row_range.start..row_range.start + count {
            let row = &mut self.rows[row_idx];
            for col in col_range.clone() {
                row[Column(col)].reset(&template);
            }
            row.set_occ(row.cols());
        }

        self.dirty.mark_range(row_range);
    }

    /// Scroll a range of rows up by `count` using O(1) rotation.
    ///
    /// Top rows rotate to the bottom and are reset with BCE background.
    fn scroll_range_up(&mut self, range: Range<usize>, count: usize) {
        let len = range.end - range.start;
        if len == 0 {
            return;
        }
        let count = count.min(len);
        if count == 0 {
            return;
        }
        let template = Cell::from(self.cursor.template.bg);

        self.rows[range.start..range.end].rotate_left(count);

        for i in (range.end - count)..range.end {
            self.rows[i].reset(self.cols, &template);
        }

        self.dirty.mark_range(range);
    }

    /// Scroll a range of rows down by `count` using O(1) rotation.
    ///
    /// Bottom rows rotate to the top and are reset with BCE background.
    fn scroll_range_down(&mut self, range: Range<usize>, count: usize) {
        let len = range.end - range.start;
        if len == 0 {
            return;
        }
        let count = count.min(len);
        if count == 0 {
            return;
        }
        let template = Cell::from(self.cursor.template.bg);

        self.rows[range.start..range.end].rotate_right(count);

        for i in range.start..range.start + count {
            self.rows[i].reset(self.cols, &template);
        }

        self.dirty.mark_range(range);
    }
}

#[cfg(test)]
mod tests;
