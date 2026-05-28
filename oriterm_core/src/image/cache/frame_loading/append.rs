//! Append-family methods for the kitty `a=f` frame-load pipeline.
//!
//! Owns: `push_composed_frame` (slow path append), `push_delta_frame` +
//! `try_push_delta_append` + `resolve_base_metadata` (Δ-storage fast path),
//! `auto_promote_static_root` (promote-on-first-c=1 helper). All paths
//! reach here via [`super::ImageCache::put_frame`] dispatch.

use std::sync::Arc;
use std::time::Duration;

use super::super::super::{
    AnimationState, CanvasSource, FrameLoadRequest, FrameTarget, ImageError, ImageId,
};
use super::super::frame_entry::{
    FrameEntry, FrameId, MAX_CUMULATIVE_DRAWN_AREA_RATIO, MAX_DELTA_CHAIN_DEPTH,
};
use super::ImageCache;

/// A Δ frame to append, bundling its chain-base metadata with the
/// sub-rect payload that [`ImageCache::push_delta_frame`] stores.
struct DeltaFrameSpec {
    /// `FrameId` of the base frame this Δ references.
    base: FrameId,
    /// Δ-chain depth (0 = first hop off a Materialized base).
    depth: u8,
    /// Cumulative drawn area across the chain up to and including this Δ.
    cumulative_drawn_area: u32,
    /// Sub-rect this Δ paints relative to the canvas.
    sub_rect: crate::image::BlitRect,
    /// Sub-rect RGBA bytes (Δ payload, not the full canvas).
    payload: Arc<Vec<u8>>,
    /// Composition mode applied when materializing the canvas.
    compose_mode: crate::image::CompositionMode,
}

impl ImageCache {
    /// Auto-promote a static image's root frame into `animation_frames`.
    ///
    /// Mints a fresh `FrameId` for the root so subsequent `c=N` appends
    /// can target it as a Δ base. Idempotent — no-op when the image is
    /// already promoted. Used by `put_frame`'s Δ-storage entry to ensure
    /// the first `c=1` append against a static image produces a Δ
    /// referencing the root, NOT a force-materialized canvas.
    fn auto_promote_static_root(&mut self, image_id: ImageId) -> Result<(), ImageError> {
        if self.animations.contains_key(&image_id) {
            return Ok(());
        }
        let root_bytes = self
            .images
            .get(&image_id)
            .map(|img| img.data.clone())
            .ok_or(ImageError::MissingImage { id: image_id.0 })?;
        let root_id = self.alloc_frame_id();
        self.animations
            .insert(image_id, AnimationState::new(vec![Duration::ZERO], None));
        self.animation_frames.insert(
            image_id,
            vec![FrameEntry::Materialized {
                id: root_id,
                data: root_bytes,
            }],
        );
        Ok(())
    }

    /// Resolve base-frame metadata for a `_Ga=f, c=N` Δ-storage opportunity.
    ///
    /// Returns `Some((base_id, is_delta_base, base_depth, base_cumulative))`
    /// when frame `n` (1-based) exists in `animation_frames[image_id]`.
    /// `None` when the image is unpromoted (no `FrameId` minted for the
    /// static root yet — Δ storage is deferred to the next append) or
    /// the index is out of range.
    ///
    /// `is_delta_base` distinguishes the chain-depth rule:
    /// - base is Materialized → new Δ has depth 0 (first hop)
    /// - base is Δ → new Δ has depth = base.depth + 1
    fn resolve_base_metadata(&self, image_id: ImageId, n: u32) -> Option<(FrameId, bool, u8, u32)> {
        if n == 0 {
            return None;
        }
        let frames = self.animation_frames.get(&image_id)?;
        let entry = frames.get((n - 1) as usize)?;
        let is_delta_base = matches!(entry, FrameEntry::Delta { .. });
        Some((
            entry.id(),
            is_delta_base,
            entry.depth(),
            entry.cumulative_drawn_area(),
        ))
    }

