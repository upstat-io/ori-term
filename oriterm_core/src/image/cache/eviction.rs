//! LRU eviction and orphan pruning for the image cache.

use super::{ImageCache, ImageId};

impl ImageCache {
    /// Evict least-recently-used images until under memory limit.
    ///
    /// Prefers reachability-poor images first (no placement AND no
    /// placeholder anchor), then falls back to placed/anchored images by
    /// LRU order. Builds the reachability set once to avoid O(n*m) per-
    /// eviction scans.
    pub(super) fn evict_lru(&mut self) {
        let reachable = self.placed_or_anchored_id_set();

        while self.memory_used > self.memory_limit && !self.images.is_empty() {
            if !self.evict_one(&reachable) {
                break;
            }
        }
    }

    /// Evict the single least-recently-used image. Returns `true` if
    /// an image was evicted.
    ///
    /// Uses a precomputed reachability set (placements + placeholder
    /// anchors). Reachable images are NEVER evicted — when only
    /// reachable images remain, `evict_one` returns `false` so the
    /// caller can surface `MemoryLimitExceeded` instead of corrupting
    /// the visible scene.
    pub(super) fn evict_one(&mut self, reachable: &std::collections::HashSet<ImageId>) -> bool {
        let victim = self
            .images
            .iter()
            .filter(|(id, _)| !reachable.contains(id))
            .map(|(id, img)| (*id, img.last_accessed))
            .min_by_key(|(_, last)| *last);

        if let Some((id, _)) = victim {
            self.remove_image(id);
            true
        } else {
            false
        }
    }

    /// Build a `HashSet` of image IDs that are reachable from the grid —
    /// either via at least one placement, or as a kitty unicode-placeholder
    /// (`U=1`) anchor.
    pub(super) fn placed_or_anchored_id_set(&self) -> std::collections::HashSet<ImageId> {
        let mut s: std::collections::HashSet<ImageId> =
            self.placements.iter().map(|p| p.image_id).collect();
        s.extend(self.placeholder_anchors.iter().copied());
        s
    }
}
