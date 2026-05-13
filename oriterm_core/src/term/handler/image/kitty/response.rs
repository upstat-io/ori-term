//! Kitty graphics response emission — protocol reply framing.

use std::fmt::Write as _;

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::image::kitty::KittyCommand;
use crate::term::Term;

/// Reply identifiers + echoed metadata sourced from the request's control
/// data. Matches kitty's `finish_command_response` reply-framing contract:
/// echoes every identifier/metadata the request provided so the client can
/// correlate the reply to the originating command. Replaces an earlier 4-ary
/// `kitty_respond` signature that outgrew its call sites (4+ params
/// → config struct).
#[derive(Debug, Default, Clone, Copy)]
pub(in crate::term::handler::image::kitty) struct KittyReplyContext {
    /// Image id from `i=`; `0` means absent.
    pub image_id: u32,
    /// Image number from `I=`; `None` means absent.
    pub image_number: Option<u32>,
    /// Placement id from `p=`; `None` means absent (echoed by kitty for `a=p`).
    pub placement_id: Option<u32>,
    /// Frame number echoed by `a=f` / `a=a` replies; `None` means absent.
    pub frame_num: Option<u32>,
    /// Quiet level from `q=` (0=all replies, 1=suppress OK, 2=suppress all).
    pub quiet: u8,
}

impl KittyReplyContext {
    /// Build a reply context from a request command, extracting the five
    /// fields kitty's `finish_command_response` cares about.
    pub(in crate::term::handler::image::kitty) fn from_cmd(cmd: &KittyCommand) -> Self {
        Self {
            image_id: cmd.image_id.unwrap_or(0),
            image_number: cmd.image_number,
            placement_id: cmd.placement_id,
            frame_num: None,
            quiet: cmd.quiet,
        }
    }

    /// Override the resolved `image_id` (used by callers that auto-assign
    /// after a chunked transmit or place a pre-existing image).
    pub(in crate::term::handler::image::kitty) fn with_image_id(mut self, image_id: u32) -> Self {
        self.image_id = image_id;
        self
    }

    /// Override the frame number — used by `a=f` (newly-added frame index,
    /// 1-based) and `a=a` (current frame after state mutation, 1-based) per
    /// the response shape in kitty `finish_command_response` at `graphics.c:802-806`.
    pub(in crate::term::handler::image::kitty) fn with_frame_num(mut self, frame_num: u32) -> Self {
        self.frame_num = Some(frame_num);
        self
    }
}

impl<S: EffectSink> Term<S> {
    /// Send a Kitty graphics response.
    ///
    /// Echoes `i=<image_id>`, `I=<image_number>`, `,p=<placement_id>`, and
    /// `,r=<frame_num>` when the request provided them. When neither `i=`
    /// nor `I=` is present, `ori_term` emits the `i=0` sentinel so clients
    /// still receive a correlatable EINVAL / ENOENT on error paths — a
    /// documented deviation from kitty's `finish_command_response`
    /// (which suppresses replies for un-correlatable requests); the
    /// deviation is pinned by the §13.1/§13.2 catalog rows as
    /// `verified-with-deviation`.
    pub(super) fn kitty_respond(&self, ctx: &KittyReplyContext, msg: &str) {
        if ctx.quiet >= 2 {
            return;
        }
        if ctx.quiet >= 1 && msg == "OK" {
            return;
        }

        // Build `i=<id>` / `I=<num>` / `i=<id>,I=<num>` head. When both
        // identifiers are absent, fall back to the `i=0` sentinel — the
        // ori_term deviation that preserves a correlatable-but-synthetic
        // id for client recovery paths.
        let head = match (ctx.image_id, ctx.image_number) {
            (0, Some(n)) => format!("I={n}"),
            (id, Some(n)) => format!("i={id},I={n}"),
            (id, None) => format!("i={id}"),
        };
        // Append `,p=<pid>` and `,r=<frame>` when present.
        let mut qualifiers = String::new();
        if let Some(pid) = ctx.placement_id {
            let _ = write!(qualifiers, ",p={pid}");
        }
        if let Some(frame) = ctx.frame_num {
            let _ = write!(qualifiers, ",r={frame}");
        }
        let response = format!("\x1b_G{head}{qualifiers};{msg}\x1b\\");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::ImageProtocolReply,
        }));
    }
}
