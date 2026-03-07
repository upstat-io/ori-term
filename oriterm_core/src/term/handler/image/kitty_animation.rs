//! Kitty animation handler — `a=f` (frame) and `a=a` (animate) actions.
//!
//! Reinterprets `KittyCommand` fields for animation context, since the
//! Kitty protocol overloads key names across actions. See comments on
//! each method for the key reinterpretation table.

use std::sync::Arc;

use log::{debug, warn};

use crate::event::EventListener;
use crate::image::kitty::KittyCommand;
use crate::image::{CompositionMode, ImageId};
use crate::term::Term;

impl<T: EventListener> Term<T> {
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

    /// Handle `a=a` — control animation playback.
    ///
    /// Key reinterpretation for `a=a`:
    /// - `source_width` (`s=`) → action (1=stop, 2=run wait, 3=run)
    /// - `display_rows` (`r=`) → set current frame
    /// - `z_index` (`z=`) → set gap for current frame (ms)
    /// - `display_cols` (`c=`) → set displayed frame
    /// - `source_height` (`v=`) → loop count (0=infinite)
    pub(super) fn kitty_animate(&mut self, cmd: &KittyCommand) {
        let Some(image_id) = cmd.image_id else {
            debug!("kitty animate: no image_id");
            return;
        };

        let id = ImageId(image_id);

        // `s=` → animation action.
        if cmd.source_width > 0 {
            self.image_cache_mut()
                .set_animation_action(id, cmd.source_width);
        }

        // `v=` → loop count.
        if cmd.source_height > 0 {
            self.image_cache_mut()
                .set_animation_loops(id, cmd.source_height);
        }

        // `r=` → set current frame (1-based in Kitty protocol).
        if let Some(frame) = cmd.display_rows {
            if frame > 0 {
                self.image_cache_mut()
                    .set_current_frame(id, (frame - 1) as usize);
            }
        }

        // `c=` → set displayed frame (1-based).
        if let Some(frame) = cmd.display_cols {
            if frame > 0 {
                self.image_cache_mut()
                    .set_current_frame(id, (frame - 1) as usize);
            }
        }

        // `z=` → set gap for current frame.
        if cmd.z_index > 0 {
            let gap = std::time::Duration::from_millis(cmd.z_index as u64);
            if let Some(state) = self.image_cache().animation_state(id) {
                let frame_idx = state.current_frame;
                self.image_cache_mut().set_frame_gap(id, frame_idx, gap);
            }
        }

        self.kitty_respond(image_id, cmd.quiet, "OK");
    }
}