    /// Push a Δ frame onto a promoted animation. Mirrors
    /// [`Self::push_composed_frame`] but stores `FrameEntry::Delta`
    /// without materializing the full canvas — `payload` is the sub-rect
    /// RGBA bytes only.
    ///
    /// Caller is responsible for ensuring guards (depth + cumulative
    /// area) permit Δ storage; this helper assumes the decision is
    /// already made.
    fn push_delta_frame(
        &mut self,
        image_id: ImageId,
        spec: DeltaFrameSpec,
        gap: Duration,
    ) -> Result<u32, ImageError> {
        let DeltaFrameSpec {
            base,
            depth,
            cumulative_drawn_area,
            sub_rect,
            payload,
            compose_mode,
        } = spec;
        let payload_len = payload.len();

        // Eviction loop — same shape as push_composed_frame, just with
        // the smaller Δ payload size as the memory delta.
        let mut reachable = self.placed_or_anchored_id_set();
        reachable.insert(image_id);
        while self.memory_used + payload_len > self.memory_limit && !self.images.is_empty() {
            if !self.evict_one(&reachable) {
                return Err(ImageError::MemoryLimitExceeded);
            }
        }

        let new_id = self.alloc_frame_id();
        debug_assert!(
            base.0 < new_id.0,
            "Δ cycle invariant: base FrameId ({}) must be strictly less than new FrameId ({})",
            base.0,
            new_id.0,
        );

        let anim_frames = self.animation_frames.entry(image_id).or_default();
        anim_frames.push(FrameEntry::Delta {
            id: new_id,
            base,
            sub_rect,
            payload,
            compose_mode,
            depth,
            cumulative_drawn_area,
        });
        self.memory_used += payload_len;

        let total = anim_frames.len();
        let state = self
            .animations
            .get_mut(&image_id)
            .expect("Δ storage requires promoted animation");
        state.frame_durations.push(gap);
        state.total_frames = total;
        if state.wait_mode && state.is_finished() {
            state.loops_completed = 0;
        }
        self.dirty = true;
        Ok(total as u32)
    }

    /// Δ-storage append fast-path. Returns:
    /// - `Ok(Some(...))` — request landed as `FrameEntry::Delta`; caller returns immediately.
    /// - `Ok(None)` — Δ path did not apply (wrong target/canvas, base unresolvable, or
    ///   chain/area guard tripped); caller falls through to the materialize path.
    /// - `Err(...)` — auto-promote of the static root failed.
    ///
    /// Keeps per-append allocator pressure bounded by the sub-rect payload size
    /// (typically ≤ 1 KB) instead of the full image canvas (~2.64 MB on xray's
    /// 1000×660 transmits).
    pub(super) fn try_push_delta_append(
        &mut self,
        req: &FrameLoadRequest,
        image_w: u32,
        image_h: u32,
    ) -> Result<Option<Result<u32, ImageError>>, ImageError> {
        let (FrameTarget::Append { gap }, CanvasSource::Frame(n)) = (&req.target, &req.canvas)
        else {
            return Ok(None);
        };

        // Auto-promote static images on c=1 so the root has a FrameId
        // Δ.base can reference — first c=N append on a static image
        // still produces Δ storage.
        if *n == 1 && !self.animations.contains_key(&req.image_id) {
            self.auto_promote_static_root(req.image_id)?;
        }

        let Some((base_id, base_is_delta, base_depth, base_cumulative)) =
            self.resolve_base_metadata(req.image_id, *n)
        else {
            return Ok(None);
        };

        // Δ directly off Materialized has depth 0; Δ off Δ increments base.depth by 1.
        let new_depth = if base_is_delta {
            base_depth.saturating_add(1)
        } else {
            0
        };
        let blit_area = (req.blit.width as u64).saturating_mul(req.blit.height as u64);
        let new_cumulative = (u64::from(base_cumulative)).saturating_add(blit_area);
        let area_cap = (image_w as u64)
            .saturating_mul(image_h as u64)
            .saturating_mul(u64::from(MAX_CUMULATIVE_DRAWN_AREA_RATIO));
        let depth_ok = new_depth <= MAX_DELTA_CHAIN_DEPTH;
        // `<=` so the Δ that lands EXACTLY at the cap is still stored as Δ;
        // the NEXT append (which would exceed the cap) force-materializes
        // per kitty's `drawn_area >= image_w * image_h * 2` trigger
        // (graphics.c:1546).
        let area_ok = new_cumulative <= area_cap;
        if !(depth_ok && area_ok) {
            // Guard tripped — fall through to materialize path which
            // force-materializes the new frame, resetting the chain.
            return Ok(None);
        }

        Ok(Some(self.push_delta_frame(
            req.image_id,
            DeltaFrameSpec {
                base: base_id,
                depth: new_depth,
                cumulative_drawn_area: new_cumulative as u32,
                sub_rect: req.blit,
                payload: req.frame_data.clone(),
                compose_mode: req.composition_mode,
            },
            *gap,
        )))
    }

