//! Public test probes for spec-chain integration tests.
//!
//! Integration tests under `term_repo/oriterm_core/tests/spec_chain/` compile
//! as an external crate; they cannot reach `pub(crate)` items, and
//! `#[cfg(test)]` items are not in scope for integration test binaries.
//! These `#[doc(hidden)] pub fn` probes mirror the existing
//! [`super::ImageCache::animation_snapshot`] accessibility pattern — they
//! exist to let spec-chain tests pin byte-exact and structural invariants
//! without forcing the rest of the cache surface to widen its visibility.
//!
//! **NOT a stable production API.** Consumers outside the test tree MUST
//! use the narrow `animation_snapshot` / `animation_frame_gaps` accessors.

use std::sync::Arc;

use super::super::{ImageId, ImagePlacement};
use super::ImageCache;

impl ImageCache {
    /// Snapshot of every active placement, cloned for test inspection.
    ///
    /// Spec_chain tests use this to pin per-protocol placement creation
    /// (e.g. iterm2 §14.1 asserts `OSC 1337;File=` produces a placement
    /// at the cursor row). Production code reads placements via the
    /// `pub(crate)` `viewport_placements` zero-allocation path; this
    /// probe exists so external integration tests can assert without
    /// widening the production surface.
    #[doc(hidden)]
    pub fn placements_for_test(&self) -> Vec<ImagePlacement> {
        self.placements.clone()
    }

    /// Pixel width of the stored image, or `None` if `id` is absent.
    #[doc(hidden)]
    pub fn image_width_for_test(&self, id: ImageId) -> Option<u32> {
        self.images.get(&id).map(|img| img.width)
    }

    /// Pixel height of the stored image, or `None` if `id` is absent.
    #[doc(hidden)]
    pub fn image_height_for_test(&self, id: ImageId) -> Option<u32> {
        self.images.get(&id).map(|img| img.height)
    }

    /// Decoded format of the stored image, or `None` if `id` is absent.
    /// Single-frame protocol images store as [`super::super::ImageFormat::Rgba`]
    /// after decode; spec_chain tests pin this invariant per protocol arm.
    #[doc(hidden)]
    pub fn image_format_for_test(&self, id: ImageId) -> Option<super::super::ImageFormat> {
        self.images.get(&id).map(|img| img.format)
    }

    /// Byte-exact pixel data for the 1-based frame `frame_num`.
    ///
    /// Delegates to [`Self::frame_for_number`]; spec-chain tests use this
    /// to assert the canvas + payload composite. Returns `None` for
    /// `frame_num == 0`, out-of-range indices, or missing images.
    #[doc(hidden)]
    pub fn frame_bytes_for_test(&self, id: ImageId, frame_num: u32) -> Option<Arc<Vec<u8>>> {
        self.frame_for_number(id, frame_num)
    }

    /// Total frames stored for `id` (1-based; static images return 1).
    ///
    /// Wraps [`Self::animation_total_frames`]; tests use this instead of
    /// reaching into the private `animation_frames` map.
    #[doc(hidden)]
    pub fn total_frames_for_test(&self, id: ImageId) -> u32 {
        self.animation_total_frames(id)
    }

    /// True iff `id` has been promoted to an animated image (i.e. has
    /// an `AnimationState` entry).
    #[doc(hidden)]
    pub fn animation_promoted_for_test(&self, id: ImageId) -> bool {
        self.animations.contains_key(&id)
    }

    /// Bytes currently held in `images[id].data` (the display surface
    /// passed to the GPU layer). Differs from `frame_bytes_for_test(id,
    /// current_frame + 1)` for promoted animations after a `put_frame`
    /// edit that does NOT touch the displayed slot.
    #[doc(hidden)]
    pub fn image_data_bytes_for_test(&self, id: ImageId) -> Option<Arc<Vec<u8>>> {
        self.images.get(&id).map(|img| img.data.clone())
    }

    /// True iff the cache currently holds an image with `id`.
    #[doc(hidden)]
    pub fn contains_image_for_test(&self, id: ImageId) -> bool {
        self.images.contains_key(&id)
    }
}
