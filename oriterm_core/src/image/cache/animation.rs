//! Animation support for the image cache.
//!
//! Timer-driven frame switching for animated images (GIF multi-frame,
//! Kitty `a=animate`). Only images visible in the viewport are animated
//! to save CPU/GPU.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::grid::StableRowIndex;

use super::super::{AnimationState, CompositionMode, ImageData, ImageError, ImageId};
use super::ImageCache;

/// Apply a specific frame's data to the image, setting `dirty`.
///
/// Takes disjoint field references so callers can hold a simultaneous
/// borrow on `animations` (the borrow checker can't see inside `&mut
/// self` method calls).
fn apply_frame(
    animation_frames: &HashMap<ImageId, Vec<Arc<Vec<u8>>>>,
    images: &mut HashMap<ImageId, ImageData>,
    dirty: &mut bool,
    id: ImageId,
    frame_idx: usize,
) {
    let Some(frames) = animation_frames.get(&id) else {
        return;
    };
    let Some(img) = images.get_mut(&id) else {
        return;
    };
    let idx = frame_idx.min(frames.len() - 1);
    img.data = frames[idx].clone();
    *dirty = true;
}

impl ImageCache {
    /// Enable or disable animation. When disabled, animated images show
    /// frame 0 only.
    pub(crate) fn set_animation_enabled(&mut self, enabled: bool) {
        if self.animation_enabled == enabled {
            return;
        }
        self.animation_enabled = enabled;
        if !enabled {
            // Reset all animation states to frame 0 and collect IDs
            // that need their image data updated.
            for (id, state) in &mut self.animations {
                if state.current_frame != 0 {
                    state.current_frame = 0;
                    state.loops_completed = 0;
                    apply_frame(
                        &self.animation_frames,
                        &mut self.images,
                        &mut self.dirty,
                        *id,
                        0,
                    );
                }
            }
        }
    }

    /// Store an animated image with multiple frames.
    ///
    /// `frames[0]` becomes the initial `ImageData.data`. All frames are
    /// retained in `animation_frames` for timer-driven switching.
    pub(crate) fn store_animated(
        &mut self,
        mut data: ImageData,
        frames: Vec<Arc<Vec<u8>>>,
        durations: Vec<Duration>,
        loop_count: Option<u32>,
    ) -> Result<ImageId, ImageError> {
        if frames.is_empty() {
            return self.store(data);
        }

        // Set initial data to first frame.
        data.data = frames[0].clone();

        // Calculate total memory for all frames.
        let total_size: usize = frames.iter().map(|f| f.len()).sum();
        if total_size > self.max_single_image_bytes {
            return Err(ImageError::OversizedImage);
        }

        let id = self.store(data)?;

        // Account for frames 1..N in memory tracking. Frame 0 is already
        // counted by `store()` (it's the initial `data.data`).
        let extra_frame_bytes: usize = frames.iter().skip(1).map(|f| f.len()).sum();
        self.memory_used += extra_frame_bytes;

        let state = AnimationState::new(durations, loop_count);
        self.animations.insert(id, state);
        self.animation_frames.insert(id, frames);

        Ok(id)
    }

    /// Advance all active animations. Returns the next frame deadline.
    ///
    /// Only advances images that have placements in the given viewport.
    /// Call once per frame before `renderable_content_into()`.
    pub(crate) fn advance_animations(
        &mut self,
        now: Instant,
        viewport_top: StableRowIndex,
        viewport_bottom: StableRowIndex,
    ) -> Option<Instant> {
        if !self.animation_enabled || self.animations.is_empty() {
            return None;
        }

        // Collect animated image IDs visible in viewport.
        let visible_ids: Vec<ImageId> = self
            .animations
            .keys()
            .filter(|id| {
                self.placements.iter().any(|p| {
                    p.image_id == **id && p.intersects_viewport(viewport_top, viewport_bottom)
                })
            })
            .copied()
            .collect();

        let mut next_deadline: Option<Instant> = None;

        for id in visible_ids {
            let Some(state) = self.animations.get_mut(&id) else {
                continue;
            };
            if state.paused || state.is_finished() || state.total_frames <= 1 {
                continue;
            }

            // Initialize frame start if first time.
            let frame_start = *self.frame_starts.entry(id).or_insert(now);
            let elapsed = now.duration_since(frame_start);
            let frame_dur = state.current_duration();

            if elapsed >= frame_dur && state.advance() {
                let frame = state.current_frame;
                apply_frame(
                    &self.animation_frames,
                    &mut self.images,
                    &mut self.dirty,
                    id,
                    frame,
                );
                self.frame_starts.insert(id, now);
            }

            // Compute deadline for this image's next frame switch.
            if !state.is_finished() {
                let start = *self.frame_starts.get(&id).unwrap_or(&now);
                let dur = state.current_duration();
                let deadline = start + dur;
                next_deadline = Some(match next_deadline {
                    Some(d) => d.min(deadline),
                    None => deadline,
                });
            }
        }

        next_deadline
    }

    /// Whether this cache has any animated images.
    #[cfg(test)]
    pub(crate) fn has_animations(&self) -> bool {
        !self.animations.is_empty()
    }

    /// Get the animation state for an image (if animated).
    ///
    /// Public so integration tests under `oriterm_core/tests/spec_chain/`
    /// can observe `current_frame`, `loop_count`, and `frame_durations`
    /// after `a=a` state mutations — the kitty animate-reply verification
    /// chain needs a read-only window into `AnimationState`.
    pub fn animation_state(&self, id: ImageId) -> Option<&AnimationState> {
        self.animations.get(&id)
    }

