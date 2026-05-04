//! Kitty graphics `a=a` (animation playback control) action.

use crate::effect::sink::EffectSink;
use crate::image::ImageId;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

use super::KittyReplyContext;

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
        // : missing `i=` MUST emit ENOENT, mirroring `kitty_place`
        // (place.rs:16-19). Additionally support `I=` (image_number) fallback
        // per kitty's graphics.c::handle_animate_command — when no `i=` is
        // given, try resolving via `newest_by_image_number(I)` before
        // emitting ENOENT.
        let ctx = KittyReplyContext::from_cmd(cmd);
        let image_id = if let Some(id) = cmd.image_id {
            id
        } else if let Some(id) = cmd.image_number.and_then(|n| {
            self.image_cache()
                .newest_by_image_number(n)
                .map(|ImageId(id)| id)
        }) {
            id
        } else {
            self.kitty_respond(&ctx, "ENOENT");
            return;
        };

        let id = ImageId(image_id);

        // `s=` → animation action.
        if cmd.source_width > 0 {
            self.image_cache_mut()
                .set_animation_action(id, cmd.source_width);
        }

        // `v=` → loop count. `loop_count: Option<u32>` distinguishes "key
        // absent" (None, leave unchanged) from "v=0" (Some(0), infinite
        // loops per kitty graphics-protocol.rst §Animation control) —
        // source_height: u32 alone cannot tell these apart.
        if let Some(loops) = cmd.loop_count {
            self.image_cache_mut().set_animation_loops(id, loops);
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

        // kitty finish_command_response (graphics.c:802-806) echoes `,r=<frame_num>`
        // on a=a replies with the post-mutation current frame (1-based) so the
        // client can correlate the OK to the resulting animation state.
        let mut ctx = KittyReplyContext::from_cmd(cmd).with_image_id(image_id);
        if let Some(state) = self.image_cache().animation_state(id) {
            ctx = ctx.with_frame_num(state.current_frame as u32 + 1);
        }
        self.kitty_respond(&ctx, "OK");
    }
}
