//! Placement management for `ImageCache`.
//!
//! Owns the create / mutate / prune / query API for the `placements`
//! vector. Extracted from `cache/mod.rs` in BUG-06-092 to satisfy the
//! 500-line file-size cap. All methods preserved verbatim from the
//! pre-extraction shape; behavior unchanged.
//!
//! SSOT for the LRU-on-place contract: [`ImageCache::place`] bumps
//! `last_accessed`; `eviction::evict_one` ranks the unplaced pool by
//! that field. Per-protocol bumps would scatter the contract across
//! N consumer sites; the canonical home is here.

use crate::grid::StableRowIndex;

use super::super::{ImageId, ImagePlacement};
use super::ImageCache;

impl ImageCache {
    /// Add a placement for an existing image.
    ///
    /// Bumps the underlying image's `last_accessed` recency stamp so that
    /// re-placement of an existing image refreshes its eviction priority
    /// among the unplaced pool. This is the canonical recency-update path
    /// for all image-protocol create-placement flows: kitty
    /// (`kitty_create_placement`), sixel (`sixel_create_placement`), and
    /// iterm2 (`iterm2_create_placement`) all funnel through here, so
    /// `eviction::evict_one`'s `min_by_key(last_accessed)` rank stays
    /// LRU-on-place across every protocol — never falling back to the
    /// pre-§13.6.1 FIFO-on-store-order behavior for any one protocol.
    ///
    /// Per-protocol bumps would scatter the LRU contract across N
    /// consumer sites; an `ImageCache`-owned API is the SSOT.
    pub(crate) fn place(&mut self, placement: ImagePlacement) {
        let image_id = placement.image_id;
        // Per kitty graphics protocol: a new placement with the same
        // `(image_id, placement_id)` REPLACES the existing one. Without
        // this dedup, moving a kitty-graphics plane (notcurses-demo
        // `intro` orca: `ncplane_move_yx` + re-Place at the new offset)
        // appends a new placement per frame and the old placements stay
        // alive — every previous position renders as a ghost, producing
        // the visible "trail" of orcas the operator reports.
        //
        // Complexity: O(N) per call over `self.placements`. Acceptable
        // for typical workloads where placements ≤ low double digits
        // (viewport-visible imagery cap). High-placement scenarios may
        // benefit from a parallel `(image_id, placement_id) → index`
        // map for O(1) dedup — see `bug-tracker/section-06-rendering-perf.md`.
        self.placements
            .retain(|p| !(p.image_id == image_id && p.placement_id == placement.placement_id));
        self.placements.push(placement);
        self.access_counter += 1;
        if let Some(img) = self.images.get_mut(&image_id) {
            img.last_accessed = self.access_counter;
        }
        self.dirty = true;
    }

