//! Kitty graphics `a=a` (animation playback control) action.

use log::debug;

use crate::effect::sink::EffectSink;
use crate::image::ImageId;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

impl<S: EffectSink> Term<S> {
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
