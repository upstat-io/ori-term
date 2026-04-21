//! Kitty graphics `a=q` (query) action.

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// Query: send OK response without modifying state.
    pub(super) fn kitty_query(&self, cmd: &KittyCommand) {
        let id = cmd.image_id.unwrap_or(0);
        self.kitty_respond(id, cmd.quiet, "OK");
    }
}
