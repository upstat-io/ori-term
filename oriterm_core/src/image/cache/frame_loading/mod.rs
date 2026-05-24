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

mod append;
mod edit;
mod replace;
mod resolve;

use std::sync::Arc;

use super::super::{
    BlitRect, CanvasSource, CompositionMode, FrameLoadRequest, FrameTarget, ImageError, ImageId,
};
use super::ImageCache;

/// Parameters for [`blit_subrect_into_canvas`].
///
/// Collapses what would otherwise be a 5-arg signature (canvas + dims +
/// blit + mode) into one struct to keep the function signature narrow.
#[derive(Debug, Clone, Copy)]
pub(in crate::image::cache) struct BlitOp {
    pub(in crate::image::cache) canvas_w: u32,
    pub(in crate::image::cache) canvas_h: u32,
    pub(in crate::image::cache) blit: BlitRect,
    pub(in crate::image::cache) mode: CompositionMode,
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
            None => u32::from(self.images.contains_key(&id)),
        }
    }

    pub(crate) fn put_frame(&mut self, req: &FrameLoadRequest) -> Result<u32, ImageError> {
        let (image_w, image_h) = self.validate_frame_request(req)?;

        // (Step 6.6.5) Δ-storage opportunity: when the request is an
        // append + canvas references an existing promoted frame, AND the
        // chain-depth + cumulative-area guards permit, store as
        // `FrameEntry::Delta` without materializing the full canvas.
        if let Some(delta_result) = self.try_push_delta_append(req, image_w, image_h)? {
            return delta_result;
        }

        // (Step 6.6.6) In-place Edit fast path. notcurses' kitty animation
        // sends many `_Ga=f,r=N` Edits per frame with tiny sub-rect blits
        // onto a multi-megabyte canvas; the default slow path clones the
        // entire prev canvas (~7 MiB) per Edit just to mutate ~7 KiB of
        // pixels. Try `Arc::make_mut` to mutate the existing canvas in
        // place when the Arc is uniquely held — detach the
        // `images[id].data` sync-mirror first so `make_mut` sees a unique
        // Arc, then restore the mirror to the mutated Arc.
        if matches!(
            (&req.target, &req.canvas),
            (FrameTarget::Edit { .. }, CanvasSource::EditTarget),
        ) && self.animations.contains_key(&req.image_id)
            && let Some(result) = self.try_edit_in_place(req, (image_w, image_h))
        {
            return result;
        }

        // (Step 6.7) Resolve canvas bytes + (Step 6.8/6.9) blit the payload.
        let mut composed = self.resolve_canvas_bytes(req, image_w, image_h)?;
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
        match &req.target {
            FrameTarget::Append { gap } => {
                let composed_arc = Arc::new(composed);
                self.push_composed_frame(req.image_id, &composed_arc, *gap)
            }
            FrameTarget::Edit {
                frame_num,
                gap_update,
            } => self.put_frame_edit(req.image_id, *frame_num, composed, *gap_update),
        }
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
pub(in crate::image::cache) fn blit_subrect_into_canvas(
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
