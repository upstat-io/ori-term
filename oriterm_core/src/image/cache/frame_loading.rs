//! Frame-load pipeline for kitty `a=f` and `a=a r=` paths.
//!
//! Public entry: [`ImageCache::put_frame`]. One unified request struct
//! drives per-arm dispatch (default-append / `c=N` append / `r=N` edit).
//! The composed canvas (`Y=` solid, frame-N pixels, or edit-target frame)
//! is built in a full-image-sized buffer; the payload is blit'd into the
//! destination sub-rect via [`blit_subrect_into_canvas`]; the result is
//! pushed (append arm) or written back to the target slot (edit arm).
//!
//! All gap normalization happens upstream at the dispatch site
//! (`term/handler/image/kitty/frame_keys.rs`) — `put_frame` receives a
//! pre-normalized `FrameTarget` and applies the per-arm gap update via
//! the helper [`ImageCache::ensure_animation_state_for_root_gap`] when
//! the target is the root slot of a still-static image.

use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::time::Duration;

use super::super::{
    AnimationState, BlitRect, CanvasSource, CompositionMode, FrameLoadRequest, FrameTarget,
    ImageError, ImageId,
};
use super::ImageCache;

/// Parameters for [`blit_subrect_into_canvas`].
///
/// Collapses what would otherwise be a 5-arg signature (canvas + dims +
/// blit + mode) into one struct to keep the function signature narrow.
#[derive(Debug, Clone, Copy)]
pub(super) struct BlitOp {
    pub(super) canvas_w: u32,
    pub(super) canvas_h: u32,
    pub(super) blit: BlitRect,
    pub(super) mode: CompositionMode,
}

impl ImageCache {
    /// Total frames currently stored for `id` (1-based: root counts as 1).
    ///
    /// Used by kitty `a=f` dispatch to compute the r-clamp boundary
    /// (`effective_target = if r > total { total + 1 } else { r }`).
    /// Static (un-promoted) images always return 1.
    pub(crate) fn animation_total_frames(&self, id: ImageId) -> u32 {
        match self.animation_frames.get(&id) {
            Some(frames) => frames.len() as u32,
            None => {
                if self.images.contains_key(&id) {
                    1
                } else {
                    0
                }
            }
        }
    }

    /// Resolve a 1-based frame index to its pixel bytes.
    ///
    /// Matches the kitty `frame_for_number` semantics (graphics.c:1334-1344):
    /// - `n == 0` → `None`
    /// - `n == 1` → root: `images[id].data` when unpromoted, else
    ///   `animation_frames[id][0]`. NEVER returns the displayed-frame
    ///   slot (`images[id].data` mid-playback) for promoted animations —
    ///   that slot is mutated by `apply_frame` on every advance, so it
    ///   is NOT a stable root reference. Cluster A pin in §03.
    /// - `n > 1` → `animation_frames[id][n - 1]` when in range, else
    ///   `None`.
    pub(crate) fn frame_for_number(&self, id: ImageId, n: u32) -> Option<Arc<Vec<u8>>> {
        match n {
            0 => None,
            1 if !self.animations.contains_key(&id) => {
                self.images.get(&id).map(|img| img.data.clone())
            }
            1 => self
                .animation_frames
                .get(&id)
                .and_then(|frames| frames.first())
                .cloned(),
            n => self
                .animation_frames
                .get(&id)
                .and_then(|frames| frames.get((n - 1) as usize))
                .cloned(),
        }
    }

