//! Kitty graphics `a=f` (animation frame transmit) action.

use std::sync::Arc;

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyCommand;
use crate::image::{CompositionMode, ImageId};
use crate::term::Term;

use super::KittyReplyContext;

impl<S: EffectSink> Term<S> {
    /// Handle `a=f` — transmit an animation frame.
    ///
    /// Adds a frame to an existing image. If the image is not yet
    /// animated, it is promoted to animated (existing data = frame 0).
    /// Key reinterpretation for `a=f`:
    /// - `display_cols` (`c=`) → PARSED-BUT-IGNORED; per kitty spec
    ///   selects frame-replace semantics for frame N. Current
    ///   dispatch always appends.
    /// - `display_rows` (`r=`) → PARSED-BUT-IGNORED; per kitty spec
    ///   selects frame-edit semantics for frame N. Current dispatch
    ///   always appends.
    /// - `z_index` (`z=`) → gap in ms before this frame
    /// - `cell_x_offset` (`X=`) → composition mode (0=blend, 1=overwrite)
    ///
    /// See: bug-tracker/plans/BUG-08-041/
    pub(super) fn kitty_frame(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let (image_id, mut merged) = self.kitty_finalize_payload(cmd);
        let ctx = KittyReplyContext::from_cmd(&merged).with_image_id(image_id);

        if self.image_cache().get_no_touch(ImageId(image_id)).is_none() {
            self.kitty_respond(&ctx, "ENOENT");
            return;
        }

        // Decode the frame pixel data using first-chunk format/dimensions.
        let payload = std::mem::take(&mut merged.payload);
        let (rgba_data, _w, _h) = match Self::kitty_decode_pixels(
            payload,
            merged.format,
            merged.source_width,
            merged.source_height,
        ) {
            Ok(result) => result,
            Err(msg) => {
                warn!("kitty frame decode failed: {msg}");
                self.kitty_respond(&ctx, &msg);
                return;
            }
        };

        let gap_ms = merged.z_index.max(0) as u64;
        let gap = std::time::Duration::from_millis(gap_ms);

        let composition_mode = if merged.cell_x_offset == 1 {
            CompositionMode::Overwrite
        } else {
            CompositionMode::AlphaBlend
        };

        let frame_data = Arc::new(rgba_data);

        let frame_num = match self.image_cache_mut().add_animation_frame(
            ImageId(image_id),
            frame_data,
            gap,
            composition_mode,
        ) {
            Ok(idx) => idx,
            Err(e) => {
                warn!("kitty frame add failed: {e}");
                self.kitty_respond(&ctx, &format!("ENOMEM: {e}"));
                return;
            }
        };

        // kitty finish_command_response (graphics.c:802-806) echoes `,r=<frame_num>`
        // on a=f replies so the client can correlate the OK to the added frame.
        let ctx = ctx.with_frame_num(frame_num);
        self.kitty_respond(&ctx, "OK");
    }
}
