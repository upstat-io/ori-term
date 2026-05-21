//! Image cache with LRU eviction and configurable memory limits.

mod animation;
mod deletion;
mod eviction;
mod lifecycle;
mod placeholder;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use crate::grid::StableRowIndex;

use super::{AnimationState, ImageData, ImageError, ImageId, ImagePlacement};

/// Default memory limit for decoded image data (320 MiB, matching Ghostty).
const DEFAULT_MEMORY_LIMIT: usize = 320 * 1024 * 1024;

/// Default maximum size for a single image (64 MiB).
const DEFAULT_MAX_SINGLE_IMAGE: usize = 64 * 1024 * 1024;

/// Starting ID for auto-assigned images (mid-range u32 to avoid
/// collisions with client-assigned IDs that start at 1).
const AUTO_ID_START: u32 = 2_147_483_647;

/// In-memory image cache with reference counting, eviction, and
/// configurable memory limits.
///
/// Each terminal screen (primary and alternate) owns its own cache.
/// Placements use `StableRowIndex` so they scroll with text
/// automatically.
pub struct ImageCache {
    /// Image data store keyed by image ID.
    pub(super) images: HashMap<ImageId, ImageData>,
    /// Active placements sorted by row for efficient viewport queries.
    pub(super) placements: Vec<ImagePlacement>,
    /// Total bytes of decoded image data currently stored.
    memory_used: usize,
    /// Configurable maximum memory for decoded image data.
    memory_limit: usize,
    /// Reject single images exceeding this size.
    pub(super) max_single_image_bytes: usize,
    /// Monotonic ID allocator for auto-assigned images.
    next_id: u32,
    /// Monotonic counter bumped on each image access (LRU ordering).
    access_counter: u64,
    /// Set when placements/images change; caller clears via `take_dirty()`.
    pub(super) dirty: bool,
    /// Monotonic creation-order rank for each stored image.
    ///
    /// Distinct from `last_accessed` (LRU tracking, bumped on access).
    /// `store_order[id]` is set once at `store()` time and never updated —
    /// `d=n/N` in the kitty graphics protocol resolves "newest image with
    /// number I=" by this field, not by LRU recency (see `newest_by_image_number`).
    store_order: HashMap<ImageId, u64>,
    /// Next value to assign in `store_order` when storing an image.
    next_store_order: u64,
    /// Per-image animation state for multi-frame images.
    pub(super) animations: HashMap<ImageId, AnimationState>,
    /// All decoded frames for animated images (frame 0 is also in `ImageData.data`).
    pub(super) animation_frames: HashMap<ImageId, Vec<Arc<Vec<u8>>>>,
    /// When each animated image's current frame started displaying.
    pub(super) frame_starts: HashMap<ImageId, Instant>,
    /// Whether animation is enabled (from config).
    pub(super) animation_enabled: bool,
    /// Image IDs anchored by kitty unicode-placeholder (`U=1`) protocol.
    /// Owned by `placeholder` submodule — see its module docs.
    pub(super) placeholder_anchors: HashSet<ImageId>,
    /// Per-image display grid `(cols, rows)` for U=1 placements. Owned by
    /// `placeholder` submodule — see its module docs.
    pub(super) placeholder_anchor_grid: HashMap<ImageId, (u32, u32)>,
}

impl ImageCache {
    /// Create a new empty image cache with default limits.
    pub(crate) fn new() -> Self {
        Self {
            images: HashMap::new(),
            placements: Vec::new(),
            memory_used: 0,
            memory_limit: DEFAULT_MEMORY_LIMIT,
            max_single_image_bytes: DEFAULT_MAX_SINGLE_IMAGE,
            next_id: AUTO_ID_START,
            access_counter: 0,
            dirty: false,
            store_order: HashMap::new(),
            next_store_order: 0,
            animations: HashMap::new(),
            animation_frames: HashMap::new(),
            frame_starts: HashMap::new(),
            animation_enabled: true,
            placeholder_anchors: HashSet::new(),
            placeholder_anchor_grid: HashMap::new(),
        }
    }

    /// Returns `true` if any images or placements have changed since
    /// the last `take_dirty()` call.
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Returns the current dirty flag and clears it.
    ///
    /// Called by `Term::renderable_content_into()` when building
    /// the snapshot for the renderer.
    pub(crate) fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    /// Total bytes of decoded image data in the cache.
    #[cfg(test)]
    pub(crate) fn memory_used(&self) -> usize {
        self.memory_used
    }

    /// Number of stored images.
    pub fn image_count(&self) -> usize {
        self.images.len()
    }

    /// Number of active placements.
    pub fn placement_count(&self) -> usize {
        self.placements.len()
    }

    /// Maximum allowed size for a single image in bytes.
    pub(crate) fn max_single_image_bytes(&self) -> usize {
        self.max_single_image_bytes
    }

    /// Update the memory limit. Triggers eviction if currently over.
    pub fn set_memory_limit(&mut self, limit: usize) {
        self.memory_limit = limit;
        self.evict_lru();
    }

    /// Update the max single image size.
    pub fn set_max_single_image(&mut self, limit: usize) {
        self.max_single_image_bytes = limit;
    }

    /// Allocate the next auto-assigned image ID.
    pub(crate) fn next_image_id(&mut self) -> ImageId {
        let id = ImageId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        id
    }

