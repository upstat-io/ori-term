//! Kitty unicode-placeholder (`U=1`) anchor management.
//!
//! `U=1` transmit/place suppresses real `ImagePlacement` creation — the client
//! writes `U+10EEEE` cells into the grid which resolve to the image at render
//! time. Without an explicit anchor, LRU eviction would priority-drop these
//! images on any cache pressure (no placement => unplaced rank). This module
//! owns the anchor set, the per-image display grid `(cols, rows)`, and the
//! `Term`-driven anchor reconciliation.
//!
//! Fields live on `ImageCache` in `mod.rs`; the impl methods live here per
//! the per-concern submodule pattern (`animation.rs`, `deletion.rs`,
//! `eviction.rs`, `lifecycle.rs`).

use std::collections::HashSet;

use super::{ImageCache, ImageId};

impl ImageCache {
    /// Record that an image is anchored by the kitty unicode-placeholder
    /// protocol. Anchored images survive LRU eviction even without an
    /// active placement.
    pub(crate) fn add_placeholder_anchor(&mut self, id: ImageId) {
        if self.placeholder_anchors.insert(id) {
            self.dirty = true;
        }
    }

    /// Record the display grid `(cols, rows)` for a U=1 anchor.
    ///
    /// Called from the kitty handler whenever a U=1 transmit/place carries
    /// `c=N,r=M`. A `(1, 1)` grid is the implicit default (single-cell
    /// placement); the caller skips this method in that case.
    pub(crate) fn set_placeholder_anchor_grid(&mut self, id: ImageId, cols: u32, rows: u32) {
        if cols == 0 || rows == 0 {
            return;
        }
        if self.placeholder_anchor_grid.insert(id, (cols, rows)) != Some((cols, rows)) {
            self.dirty = true;
        }
    }

    /// Anchor a U=1 image and optionally record its display grid in one
    /// step. SSOT for the U=1 anchor + grid contract across every kitty
    /// handler entry point that recognises `U=1` (`a=t,U=1`, `a=T,U=1`,
    /// `a=p,U=1`). Pass `Some((cols, rows))` when `c=N,r=M` accompany the
    /// transmit/place; pass `None` for the implicit `(1, 1)` default.
    pub(crate) fn anchor_placeholder_with_grid(&mut self, id: ImageId, grid: Option<(u32, u32)>) {
        self.add_placeholder_anchor(id);
        if let Some((cols, rows)) = grid {
            self.set_placeholder_anchor_grid(id, cols, rows);
        }
    }

    /// Image IDs anchored by the unicode-placeholder protocol.
    pub fn placeholder_anchors(&self) -> &HashSet<ImageId> {
        &self.placeholder_anchors
    }

    /// Display grid `(cols, rows)` for a U=1 anchor, if any.
    ///
    /// `None` means "no recorded grid" — caller renders the placeholder
    /// cell as a full single-cell image (the §13.4 baseline behavior).
    pub fn placeholder_anchor_grid_for(&self, id: ImageId) -> Option<(u32, u32)> {
        self.placeholder_anchor_grid.get(&id).copied()
    }

    /// Restrict the anchor set to the supplied survivor IDs.
    ///
    /// Called by the `Term` layer after grid mutations that may erase
    /// placeholder cells. The caller computes the surviving anchor set by
    /// walking the grid for `U+10EEEE` cells; this method retains only
    /// those entries.
    pub(crate) fn reconcile_placeholder_anchors(&mut self, survivors: &HashSet<ImageId>) {
        let before = self.placeholder_anchors.len();
        self.placeholder_anchors.retain(|id| survivors.contains(id));
        // The anchor grid mirrors the anchor set — drop entries for any
        // image_id no longer anchored.
        self.placeholder_anchor_grid
            .retain(|id, _| survivors.contains(id));
        if self.placeholder_anchors.len() != before {
            self.dirty = true;
        }
    }
}

#[cfg(test)]
mod tests;
