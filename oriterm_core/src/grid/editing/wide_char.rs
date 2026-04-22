//! Wide character boundary fixup routines.
//!
//! These handle the split-pair cleanup when wide characters (CJK, emoji)
//! are partially overwritten or erased. Extracted from `editing/mod.rs`
//! to keep that file under the 500-line limit.

use crate::cell::{Cell, CellFlags};
use crate::index::Column;

use super::super::Grid;

/// Reset a sibling cell's visible state when a wide-char pair is broken.
///
/// Clears `ch`, `fg`, `bg`, `extra` back to terminal defaults and strips
/// the specified wide-pair flag. Retains `DRAWN` because the cell was
/// previously written by the application — consumers like DECRQCRA need
/// that write-history bit to stay set. Without this reset the sibling
/// carried stale plane colors (notably the black BG painted by notcurses
/// emoji-plane bases), producing a black trail of cells that never got
/// re-rendered after the wide-char partner was cleared.
fn reset_wide_sibling(cell: &mut Cell, flag_to_strip: CellFlags) {
    let had_drawn = cell.flags.contains(CellFlags::DRAWN);
    *cell = Cell::default();
    cell.flags.remove(flag_to_strip);
    if had_drawn {
        cell.flags.insert(CellFlags::DRAWN);
    }
}

impl Grid {
    /// Fix wide char pairs split by an erase of `[start..end)`.
    ///
    /// Clears orphaned halves OUTSIDE the range. Call BEFORE resetting.
    ///
    /// This is the correct helper for band-aware cleanup (unlike
    /// `clear_wide_char_at` which clears both halves unconditionally):
    /// it leaves in-band pairs intact and only removes the partner that
    /// would be orphaned by destroying the range `[start..end)`.
    pub(in crate::grid) fn fix_wide_boundaries(&mut self, line: usize, start: usize, end: usize) {
        let cols = self.cols;
        if start > 0
            && start < cols
            && self.rows[line][Column(start)]
                .flags
                .contains(CellFlags::WIDE_CHAR_SPACER)
        {
            // Out-of-band partner at start-1 must be fully reset — its
            // fg/bg/extra reflected the now-defunct wide-char pair.
            reset_wide_sibling(
                &mut self.rows[line][Column(start - 1)],
                CellFlags::WIDE_CHAR,
            );
            // In-band spacer at `start` also loses its spacer flag; the
            // caller will reset the cell fully via template on the way to
            // destroying the range.
            self.rows[line][Column(start)]
                .flags
                .remove(CellFlags::WIDE_CHAR_SPACER);
            self.dirty.mark_cols(line, start - 1, start);
        }
        if end > 0
            && end < cols
            && self.rows[line][Column(end - 1)]
                .flags
                .contains(CellFlags::WIDE_CHAR)
        {
            // Out-of-band partner at `end` must be fully reset — its
            // fg/bg/extra reflected the now-defunct wide-char pair.
            reset_wide_sibling(
                &mut self.rows[line][Column(end)],
                CellFlags::WIDE_CHAR_SPACER,
            );
            // In-band base at end-1 also loses its wide-char flag.
            self.rows[line][Column(end - 1)]
                .flags
                .remove(CellFlags::WIDE_CHAR);
            self.dirty.mark_cols(line, end - 1, end);
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
        // Fully reset the partner — leaving its stale fg/bg/extra behind
        // produces the mojibake "black trail" (BUG-06-016): the emoji
        // plane's opaque black bg survives on the sibling cell because
        // notcurses's per-frame composite may skip re-writing it.
        if flags.contains(CellFlags::WIDE_CHAR_SPACER) && col > 0 {
            reset_wide_sibling(&mut self.rows[line][Column(col - 1)], CellFlags::WIDE_CHAR);
            self.dirty.mark_cols(line, col - 1, col - 1);
        }

        // Overwriting a wide char: clear its spacer.
        if flags.contains(CellFlags::WIDE_CHAR) && col + 1 < cols {
            reset_wide_sibling(
                &mut self.rows[line][Column(col + 1)],
                CellFlags::WIDE_CHAR_SPACER,
            );
            self.dirty.mark_cols(line, col + 1, col + 1);
        }
    }
}
