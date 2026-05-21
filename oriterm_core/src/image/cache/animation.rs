//! Animation support for the image cache.
//!
//! Timer-driven frame switching for animated images (GIF multi-frame,
//! kitty graphics protocol `a=animate`). Only images visible in the
//! viewport are animated to save CPU/GPU.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::grid::StableRowIndex;

use super::super::{AnimationSnapshot, AnimationState, ImageData, ImageError, ImageId};
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
            // All-gapless guard — equivalent to kitty
            // `image_is_animatable` (graphics.c:1773-1775). When every
            // frame's gap is `Duration::ZERO`, computing a deadline yields
            // `now`, causing per-call churn. Skip the image entirely.
            if state.frame_durations.iter().all(|d| d.is_zero()) {
                continue;
            }

            // Initialize frame start if first time.
            let frame_start = *self.frame_starts.entry(id).or_insert(now);
            let elapsed = now.duration_since(frame_start);
            let frame_dur = state.current_duration();

            if elapsed >= frame_dur && state.advance_consuming_gapless() {
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
    /// `pub(crate)` — internal callers that need the full state use this.
    /// External consumers (other workspace crates + integration tests) MUST
    /// use the narrow [`Self::animation_snapshot`] +
    /// [`Self::animation_frame_gaps`] accessors below to decouple from the
    /// internal [`AnimationState`] field layout. (Demoted after migration
    /// of all external call sites — pre-migration this was `pub`.)
    pub(crate) fn animation_state(&self, id: ImageId) -> Option<&AnimationState> {
        self.animations.get(&id)
    }

    /// Snapshot of an animation's protocol-observable facts.
    ///
    /// Decouples consumers from the internal [`AnimationState`] field
    /// layout. Includes a borrowed `&[Duration]` slice of per-frame gaps so
    /// callers do not need a second accessor for gap reads.
    /// Returns `None` if the image is not animated.
    pub fn animation_snapshot(&self, id: ImageId) -> Option<AnimationSnapshot<'_>> {
        self.animations.get(&id).map(AnimationSnapshot::from)
    }

    /// Set animation playback state (Kitty `a=a`, `s=` key).
    ///
    /// `action`: 1=stop, 2=run (wait), 3=run.
    pub(crate) fn set_animation_action(&mut self, id: ImageId, action: u32) {
        if let Some(state) = self.animations.get_mut(&id) {
            match action {
                // Per kitty `graphics.c:1764` `current_loop = 0` runs
                // unconditionally for ALL s= actions, including s=1 stop.
                // Resetting loops_completed on s=1 means a subsequent s=3
                // resume on a bounded-loop animation replays the full loop
                // count rather than continuing from where it stopped.
                1 => {
                    state.paused = true;
                    state.loops_completed = 0;
                }
                // s=2 is run-wait — animation runs through its loop count
                // then halts at the last frame waiting for new frames (via
                // a=f) to extend it. s=3 is run — same loop playthrough but
                // stops permanently when loops exhaust. The distinguishing
                // behavior is in `add_animation_frame`, which auto-resumes
                // only when wait_mode is set.
                2 => {
                    state.paused = false;
                    state.loops_completed = 0;
                    state.wait_mode = true;
                    self.frame_starts.insert(id, Instant::now());
                }
                3 => {
                    state.paused = false;
                    state.loops_completed = 0;
                    state.wait_mode = false;
                    self.frame_starts.insert(id, Instant::now());
                }
                _ => {}
            }
            self.dirty = true;
        }
    }

    /// Set the loop count for an animated image (Kitty `v=` key).
    ///
    /// Per kitty `graphics.c:1766-1768` + `graphics-protocol.rst:942-945`:
    /// - `v=0` is ignored (no mutation; the C-side `if (g->loop_count)` guard).
    /// - `v=1` → infinite (`loop_count = None`, matching kitty `max_loops = 0`
    ///   and the `!max_loops` shortcircuit at `graphics.c:1775`).
    /// - `v=N>1` → finite N-1 loops (`loop_count = Some(N - 1)`).
    pub(crate) fn set_animation_loops(&mut self, id: ImageId, loops: u32) {
        if loops == 0 {
            return;
        }
        if let Some(state) = self.animations.get_mut(&id) {
            state.loop_count = if loops == 1 { None } else { Some(loops - 1) };
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

    /// Jump to a specific frame (Kitty `c=` in `a=a` — seek operation).
    ///
    /// Per kitty `graphics.c:1737-1743`:
    /// `if (frame_idx != img->current_frame_index && frame_idx <= img->extra_framecnt) { ... }`
    /// — idempotent seek (`c=current`) is a no-op; this guard prevents
    /// `frame_starts` timer reset stutter when clients periodically re-emit
    /// `c=N` for the already-current frame.
    pub(crate) fn set_current_frame(&mut self, id: ImageId, frame_idx: usize) {
        let should_apply = self.animations.get_mut(&id).is_some_and(|state| {
            if frame_idx < state.total_frames && state.current_frame != frame_idx {
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

