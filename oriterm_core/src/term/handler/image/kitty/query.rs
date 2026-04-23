//! Kitty graphics `a=q` (query) action.

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

use super::KittyReplyContext;

impl<S: EffectSink> Term<S> {
    /// Query: send OK response without modifying state.
    pub(super) fn kitty_query(&self, cmd: &KittyCommand) {
        // `kitty_respond` echoes whichever identifier(s) the sender
        // provided (`i=`, `I=`, or both); when neither is present it
        // falls back to the `i=0` sentinel — ori_term deviation from
        // kitty's suppress-on-un-correlatable behavior, pinned
        // `verified-with-deviation` in the §13.1/§13.2 catalog rows.
        self.kitty_respond(&KittyReplyContext::from_cmd(cmd), "OK");
    }
}
