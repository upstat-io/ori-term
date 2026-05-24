//! Edit-arm methods for the kitty `a=f,r=N` frame-load pipeline (`ImageCache`).
//!
//! Owns: `try_edit_in_place` (`Arc::make_mut` fast path),
//! `put_frame_edit` (thin Vec → Arc wrapper that delegates to
//! [`super::replace`]'s `replace_frame_bytes` via inherent method
//! dispatch), and `ensure_animation_state_for_root_gap` (the root-gap
//! auto-promote helper shared with `term/handler/image/kitty/animate.rs`).

use std::sync::Arc;
use std::time::Duration;

use super::super::super::{AnimationState, FrameTarget, ImageError, ImageId};
use super::super::frame_entry::FrameEntry;
use super::BlitOp;
use super::ImageCache;
use super::blit_subrect_into_canvas;

impl ImageCache {
    /// Ensure the root frame's gap is recorded.
    ///
    /// Used by both `a=f,r=1,z!=0` (frame.rs) and `a=a,r=1,z!=0`
    /// (animate.rs) on still-static images: kitty stores the root gap
    /// on `Frame.gap` (graphics.c) but `ori_term`'s `ImageData` has no
    /// equivalent. Path: auto-promote to a 1-frame animation whose
    /// `frame_durations[0]` is the requested gap.
    ///
    /// - `gap_update == None` → no-op (kitty parity: z=0 means
    ///   unspecified, do not touch existing gap). Returns `Ok(())`.
    /// - `gap_update == Some(gap)` AND image is static: promote and
    ///   record `gap` as the root duration.
    /// - `gap_update == Some(gap)` AND image is already promoted:
    ///   overwrite `frame_durations[0]`.
    /// - Image missing: `Err(MissingImage)` so the dispatch reply path
    ///   can route to ENOENT per kitty graphics.c:2233-2235.
    pub(crate) fn ensure_animation_state_for_root_gap(
        &mut self,
        id: ImageId,
        gap_update: Option<Duration>,
    ) -> Result<(), ImageError> {
        let Some(gap) = gap_update else {
            return Ok(());
        };
        if !self.images.contains_key(&id) {
            return Err(ImageError::MissingImage { id: id.0 });
        }
        let needs_promote = !self.animations.contains_key(&id);
        if needs_promote {
            let root_bytes = self
                .images
                .get(&id)
                .map(|img| img.data.clone())
                .ok_or(ImageError::MissingImage { id: id.0 })?;
            let root_id = self.alloc_frame_id();
            self.animations
                .insert(id, AnimationState::new(vec![gap], None));
            self.animation_frames.insert(
                id,
                vec![FrameEntry::Materialized {
                    id: root_id,
                    data: root_bytes,
                }],
            );
        } else if let Some(state) = self.animations.get_mut(&id) {
            if !state.frame_durations.is_empty() {
                state.frame_durations[0] = gap;
            }
        } else {
            // Already promoted but no animation state — no gap to record.
        }
        Ok(())
    }

    /// In-place Edit fast path. Mutates `animation_frames[idx]`'s
    /// underlying `Vec<u8>` via `Arc::make_mut` to apply the blit without
    /// allocating a fresh canvas. Returns `None` when the entry isn't a
    /// `Materialized` frame (forces fallback to the clone-then-blit path
    /// in `put_frame`); returns `Some(Ok(frame_num))` on success.
    ///
    /// To make `Arc::make_mut` mutate in place rather than clone, the
    /// canonical sync-mirror in `images[id].data` is first detached. If
    /// the Arc is still shared (e.g., the IO thread published a snapshot
    /// whose buffer references this Arc), `make_mut` falls back to a
    /// clone; the caller still avoids the explicit `(*bytes).clone()`
    /// memcpy that the slow path performs unconditionally.
    pub(super) fn try_edit_in_place(
        &mut self,
        req: &crate::image::FrameLoadRequest,
        image_dims: (u32, u32),
    ) -> Option<Result<u32, ImageError>> {
        let FrameTarget::Edit {
            frame_num,
            gap_update,
        } = req.target
        else {
            return None;
        };
        let id = req.image_id;
        let idx = (frame_num as usize).checked_sub(1)?;
        let (image_w, image_h) = image_dims;

        let frames = self.animation_frames.get_mut(&id)?;
        let entry = frames.get_mut(idx)?;
        let FrameEntry::Materialized { data, .. } = entry else {
            return None;
        };

        // Detach `images[id].data` mirror so the Arc has one fewer
        // strong ref before `make_mut` decides whether to clone.
        let img_was_shared = self
            .images
            .get(&id)
            .is_some_and(|img| Arc::ptr_eq(&img.data, data));
        if img_was_shared && let Some(img) = self.images.get_mut(&id) {
            img.data = Arc::new(Vec::new());
        }

        // Re-borrow frames after touching self.images.
        let frames = self.animation_frames.get_mut(&id)?;
        let entry = frames.get_mut(idx)?;
        let FrameEntry::Materialized { data, .. } = entry else {
            return None;
        };

        let canvas = Arc::make_mut(data);
        let blit_op = BlitOp {
            canvas_w: image_w,
            canvas_h: image_h,
            blit: req.blit,
            mode: req.composition_mode,
        };
        if let Err(e) = blit_subrect_into_canvas(canvas, &req.frame_data, blit_op) {
            let restored = data.clone();
            if img_was_shared && let Some(img) = self.images.get_mut(&id) {
                img.data = restored;
            }
            return Some(Err(e));
        }

        let new_arc = data.clone();
        if img_was_shared && let Some(img) = self.images.get_mut(&id) {
            img.data = new_arc;
            img.pixel_generation = img.pixel_generation.wrapping_add(1);
        }
        if let Some(gap) = gap_update
            && let Some(state) = self.animations.get_mut(&id)
            && idx < state.frame_durations.len()
        {
            state.frame_durations[idx] = gap;
        }
        self.dirty = true;
        Some(Ok(frame_num))
    }

    /// Edit-arm body for `put_frame`: replace `frame_num`'s bytes with
    /// `composed`, optionally update its gap, handle the static-image
    /// root-edit auto-promote per kitty parity.
    ///
    /// Thin caller wrapping [`Self::replace_frame_bytes`] — `put_frame_edit`
    /// owns the `Vec<u8>` → `Arc<Vec<u8>>` conversion; the shared helper
    /// (in `replace.rs`) owns the static-root / promoted-non-displayed /
    /// promoted-displayed dispatch. Compose reuses the same dispatch via
    /// the shared helper.
    pub(super) fn put_frame_edit(
        &mut self,
        id: ImageId,
        frame_num: u32,
        composed: Vec<u8>,
        gap_update: Option<Duration>,
    ) -> Result<u32, ImageError> {
        self.replace_frame_bytes(id, frame_num, Arc::new(composed), gap_update)
    }
}