    /// Ensure the root frame's gap is recorded.
    ///
    /// Used by both `a=f,r=1,z!=0` (frame.rs) and `a=a,r=1,z!=0`
    /// (animate.rs) on still-static images: kitty stores the root gap
    /// on `Frame.gap` (graphics.c) but ori_term's `ImageData` has no
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
        match self.animations.entry(id) {
            Entry::Vacant(e) => {
                let root_bytes = self
                    .images
                    .get(&id)
                    .map(|img| img.data.clone())
                    .ok_or(ImageError::MissingImage { id: id.0 })?;
                e.insert(AnimationState::new(vec![gap], None));
                self.animation_frames.insert(id, vec![root_bytes]);
            }
            Entry::Occupied(mut e) => {
                let state = e.get_mut();
                if !state.frame_durations.is_empty() {
                    state.frame_durations[0] = gap;
                }
            }
        }
        Ok(())
    }

    /// Load a frame into the image cache (kitty `a=f` unified dispatch).
    ///
    /// Per-arm semantics (selected by `req.target` + `req.canvas`):
    /// - Default-append (`FrameTarget::Append` + `CanvasSource::SolidColor`):
    ///   canvas = `Y=` RGBA solid; payload blit'd into sub-rect; new frame
    ///   pushed. Returns the 1-based index of the new frame.
    /// - `c=N` append (`FrameTarget::Append` + `CanvasSource::Frame(n)`):
    ///   canvas = frame N's bytes (root-aware via `frame_for_number`);
    ///   payload blit'd; new frame pushed. Frame N is NOT mutated.
    /// - `r=N` edit (`FrameTarget::Edit` + `CanvasSource::EditTarget`):
    ///   canvas = target frame's existing bytes; payload composed on top
    ///   per `composition_mode`; result REPLACES frame N. Returns N.
    ///
    /// Validates payload size, image existence, blit dims, and frame-num
    /// range before touching cache state.
    pub(crate) fn put_frame(&mut self, req: FrameLoadRequest) -> Result<u32, ImageError> {
        // (Step 6.1) Image must exist.
        let (image_w, image_h) = match self.images.get(&req.image_id) {
            Some(img) => (img.width, img.height),
            None => return Err(ImageError::MissingImage { id: req.image_id.0 }),
        };

        // (Step 6.2) Blit dims must fit image. `==` passes; only `>` rejects.
        if req.blit.width > image_w || req.blit.height > image_h {
            return Err(ImageError::OversizedBlit {
                blit_w: req.blit.width,
                blit_h: req.blit.height,
                image_w,
                image_h,
            });
        }

        // (Step 6.3) Reject oversized payload.
        if req.frame_data.len() > self.max_single_image_bytes {
            return Err(ImageError::OversizedImage);
        }

        // (Step 6.4) Payload-size invariant: overflow-safe multiplication.
        let expected = (req.blit.width as usize)
            .checked_mul(req.blit.height as usize)
            .and_then(|wh| wh.checked_mul(4))
            .ok_or(ImageError::InvalidFormat)?;
        if req.frame_data.len() != expected {
            return Err(ImageError::InvalidFormat);
        }

        // (Step 6.5) Request consistency: EditTarget pairs with Edit;
        // SolidColor/Frame pair with Append.
        match (&req.canvas, &req.target) {
            (CanvasSource::EditTarget, FrameTarget::Edit { .. }) => {}
            (CanvasSource::SolidColor(_) | CanvasSource::Frame(_), FrameTarget::Append { .. }) => {}
            _ => return Err(ImageError::InvalidFormat),
        }

        // (Step 6.6) Edit target frame-num range.
        if let FrameTarget::Edit { frame_num, .. } = &req.target {
            let total = self.animation_total_frames(req.image_id);
            if *frame_num < 1 || *frame_num > total {
                return Err(ImageError::InvalidFrameRef {
                    requested: *frame_num,
                    total,
                });
            }
        }

        // (Step 6.7) Resolve canvas bytes.
        let canvas_bytes = match req.canvas {
            CanvasSource::SolidColor(rgba) => {
                let pixel = rgba_u32_to_bytes(rgba);
                let pixels = (image_w as usize) * (image_h as usize);
                let mut buf = Vec::with_capacity(pixels * 4);
                for _ in 0..pixels {
                    buf.extend_from_slice(&pixel);
                }
                buf
            }
            CanvasSource::Frame(n) => match self.frame_for_number(req.image_id, n) {
                Some(bytes) => (*bytes).clone(),
                None => {
                    return Err(ImageError::InvalidFrameRef {
                        requested: n,
                        total: self.animation_total_frames(req.image_id),
                    });
                }
            },
            CanvasSource::EditTarget => {
                let frame_num = match &req.target {
                    FrameTarget::Edit { frame_num, .. } => *frame_num,
                    FrameTarget::Append { .. } => unreachable!("validated above"),
                };
                match self.frame_for_number(req.image_id, frame_num) {
                    Some(bytes) => (*bytes).clone(),
                    None => {
                        return Err(ImageError::InvalidFrameRef {
                            requested: frame_num,
                            total: self.animation_total_frames(req.image_id),
                        });
                    }
                }
            }
        };

        // (Step 6.8/6.9) Blit the payload onto the canvas.
        let mut composed = canvas_bytes;
        blit_subrect_into_canvas(
            &mut composed,
            &req.frame_data,
            BlitOp {
                canvas_w: image_w,
                canvas_h: image_h,
                blit: req.blit,
                mode: req.composition_mode,
            },
        )?;

        // (Step 6.10) Dispatch by target.
        match req.target {
            FrameTarget::Append { gap } => {
                let composed_arc = Arc::new(composed);
                self.push_composed_frame(req.image_id, composed_arc, gap)
            }
            FrameTarget::Edit {
                frame_num,
                gap_update,
            } => self.put_frame_edit(req.image_id, frame_num, composed, gap_update),
        }
    }

    /// Edit-arm body for `put_frame`: replace `frame_num`'s bytes with
    /// `composed`, optionally update its gap, handle the static-image
    /// root-edit auto-promote per kitty parity.
    ///
    /// Thin caller wrapping [`Self::replace_frame_bytes`] — `put_frame_edit`
    /// owns the `Vec<u8>` → `Arc<Vec<u8>>` conversion; the shared helper
    /// owns the static-root / promoted-non-displayed / promoted-displayed
    /// dispatch. Compose reuses the same dispatch via the shared helper.
    fn put_frame_edit(
        &mut self,
        id: ImageId,
        frame_num: u32,
        composed: Vec<u8>,
        gap_update: Option<Duration>,
    ) -> Result<u32, ImageError> {
        self.replace_frame_bytes(id, frame_num, Arc::new(composed), gap_update)
    }

    /// Replace `frame_num`'s pixel bytes, handling static-root /
    /// promoted-non-displayed / promoted-displayed dispatch.
    ///
    /// Shared between [`Self::put_frame`] edit arm (`a=f,r=N`) and
    /// [`Self::compose_frame`] (`a=c`). Callers with `Vec<u8>` go through
    /// [`Self::put_frame_edit`] which `Arc::new`s for them; callers
    /// holding `Arc<Vec<u8>>` directly call here.
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
            // Static-image root edit.
            if let Some(img) = self.images.get_mut(&id) {
                img.data = composed.clone();
            }
            if gap_update.is_some() {
                // Auto-promote to record root gap; ensures gap survives.
                self.ensure_animation_state_for_root_gap(id, gap_update)?;
                if let Some(frames) = self.animation_frames.get_mut(&id) {
                    frames[0] = composed;
                }
            }
            self.dirty = true;
            return Ok(1);
        }

        // Promoted: write to animation_frames[id][frame_num - 1].
        let displayed = if let Some(frames) = self.animation_frames.get_mut(&id) {
            let idx = (frame_num - 1) as usize;
            if idx >= frames.len() {
                return Err(ImageError::InvalidFrameRef {
                    requested: frame_num,
                    total: frames.len() as u32,
                });
            }
            frames[idx] = composed.clone();
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
        composed: Arc<Vec<u8>>,
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
        let added_1based = match self.animations.entry(id) {
            Entry::Vacant(e) => {
                let img_data = match self.images.get(&id) {
                    Some(img) => img.data.clone(),
                    None => return Err(ImageError::MissingImage { id: id.0 }),
                };
                // Root frame gap is ZERO per kitty_tests/graphics.py:1189.
                let durations = vec![Duration::ZERO, gap];
                let frames = vec![img_data, composed.clone()];
                self.memory_used += composed.len();
                e.insert(AnimationState::new(durations, None));
                self.animation_frames.insert(id, frames);
                2
            }
            Entry::Occupied(mut e) => {
                let anim_frames = self.animation_frames.entry(id).or_default();
                anim_frames.push(composed.clone());
                self.memory_used += composed.len();

                let state = e.get_mut();
                state.frame_durations.push(gap);
                state.total_frames = anim_frames.len();
                if state.wait_mode && state.is_finished() {
                    state.loops_completed = 0;
                }
                anim_frames.len() as u32
            }
        };

        self.dirty = true;
        Ok(added_1based)
    }
}

