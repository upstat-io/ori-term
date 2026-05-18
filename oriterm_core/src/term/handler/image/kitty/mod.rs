//! Kitty graphics protocol handler — action dispatch + shared state-staging.
//!
//! Per-action handlers live in submodules (transmit, place, delete, query,
//! frame, animate, response, store). This file owns the APC entry point,
//! the `KittyAction` match, and the chunked-upload + payload-finalization
//! helpers shared across transmit + frame.

use log::warn;

use crate::effect::sink::EffectSink;
use crate::image::kitty::{
    KittyAction, KittyCommand, KittyError, KittyTransmission, LoadingImage,
    parse_kitty_command_into,
};
use crate::term::Term;

mod animate;
mod delete;
mod frame;
mod place;
pub(crate) mod placeholder;
mod query;
mod response;
mod store;
mod transmit;

pub(in crate::term::handler::image::kitty) use response::KittyReplyContext;

/// Parameters for storing an image via Kitty protocol.
pub(super) struct KittyStoreParams {
    pub(super) image_id: u32,
    pub(super) image_number: Option<u32>,
    pub(super) payload: Vec<u8>,
    pub(super) format: u32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) transmission: KittyTransmission,
    /// Compression flag from `o=` (`Some(b'z')` for zlib). §13.5 guard at
    /// `kitty_store_image` entry rejects `Some(b'z')` with EINVAL —
    /// preserves the parsed value so the guard can fire without re-reading
    /// the source command.
    pub(super) compression: Option<u8>,
}

impl KittyStoreParams {
    /// Build storage parameters from a merged command + resolved `image_id`.
    ///
    /// Consumes the payload out of `cmd` so the caller can keep `cmd`
    /// around for subsequent placement / reply steps without cloning the
    /// pixel buffer.
    pub(super) fn from_merged(image_id: u32, cmd: &mut KittyCommand) -> Self {
        Self {
            image_id,
            image_number: cmd.image_number,
            payload: std::mem::take(&mut cmd.payload),
            format: cmd.format,
            width: cmd.source_width,
            height: cmd.source_height,
            transmission: cmd.transmission,
            compression: cmd.compression,
        }
    }
}

impl<S: EffectSink> Term<S> {
    /// Parse and execute a Kitty graphics command.
    pub(super) fn handle_kitty_graphics(&mut self, data: &[u8]) {
        if !self.image_protocol_enabled {
            return;
        }
        let mut cmd = KittyCommand::default();
        if let Err(e) = parse_kitty_command_into(&mut cmd, data) {
            warn!("kitty graphics parse error: {e}");
            // Option A (§13.2): malformed base64 payloads get a spec-
            // aligned EINVAL reply so clients can recover. Control-data
            // parsing runs BEFORE base64 decode in parse_kitty_command_into,
            // so a partial cmd carries any i=/I=/q= the sender included —
            // use them to echo the image_id kitty clients expect. Fall
            // back to i=0 only when the sender omitted i=. Other parse
            // errors (InvalidControlData / UnsupportedFormat — the latter
            // is currently unreachable from parse_kitty_command_into)
            // retain the silent-drop behavior pending §13.5's reply-point
            // inventory.
            if matches!(e, KittyError::InvalidBase64) {
                self.handle_invalid_base64_chunk(&cmd);
            }
            return;
        }

        // Per-command trace: high-frequency under chunked kitty transmits
        // (notcurses-demo emits thousands per graphics scene). Kept at
        // debug level so default INFO logging doesn't pay sync-write
        // cost per chunk on the IO thread. Enable via
        // RUST_LOG=oriterm_core::term::handler::image::kitty=debug.
        log::debug!(
            target: "oriterm_core::term::handler::image::kitty",
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
            KittyAction::Compose => self.kitty_compose_reject(&cmd),
        }
    }

    /// Reject `a=c` Compose action with an `EINVAL: action `c` not implemented`
    /// reply. The reject helper makes the missing handler observable to
    /// clients via a spec-compliant EINVAL reply while keeping the catalog
    /// row `KG-ACTION-COMPOSE` honest about the absent implementation.
    pub(super) fn kitty_compose_reject(&self, cmd: &KittyCommand) {
        let ctx = KittyReplyContext::from_cmd(cmd);
        self.kitty_respond(&ctx, "EINVAL: action `c` not implemented");
    }

