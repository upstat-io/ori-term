//! Column insert / delete primitives (DEC §09A.7).
//!
//! Hosts the row-mutation logic invoked by the DEC column-ops handlers
//! (DECIC, DECDC) and the ESC-path back/forward index ops (DECBI,
//! DECFI). Matches xterm's per-row semantics: each column op walks
//! every row in the scroll region and applies a single-row shift at
//! `at_col`, preserving DECLRMM horizontal clamping (`lft_marg..=
//! rgt_marg`) and the vertical scroll region (`top_marg..=bot_marg`).
//! See `xterm util.c:819 xtermColScroll` for the canonical loop.
//!
//! `at_col` is 0-based. The caller is responsible for clamping it to
//! the grid width and the DECLRMM band before invoking these
//! primitives; the grid layer trusts its input and asserts bounds via
//! `debug_assert!`. `selection_dirty` is the handler's responsibility
//! — this module never touches Term state.

use crate::cell::{Cell, CellFlags};
use crate::index::Column;

use super::super::Grid;

/// Geometry of a single-row column shift within a DECLRMM band.
///
/// Bundles the per-row parameters the [`Grid::row_insert_columns`] /
/// [`Grid::row_delete_columns`] primitives need: the target `line`, the
/// `at_col` shift origin, the `count` of columns moved, and the
/// exclusive right band edge `col_end_excl`. All values are 0-based.
#[derive(Debug, Clone, Copy)]
struct ColumnShift {
    /// Row index to shift.
    line: usize,
    /// 0-based column where the shift originates.
    at_col: usize,
    /// Number of columns inserted / deleted.
    count: usize,
    /// Exclusive right edge of the DECLRMM band.
    col_end_excl: usize,
}

impl Grid {
    /// DECIC primitive: insert `count` blank columns at `at_col` on
    /// every row in the active scroll region.
    ///
    /// Content from `at_col` to the right band (right margin when
    /// DECLRMM is active, else the last column) shifts right by
    /// `count` on each row; cells that shift past the right band are
    /// lost. `at_col` must lie within the DECLRMM band when margins are
    /// active — handlers clamp this before invoking the primitive.
    /// BCE: the inserted blank cells inherit the cursor template's
    /// current background, matching the per-row `insert_blank` path.
    pub fn insert_columns(&mut self, count: usize, at_col: usize) {
        if count == 0 {
            return;
        }
        let Some((row_start, row_end_excl, col_start, col_end_excl)) =
            self.column_op_extent(at_col)
        else {
            return;
        };
        let _ = col_start; // caller already clamped `at_col` into the band
        let template = Cell::from(self.cursor.template.bg);
        let cols = self.cols;
        let count = count.min(col_end_excl - at_col);

        for line in row_start..row_end_excl {
            self.row_insert_columns(
                ColumnShift {
                    line,
                    at_col,
                    count,
                    col_end_excl,
                },
                &template,
            );
            // Cells shifted right; mark everything from at_col to the
            // right edge of the band dirty.
            self.dirty
                .mark_cols(line, at_col, col_end_excl.saturating_sub(1));
            let row = &mut self.rows[line];
            row.set_occ((row.occ() + count).min(cols));
        }
    }

    /// DECDC primitive: delete `count` columns at `at_col` on every row
    /// in the active scroll region.
    ///
    /// Content from `at_col + count` to the right band shifts left by
    /// `count`; blanks fill the right edge of the band. `at_col` must
    /// lie within the DECLRMM band when margins are active — handlers
    /// clamp this before invoking the primitive. BCE: the vacated
    /// cells inherit the cursor template's current background.
    pub fn delete_columns(&mut self, count: usize, at_col: usize) {
        if count == 0 {
            return;
        }
        let Some((row_start, row_end_excl, col_start, col_end_excl)) =
            self.column_op_extent(at_col)
        else {
            return;
        };
        let _ = col_start; // caller already clamped `at_col` into the band
        let template = Cell::from(self.cursor.template.bg);
        let count = count.min(col_end_excl - at_col);
        let cols = self.cols;

        for line in row_start..row_end_excl {
            self.row_delete_columns(
                ColumnShift {
                    line,
                    at_col,
                    count,
                    col_end_excl,
                },
                &template,
            );
            self.dirty
                .mark_cols(line, at_col, col_end_excl.saturating_sub(1));
            if !template.is_empty() {
                self.rows[line].set_occ(cols);
            }
        }
    }

