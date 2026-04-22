//! Kitty graphics protocol handler — action dispatch + shared state-staging.
//!
//! Per-action handlers live in submodules (transmit, place, delete, query,
//! frame, animate, response, store). This file owns the APC entry point,
//! the `KittyAction` match, and the chunked-upload + payload-finalization
//! helpers shared across transmit + frame.

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::{
    KittyAction, KittyCommand, KittyTransmission, LoadingImage, parse_kitty_command,
};
use crate::term::Term;

mod animate;
mod delete;
mod frame;
mod place;
mod query;
mod response;
mod store;
mod transmit;

/// Parameters for storing an image via Kitty protocol.
pub(super) struct KittyStoreParams {
    pub(super) image_id: u32,
    pub(super) image_number: Option<u32>,
    pub(super) payload: Vec<u8>,
    pub(super) format: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) transmission: KittyTransmission,
}

impl<S: EffectSink> Term<S> {
    /// Parse and execute a Kitty graphics command.
    pub(super) fn handle_kitty_graphics(&mut self, data: &[u8]) {
        if !self.image_protocol_enabled {
            return;
        }
        let cmd = match parse_kitty_command(data) {
            Ok(cmd) => cmd,
            Err(e) => {
                warn!("kitty graphics parse error: {e}");
                return;
            }
        };

        log::info!(
            "kitty: action={:?} id={:?} pid={:?} payload={}B",
            cmd.action,
            cmd.image_id,
            cmd.placement_id,
            cmd.payload.len(),
        );

        match cmd.action {
            KittyAction::Query => self.kitty_query(&cmd),
            KittyAction::Transmit => self.kitty_transmit(cmd),
            KittyAction::TransmitAndPlace => self.kitty_transmit_and_place(cmd),
            KittyAction::Place => self.kitty_place(&cmd),
            KittyAction::Delete => self.kitty_delete(&cmd),
            KittyAction::Frame => self.kitty_frame(cmd),
            KittyAction::Animate => self.kitty_animate(&cmd),
        }
    }

    /// Finalize payload from accumulated chunks or single command.
    pub(super) fn kitty_finalize_payload(&mut self, cmd: &KittyCommand) -> KittyStoreParams {
        let (payload, format, width, height, transmission, image_number) =
            if let Some(mut loading) = self.loading_image.take() {
                loading.payload.extend_from_slice(&cmd.payload);
                (
                    loading.payload,
                    loading.format,
                    loading.width,
                    loading.height,
                    loading.transmission,
                    loading.image_number,
                )
            } else {
                (
                    cmd.payload.clone(),
                    cmd.format,
                    cmd.source_width,
                    cmd.source_height,
                    cmd.transmission,
                    cmd.image_number,
                )
            };

        let image_id = cmd
            .image_id
            .unwrap_or_else(|| self.image_cache_mut().next_image_id().0);

        KittyStoreParams {
            image_id,
            image_number,
            payload,
            format,
            width,
            height,
            transmission,
        }
    }

    /// Accumulate a chunk for multi-part transmission.
    pub(super) fn kitty_accumulate_chunk(&mut self, cmd: KittyCommand) {
        let max_bytes = self.image_cache().max_single_image_bytes();

        if let Some(ref mut loading) = self.loading_image {
            loading.payload.extend_from_slice(&cmd.payload);
            if loading.payload.len() > max_bytes {
                warn!("kitty chunked transfer exceeds max size, discarding");
                self.loading_image = None;
            }
        } else {
            let image_id = cmd
                .image_id
                .unwrap_or_else(|| self.image_cache_mut().next_image_id().0);
            self.loading_image = Some(LoadingImage {
                image_id,
                image_number: cmd.image_number,
                payload: cmd.payload,
                format: cmd.format,
                width: cmd.source_width,
                height: cmd.source_height,
                compression: cmd.compression,
                transmission: cmd.transmission,
            });
        }
    }
}
