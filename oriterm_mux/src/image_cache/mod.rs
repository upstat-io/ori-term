//! Bounded LRU image cache for daemon-mode wire-format pixel data.
//!
//! Used by BOTH server and client sides to bound image pixel-data memory and
//! preserve the static-image-rendering correctness path:
//!
//! - **Server side** (`SnapshotCache.image_data_store`) — keeps the most-recent
//!   `Arc<RenderableImageData>` per `(PaneId, ImageId)` so the per-client dispatch
//!   slow path can build `WireImageData` entries on demand without re-extracting
//!   from `Term`.
//! - **Client side** (`MuxClient.image_cache`) — resolves placements arriving in a
//!   wire `PaneSnapshot` whose `image_data` was filtered out by the server (steady
//!   state — client already has the bytes).
//!
//! Eviction is **reachability-bounded**: an entry is NEVER evicted if its
//! `(PaneId, ImageId)` is currently referenced by a placement in the latest
//! snapshot for that pane. This prevents the static-image-loss path that R2 Plan
//! TPR surfaced — under memory pressure, the cache keeps still-referenced entries
//! even past `memory_cap`. The cap is a soft limit; correctness wins.
//!
//! See: bug-tracker/plans/BUG-06-072/

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use oriterm_core::{ImageId, RenderableImageData};

use crate::PaneId;

/// Default cache memory cap (320 MiB, matching `oriterm_core::ImageCache::DEFAULT_MEMORY_LIMIT`).
pub const DEFAULT_MEMORY_CAP_BYTES: usize = 320 * 1024 * 1024;

/// Bounded LRU cache mapping `(PaneId, ImageId)` to `Arc<RenderableImageData>`.
///
/// `F` is the **reachability oracle** — a closure that returns `true` iff the
/// given `(PaneId, ImageId)` is currently referenced by a placement in the
/// caller's latest snapshot for that pane. Each insert path computes its own
/// reachability set at eviction time. See `insert` for the contract.
///
/// Internally Mutex-free; callers wrap in `Mutex<ImageCache<...>>` when they
/// need interior mutability across `&self` API boundaries (see
/// `MuxBackend::pane_image_data`).
pub struct ImageCache {
    /// Pixel data store.
    images: HashMap<(PaneId, ImageId), Arc<RenderableImageData>>,
    /// LRU access tracker (most-recently-used at the back).
    lru: VecDeque<(PaneId, ImageId)>,
    /// Sum of `image.data.len()` for every entry in `images`.
    memory_used: usize,
    /// Soft cap. Once exceeded, `insert` evicts unreferenced entries until
    /// `memory_used <= memory_cap` OR all entries are still-referenced.
    memory_cap: usize,
}

impl ImageCache {
    /// Create an empty cache with the default 320 MiB cap.
    #[must_use]
    pub fn new() -> Self {
        Self::with_memory_cap(DEFAULT_MEMORY_CAP_BYTES)
    }

    /// Create an empty cache with a custom memory cap.
    #[must_use]
    pub fn with_memory_cap(memory_cap: usize) -> Self {
        Self {
            images: HashMap::new(),
            lru: VecDeque::new(),
            memory_used: 0,
            memory_cap,
        }
    }

    /// Insert (or replace) the pixel data for `(pane_id, id)`.
    ///
    /// Runs reachability-bounded eviction until `memory_used <= memory_cap` OR
    /// all entries are still-referenced (in which case `memory_used` may exceed
    /// the soft cap). `reachable` is the reachability oracle — `reachable(p, i)`
    /// must return `true` iff `(p, i)` is referenced by a placement in the
    /// caller's latest snapshot for pane `p`.
    ///
    /// Returns the list of evicted keys (caller propagates server-driven
    /// invalidation — server-side `sent_images` must drop the evicted IDs).
    /// See: bug-tracker/plans/BUG-06-072/
    pub fn insert<F>(
        &mut self,
        pane_id: PaneId,
        id: ImageId,
        data: Arc<RenderableImageData>,
        reachable: F,
    ) -> Vec<(PaneId, ImageId)>
    where
        F: Fn(PaneId, ImageId) -> bool,
    {
        // Replace existing entry: subtract its bytes, bump LRU.
        if let Some(prev) = self.images.get(&(pane_id, id)) {
            self.memory_used = self.memory_used.saturating_sub(prev.data.len());
            self.lru.retain(|key| *key != (pane_id, id));
        }
        self.memory_used = self.memory_used.saturating_add(data.data.len());
        self.images.insert((pane_id, id), data);
        self.lru.push_back((pane_id, id));

        self.evict_until_under_cap(reachable)
    }

    /// Look up `(pane_id, id)`. On hit, bump LRU and return the Arc (cloned —
    /// cheap refcount).
    #[must_use]
    pub fn get(&mut self, pane_id: PaneId, id: ImageId) -> Option<Arc<RenderableImageData>> {
        let entry = self.images.get(&(pane_id, id))?.clone();
        self.lru.retain(|key| *key != (pane_id, id));
        self.lru.push_back((pane_id, id));
        Some(entry)
    }

    /// Drop every entry whose `PaneId` matches `pane_id`. Called on pane close.
    pub fn drop_pane(&mut self, pane_id: PaneId) {
        let to_remove: Vec<(PaneId, ImageId)> = self
            .images
            .keys()
            .filter(|(p, _)| *p == pane_id)
            .copied()
            .collect();
        for key in to_remove {
            if let Some(removed) = self.images.remove(&key) {
                self.memory_used = self.memory_used.saturating_sub(removed.data.len());
            }
        }
        self.lru.retain(|(p, _)| *p != pane_id);
    }

    /// Total bytes currently stored.
    #[must_use]
    pub fn memory_used(&self) -> usize {
        self.memory_used
    }

    /// Configured soft cap.
    #[must_use]
    pub fn memory_cap(&self) -> usize {
        self.memory_cap
    }

    /// Number of entries currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.images.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.images.is_empty()
    }

    /// Reachability-bounded LRU eviction.
    ///
    /// Walks `lru` front-to-back (oldest first); skips entries reported as
    /// still-referenced by `reachable`; evicts the first non-referenced entry
    /// and repeats until `memory_used <= memory_cap`. If every entry is
    /// still-referenced, returns without further eviction (soft-cap correctness
    /// contract).
    fn evict_until_under_cap<F>(&mut self, reachable: F) -> Vec<(PaneId, ImageId)>
    where
        F: Fn(PaneId, ImageId) -> bool,
    {
        let mut evicted = Vec::new();
        if self.memory_used <= self.memory_cap {
            return evicted;
        }
        // Iterate LRU oldest-first; collect a single non-referenced victim per
        // pass, evict, repeat. O(N) per eviction; bounded by total entry count.
        loop {
            if self.memory_used <= self.memory_cap {
                break;
            }
            let victim = self
                .lru
                .iter()
                .copied()
                .find(|(p, i)| !reachable(*p, *i));
            let Some(key) = victim else {
                // Every entry is still-referenced; soft cap exceeded.
                break;
            };
            if let Some(removed) = self.images.remove(&key) {
                self.memory_used = self.memory_used.saturating_sub(removed.data.len());
            }
            self.lru.retain(|k| *k != key);
            evicted.push(key);
        }
        evicted
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