    /// Store image data in the cache.
    ///
    /// Rejects images exceeding `max_single_image_bytes`. Evicts LRU
    /// images if the cache would exceed `memory_limit`.
    pub(crate) fn store(&mut self, mut data: ImageData) -> Result<ImageId, ImageError> {
        let size = data.data.len();
        if size > self.max_single_image_bytes {
            return Err(ImageError::OversizedImage);
        }

        // Evict until we have room (or can't evict more).
        let reachable = self.placed_or_anchored_id_set();
        while self.memory_used + size > self.memory_limit && !self.images.is_empty() {
            if !self.evict_one(&reachable) {
                return Err(ImageError::MemoryLimitExceeded);
            }
        }

        // Stamp access counter so store order determines initial LRU rank.
        self.access_counter += 1;
        data.last_accessed = self.access_counter;

        let id = data.id;

        // If replacing an existing image, fully remove the old one
        // (including animation state and frame memory) before inserting
        // the new data. Uses `remove_image()` which handles all cleanup.
        if self.images.contains_key(&id) {
            // High-frequency on animated kitty graphics (notcurses
            // re-transmits same image_id per frame). Kept at debug so
            // default INFO doesn't pay sync-write cost on the IO thread.
            log::debug!(
                target: "oriterm_core::image::cache::store",
                "image store: replacing existing id={}", id.0
            );
            self.remove_image(id);
        }

        self.memory_used += size;
        self.images.insert(id, data);
        self.next_store_order = self.next_store_order.wrapping_add(1);
        self.store_order.insert(id, self.next_store_order);
        self.dirty = true;
        Ok(id)
    }

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
    /// Per `plans/spec-conformance/section-13-kitty-graphics.md §13.6.1`
    /// — per-protocol bumps would scatter the LRU contract across N
    /// consumer sites; an `ImageCache`-owned API is the SSOT.
    pub(crate) fn place(&mut self, placement: ImagePlacement) {
        let image_id = placement.image_id;
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

    /// Remove an image and all its placements.
    ///
    /// Memory-accounting invariant: animated images use `animation_frames[id]`
    /// as the single SSOT for size (sum of every frame); `img.data` would
    /// double- or under-count because it points at the currently-displayed
    /// frame, which drifts as the animation advances. Static images fall
    /// back to `img.data`.
    ///
    /// Map-sync invariant: the `placeholder_anchor_grid` entry is dropped
    /// alongside the anchor so every direct image-removal path (`d=I`, `d=F`,
    /// LRU eviction, U=1 anchor-orphaned cleanup, store-triggered replacement)
    /// keeps the two maps in sync. A drift here would leak grid entries
    /// across image create/delete cycles AND let a reused `ImageId` read a
    /// stale `(cols, rows)` tuple producing broken UV slices.
    pub(crate) fn remove_image(&mut self, id: ImageId) {
        if let Some(img) = self.images.remove(&id) {
            if let Some(frames) = self.animation_frames.remove(&id) {
                let total: usize = frames.iter().map(|f| f.len()).sum();
                self.memory_used = self.memory_used.saturating_sub(total);
            } else {
                self.memory_used = self.memory_used.saturating_sub(img.data.len());
            }

            self.placements.retain(|p| p.image_id != id);
            self.animations.remove(&id);
            self.frame_starts.remove(&id);
            self.store_order.remove(&id);
            self.placeholder_anchors.remove(&id);
            self.placeholder_anchor_grid.remove(&id);
            self.dirty = true;
        }
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

    /// Remove all images and placements.
    pub(crate) fn clear(&mut self) {
        if !self.images.is_empty() || !self.placements.is_empty() {
            self.dirty = true;
        }
        self.images.clear();
        self.placements.clear();
        self.animations.clear();
        self.animation_frames.clear();
        self.frame_starts.clear();
        self.store_order.clear();
        self.placeholder_anchors.clear();
        // Drop placement-grid entries alongside anchors — keeps the two
        // maps in sync at cache reset (e.g., `\x1b_Gd=A` deletes all
        // images; switching primary/alt screens with full reset).
        self.placeholder_anchor_grid.clear();
        self.memory_used = 0;
    }

    /// Get image data by ID, updating access counter for LRU.
    #[cfg(test)]
    pub(crate) fn get(&mut self, id: ImageId) -> Option<&ImageData> {
        self.access_counter += 1;
        let counter = self.access_counter;
        if let Some(img) = self.images.get_mut(&id) {
            img.last_accessed = counter;
            Some(img)
        } else {
            None
        }
    }

    /// Get image data by ID without updating access counter.
    pub fn get_no_touch(&self, id: ImageId) -> Option<&ImageData> {
        self.images.get(&id)
    }

    /// Test-only probe for `frame_starts[id]` — returns the start instant
    /// if one has been seeded by `advance_animations`, else `None`.
    #[cfg(test)]
    pub(crate) fn frame_starts_for_test(&self, id: ImageId) -> Option<Instant> {
        self.frame_starts.get(&id).copied()
    }
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for ImageCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageCache")
            .field("images", &self.images.len())
            .field("placements", &self.placements.len())
            .field("memory_used", &self.memory_used)
            .field("memory_limit", &self.memory_limit)
            .field("max_single_image_bytes", &self.max_single_image_bytes)
            .field("next_id", &self.next_id)
            .field("access_counter", &self.access_counter)
            .field("dirty", &self.dirty)
            .field("store_order", &self.store_order.len())
            .field("next_store_order", &self.next_store_order)
            .field("animations", &self.animations.len())
            .field("animation_frames", &self.animation_frames.len())
            .field("frame_starts", &self.frame_starts.len())
            .field("animation_enabled", &self.animation_enabled)
            .field("placeholder_anchors", &self.placeholder_anchors.len())
            .field(
                "placeholder_anchor_grid",
                &self.placeholder_anchor_grid.len(),
            )
            .finish()
    }
}

#[cfg(test)]
mod matrix_tests;
#[cfg(test)]
mod tests;
