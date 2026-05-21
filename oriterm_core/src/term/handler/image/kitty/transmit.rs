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

        if let Err(err) = self.kitty_store_image(params) {
            let msg = err.to_string();
            warn!("kitty transmit failed: {msg}");
            self.kitty_respond(&ctx, &msg);
            return;
        }

        // U=1: store anchors the image so LRU eviction doesn't drop it
        // before the program writes the placeholder cells. When `c=N,r=M`
        // accompany the U=1 transmit, record the display grid so the GPU
        // emit path can compute per-cell UV slices for the placeholder
        // cells. See §13.6.1 multi-cell UV slicing.
        if unicode_placeholder {
            let id = ImageId::from_raw(image_id);
            let grid = match (merged.display_cols, merged.display_rows) {
                (Some(cols), Some(rows)) => Some((cols, rows)),
                _ => None,
            };
            self.image_cache_mut()
                .anchor_placeholder_with_grid(id, grid);
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

        if let Err(err) = self.kitty_store_image(params) {
            let msg = err.to_string();
            warn!("kitty transmit+place failed: {msg}");
            self.kitty_respond(&ctx, &msg);
            return;
        }

        // U=1: image stored but placement deferred to unicode placeholder
        // chars (U+10EEEE) that the program writes into cells. Anchor the
        // image so LRU eviction doesn't drop it. Inherits from the first
        // chunk's command per kitty's control-key inheritance rules. When
        // `c=N,r=M` are present, record the placeholder display grid so
        // the GPU emit path can slice UVs per cell.
        if merged.unicode_placeholder {
            let id = ImageId::from_raw(image_id);
            let grid = match (merged.display_cols, merged.display_rows) {
                (Some(cols), Some(rows)) => Some((cols, rows)),
                _ => None,
            };
            self.image_cache_mut()
                .anchor_placeholder_with_grid(id, grid);
        } else {
            self.kitty_create_placement(image_id, &merged);
        }
        self.kitty_respond(&ctx, "OK");
    }
}
