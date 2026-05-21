//! Kitty graphics `a=a` (animation playback control) action.

use crate::effect::sink::EffectSink;
use crate::image::ImageId;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

use super::KittyReplyContext;

impl<S: EffectSink> Term<S> {
    /// Handle `a=a` — control animation playback.
    ///
    /// Key reinterpretation for `a=a` per kitty graphics-protocol.rst §Animation
    /// control + graphics.c:1729-1743:
    /// - `source_width` (`s=`) → action (1=stop, 2=run wait, 3=run)
    /// - `display_rows` (`r=`) → gap-target frame selector (paired with `z=`)
    /// - `z_index` (`z=`) → gap value in ms; consumed only via `r=`; negative
    ///   clamps to ZERO (gapless); 0 means no gap change
    /// - `display_cols` (`c=`) → current-frame seek (1-based)
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

        // `r=` + `z=` → set gap of frame r (gap-target selector). Per kitty
        // graphics.c:1729-1735, z= is consumed only via r=; standalone z= is
        // a no-op. z=0 means "no gap change" (kitty's `if (g->gap)` guard at
        // 1734); negative z= clamps to ZERO (gapless edit, per change_gap at
        // graphics.c:1348-1350).
        if let Some(frame) = cmd.display_rows {
            if frame > 0 && cmd.z_index != 0 {
                let gap = if cmd.z_index < 0 {
                    std::time::Duration::ZERO
                } else {
                    std::time::Duration::from_millis(cmd.z_index as u64)
                };
                self.image_cache_mut()
                    .set_frame_gap(id, (frame - 1) as usize, gap);
            }
        }

        // `c=` → seek to frame c (current-frame selector). Per kitty
        // graphics.c:1737-1743, c=N updates current_frame_index when in range.
        if let Some(frame) = cmd.display_cols {
            if frame > 0 {
                self.image_cache_mut()
                    .set_current_frame(id, (frame - 1) as usize);
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