    /// Compute the `(row_start, row_end_excl, col_start, col_end_excl)`
    /// extent for a DECIC / DECDC / DECBI / DECFI column op.
    ///
    /// Returns `None` when the operation is a no-op — cursor outside
    /// the vertical scroll region, `at_col` outside the DECLRMM band,
    /// or zero-area extent.
    fn column_op_extent(&self, at_col: usize) -> Option<(usize, usize, usize, usize)> {
        if self.lines == 0 || self.cols == 0 {
            return None;
        }

        // Vertical band: scroll region (DECSTBM).
        let row_start = self.scroll_region.start;
        let row_end_excl = self.scroll_region.end;
        if row_start >= row_end_excl {
            return None;
        }
        // Cursor must be inside the vertical scroll region (xterm util.c:835).
        let cur_row = self.cursor.line();
        if cur_row < row_start || cur_row >= row_end_excl {
            return None;
        }

        // Horizontal band: DECLRMM left/right margins (inclusive).
        let (col_start, col_end_incl) = if self.has_horizontal_margins() {
            (self.left_margin, self.right_margin)
        } else {
            (0, self.cols - 1)
        };
        let col_end_excl = col_end_incl + 1;
        // `at_col` must lie inside the horizontal band.
        if at_col < col_start || at_col >= col_end_excl {
            return None;
        }
        Some((row_start, row_end_excl, col_start, col_end_excl))
    }

    /// Shift a single row right by `count` starting at `at_col`, within
    /// the DECLRMM band `[col_start, col_end_excl)`. Mirrors the body
    /// of `insert_blank` but operates on an arbitrary line (not the
    /// cursor line) and skips the cursor-based no-op guards because
    /// the caller has already clamped.
    fn row_insert_columns(&mut self, shift: ColumnShift, template: &Cell) {
        let ColumnShift {
            line,
            at_col,
            count,
            col_end_excl,
        } = shift;
        let cols = self.cols;
        // Clean the partner of any wide-char pair at the insertion
        // point, then strip the cell's own wide flag so the shifted
        // copy doesn't carry a stale WIDE_CHAR or WIDE_CHAR_SPACER to
        // its new position.
        self.clear_wide_char_at(line, at_col);
        self.rows[line][Column(at_col)]
            .flags
            .remove(CellFlags::WIDE_CHAR | CellFlags::WIDE_CHAR_SPACER);
        // Clean wide char at the right boundary (cell being pushed off
        // the right edge of the band).
        if col_end_excl < cols {
            self.clear_wide_char_at(line, col_end_excl.saturating_sub(1));
        }

        let row = &mut self.rows[line];
        let cells = row.as_mut_slice();

        // Shift cells right by swapping within [at_col, col_end_excl).
        for i in (at_col + count..col_end_excl).rev() {
            cells.swap(i, i - count);
        }

        // Reset the gap cells to the BCE template.
        for cell in &mut cells[at_col..at_col + count] {
            cell.reset(template);
        }

        // Fix wide-char base pushed to the right boundary (its spacer
        // fell off).
        if col_end_excl > 0 && cells[col_end_excl - 1].flags.contains(CellFlags::WIDE_CHAR) {
            cells[col_end_excl - 1].ch = ' ';
            cells[col_end_excl - 1].flags.remove(CellFlags::WIDE_CHAR);
        }
    }

    /// Shift a single row left by `count` starting at `at_col`, within
    /// the DECLRMM band `[col_start, col_end_excl)`. Mirrors the body
    /// of `delete_chars`.
    fn row_delete_columns(&mut self, shift: ColumnShift, template: &Cell) {
        let ColumnShift {
            line,
            at_col,
            count,
            col_end_excl,
        } = shift;
        // Clean wide-char pair at `at_col` so stale flags don't
        // persist.
        self.clear_wide_char_at(line, at_col);
        // Spacer at the first shifted position: its base is in the
        // delete zone and would otherwise be left as a dangling spacer.
        if at_col + count < col_end_excl
            && self.rows[line][Column(at_col + count)]
                .flags
                .contains(CellFlags::WIDE_CHAR_SPACER)
        {
            self.rows[line][Column(at_col + count)].ch = ' ';
            self.rows[line][Column(at_col + count)]
                .flags
                .remove(CellFlags::WIDE_CHAR_SPACER);
        }

        let row = &mut self.rows[line];
        let cells = row.as_mut_slice();

        // Shift cells left within [at_col, col_end_excl).
        for i in at_col..col_end_excl - count {
            cells.swap(i, i + count);
        }

        // Reset the vacated cells at the right end of the band.
        for cell in &mut cells[col_end_excl - count..col_end_excl] {
            cell.reset(template);
        }
    }
}

#[cfg(test)]
mod tests;
