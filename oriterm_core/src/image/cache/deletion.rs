//! Kitty graphics `a=d` delete-dispatch helpers for `ImageCache`.
//!
//! Each per-specifier primitive returns the affected image IDs so callers
//! (`kitty_delete` in `term/handler/image/kitty/delete/mod.rs`) can decide
//! whether to additionally prune orphaned image data for the uppercase
//! variant via `prune_if_orphaned`.

use std::sync::Arc;

use super::super::{ImageId, ImagePlacement};
use super::ImageCache;
use crate::grid::StableRowIndex;

impl ImageCache {
    /// Remove placements that intersect the cell at `(col, row)`.
    ///
    /// Per kitty `a=d,d=c/C/p/P` — "Delete all placements that intersect
    /// with the current cursor position / a specific cell"
    /// (graphics-protocol.rst §Deleting images, lines 762 and 764).
    /// A multi-cell placement spanning `(cell_col..cell_col+cols,
    /// cell_row..cell_row+rows)` matches if `(col, row)` lies inside
    /// that rectangle.
    pub(crate) fn remove_placements_intersecting_cell(
        &mut self,
        col: usize,
        row: StableRowIndex,
    ) -> Vec<ImageId> {
        self.remove_placements_where(|p| placement_intersects_cell(p, col, row))
    }

    /// Remove placements that intersect `(col, row)` AND carry z-index `z`.
    ///
    /// Per kitty `a=d,d=q` / `d=Q` — cell intersection plus exact z-index match
    /// (graphics-protocol.rst line 765).
    pub(crate) fn remove_placements_at_cell_with_z(
        &mut self,
        col: usize,
        row: StableRowIndex,
        z: i32,
    ) -> Vec<ImageId> {
        self.remove_placements_where(|p| placement_intersects_cell(p, col, row) && p.z_index == z)
    }

    /// Remove placements visible in the given stable row range (inclusive).
    ///
    /// Per kitty `a=d,d=a` / `d=A` — "delete all placements visible on screen".
    pub(crate) fn remove_visible_placements(
        &mut self,
        viewport_top: StableRowIndex,
        viewport_bottom: StableRowIndex,
    ) -> Vec<ImageId> {
        self.remove_placements_where(|p| p.intersects_viewport(viewport_top, viewport_bottom))
    }

    /// Remove placements of images whose id is in the inclusive range `[lo, hi]`.
    ///
    /// Per kitty `a=d,d=r` / `d=R` — delete images by id range (kitty 0.33.0+).
    pub(crate) fn remove_placements_in_id_range(
        &mut self,
        lo: ImageId,
        hi: ImageId,
    ) -> Vec<ImageId> {
        self.remove_placements_where(|p| p.image_id >= lo && p.image_id <= hi)
    }

    /// Resolve the newest image with the given client-supplied `I=` number.
    ///
    /// "Newest" = highest `store_order` (monotonic creation rank) among
    /// images matching `image_number`. Does NOT use `last_accessed` —
    /// that tracks LRU recency, which would pick the last-rendered image
    /// instead of the last-created one. Matches kitty `graphics.c`
    /// `img_by_client_number`, which resolves by creation order.
    /// Returns `None` if no image has that number.
    pub(crate) fn newest_by_image_number(&self, number: u32) -> Option<ImageId> {
        self.images
            .values()
            .filter(|img| img.image_number == Some(number))
            .filter_map(|img| self.store_order.get(&img.id).map(|&o| (img.id, o)))
            .max_by_key(|(_, order)| *order)
            .map(|(id, _)| id)
    }

    /// Whether the image has animation frames beyond the root frame.
    ///
    /// Static images return `false`; animated images with 2+ stored frames
    /// return `true`. Used by `d=f/F` to decide static-vs-animated handling.
    pub(crate) fn has_extra_animation_frames(&self, id: ImageId) -> bool {
        self.animation_frames.get(&id).is_some_and(|f| f.len() > 1)
    }

