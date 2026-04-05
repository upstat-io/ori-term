//! LRU eviction and orphan pruning for the image cache.

use super::{ImageCache, ImageId};

impl ImageCache {
    /// Evict least-recently-used images until under memory limit.
    ///
    /// Prefers images with zero placements first, then evicts placed
    /// images by LRU order (Ghostty pattern). Builds a placed-ID set
    /// once to avoid O(n*m) per-eviction placement scans.
    pub(super) fn evict_lru(&mut self) {
        let placed = self.placed_id_set();

        while self.memory_used > self.memory_limit && !self.images.is_empty() {
            if !self.evict_one(&placed) {
                break;
            }
        }
    }

    /// Evict the single least-recently-used image. Returns `true` if
    /// an image was evicted.
    ///
    /// Uses a precomputed set of placed image IDs for O(n) candidate
    /// selection (unplaced first, then oldest access counter).
    pub(super) fn evict_one(&mut self, placed: &std::collections::HashSet<ImageId>) -> bool {
        let victim = self
            .images
            .iter()
            .map(|(id, img)| (*id, img.last_accessed, placed.contains(id)))
            .min_by(|a, b| {
                a.2.cmp(&b.2) // false (no placements) < true
                    .then(a.1.cmp(&b.1)) // oldest access first
            });

        if let Some((id, _, _)) = victim {
            self.remove_image(id);
            true
        } else {
            false
        }
    }

    /// Build a `HashSet` of image IDs that have at least one placement.
    pub(super) fn placed_id_set(&self) -> std::collections::HashSet<ImageId> {
        self.placements.iter().map(|p| p.image_id).collect()
    }

    /// Remove images that have no remaining placements.
    pub(crate) fn remove_orphans(&mut self) {
        let placed = self.placed_id_set();
        let orphans: Vec<ImageId> = self
            .images
            .keys()
            .filter(|id| !placed.contains(id))
            .copied()
            .collect();

        for id in orphans {
            // Delegates to `remove_image` to clean up all associated state:
            // animations, animation_frames, frame_starts, and memory tracking.
            self.remove_image(id);
        }
    }
}
