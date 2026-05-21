//! `ImageCache::compose_frame` impl — handles `a=c` per `graphics.c:1819-1880`.

use std::sync::Arc;

use super::super::{BlitRect, ComposeRequest, ImageError};
use super::ImageCache;
use super::frame_loading::{BlitOp, blit_subrect_into_canvas};

impl ImageCache {
    /// Compose a sub-rect of `src_frame` onto `dst_frame` per kitty
    /// `graphics.c:1819-1880 handle_compose_command`.
    ///
    /// Step order MATCHES kitty's reference implementation: frame
    /// resolution runs BEFORE rect-bounds validation. A command with both
    /// a missing frame AND an out-of-bounds rect emits `ENOENT` (matching
    /// kitty), not `EINVAL`.
    ///
    /// Error mapping (consumed by the dispatcher's `emit_compose_error_reply`):
    /// - [`ImageError::MissingImage`] → image not in cache.
    /// - [`ImageError::InvalidFrameRef`] → source or dest frame number
    ///   out of range (kitty `graphics.c:1820-1828` ENOENT).
    /// - [`ImageError::OversizedBlit`] → source or dest rect goes out of
    ///   image bounds (kitty `graphics.c:1833-1840` EINVAL).
    /// - [`ImageError::OverlappingFrames`] → same-frame compose with
    ///   overlapping source/dest rects (kitty `graphics.c:1841-1849` EINVAL).
    pub(crate) fn compose_frame(&mut self, req: ComposeRequest) -> Result<(), ImageError> {
        // Step 1: image exists (ENOENT precondition).
        let (image_w, image_h) = match self.images.get(&req.image_id) {
            Some(img) => (img.width, img.height),
            None => {
                return Err(ImageError::MissingImage {
                    id: req.image_id.0,
                });
            }
        };

        // Step 2: resolve source frame (ENOENT per graphics.c:1820-1823).
        // Matches kitty's order — frame existence is validated BEFORE
        // rect-bounds checks so mixed-invalid commands emit ENOENT.
        let src_bytes = self
            .frame_for_number(req.image_id, req.src_frame)
            .ok_or_else(|| ImageError::InvalidFrameRef {
                requested: req.src_frame,
                total: self.animation_total_frames(req.image_id),
            })?;

        // Step 3: resolve dest frame (ENOENT per graphics.c:1825-1828).
        let dst_bytes = self
            .frame_for_number(req.image_id, req.dst_frame)
            .ok_or_else(|| ImageError::InvalidFrameRef {
                requested: req.dst_frame,
                total: self.animation_total_frames(req.image_id),
            })?;

        // Step 4: resolve rect dims (0 = full image per graphics.c:1830-1831).
        let width = if req.width == 0 { image_w } else { req.width };
        let height = if req.height == 0 { image_h } else { req.height };

        // Step 5: bounds checks (overflow-safe; reject if rect leaves the image).
        let dest_ok = req
            .dst_x
            .checked_add(width)
            .is_some_and(|s| s <= image_w)
            && req
                .dst_y
                .checked_add(height)
                .is_some_and(|s| s <= image_h);
        let src_ok = req
            .src_x
            .checked_add(width)
            .is_some_and(|s| s <= image_w)
            && req
                .src_y
                .checked_add(height)
                .is_some_and(|s| s <= image_h);
        if !dest_ok || !src_ok {
            return Err(ImageError::OversizedBlit {
                blit_w: width,
                blit_h: height,
                image_w,
                image_h,
            });
        }

        // Step 6: same-frame overlap check (kitty graphics.c:1844-1850).
        // Half-open intersection: strict `<` allows adjacent rects.
        if req.src_frame == req.dst_frame {
            let x_overlap =
                req.src_x.max(req.dst_x) < req.src_x.min(req.dst_x).saturating_add(width);
            let y_overlap =
                req.src_y.max(req.dst_y) < req.src_y.min(req.dst_y).saturating_add(height);
            if x_overlap && y_overlap {
                return Err(ImageError::OverlappingFrames);
            }
        }

        // Step 7: extract source sub-rect into a tightly-packed payload.
        let mut payload = Vec::with_capacity((width as usize) * (height as usize) * 4);
        let stride = image_w as usize * 4;
        for row in 0..height as usize {
            let off = (req.src_y as usize + row) * stride + req.src_x as usize * 4;
            payload.extend_from_slice(&src_bytes[off..off + width as usize * 4]);
        }

        // Step 8: blit onto a fresh copy of the dest frame's bytes.
        let mut composed = (*dst_bytes).clone();
        blit_subrect_into_canvas(
            &mut composed,
            &payload,
            BlitOp {
                canvas_w: image_w,
                canvas_h: image_h,
                blit: BlitRect {
                    dest_x: req.dst_x,
                    dest_y: req.dst_y,
                    width,
                    height,
                },
                mode: req.mode,
            },
        )?;

        // Step 9: write back via shared helper (no gap update for compose).
        self.replace_frame_bytes(req.image_id, req.dst_frame, Arc::new(composed), None)?;
        Ok(())
    }
}
