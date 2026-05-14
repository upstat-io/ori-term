//! Kitty graphics `a=t` (transmit) and `a=T` (transmit + place) actions.

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::ImageId;
use crate::image::kitty::KittyCommand;
use crate::term::Term;

use super::{KittyReplyContext, KittyStoreParams};

impl<S: EffectSink> Term<S> {
    /// Transmit: upload image data (possibly chunked).
    pub(super) fn kitty_transmit(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let (image_id, mut merged) = self.kitty_finalize_payload(cmd);
        let ctx = KittyReplyContext::from_cmd(&merged).with_image_id(image_id);
        let unicode_placeholder = merged.unicode_placeholder;
        let params = KittyStoreParams::from_merged(image_id, &mut merged);

        if let Err(msg) = self.kitty_store_image(params) {
            warn!("kitty transmit failed: {msg}");
            self.kitty_respond(&ctx, &msg);
            return;
        }

        // U=1: store anchors the image so LRU eviction doesn't drop it
        // before the program writes the placeholder cells.
        if unicode_placeholder {
            self.image_cache_mut()
                .add_placeholder_anchor(ImageId::from_raw(image_id));
        }
        self.kitty_respond(&ctx, "OK");
    }

    /// Transmit and place in one step.
    pub(super) fn kitty_transmit_and_place(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let (image_id, mut merged) = self.kitty_finalize_payload(cmd);
        let ctx = KittyReplyContext::from_cmd(&merged).with_image_id(image_id);
        let params = KittyStoreParams::from_merged(image_id, &mut merged);

        if let Err(msg) = self.kitty_store_image(params) {
            warn!("kitty transmit+place failed: {msg}");
            self.kitty_respond(&ctx, &msg);
            return;
        }

        // U=1: image stored but placement deferred to unicode placeholder
        // chars (U+10EEEE) that the program writes into cells. Anchor the
        // image so LRU eviction doesn't drop it. Inherits from the first
        // chunk's command per kitty's control-key inheritance rules.
        if merged.unicode_placeholder {
            self.image_cache_mut()
                .add_placeholder_anchor(ImageId::from_raw(image_id));
        } else {
            self.kitty_create_placement(image_id, &merged);
        }
        self.kitty_respond(&ctx, "OK");
    }
}
