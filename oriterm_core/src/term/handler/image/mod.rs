//! Image protocol handler implementations (Kitty, Sixel, iTerm2).
//!
//! Routes APC/DCS/OSC sequences to the appropriate image protocol
//! parser and executes commands against the `ImageCache`.

mod iterm2;
mod kitty;
mod sixel;

use log::debug;

use crate::effect::sink::EffectSink;
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// Handle an APC sequence dispatched by the VTE parser.
    ///
    /// The first byte identifies the protocol: `G` = Kitty graphics.
    pub(in crate::term::handler) fn handle_apc_dispatch(&mut self, payload: &[u8]) {
        log::info!(
            "apc_dispatch: {} bytes, first={:?}, head={:?}",
            payload.len(),
            payload.first().map(|b| *b as char),
            String::from_utf8_lossy(&payload[..payload.len().min(32)])
        );
        if payload.is_empty() {
            return;
        }

        match payload[0] {
            b'G' => self.handle_kitty_graphics(&payload[1..]),
            _ => debug!("unknown APC command: {:?}", payload[0] as char),
        }
    }
}
