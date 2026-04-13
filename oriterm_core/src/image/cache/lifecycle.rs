//! Placement lifecycle operations: scrollback pruning, region erasure,
//! cell-coverage updates, and (future) resize/reflow remapping.

use crate::grid::StableRowIndex;

use super::ImageCache;
use crate::image::PlacementSizing;

impl ImageCache {
    /// Remove placements whose `cell_row` is before the eviction boundary.
    ///
    /// Called when scrollback evicts rows so stale placements don't
    /// accumulate. Also removes images with zero remaining placements
    /// (Ghostty pattern: unused images evicted first). Only prunes
    /// images whose placements were actually evicted — preserves Kitty
    /// deferred-placement images stored without placements.
    pub(crate) fn prune_scrollback(&mut self, evicted_before: StableRowIndex) {
        let ids = self.remove_placements_where(|p| p.cell_row < evicted_before);
        self.prune_if_orphaned(&ids);
    }

    /// Remove placements overlapping a rectangular region.
    ///
    /// Used by ED/EL erase operations. If `left`/`right` are `None`,
    /// the full row width is cleared. Prunes orphaned image data for
    /// the specific images whose placements were removed — erase
    /// operations should not leave stale image payloads.
    pub(crate) fn remove_placements_in_region(
        &mut self,
        top: StableRowIndex,
        bottom: StableRowIndex,
        left: Option<usize>,
        right: Option<usize>,
    ) {
        let ids = self.remove_placements_where(|p| {
            let pb = StableRowIndex(p.cell_row.0 + p.rows.saturating_sub(1) as u64);
            let pr = p.cell_col + p.cols.saturating_sub(1);
            let row_overlap = p.cell_row <= bottom && pb >= top;
            if !row_overlap {
                return false;
            }
            match (left, right) {
                (Some(l), Some(r)) => p.cell_col <= r && pr >= l,
                (Some(l), None) => pr >= l,
                (None, Some(r)) => p.cell_col <= r,
                (None, None) => true,
            }
        });
        self.prune_if_orphaned(&ids);
    }

    /// Recalculate `cols`/`rows` for `FixedPixels` placements.
    ///
    /// Called when cell pixel dimensions change (font size, zoom) so
    /// viewport intersection and region queries use correct cell counts.
    pub(crate) fn update_cell_coverage(&mut self, cell_w: u16, cell_h: u16) {
        let cw = cell_w.max(1) as usize;
        let ch = cell_h.max(1) as usize;

        for p in &mut self.placements {
            if let PlacementSizing::FixedPixels { width, height } = p.sizing {
                let new_cols = (width as usize).div_ceil(cw);
                let new_rows = (height as usize).div_ceil(ch);
                if p.cols != new_cols || p.rows != new_rows {
                    p.cols = new_cols;
                    p.rows = new_rows;
                    self.dirty = true;
                }
            }
        }
    }
}