    /// Add an animation frame to an existing image (Kitty `a=f`).
    ///
    /// If the image is not yet animated, promotes it: the existing data
    /// becomes frame 1 with a default gap, and the new data becomes frame 2.
    /// `gap` is the duration before displaying this frame.
    /// `composition_mode` controls how the frame is built from existing data.
    ///
    /// Returns the 1-based frame index of the newly added frame — kitty
    /// `finish_command_response` (`graphics.c:802-806`) echoes this via the
    /// `,r=<frame_num>` qualifier on the `a=f` OK reply so the client can
    /// correlate the mutation to a specific frame.
    pub(crate) fn add_animation_frame(
        &mut self,
        id: ImageId,
        frame_data: Arc<Vec<u8>>,
        gap: Duration,
        composition_mode: CompositionMode,
    ) -> Result<u32, ImageError> {
        let img = self.images.get(&id).ok_or(ImageError::InvalidFormat)?;
        let img_data = img.data.clone();

        // Check total memory with new frame.
        if frame_data.len() > self.max_single_image_bytes {
            return Err(ImageError::OversizedImage);
        }

        let added_index_1based = match self.animations.entry(id) {
            Entry::Vacant(e) => {
                // Promote static image to animated: frame 0 = existing data.
                // Frame 0 (img_data) is already counted in memory_used from
                // the original store(). Only count the new frame.
                self.memory_used += frame_data.len();
                let frames = vec![img_data, frame_data];
                let durations = vec![gap, gap];
                e.insert(AnimationState::new(durations, None));
                self.animation_frames.insert(id, frames);
                // The promotion pushes frame 0 (pre-existing) + frame 1 (new).
                // kitty's 1-based index for the new frame is 2.
                2
            }
            Entry::Occupied(mut e) => {
                let anim_frames = self.animation_frames.entry(id).or_default();

                // Apply composition mode for the new frame.
                let composed = match composition_mode {
                    CompositionMode::Overwrite => frame_data,
                    CompositionMode::AlphaBlend => {
                        if let Some(prev) = anim_frames.last() {
                            Arc::new(alpha_blend_frames(prev, &frame_data))
                        } else {
                            frame_data
                        }
                    }
                };

                self.memory_used += composed.len();
                anim_frames.push(composed);

                let state = e.get_mut();
                state.frame_durations.push(gap);
                state.total_frames = anim_frames.len();
                // Newly-pushed frame is at 0-based index `len - 1`; kitty
                // reports it 1-based.
                anim_frames.len() as u32
            }
        };

        self.dirty = true;
        Ok(added_index_1based)
    }

    /// Set animation playback state (Kitty `a=a`, `s=` key).
    ///
    /// `action`: 1=stop, 2=run (wait), 3=run.
    pub(crate) fn set_animation_action(&mut self, id: ImageId, action: u32) {
        if let Some(state) = self.animations.get_mut(&id) {
            match action {
                1 => state.paused = true,
                2 | 3 => {
                    state.paused = false;
                    state.loops_completed = 0;
                    self.frame_starts.insert(id, Instant::now());
                }
                _ => {}
            }
            self.dirty = true;
        }
    }

    /// Set the loop count for an animated image (Kitty `v=` key).
    pub(crate) fn set_animation_loops(&mut self, id: ImageId, loops: u32) {
        if let Some(state) = self.animations.get_mut(&id) {
            state.loop_count = if loops == 0 { None } else { Some(loops) };
            state.loops_completed = 0;
        }
    }

    /// Set the gap (frame duration) for a specific frame.
    pub(crate) fn set_frame_gap(&mut self, id: ImageId, frame_idx: usize, gap: Duration) {
        if let Some(state) = self.animations.get_mut(&id) {
            if frame_idx < state.frame_durations.len() {
                state.frame_durations[frame_idx] = gap;
            }
        }
    }

    /// Jump to a specific frame (Kitty `r=` or `c=` in `a=a`).
    pub(crate) fn set_current_frame(&mut self, id: ImageId, frame_idx: usize) {
        let should_apply = self.animations.get_mut(&id).is_some_and(|state| {
            if frame_idx < state.total_frames {
                state.current_frame = frame_idx;
                true
            } else {
                false
            }
        });
        if should_apply {
            apply_frame(
                &self.animation_frames,
                &mut self.images,
                &mut self.dirty,
                id,
                frame_idx,
            );
            self.frame_starts.insert(id, Instant::now());
        }
    }
}

/// Alpha-blend `src` over `dst` (both RGBA, same length).
fn alpha_blend_frames(dst: &[u8], src: &[u8]) -> Vec<u8> {
    let len = dst.len().min(src.len());
    let mut out = dst[..len].to_vec();

    for i in (0..len).step_by(4) {
        if i + 3 >= len {
            break;
        }
        let sa = src[i + 3] as u32;
        if sa == 0 {
            continue;
        }
        if sa == 255 {
            out[i] = src[i];
            out[i + 1] = src[i + 1];
            out[i + 2] = src[i + 2];
            out[i + 3] = 255;
            continue;
        }
        let da = out[i + 3] as u32;
        let inv_sa = 255 - sa;
        // Porter-Duff "source over" blend.
        let oa = sa + (da * inv_sa) / 255;
        if oa == 0 {
            continue;
        }
        out[i] = ((src[i] as u32 * sa + out[i] as u32 * da * inv_sa / 255) / oa) as u8;
        out[i + 1] = ((src[i + 1] as u32 * sa + out[i + 1] as u32 * da * inv_sa / 255) / oa) as u8;
        out[i + 2] = ((src[i + 2] as u32 * sa + out[i + 2] as u32 * da * inv_sa / 255) / oa) as u8;
        out[i + 3] = oa.min(255) as u8;
    }

    out
}
