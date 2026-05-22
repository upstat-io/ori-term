//! Dispatch for `a=c` Compose — wires `Term::kitty_compose` and reply mapping.

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyCommand;
use crate::image::{ComposeRequest, ImageError, ImageId};
use crate::term::Term;

use super::KittyReplyContext;
use super::compose_keys::extract_a_c_keys;

impl<S: EffectSink> Term<S> {
    /// Handle `a=c` — compose a sub-rect of one frame onto another.
    ///
    /// Reply codes (matching kitty `graphics.c:1820-1880 handle_compose_command`):
    /// - `OK` on successful composition.
    /// - `ENOENT` for missing image, missing source frame, missing dest frame,
    ///   or `r=`/`c=` absent / 0.
    /// - `EINVAL` for out-of-bounds rect or same-frame overlapping rects.
    /// - `ENOMEM` for memory-limit exhaustion (defensive — compose's only
    ///   allocation is the composed buffer, capped by the memory limit).
    ///   Kitty uses `ENOSPC` at `graphics.c:1870` for its disk-cache writeback
    ///   path; this crate has no disk cache, so the RAM-bound `ENOMEM` matches
    ///   the `emit_error_reply` convention in `frame.rs:153`.
    pub(super) fn kitty_compose(&mut self, cmd: &KittyCommand) {
        // ID resolution: `i=` takes priority; fall back to `I=` via
        // `newest_by_image_number` (kitty `graphics.c:2264` accept
        // id-or-image-number). Mirrors `animate.rs:31`.
        let image_id = if let Some(id) = cmd.image_id {
            id
        } else if let Some(id) = cmd.image_number.and_then(|n| {
            self.image_cache()
                .newest_by_image_number(n)
                .map(|ImageId(id)| id)
        }) {
            id
        } else {
            let ctx = KittyReplyContext::from_cmd(cmd);
            self.kitty_respond(&ctx, "ENOENT");
            return;
        };
        let ctx = KittyReplyContext::from_cmd(cmd).with_image_id(image_id);

        // ENOENT precheck before any cache mutation.
        if self.image_cache().get_no_touch(ImageId(image_id)).is_none() {
            self.kitty_respond(&ctx, "ENOENT");
            return;
        }

        let keys = extract_a_c_keys(cmd);
        // r= and c= are mandatory for compose; 0 / absent → ENOENT.
        if keys.src_frame == 0 || keys.dst_frame == 0 {
            self.kitty_respond(&ctx, "ENOENT");
            return;
        }

        let req = ComposeRequest {
            image_id: ImageId(image_id),
            src_frame: keys.src_frame,
            dst_frame: keys.dst_frame,
            width: keys.width,
            height: keys.height,
            src_x: keys.src_x,
            src_y: keys.src_y,
            dst_x: keys.dst_x,
            dst_y: keys.dst_y,
            mode: keys.mode,
        };

        match self.image_cache_mut().compose_frame(req) {
            Ok(()) => self.kitty_respond(&ctx, "OK"),
            Err(e) => emit_compose_error_reply(self, ctx, &e),
        }
    }
}

/// Map compose-path `ImageError` to a kitty reply.
///
/// **Spec mapping notes:**
/// - `MissingImage` / `InvalidFrameRef` → `ENOENT`, matching kitty
///   `graphics.c:1820-1828` (`No source/dest frame number %u exists`).
/// - `OverlappingFrames` / `OversizedBlit` → `EINVAL`, matching kitty
///   `graphics.c:1841-1849` (overlap) + `graphics.c:1833-1840` (bounds).
/// - `OversizedImage` / `MemoryLimitExceeded` → `ENOMEM`. Kitty uses
///   `ENOSPC` at `graphics.c:1870` for its disk-cache writeback failure
///   (`Failed to store image data in disk cache`). This crate has no disk
///   cache — the analogous failure is RAM-bound `MemoryLimitExceeded`,
///   so `ENOMEM` is the correct standard-libc code AND matches the
///   `emit_error_reply` convention in `frame.rs:153`. Conscious divergence
///   from kitty's literal `ENOSPC` reply code; accepted as an
///   internal-consistency choice.
/// - `InvalidFormat` / `DecodeFailed` → `EINVAL` (defensive; compose
///   does not decode pixels — these variants are unreachable from
///   `compose_frame` but the match remains exhaustive).
///
/// Mirrors `emit_error_reply` at `frame.rs:135-159` structurally with two
/// spec-required differences: (1) compose maps `InvalidFrameRef` → ENOENT
/// per `graphics.c:1822` vs frame's EINVAL; (2) compose's `MissingImage`
/// reply includes the image-id detail while frame's emits bare `"ENOENT"`.
/// Documented divergence preferred over closure-parameterized consolidation
/// that would obscure the spec mapping.
fn emit_compose_error_reply<S: EffectSink>(
    term: &mut Term<S>,
    ctx: KittyReplyContext,
    err: &ImageError,
) {
    match err {
        ImageError::MissingImage { .. } | ImageError::InvalidFrameRef { .. } => {
            term.kitty_respond(&ctx, &format!("ENOENT: {err}"));
        }
        ImageError::OverlappingFrames
        | ImageError::OversizedBlit { .. }
        | ImageError::InvalidFormat => {
            term.kitty_respond(&ctx, &format!("EINVAL: {err}"));
        }
        ImageError::OversizedImage | ImageError::MemoryLimitExceeded => {
            term.kitty_respond(&ctx, &format!("ENOMEM: {err}"));
        }
        ImageError::DecodeFailed(_) => {
            warn!("kitty compose: unexpected DecodeFailed: {err}");
            term.kitty_respond(&ctx, &format!("EINVAL: {err}"));
        }
    }
}