    /// Push a fully-composed frame onto the animation (append arm).
    ///
    /// Shared bookkeeping for the default-append and `c=N` append paths:
    /// 1. Reject oversized composed buffer.
    /// 2. Run LRU eviction with explicit target-id protection (the target
    ///    image MUST NOT evict itself out from under the append per AC2).
    /// 3. Vacant entry → auto-promote with `[ZERO, gap]` durations so the
    ///    root frame's gap matches kitty parity (`root_frame_gap == 0`).
    /// 4. Occupied entry → append the composed bytes + gap; clear
    ///    `wait_mode`-finished latch so `advance()` ticks into the new
    ///    frames.
    pub(crate) fn push_composed_frame(
        &mut self,
        id: ImageId,
        composed: &Arc<Vec<u8>>,
        gap: Duration,
    ) -> Result<u32, ImageError> {
        // (1) Oversized image gate FIRST — matches store_animated.
        if composed.len() > self.max_single_image_bytes {
            return Err(ImageError::OversizedImage);
        }

        // (2) Eviction loop with explicit target protection.
        let mut reachable = self.placed_or_anchored_id_set();
        reachable.insert(id);
        while self.memory_used + composed.len() > self.memory_limit && !self.images.is_empty() {
            if !self.evict_one(&reachable) {
                return Err(ImageError::MemoryLimitExceeded);
            }
        }

        // (3) Append (auto-promote when needed).
        let promoted = self.animations.contains_key(&id);
        let added_1based = if promoted {
            let new_id = self.alloc_frame_id();
            let anim_frames = self.animation_frames.entry(id).or_default();
            anim_frames.push(FrameEntry::Materialized {
                id: new_id,
                data: composed.clone(),
            });
            self.memory_used += composed.len();

            let total = anim_frames.len();
            let state = self
                .animations
                .get_mut(&id)
                .expect("promoted: animations entry present");
            state.frame_durations.push(gap);
            state.total_frames = total;
            if state.wait_mode && state.is_finished() {
                state.loops_completed = 0;
            }
            total as u32
        } else {
            let img_data = match self.images.get(&id) {
                Some(img) => img.data.clone(),
                None => return Err(ImageError::MissingImage { id: id.0 }),
            };
            // Root frame gap is ZERO per kitty_tests/graphics.py:1189.
            let durations = vec![Duration::ZERO, gap];
            let root_id = self.alloc_frame_id();
            let composed_id = self.alloc_frame_id();
            let frames = vec![
                FrameEntry::Materialized {
                    id: root_id,
                    data: img_data,
                },
                FrameEntry::Materialized {
                    id: composed_id,
                    data: composed.clone(),
                },
            ];
            self.memory_used += composed.len();
            self.animations
                .insert(id, AnimationState::new(durations, None));
            self.animation_frames.insert(id, frames);
            2
        };

        self.dirty = true;
        Ok(added_1based)
    }
}
