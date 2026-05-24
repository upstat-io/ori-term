//! Kitty graphics `a=f` (animation frame transmit) action.

use std::sync::Arc;

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyCommand;
use crate::image::{BlitRect, CanvasSource, FrameLoadRequest, FrameTarget, ImageError, ImageId};
use crate::term::Term;

use super::KittyReplyContext;
use super::frame_keys::{KittyFrameKeys, extract_a_f_keys};
use super::prepare::prepare_image_bytes;
use super::store::expected_decoded_size_for_format;

impl<S: EffectSink> Term<S> {
    /// Handle `a=f` — transmit an animation frame.
    ///
    /// Three spec arms per kitty `graphics-protocol.rst` lines 880-903:
    /// - **Default-append**: neither `c=` nor `r=` set → append a new
    ///   frame, canvas = `Y=` RGBA solid color.
    /// - **`c=N` append**: `c=N` set, `r=` unset or clamped to next-append
    ///   → append a new frame using frame N's pixel data as canvas.
    /// - **`r=N` edit**: `r=N` within existing-frame range → edit frame N
    ///   in place; canvas IS frame N; payload composed on top.
    ///
    /// `r=N > total` clamps to the next-append index per kitty
    /// graphics.c:1558-1561 (NOT EINVAL). `c=` is consulted only on the
    /// append path (graphics.c:1606-1635); the edit arm silently ignores
    /// it. Sub-rect (`x=, y=, s=, v=`) applies to all three arms.
    /// Composition mode (`X=`) selects `AlphaBlend` (0) or Overwrite (1).
    pub(super) fn kitty_frame(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let (image_id, mut merged) = self.kitty_finalize_payload(cmd);
        let ctx = KittyReplyContext::from_cmd(&merged).with_image_id(image_id);

        // ENOENT precheck BEFORE decode (kitty graphics.c:2230-2235).
        if self.image_cache().get_no_touch(ImageId(image_id)).is_none() {
            self.kitty_respond(&ctx, "ENOENT");
            return;
        }

        // Decode payload — captures decoded dims for blit (load-bearing
        // for PNG `f=100` where `s=` and decoded dims can disagree because
        // `decode_to_rgba` reads dims from the PNG header).
        let payload = std::mem::take(&mut merged.payload);
        let expected_size = expected_decoded_size_for_format(
            merged.format,
            merged.source_width,
            merged.source_height,
        );
        let max_bytes = self.image_cache().max_single_image_bytes();
        let payload =
            match prepare_image_bytes(payload, merged.compression, expected_size, max_bytes) {
                Ok(buf) => buf,
                Err(e) => {
                    let msg = e.to_string();
                    warn!("kitty frame prepare failed: {msg}");
                    self.kitty_respond(&ctx, &msg);
                    return;
                }
            };
        let (rgba_data, decoded_w, decoded_h) = match Self::kitty_decode_pixels(
            payload,
            merged.format,
            merged.source_width,
            merged.source_height,
            max_bytes,
        ) {
            Ok(result) => result,
            Err(msg) => {
                warn!("kitty frame decode failed: {msg}");
                self.kitty_respond(&ctx, &msg);
                return;
            }
        };

        let keys = extract_a_f_keys(&merged, decoded_w, decoded_h);

        // r-clamp + arm resolution (kitty graphics.c:1558-1561 + 1606-1635).
        let total_frames = self.image_cache().animation_total_frames(ImageId(image_id));
        let r_target = keys.target_frame.unwrap_or(0);
        let effective_target = if r_target == 0 || r_target > total_frames {
            total_frames + 1
        } else {
            r_target
        };
        let is_append = effective_target == total_frames + 1;

        let request = build_request(image_id, &keys, &rgba_data, is_append, effective_target);

        match self.image_cache_mut().put_frame(&request) {
            Ok(result_frame_num) => {
                let ctx = ctx.with_frame_num(result_frame_num);
                self.kitty_respond(&ctx, "OK");
            }
            Err(e) => emit_error_reply(self, ctx, &e),
        }
    }
}

/// Build the `FrameLoadRequest` for the resolved arm.
fn build_request(
    image_id: u32,
    keys: &KittyFrameKeys,
    rgba_data: &[u8],
    is_append: bool,
    effective_target: u32,
) -> FrameLoadRequest {
    let target = if is_append {
        FrameTarget::Append {
            gap: keys.append_gap,
        }
    } else {
        FrameTarget::Edit {
            frame_num: effective_target,
            gap_update: keys.edit_gap_update,
        }
    };
    let canvas = if is_append {
        if let Some(c) = keys.canvas_frame {
            CanvasSource::Frame(c)
        } else {
            CanvasSource::SolidColor(keys.background_rgba)
        }
    } else {
        CanvasSource::EditTarget
    };
    FrameLoadRequest {
        image_id: ImageId(image_id),
        target,
        canvas,
        blit: BlitRect {
            dest_x: keys.blit_x,
            dest_y: keys.blit_y,
            width: keys.blit_w,
            height: keys.blit_h,
        },
        frame_data: Arc::new(rgba_data.to_vec()),
        composition_mode: keys.compose_mode,
    }
}

/// Map `put_frame` errors to kitty reply codes.
fn emit_error_reply<S: EffectSink>(term: &Term<S>, ctx: KittyReplyContext, err: &ImageError) {
    match err {
        // ENOENT per kitty graphics.c:2233-2235; preserve `,r=` omission so
        // existing `kitty_frame_reply_r_qualifier_omitted_on_missing_image_enoent`
        // regression pin survives.
        ImageError::MissingImage { .. } => {
            term.kitty_respond(&ctx, "ENOENT");
        }
        ImageError::InvalidFrameRef { .. }
        | ImageError::OversizedBlit { .. }
        | ImageError::InvalidFormat
        | ImageError::DecodeFailed(_) => {
            term.kitty_respond(&ctx, &format!("EINVAL: {err}"));
        }
        ImageError::OversizedImage | ImageError::MemoryLimitExceeded => {
            term.kitty_respond(&ctx, &format!("ENOMEM: {err}"));
        }
        ImageError::OverlappingFrames => {
            unreachable!("OverlappingFrames is compose-specific; frame.rs cannot emit it")
        }
    }
}
