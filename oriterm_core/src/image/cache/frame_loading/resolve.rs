//! Frame resolution + canvas/request validation for the kitty `a=f` pipeline.
//!
//! Owns:
//! - `frame_for_number` / `frame_for_number_by_id` — public resolvers from
//!   1-based protocol indices or stable `FrameId` to pixel bytes.
//! - `compose_entry` — recursive Δ-resolver (visibility elevated to
//!   `pub(in crate::image::cache)` so `cache/deletion.rs` cross-sibling
//!   call at line 243 still resolves).
//! - `resolve_canvas_bytes` — picks `CanvasSource` (solid color, copy of
//!   frame N, or copy of edit-target frame).
//! - `validate_frame_request` — image existence + blit-fit + size +
//!   request-consistency + edit-target-range gate.

use std::sync::Arc;

use super::super::super::{CanvasSource, FrameLoadRequest, FrameTarget, ImageError, ImageId};
use super::super::frame_entry::{FrameEntry, FrameId};
use super::BlitOp;
use super::ImageCache;
use super::blit_subrect_into_canvas;

impl ImageCache {
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
            n => {
                let frames = self.animation_frames.get(&id)?;
                let entry = frames.get((n - 1) as usize)?;
                self.compose_entry(id, entry)
            }
        }
    }

    /// Resolve a stable `FrameId` to its pixel bytes within `image_id`.
    ///
    /// Linear scan of `animation_frames[image_id]` for the entry whose
    /// `FrameEntry::id()` matches — matches kitty's `frame_for_id` linear
    /// scan at `graphics.c:1328-1331`. Recursive compose-on-demand for
    /// `Delta` variants per §05 Item 2.
    pub(crate) fn frame_for_number_by_id(
        &self,
        image_id: ImageId,
        fid: FrameId,
    ) -> Option<Arc<Vec<u8>>> {
        let frames = self.animation_frames.get(&image_id)?;
        let entry = frames.iter().find(|e| e.id() == fid)?;
        self.compose_entry(image_id, entry)
    }

    /// Compose-on-demand resolver for a single `FrameEntry`.
    ///
    /// `Materialized` → clone Arc.
    /// `Delta` → recurse on `base` to materialize the canvas, allocate a
    /// fresh `Vec<u8>`, blit the sub-rect payload via the existing
    /// `blit_subrect_into_canvas` kernel, wrap in a new `Arc`. NO
    /// memoization per the §02 anti-cache rule — every render allocates
    /// fresh so unbounded growth is impossible.
    ///
    /// Visibility: `pub(in crate::image::cache)` so cross-sibling callers
    /// (e.g. `cache/deletion.rs:243`) can reach it after the split.
    pub(in crate::image::cache) fn compose_entry(
        &self,
        image_id: ImageId,
        entry: &FrameEntry,
    ) -> Option<Arc<Vec<u8>>> {
        match entry {
            FrameEntry::Materialized { data, .. } => Some(data.clone()),
            FrameEntry::Delta {
                base,
                sub_rect,
                payload,
                compose_mode,
                ..
            } => {
                let base_bytes = self.frame_for_number_by_id(image_id, *base)?;
                let (image_w, image_h) = self
                    .images
                    .get(&image_id)
                    .map(|img| (img.width, img.height))?;
                let mut canvas: Vec<u8> = (*base_bytes).clone();
                blit_subrect_into_canvas(
                    &mut canvas,
                    payload,
                    BlitOp {
                        canvas_w: image_w,
                        canvas_h: image_h,
                        blit: *sub_rect,
                        mode: *compose_mode,
                    },
                )
                .ok()?;
                Some(Arc::new(canvas))
            }
        }
    }

    /// Validate `req` against image existence, blit-fit, size invariants,
    /// canvas/target consistency, and edit-target frame-num range.
    /// Returns the target image's canvas dimensions on success.
    pub(super) fn validate_frame_request(
        &self,
        req: &FrameLoadRequest,
    ) -> Result<(u32, u32), ImageError> {
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
            (CanvasSource::EditTarget, FrameTarget::Edit { .. })
            | (CanvasSource::SolidColor(_) | CanvasSource::Frame(_), FrameTarget::Append { .. }) => {
            }
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

        Ok((image_w, image_h))
    }

    /// Resolve the canvas bytes for `req` per `CanvasSource`:
    /// solid-color fill, copy of frame `n`, or copy of the edit-target frame.
    pub(super) fn resolve_canvas_bytes(
        &self,
        req: &FrameLoadRequest,
        image_w: u32,
        image_h: u32,
    ) -> Result<Vec<u8>, ImageError> {
        match &req.canvas {
            CanvasSource::SolidColor(rgba) => {
                let pixel = super::rgba_u32_to_bytes(*rgba);
                let pixels = (image_w as usize) * (image_h as usize);
                let mut buf = Vec::with_capacity(pixels * 4);
                for _ in 0..pixels {
                    buf.extend_from_slice(&pixel);
                }
                Ok(buf)
            }
            CanvasSource::Frame(n) => match self.frame_for_number(req.image_id, *n) {
                Some(bytes) => Ok((*bytes).clone()),
                None => Err(ImageError::InvalidFrameRef {
                    requested: *n,
                    total: self.animation_total_frames(req.image_id),
                }),
            },
            CanvasSource::EditTarget => {
                let frame_num = match &req.target {
                    FrameTarget::Edit { frame_num, .. } => *frame_num,
                    FrameTarget::Append { .. } => unreachable!("validated above"),
                };
                match self.frame_for_number(req.image_id, frame_num) {
                    Some(bytes) => Ok((*bytes).clone()),
                    None => Err(ImageError::InvalidFrameRef {
                        requested: frame_num,
                        total: self.animation_total_frames(req.image_id),
                    }),
                }
            }
        }
    }
}