    /// Mutate the `z_index` of an existing placement in place.
    ///
    /// Used by §13.6 cross-protocol GPU pilots (e.g.
    /// `kitty_sixel_mixed_z_order`) to inject a non-zero z onto a sixel
    /// placement: the sixel DCS protocol has no `z=` field so the
    /// production handler always lands at `z_index: 0`. The pilot drives
    /// real protocol bytes through the production handler — preserving
    /// §13.6.1's architectural pin that cross-stack tests must NOT
    /// synthesize cache entries — then calls this mutator to simulate
    /// the cross-protocol layering scenario the catalog row
    /// `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER` pins.
    ///
    /// Returns `true` if a matching placement was found and updated.
    ///
    /// Test-only API per BUG-06-092 §05 cfg-gate cleanup. The `_for_test`
    /// naming pattern + `#[cfg(any(test, feature = "test-support"))]`
    /// gate make the test-only intent enforceable by the compiler.
    /// Consumers in `oriterm/src/gpu/visual_regression/spec_chain/pilots/`
    /// (themselves `#[cfg(test)]`-gated) reach this via `oriterm`'s
    /// `[dev-dependencies] oriterm_core = { features = ["test-support"] }`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn set_placement_z_index_for_test(
        &mut self,
        image_id: ImageId,
        placement_id: Option<u32>,
        z: i32,
    ) -> bool {
        for p in &mut self.placements {
            if p.image_id == image_id && p.placement_id == placement_id {
                p.z_index = z;
                self.dirty = true;
                return true;
            }
        }
        false
    }

    /// Remove a specific placement by image ID and placement ID.
    pub(crate) fn remove_placement(&mut self, image_id: ImageId, placement_id: u32) {
        let before = self.placements.len();
        self.placements
            .retain(|p| !(p.image_id == image_id && p.placement_id == Some(placement_id)));
        if self.placements.len() != before {
            self.dirty = true;
        }
    }

    /// Remove all placements for an image ID (without removing the image data).
    pub(crate) fn remove_placements_for_image(&mut self, image_id: ImageId) {
        let before = self.placements.len();
        self.placements.retain(|p| p.image_id != image_id);
        if self.placements.len() != before {
            self.dirty = true;
        }
    }

    /// Remove placements at a specific column.
    ///
    /// Returns the image IDs of removed placements for targeted orphan
    /// pruning. Callers wanting to also remove orphaned image data should
    /// pass the returned IDs to [`prune_if_orphaned`].
    pub(crate) fn remove_placements_at_column(&mut self, col: usize) -> Vec<ImageId> {
        self.remove_placements_where(|p| {
            let right = p.cell_col + p.cols.saturating_sub(1);
            p.cell_col <= col && right >= col
        })
    }

    /// Remove placements at a specific row.
    ///
    /// Returns the image IDs of removed placements for targeted orphan
    /// pruning.
    pub(crate) fn remove_placements_at_row(&mut self, row: StableRowIndex) -> Vec<ImageId> {
        self.remove_placements_where(|p| {
            let bottom = StableRowIndex(p.cell_row.0 + p.rows.saturating_sub(1) as u64);
            p.cell_row <= row && bottom >= row
        })
    }

    /// Remove placements with a specific z-index.
    ///
    /// Returns the image IDs of removed placements for targeted orphan
    /// pruning.
    pub(crate) fn remove_placements_by_z_index(&mut self, z: i32) -> Vec<ImageId> {
        self.remove_placements_where(|p| p.z_index == z)
    }

    /// Remove placements whose origin cell equals `(col, row)` exactly.
    ///
    /// Origin-cell equality — NOT span intersection. Used by sixel
    /// re-placement to drop the previous placement anchored at this
    /// exact origin before writing a fresh one. Kitty `d=c/C/p/P` does
    /// NOT use this; see `remove_placements_intersecting_cell`.
    pub(crate) fn remove_by_position(&mut self, col: usize, row: StableRowIndex) -> Vec<ImageId> {
        self.remove_placements_where(|p| p.cell_col == col && p.cell_row == row)
    }

    /// Prune specific images that have become orphaned (zero placements
    /// AND no placeholder anchor).
    ///
    /// Only checks the given IDs — does NOT sweep the entire cache.
    /// Anchored images are skipped because the kitty `U=1` protocol stores
    /// the image without a real placement; the placeholder cells in the
    /// grid are what keep it reachable.
    pub(crate) fn prune_if_orphaned(&mut self, ids: &[ImageId]) {
        for &id in ids {
            let has_placement = self.placements.iter().any(|p| p.image_id == id);
            let anchored = self.placeholder_anchors.contains(&id);
            if !has_placement && !anchored {
                self.remove_image(id);
            }
        }
    }

    /// Remove placements matching a predicate and return affected image IDs.
    pub(super) fn remove_placements_where(
        &mut self,
        predicate: impl Fn(&ImagePlacement) -> bool,
    ) -> Vec<ImageId> {
        let mut affected = Vec::new();
        let before = self.placements.len();
        self.placements.retain(|p| {
            if predicate(p) {
                affected.push(p.image_id);
                false
            } else {
                true
            }
        });
        if self.placements.len() != before {
            self.dirty = true;
        }
        affected
    }

    /// Iterate placements visible in the given stable row range (inclusive).
    ///
    /// Zero-allocation: returns an iterator that filters in-place. Preferred
    /// for the hot render path where no intermediate buffer is needed.
    pub(crate) fn viewport_placements(
        &self,
        top_row: StableRowIndex,
        bottom_row: StableRowIndex,
    ) -> impl Iterator<Item = &ImagePlacement> {
        self.placements
            .iter()
            .filter(move |p| p.intersects_viewport(top_row, bottom_row))
    }

    /// Convenience wrapper returning a new `Vec` of visible placements.
    ///
    /// For hot-path rendering, prefer [`viewport_placements`] (zero-allocation
    /// iterator). This allocates on every call — intended for tests only.
    ///
    /// [`viewport_placements`]: Self::viewport_placements
    #[cfg(test)]
    pub(crate) fn placements_in_viewport(
        &self,
        top_row: StableRowIndex,
        bottom_row: StableRowIndex,
    ) -> Vec<&ImagePlacement> {
        self.viewport_placements(top_row, bottom_row).collect()
    }
}