    /// Delete a specific animation frame (1-based per kitty convention).
    ///
    /// Frame 1 = root (the base image); frames 2..N = extras. After
    /// removal, `ImageData.data` is re-synced to the (adjusted)
    /// `current_frame`'s bytes and the per-image `frame_starts` timer is
    /// cleared so `advance_animations` re-initializes timing.
    ///
    /// Returns `true` if the image is left with no extra frames (caller
    /// uses this to decide image-data pruning on `d=F`).
    pub(crate) fn remove_animation_frame(&mut self, id: ImageId, frame_number: u32) -> bool {
        // Phase 1 (immutable scan): identify the target frame's stable
        // FrameId + collect indices of any Δ dependents whose `base`
        // matches it. Drop the borrow before materializing.
        let (idx, target_fid, dependent_indices) = {
            let Some(frames) = self.animation_frames.get(&id) else {
                return true; // static image
            };
            if frames.len() <= 1 {
                let _ = frames;
                self.animations.remove(&id);
                self.animation_frames.remove(&id);
                self.frame_starts.remove(&id);
                return true;
            }
            let total = frames.len() as u32;
            let mut requested = if frame_number == 0 { 1 } else { frame_number };
            requested = requested.min(total);
            let idx = (requested - 1) as usize;
            let target_fid = frames[idx].id();
            let dependents: Vec<usize> = frames
                .iter()
                .enumerate()
                .filter_map(|(i, entry)| {
                    if i == idx {
                        return None;
                    }
                    match entry {
                        super::frame_entry::FrameEntry::Delta { base, .. }
                            if *base == target_fid =>
                        {
                            Some(i)
                        }
                        _ => None,
                    }
                })
                .collect();
            (idx, target_fid, dependents)
        };

        let materialized = self.materialize_delta_dependents(id, &dependent_indices);

        // Phase 3 (mutable replace + remove): re-acquire the mut borrow
        // and apply cascade results, then delete the target frame.
        let _ = target_fid; // captured for assertions only
        let Some(frames) = self.animation_frames.get_mut(&id) else {
            return true;
        };

        // Account for memory inflation: each Δ→Mat replacement increases
        // the in-cache footprint from `payload.len()` to the full canvas
        // size. Reuse the existing FrameId so back-references would still
        // resolve to the same entry if anything held a stale FrameId
        // reference (defensive).
        for (dep_idx, bytes) in materialized {
            let fid = frames[dep_idx].id();
            let old_bytes = frames[dep_idx].memory_bytes();
            let new_bytes = bytes.len();
            frames[dep_idx] = super::frame_entry::FrameEntry::Materialized {
                id: fid,
                data: bytes,
            };
            if new_bytes > old_bytes {
                self.memory_used = self.memory_used.saturating_add(new_bytes - old_bytes);
            } else {
                self.memory_used = self.memory_used.saturating_sub(old_bytes - new_bytes);
            }
        }

        // Re-borrow after cascade for the actual removal.
        let frames = self
            .animation_frames
            .get_mut(&id)
            .expect("present after cascade phase");

        let removed = frames.remove(idx);
        self.memory_used = self.memory_used.saturating_sub(removed.memory_bytes());

        if let Some(state) = self.animations.get_mut(&id) {
            // Adjust `current_frame` based on its position relative to the
            // removed frame BEFORE applying size changes. A removed frame
            // strictly before the current one shifts current down by 1;
            // removing the current frame or one after it leaves the index
            // pointing at the next frame, clamped below.
            if idx < state.current_frame {
                state.current_frame -= 1;
            }
            if idx < state.frame_durations.len() {
                state.frame_durations.remove(idx);
            }
            state.total_frames = frames.len();
            if state.current_frame >= state.total_frames && state.total_frames > 0 {
                state.current_frame = state.total_frames - 1;
            }
        }

        // Sync `ImageData.data` to the CURRENT frame (post-adjustment), not
        // just the new root. If we removed frame 1 (idx=0) while frame 3 was
        // current, pre-removal current_frame=2 → post-adjustment=1, and the
        // displayed bytes must match `frames[1]` (old frame 3), not frames[0].
        let current_frame = self.animations.get(&id).map_or(0, |s| s.current_frame);
        if let Some(img) = self.images.get_mut(&id)
            && let Some(super::frame_entry::FrameEntry::Materialized { data, .. }) =
                frames.get(current_frame)
        {
            img.data = data.clone();
            // pixel_generation bump so the GPU re-uploads on the next render —
            // without this the cache continues serving the deleted frame's pixels.
            img.pixel_generation = img.pixel_generation.wrapping_add(1);
        }

        // Reset the frame-start timer so `advance_animations` re-initializes
        // timing for the new current frame. Without this, the old start time
        // would be compared against the new current frame's duration, causing
        // premature or delayed frame switches.
        self.frame_starts.remove(&id);

        let now_static = frames.len() <= 1;
        if now_static {
            self.animations.remove(&id);
            self.animation_frames.remove(&id);
            self.frame_starts.remove(&id);
        }
        self.dirty = true;
        now_static
    }

    /// Compose-on-demand for each Δ dependent of a soon-to-be-deleted frame.
    /// Returns `(dep_idx, materialized_bytes)` pairs ready for caller-side
    /// substitution + memory accounting. Holds only `&self` while composing
    /// so each materialization can recurse through other Δ entries cleanly.
    fn materialize_delta_dependents(
        &self,
        id: ImageId,
        dependent_indices: &[usize],
    ) -> Vec<(usize, Arc<Vec<u8>>)> {
        let mut materialized: Vec<(usize, Arc<Vec<u8>>)> =
            Vec::with_capacity(dependent_indices.len());
        for &dep_idx in dependent_indices {
            let entry = {
                let frames = self.animation_frames.get(&id).expect("present per phase 1");
                frames[dep_idx].clone()
            };
            let Some(bytes) = self.compose_entry(id, &entry) else {
                continue;
            };
            materialized.push((dep_idx, bytes));
        }
        materialized
    }
}

/// Whether a multi-cell placement rectangle contains the cell `(col, row)`.
///
/// Placement covers columns `cell_col..cell_col+cols` and stable rows
/// `cell_row..cell_row+rows`. Zero-width or zero-height placements occupy
/// no cells and therefore intersect nothing — return `false` without
/// applying a `saturating_sub(1)` origin-cell fallback (which would
/// falsely report a hit at the origin).
fn placement_intersects_cell(p: &ImagePlacement, col: usize, row: StableRowIndex) -> bool {
    if p.cols == 0 || p.rows == 0 {
        return false;
    }
    let right = p.cell_col + p.cols - 1;
    let bottom = StableRowIndex(p.cell_row.0 + (p.rows - 1) as u64);
    p.cell_col <= col && right >= col && p.cell_row <= row && bottom >= row
}
