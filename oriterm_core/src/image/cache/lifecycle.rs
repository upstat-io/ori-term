//! Placement lifecycle operations: scrollback pruning, region erasure,
//! cell-coverage updates, and resize/reflow remapping.

use crate::grid::{ReflowMapping, StableRowIndex};

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

    /// Remove placements whose starting column is entirely outside the
    /// new grid bounds.
    ///
    /// Called when the grid is resized (window resize, DECCOLM 80↔132).
    /// Placements with `cell_col >= new_cols` are removed; partially
    /// overlapping placements (start inside, end outside) survive — the
    /// renderer clips their right edge. This matches Ghostty's
    /// storage-layer policy: no clamping, just drop the fully-out-of-bounds
    /// rows.
    ///
    /// `new_rows` is accepted for forward-compatibility but unused —
    /// row bounds are handled by `prune_scrollback` via `StableRowIndex`
    /// eviction, not by this resize handler.
    pub(crate) fn on_resize(&mut self, new_cols: usize, _new_rows: usize) {
        let ids = self.remove_placements_where(|p| p.cell_col >= new_cols);
        self.prune_if_orphaned(&ids);
    }

    /// Translate placement `cell_row` values through a reflow mapping.
    ///
    /// Reflow rewrites the row topology: content that was on source row
    /// `X` now lives on output row `mapping.first_output_row[X]`.
    /// Placements stored with `StableRowIndex(old_total_evicted + X)`
    /// must be translated to `StableRowIndex(old_total_evicted +
    /// mapping.first_output_row[X])` so they continue to point at the
    /// same visual content.
    ///
    /// Uses `old_total_evicted` — NOT `new_total_evicted` — as the
    /// conversion base because `first_output_row` indexes are relative
    /// to the pre-eviction result array assembled in
    /// `reflow_cells`. `apply_reflow_result` later distributes that
    /// array into scrollback + visible rows, bumping `total_evicted`
    /// as rows overflow the scrollback ring; that bookkeeping is
    /// accounted for by subsequent `prune_scrollback` calls, not here.
    ///
    /// Placements whose pre-reflow `StableRowIndex` is below
    /// `old_total_evicted` (underflow via `checked_sub`) or whose
    /// mapped source row index falls outside the mapping range are
    /// skipped; a subsequent `prune_scrollback` will clean them up.
    /// Wrapped source rows map to the same output row as their parent
    /// via `first_output_row`, so an "empty range" never means "delete
    /// the placement" — every source row always maps to exactly one
    /// output row.
    pub(crate) fn remap_placements(&mut self, mapping: &ReflowMapping) {
        let base = mapping.old_total_evicted;
        let mapping_len = mapping.first_output_row.len();
        let mut changed = false;
        for p in &mut self.placements {
            let Some(old_abs) = p.cell_row.0.checked_sub(base) else {
                // Placement was already evicted before reflow.
                // `prune_scrollback` will clean it up.
                continue;
            };
            let old_abs = old_abs as usize;
            if old_abs >= mapping_len {
                // Row added after the mapping was built (e.g. grown
                // visible area). Leave placement unchanged — it's
                // already pointing to a valid post-reflow row.
                continue;
            }
            let new_output_row = mapping.first_output_row[old_abs] as u64;
            let new_stable = StableRowIndex(base + new_output_row);
            if p.cell_row != new_stable {
                p.cell_row = new_stable;
                changed = true;
            }
        }
        if changed {
            self.dirty = true;
        }
    }
}