/// Convert a packed-RGBA u32 (R in MSB, A in LSB — matches kitty
/// `g->bgcolor`) to the byte sequence `[R, G, B, A]`.
fn rgba_u32_to_bytes(rgba: u32) -> [u8; 4] {
    [
        (rgba >> 24) as u8,
        (rgba >> 16) as u8,
        (rgba >> 8) as u8,
        rgba as u8,
    ]
}

/// Composite `src` into the destination sub-rect of `canvas` per `op`.
///
/// Bounds policy matches the kitty implementation (graphics.c:1430-1433 + 1580-1583):
/// - Blit dims exceeding canvas dims → `OversizedBlit` rejection.
/// - Dest offset outside canvas → silent no-op (kitty parity).
/// - Dest offset + dims overflow canvas → saturating clip.
/// - Overflow-safe arithmetic: rust debug builds panic on integer overflow;
///   user-controlled offsets MUST use `min` clipping, not `+`.
pub(super) fn blit_subrect_into_canvas(
    canvas: &mut [u8],
    src: &[u8],
    op: BlitOp,
) -> Result<(), ImageError> {
    if op.blit.width > op.canvas_w || op.blit.height > op.canvas_h {
        return Err(ImageError::OversizedBlit {
            blit_w: op.blit.width,
            blit_h: op.blit.height,
            image_w: op.canvas_w,
            image_h: op.canvas_h,
        });
    }

    if op.blit.dest_x >= op.canvas_w || op.blit.dest_y >= op.canvas_h {
        // Sub-rect entirely outside canvas → silent no-op (kitty parity).
        return Ok(());
    }

    let copy_w = op.blit.width.min(op.canvas_w - op.blit.dest_x);
    let copy_h = op.blit.height.min(op.canvas_h - op.blit.dest_y);
    if copy_w == 0 || copy_h == 0 {
        return Ok(());
    }

    let canvas_w_usize = op.canvas_w as usize;
    let blit_w_usize = op.blit.width as usize;

    for row in 0..copy_h as usize {
        let dst_y = op.blit.dest_y as usize + row;
        let dst_row_off = (dst_y * canvas_w_usize + op.blit.dest_x as usize) * 4;
        let src_row_off = row * blit_w_usize * 4;

        for col in 0..copy_w as usize {
            let dst_px = dst_row_off + col * 4;
            let src_px = src_row_off + col * 4;
            if dst_px + 4 > canvas.len() || src_px + 4 > src.len() {
                break;
            }
            match op.mode {
                CompositionMode::Overwrite => {
                    canvas[dst_px..dst_px + 4].copy_from_slice(&src[src_px..src_px + 4]);
                }
                CompositionMode::AlphaBlend => {
                    blend_pixel_over(&mut canvas[dst_px..dst_px + 4], &src[src_px..src_px + 4]);
                }
            }
        }
    }

    Ok(())
}

/// Porter-Duff source-over blend of one RGBA pixel.
///
/// Matches the byte-arithmetic shape from the prior `alpha_blend_frames`
/// kernel — kept as a per-pixel helper so the sub-rect blit can apply it
/// only to the destination pixels touched by the payload.
fn blend_pixel_over(dst: &mut [u8], src: &[u8]) {
    let sa = src[3] as u32;
    if sa == 0 {
        return;
    }
    if sa == 255 {
        dst[0] = src[0];
        dst[1] = src[1];
        dst[2] = src[2];
        dst[3] = 255;
        return;
    }
    let da = dst[3] as u32;
    let inv_sa = 255 - sa;
    let oa = sa + (da * inv_sa) / 255;
    if oa == 0 {
        return;
    }
    dst[0] = ((src[0] as u32 * sa + dst[0] as u32 * da * inv_sa / 255) / oa) as u8;
    dst[1] = ((src[1] as u32 * sa + dst[1] as u32 * da * inv_sa / 255) / oa) as u8;
    dst[2] = ((src[2] as u32 * sa + dst[2] as u32 * da * inv_sa / 255) / oa) as u8;
    dst[3] = oa.min(255) as u8;
}
