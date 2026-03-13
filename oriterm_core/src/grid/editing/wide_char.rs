//! Wide character boundary fixup routines.
//!
//! These handle the split-pair cleanup when wide characters (CJK, emoji)
//! are partially overwritten or erased. Extracted from `editing/mod.rs`
//! to keep that file under the 500-line limit.

use crate::cell::CellFlags;
use crate::index::Column;

use super::super::Grid;

impl Grid {
    /// Fix wide char pairs split by an erase of `[start..end)`.
    ///
    /// Clears orphaned halves OUTSIDE the range. Call BEFORE resetting.
    pub(super) fn fix_wide_boundaries(&mut self, line: usize, start: usize, end: usize) {
        let cols = self.cols;
        if start > 0
            && start < cols
            && self.rows[line][Column(start)]
                .flags
                .contains(CellFlags::WIDE_CHAR_SPACER)
        {
            self.rows[line][Column(start - 1)].ch = ' ';
            self.rows[line][Column(start - 1)]
                .flags
                .remove(CellFlags::WIDE_CHAR);
        }
        if end > 0
            && end < cols
            && self.rows[line][Column(end - 1)]
                .flags
                .contains(CellFlags::WIDE_CHAR)
        {
            self.rows[line][Column(end)].ch = ' ';
            self.rows[line][Column(end)]
                .flags
                .remove(CellFlags::WIDE_CHAR_SPACER);
        }
    }

    /// Clear any wide char pair at the given position.
    ///
    /// If the cell is a wide char spacer, clears the preceding wide char.
    /// If the cell is a wide char, clears its trailing spacer.
    pub(super) fn clear_wide_char_at(&mut self, line: usize, col: usize) {
        let cols = self.cols;

        if col >= cols {
            return;
        }

        let flags = self.rows[line][Column(col)].flags;

        // Overwriting a spacer: clear the wide char that owns it.
        if flags.contains(CellFlags::WIDE_CHAR_SPACER) && col > 0 {
            let prev = &mut self.rows[line][Column(col - 1)];
            prev.ch = ' ';
            prev.flags.remove(CellFlags::WIDE_CHAR);
        }

        // Overwriting a wide char: clear its spacer.
        if flags.contains(CellFlags::WIDE_CHAR) && col + 1 < cols {
            let next = &mut self.rows[line][Column(col + 1)];
            next.ch = ' ';
            next.flags.remove(CellFlags::WIDE_CHAR_SPACER);
        }
    }
}
