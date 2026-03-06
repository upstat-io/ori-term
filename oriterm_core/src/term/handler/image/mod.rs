//! Image protocol handler implementations (Kitty, Sixel, iTerm2).
//!
//! Routes APC/DCS/OSC sequences to the appropriate image protocol
//! parser and executes commands against the `ImageCache`.

mod iterm2;
mod kitty;
mod sixel;

use log::debug;

use crate::event::EventListener;
use crate::term::Term;

impl<T: EventListener> Term<T> {
    /// Handle an APC sequence dispatched by the VTE parser.
    ///
    /// The first byte identifies the protocol: `G` = Kitty graphics.
    pub(in crate::term::handler) fn handle_apc_dispatch(&mut self, payload: &[u8]) {
        if payload.is_empty() {
            return;
        }

        match payload[0] {
            b'G' => self.handle_kitty_graphics(&payload[1..]),
            _ => debug!("unknown APC command: {:?}", payload[0] as char),
        }
    }
}