    /// Handle a malformed-base64 chunk: emit EINVAL once per failed upload
    /// and update the per-upload failure latch so subsequent malformed
    /// chunks of the same upload are suppressed. §13.5 EINVAL-flood
    /// coalesce: pre-fix, each malformed chunk fired its own EINVAL,
    /// amplifying the reply transcript proportionally to chunk count.
    /// `LoadingImage::failed_upload` is the canonical latch — keyed by
    /// `image_id` when the sender provides one, or by the auto-assigned id
    /// when `LoadingImage` was created on the first (valid) chunk.
    fn handle_invalid_base64_chunk(&mut self, cmd: &KittyCommand) {
        let already_failed = self
            .loading_image
            .as_ref()
            .is_some_and(|li| li.failed_upload);
        if already_failed {
            // Terminator chunk (`m=0`) of an already-failed upload: drop
            // the loading_image so a future upload with the same image_id
            // starts fresh. Non-terminator chunks remain dropped silently.
            if !cmd.more_data {
                self.loading_image = None;
            }
            return;
        }

        // Emit an EINVAL reply so the client can recover.
        // `KittyReplyContext::from_cmd` carries whatever `i=` and/or `I=`
        // the sender included before the base64 decode failed; when neither
        // is present, `kitty_respond` falls back to the `i=0` sentinel.
        self.kitty_respond(
            &KittyReplyContext::from_cmd(cmd),
            "EINVAL: base64 decode failed",
        );

        // Mark the in-flight upload (if any) as failed so subsequent
        // malformed chunks of the same upload do not re-emit. If the first
        // chunk itself was malformed and no `LoadingImage` exists yet,
        // synthesize one carrying the failure latch keyed on the sender-
        // provided image_id, so chunks 2+ of the same failed upload are
        // also coalesced. Synthesize only when the sender supplied an `i=`
        // AND `m=1` (continuation marker), so a single-shot non-chunked
        // malformed transmit doesn't leave dangling state.
        if let Some(loading) = self.loading_image.as_mut() {
            loading.failed_upload = true;
            return;
        }
        if let Some(id) = cmd.image_id
            && cmd.more_data
        {
            self.loading_image = Some(LoadingImage {
                image_id: id,
                start_cmd: KittyCommand::default(),
                failed_upload: true,
            });
        }
    }

    /// Finalize a chunked or single-APC transmission into a merged command.
    ///
    /// Returns `(resolved_image_id, merged_cmd)`. When a chunked upload is
    /// in progress, the merged cmd is the FIRST chunk's command with the
    /// final chunk's payload appended to the accumulator — preserving
    /// first-chunk control keys (`q=`, `U=`, `C=`, placement geometry,
    /// `z=`) per kitty's protocol contract that subsequent chunks carry
    /// only `m=` + payload. When no `loading_image` exists, `cmd` is
    /// returned directly with `image_id` resolved via auto-assignment if
    /// the sender omitted `i=`.
    pub(super) fn kitty_finalize_payload(&mut self, mut cmd: KittyCommand) -> (u32, KittyCommand) {
        if let Some(mut loading) = self.loading_image.take() {
            // Move the terminal (m=0) chunk's payload into the accumulator;
            // earlier chunks already landed in start_cmd.payload via
            // kitty_accumulate_chunk. `Vec::append` drains `cmd.payload`
            // in place, avoiding the separate slice-length bookkeeping
            // `extend_from_slice` would perform on an owned source.
            loading.start_cmd.payload.append(&mut cmd.payload);
            return (loading.image_id, loading.start_cmd);
        }
        if cmd.image_id.is_none() {
            let resolved = self.image_cache_mut().next_image_id().0;
            cmd.image_id = Some(resolved);
            return (resolved, cmd);
        }
        (cmd.image_id.expect("checked above"), cmd)
    }

    /// Accumulate a chunk for multi-part transmission.
    ///
    /// First chunk (`loading_image` absent): stores the full command as
    /// `start_cmd` so first-chunk control keys survive to finalize.
    /// Subsequent chunks append `cmd.payload` into `start_cmd.payload`;
    /// all other keys on subsequent chunks are discarded per the protocol
    /// — only `m=` and payload are semantically meaningful after the
    /// first chunk.
    pub(super) fn kitty_accumulate_chunk(&mut self, mut cmd: KittyCommand) {
        let max_bytes = self.image_cache().max_single_image_bytes();

        if let Some(ref mut loading) = self.loading_image {
            loading.start_cmd.payload.append(&mut cmd.payload);
            if loading.start_cmd.payload.len() > max_bytes {
                warn!("kitty chunked transfer exceeds max size, discarding");
                self.loading_image = None;
            }
        } else {
            let image_id = cmd
                .image_id
                .unwrap_or_else(|| self.image_cache_mut().next_image_id().0);
            self.loading_image = Some(LoadingImage {
                image_id,
                start_cmd: cmd,
                failed_upload: false,
            });
        }
    }
}
