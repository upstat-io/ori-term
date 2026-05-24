//! Shared write-back state machine for frame-pixel replacement.
//!
//! `replace_frame_bytes` is invoked from TWO arms:
//! 1. Kitty `a=f,r=N` edit path — `super::edit::ImageCache::put_frame_edit`
//!    wraps `Vec<u8>` → `Arc<Vec<u8>>` and delegates here.
//! 2. Kitty `a=c` compose path — `super::super::compose::ImageCache::compose_frame`
//!    holds an `Arc<Vec<u8>>` already and calls here directly.
//!
//! Owning the dispatch (static-root / promoted-non-displayed /
//! promoted-displayed) in one place keeps the kitty-parity invariants
//! (`pixel_generation` bump, displayed-frame sync-mirror update, root-gap
//! auto-promote) from drifting across the edit + compose callers.

use std::sync::Arc;
use std::time::Duration;

use super::super::super::{ImageError, ImageId};
use super::super::frame_entry::FrameEntry;
use super::ImageCache;

impl ImageCache {
    /// Replace `frame_num`'s pixel bytes, handling static-root /
    /// promoted-non-displayed / promoted-displayed dispatch.
    ///
    /// Shared between [`Self::put_frame`] edit arm (`a=f,r=N`) and
    /// [`Self::compose_frame`] (`a=c`). Callers with `Vec<u8>` go through
    /// `put_frame_edit` (in `super::edit`) which `Arc::new`s for them;
    /// callers holding `Arc<Vec<u8>>` directly call here.
    ///
    /// Optional `gap_update` for callers that also accept `z=` gap (the
    /// `a=f` edit arm); compose callers pass `None`. Returns the 1-based
    /// `frame_num` on success.
    ///
    /// Static-image root edit (`frame_num == 1 && !promoted`) AND
    /// `gap_update.is_some()` auto-promotes the image via
    /// [`Self::ensure_animation_state_for_root_gap`] so the root gap
    /// survives. Compose never triggers this path because its
    /// `gap_update` is always `None`; preserved for `put_frame_edit`
    /// parity.
    pub(crate) fn replace_frame_bytes(
        &mut self,
        id: ImageId,
        frame_num: u32,
        composed: Arc<Vec<u8>>,
        gap_update: Option<Duration>,
    ) -> Result<u32, ImageError> {
        let promoted = self.animations.contains_key(&id);

        if frame_num == 1 && !promoted {
            // Static-image root edit. Bump `pixel_generation` so the GPU
            // upload path's re-upload gate (which compares last-uploaded
            // generation to the current `pixel_generation`) re-uploads
            // the texture; without the bump the renderer would keep the
            // stale pre-edit texture even though the underlying bytes
            // changed.
            if let Some(img) = self.images.get_mut(&id) {
                img.data = composed.clone();
                img.pixel_generation = img.pixel_generation.wrapping_add(1);
            }
            if gap_update.is_some() {
                // Auto-promote to record root gap; ensures gap survives.
                self.ensure_animation_state_for_root_gap(id, gap_update)?;
                let root_id = self.alloc_frame_id();
                if let Some(frames) = self.animation_frames.get_mut(&id) {
                    frames[0] = FrameEntry::Materialized {
                        id: root_id,
                        data: composed,
                    };
                }
            }
            self.dirty = true;
            return Ok(1);
        }

        // Promoted: write to animation_frames[id][frame_num - 1].
        let new_frame_id = self.alloc_frame_id();
        let displayed = if let Some(frames) = self.animation_frames.get_mut(&id) {
            let idx = (frame_num - 1) as usize;
            if idx >= frames.len() {
                return Err(ImageError::InvalidFrameRef {
                    requested: frame_num,
                    total: frames.len() as u32,
                });
            }
            frames[idx] = FrameEntry::Materialized {
                id: new_frame_id,
                data: composed.clone(),
            };
            let state = self
                .animations
                .get(&id)
                .expect("promoted: animations entry present");
            state.current_frame == idx
        } else {
            return Err(ImageError::InvalidFrameRef {
                requested: frame_num,
                total: 0,
            });
        };

        if displayed {
            if let Some(img) = self.images.get_mut(&id) {
                img.data = composed;
            }
        }

        if let (Some(gap), Some(state)) = (gap_update, self.animations.get_mut(&id)) {
            let idx = (frame_num - 1) as usize;
            if idx < state.frame_durations.len() {
                state.frame_durations[idx] = gap;
            }
        }

        self.dirty = true;
        Ok(frame_num)
    }
}
