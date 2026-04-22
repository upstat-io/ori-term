//! Kitty graphics response emission — protocol reply framing.

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// Send a Kitty graphics response.
    pub(super) fn kitty_respond(&self, image_id: u32, quiet: u8, msg: &str) {
        if quiet >= 2 {
            return;
        }
        if quiet >= 1 && msg == "OK" {
            return;
        }

        let response = format!("\x1b_Gi={image_id};{msg}\x1b\\");
        self.effect_sink.push(Effect::Pty(PtyEffect::Write {
            bytes: response.into_bytes(),
            kind: PtyWriteKind::ImageProtocolReply,
        }));
    }
}
