//! Kitty graphics `a=f` (animation frame transmit) action.

use std::sync::Arc;

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyCommand;
use crate::image::{CompositionMode, ImageId};
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// Handle `a=f` — transmit an animation frame.
    ///
    /// Adds a frame to an existing image. If the image is not yet
    /// animated, it is promoted to animated (existing data = frame 0).
    /// Key reinterpretation for `a=f`:
    /// - `display_cols` (`c=`) → create/replace frame N
    /// - `display_rows` (`r=`) → edit frame N
    /// - `z_index` (`z=`) → gap in ms before this frame
    /// - `cell_x_offset` (`X=`) → composition mode (0=blend, 1=overwrite)
    pub(super) fn kitty_frame(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let params = self.kitty_finalize_payload(&cmd);
        let image_id = params.image_id;

        if self.image_cache().get_no_touch(ImageId(image_id)).is_none() {
            self.kitty_respond(image_id, cmd.quiet, "ENOENT");
            return;
        }

        // Decode the frame pixel data.
        let (rgba_data, _w, _h) = match Self::kitty_decode_pixels(
            params.payload,
            params.format,
            params.width,
            params.height,
        ) {
            Ok(result) => result,
            Err(msg) => {
                warn!("kitty frame decode failed: {msg}");
                self.kitty_respond(image_id, cmd.quiet, &msg);
                return;
            }
        };

        let gap_ms = cmd.z_index.max(0) as u64;
        let gap = std::time::Duration::from_millis(gap_ms);

        let composition_mode = if cmd.cell_x_offset == 1 {
            CompositionMode::Overwrite
        } else {
            CompositionMode::AlphaBlend
        };

        let frame_data = Arc::new(rgba_data);

        if let Err(e) = self.image_cache_mut().add_animation_frame(
            ImageId(image_id),
            frame_data,
            gap,
            composition_mode,
        ) {
            warn!("kitty frame add failed: {e}");
            self.kitty_respond(image_id, cmd.quiet, &format!("ENOMEM: {e}"));
            return;
        }

        self.kitty_respond(image_id, cmd.quiet, "OK");
    }
}
