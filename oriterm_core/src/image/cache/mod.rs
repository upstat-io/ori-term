//! Image cache with LRU eviction and configurable memory limits.

mod animation;
mod compose;
mod deletion;
mod eviction;
mod frame_entry;
mod frame_loading;
mod lifecycle;
mod placeholder;
mod placements;
#[cfg(any(test, feature = "test-support"))]
mod test_probes;

pub(crate) use frame_entry::{FrameEntry, FrameId};

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::sync::Arc;
use std::time::Instant;

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
    /// All stored frames for animated images (frame 0 is also in `ImageData.data`).
    ///
    /// Element type is [`FrameEntry`] — `Materialized` carries the full RGBA
    /// canvas; `Delta` carries the sub-rect payload + stable `FrameId`
    /// back-reference, composed on demand via [`Self::frame_for_number`].
    /// Vec position is the 1-based kitty protocol frame index;
    /// `FrameEntry::id()` is the stable internal reference used by
    /// `Delta.base`.
    pub(super) animation_frames: HashMap<ImageId, Vec<FrameEntry>>,
    /// Monotonic `FrameId` allocator for stable internal references.
    ///
    /// Bumped by [`alloc_frame_id`] at every `FrameEntry` creation site.
    /// Never wrapped; `u64` width matches `ImageData::pixel_generation`.
    pub(super) next_frame_id: u64,
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
            next_frame_id: 1,
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

    /// Total bytes of decoded image data in the cache. Read by the IO thread's
    /// drain-cycle diagnostic accounting (see
    /// `oriterm_mux/src/pane/io_thread/mod.rs::process_pending_bytes`).
    pub fn memory_used(&self) -> usize {
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

    /// Allocate the next monotonically-increasing `FrameId`.
    ///
    /// Single SSOT for `FrameId` assignment — every `FrameEntry` creation
    /// path (`push_composed_frame`, `store_animated`, root-promote in
    /// `ensure_animation_state_for_root_gap`, `replace_frame_bytes`'s
    /// auto-promote branch) must route through here so the
    /// `debug_assert!(base.0 < new.0)` cycle invariant on `Delta.base`
    /// holds by construction.
    pub(crate) fn alloc_frame_id(&mut self) -> FrameId {
        let id = FrameId(self.next_frame_id);
        self.next_frame_id = self.next_frame_id.saturating_add(1);
        id
    }

    /// Cloned 1-based protocol-indexed frame entry, or `None` if `n == 0`
    /// or out of range. Used by storage-layer tests; production reads
    /// route through `frame_for_number`.
    #[cfg(test)]
    pub(crate) fn frame_entry_at(&self, id: ImageId, n: u32) -> Option<FrameEntry> {
        if n == 0 {
            return None;
        }
        let frames = self.animation_frames.get(&id)?;
        frames.get((n - 1) as usize).cloned()
    }

    /// Atomically swap an image's displayed RGBA bytes and bump its
    /// `pixel_generation` counter so the GPU re-upload gate fires.
    ///
    /// Test-only helper. Production routing uses `apply_frame` (in
    /// `animation.rs`) which mutates `images[id].data` + bumps
    /// `pixel_generation` inline. Bumps use `wrapping_add(1)` to avoid
    /// debug-build overflow panics; the GPU cache's wrap test
    /// (`pixel_generation_full_wrap_to_seeded_value_forces_reupload`)
    /// pins the wrap-vs-stale behavior.
    #[cfg(test)]
    pub(crate) fn set_image_data_and_bump_generation(
        &mut self,
        id: ImageId,
        new_data: Arc<Vec<u8>>,
    ) {
        if let Some(img) = self.images.get_mut(&id) {
            img.data = new_data;
            img.pixel_generation = img.pixel_generation.wrapping_add(1);
        }
        self.dirty = true;
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
                let total: usize = frames.iter().map(FrameEntry::memory_bytes).sum();
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
            .field("next_frame_id", &self.next_frame_id)
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
mod delta_storage_tests;
#[cfg(test)]
mod matrix_tests;
#[cfg(test)]
mod tests;
