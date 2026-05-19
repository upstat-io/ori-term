//! Terminal resize logic.
//!
//! Extracted from `term/mod.rs` to keep the root module under the
//! 500-line source-file limit.

use crate::effect::sink::EffectSink;
use crate::grid::StableRowIndex;

use super::Term;

impl<S: EffectSink> Term<S> {
    /// Resize the terminal to new dimensions.
    ///
    /// When `reflow` is true, the primary grid re-wraps soft-wrapped lines
    /// to fit the new column width. When false, rows are simply truncated
    /// or extended (used on Windows where `ConPTY` handles content reflow via
    /// escape sequences — doing our own reflow races with `ConPTY`'s output).
    /// The alternate grid never reflows (full-screen apps manage their own
    /// layout).
    ///
    /// Marks all lines dirty so the renderer repaints. Also marks selection
    /// as dirty since content positions change.
    pub fn resize(&mut self, new_lines: usize, new_cols: usize, reflow: bool) {
        if new_lines == 0 || new_cols == 0 {
            return;
        }
        if self.grid.lines() == new_lines && self.grid.cols() == new_cols {
            return;
        }

        // Update DECCOLM default so CSI ? 3 l restores to window width.
        self.deccolm_default_cols = new_cols;

        // Primary grid: reflow when caller permits. The resulting
        // `ReflowMapping` (if reflow actually ran) lets us translate
        // pre-reflow image-placement `StableRowIndex` values through
        // the new row topology.
        let prev_primary = self.grid.total_evicted();
        let reflow_mapping = self.grid.resize(new_lines, new_cols, reflow);
        let new_primary = self.grid.total_evicted();

        // Image-cache lifecycle ordering matters:
        // 1. remap FIRST — translate placements' StableRowIndex values
        //    through the new row topology. Must run before
        //    prune_scrollback so that pruning compares mapped row
        //    indices against the post-reflow eviction boundary.
        // 2. prune_scrollback — drop placements whose (now-mapped)
        //    row is below the new eviction boundary.
        // 3. on_resize — drop placements whose starting column is
        //    entirely outside the new grid width.
        if let Some(ref mapping) = reflow_mapping {
            self.image_cache.remap_placements(mapping);
        }
        if new_primary > prev_primary {
            self.image_cache
                .prune_scrollback(StableRowIndex(new_primary as u64));
        }
        self.image_cache.on_resize(new_cols, new_lines);

        // Alternate grid: no reflow (apps like vim handle their own
        // layout). Alt grid has 0 scrollback capacity, so every scroll
        // evicts. Skip if alt grid hasn't been allocated yet (no app
        // has used alt screen). The alt image cache receives
        // `on_resize` column-bounds handling whenever the alt grid
        // exists — no remap is needed because the alt grid is resized
        // with `reflow: false` and therefore never produces a
        // `ReflowMapping`.
        if let Some(alt) = &mut self.alt_grid {
            let prev_alt = alt.total_evicted();
            let _alt_mapping = alt.resize(new_lines, new_cols, false);
            let new_alt = alt.total_evicted();
            if let Some(cache) = &mut self.alt_image_cache {
                if new_alt > prev_alt {
                    cache.prune_scrollback(StableRowIndex(new_alt as u64));
                }
                cache.on_resize(new_cols, new_lines);
            }
        }

        // Mark selection dirty since cell positions changed.
        // Note: both grids are already fully marked dirty by
        // `Grid::finalize_resize` → `dirty.resize()` → `mark_all()`.
        self.selection_dirty = true;

        // Resize may have evicted scrollback rows or dropped columns that
        // carried `U+10EEEE` cells on EITHER screen; re-derive both the
        // primary and alt anchor sets from their respective post-resize
        // grids. `reconcile_both_*` accesses the `self.grid` /
        // `self.image_cache` fields directly so the symmetric reconcile
        // covers the inactive pair regardless of which screen is active.
        self.reconcile_both_placeholder_anchors();
    }
}

#[cfg(test)]
mod tests;
