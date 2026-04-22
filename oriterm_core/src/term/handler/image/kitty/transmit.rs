//! Kitty graphics `a=t` (transmit) and `a=T` (transmit + place) actions.

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

impl<S: EffectSink> Term<S> {
    /// Transmit: upload image data (possibly chunked).
    pub(super) fn kitty_transmit(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let params = self.kitty_finalize_payload(&cmd);
        let image_id = params.image_id;

        if let Err(msg) = self.kitty_store_image(params) {
            warn!("kitty transmit failed: {msg}");
            self.kitty_respond(image_id, cmd.quiet, &msg);
        } else {
            self.kitty_respond(image_id, cmd.quiet, "OK");
        }
    }

    /// Transmit and place in one step.
    pub(super) fn kitty_transmit_and_place(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let params = self.kitty_finalize_payload(&cmd);
        let image_id = params.image_id;

        if let Err(msg) = self.kitty_store_image(params) {
            warn!("kitty transmit+place failed: {msg}");
            self.kitty_respond(image_id, cmd.quiet, &msg);
            return;
        }

        // U=1: image stored but placement deferred to unicode placeholder
        // chars (U+10EEEE) that the program writes into cells.
        if !cmd.unicode_placeholder {
            self.kitty_create_placement(image_id, &cmd);
        }
        self.kitty_respond(image_id, cmd.quiet, "OK");
    }
}
