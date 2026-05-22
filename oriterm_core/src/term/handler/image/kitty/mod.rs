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
mod compose;
mod compose_keys;
mod delete;
mod frame;
mod frame_keys;
mod place;
pub(crate) mod placeholder;
pub(crate) mod prepare;
mod query;
mod response;
mod store;
mod transmit;

pub(crate) use prepare::prepare_image_bytes;

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

        // Continuation/terminator chunks of a chunked upload carry only
        // `m=` + payload per kitty spec; their `a=` field is absent, so
        // `decode_action(None)` defaults to `TransmitAndPlace`. The
        // terminator chunk (`m=0`) then dispatches under that defaulted
        // action and double-creates a placement for uploads whose first
        // chunk was `a=t` (Transmit-only) — notcurses-info trips this
        // every run, producing the duplicate wordmark visible in
        // operator-side runs. Spec-aligned cure: when a chunked upload
        // is in progress, the terminator chunk inherits the first
        // chunk's action.
        if !cmd.more_data
            && let Some(loading) = self.loading_image.as_ref()
        {
            cmd.action = loading.start_cmd.action;
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

        // Mint per-command sequence id. kitty_respond reads this via
        // current_command_seq to route the reply through the sequencer in
        // command-issue order. Sync arms emit a Ready slot inline; async
        // arms (Transmit / Frame) push a Pending placeholder + emit
        // Effect::ImageDecode. Resolved later via apply_decoded_image.
        let seq = self.next_kitty_seq();
        self.set_current_command_seq(Some(seq));
        match cmd.action {
            KittyAction::Query => self.kitty_query(&cmd),
            KittyAction::Transmit => self.kitty_transmit(cmd),
            KittyAction::TransmitAndPlace => self.kitty_transmit_and_place(cmd),
            KittyAction::Place => self.kitty_place(&cmd),
            KittyAction::Delete => self.kitty_delete(&cmd),
            KittyAction::Frame => self.kitty_frame(cmd),
            KittyAction::Animate => self.kitty_animate(&cmd),
            KittyAction::Compose => self.kitty_compose(&cmd),
        }
        self.set_current_command_seq(None);
        // Drain ready slots from the sequencer head. Stops at the first
        // Pending slot (head-of-line blocking until that async resolves).
        self.flush_pending_replies_mut();
    }

    /// Apply a worker-thread decode result onto Term state and emit the
    /// corresponding kitty reply. Called by the IO thread (in `oriterm_mux`)
    /// after draining a result from the `ImageWorker` result channel. The
    /// sequencer (Phase 4 — to be wired alongside Transmit/Frame async-dispatch
    /// refactor) ensures replies emit in command-issue order even when async
    /// (worker) and synchronous (Query / Place) commands interleave.
    pub fn apply_decoded_image(
        &mut self,
        result: crate::image::worker_pipeline::ImageDecodeResult,
    ) {
        use super::super::image::kitty::response::KittyReplyContext;
        use crate::image::worker_pipeline::ImageDecodeError;
        use crate::image::{ImageData, ImageFormat, ImageId};
        use std::sync::Arc;

        let seq = result.sequence_id;
        let image_id = result.image_id;

        // Tombstone check: record_decode_applied returns false when the
        // (image_id, seq) pair was NOT in pending_image_decodes — meaning
        // Delete cleared the entire image_id entry between enqueue and
        // apply. Resolve sequencer slot to None (silent advance) and drop
        // store + placement work.
        let tombstoned = !self.record_decode_applied(image_id, seq);
        if tombstoned {
            self.resolve_pending_reply(seq, None);
            self.deferred_placements.remove(&image_id);
            log::debug!("dropped tombstoned decode result image_id={image_id} seq={seq}");
            return;
        }

        // Build the kitty reply context from the worker-pipeline shape
        // (DecodeReplyContext is the cross-crate seam; KittyReplyContext
        // is the kitty-private form used by kitty_respond's framing).
        let ctx = KittyReplyContext {
            image_id: result.reply_ctx.image_id,
            image_number: result.reply_ctx.image_number,
            placement_id: result.reply_ctx.placement_id,
            frame_num: result.reply_ctx.frame_num,
            quiet: result.reply_ctx.quiet,
        };

        let reply_effect = match result.decoded {
            Ok(decoded) => {
                let decoded_w = decoded.width;
                let decoded_h = decoded.height;
                let cell_w = self.cell_pixel_width.max(1) as u32;
                let cell_h = self.cell_pixel_height.max(1) as u32;
                let img = ImageData {
                    id: ImageId(image_id),
                    width: decoded_w,
                    height: decoded_h,
                    data: Arc::new(decoded.rgba_bytes),
                    pixel_generation: 0,
                    format: ImageFormat::Rgba,
                    source: decoded.source,
                    last_accessed: 0,
                    image_number: result.reply_ctx.image_number,
                };
                let store_result = self.image_cache_mut().store(img);
                match store_result {
                    Ok(_) => {
                        // Apply the placement that came with the request
                        // (a=T transmit-and-place) if present.
                        if let Some(params) = result.placement.clone() {
                            let placement = make_placement_from_params(
                                ImageId(image_id),
                                &params,
                                &DecodedDims {
                                    img_w: decoded_w,
                                    img_h: decoded_h,
                                    cell_w,
                                    cell_h,
                                },
                            );
                            self.image_cache_mut().place(placement);
                        }
                        // Apply any deferred placements (Place commands that
                        // arrived before the decode completed).
                        let deferred = self.take_deferred_placements(image_id);
                        for params in deferred {
                            let placement = make_placement_from_params(
                                ImageId(image_id),
                                &params,
                                &DecodedDims {
                                    img_w: decoded_w,
                                    img_h: decoded_h,
                                    cell_w,
                                    cell_h,
                                },
                            );
                            self.image_cache_mut().place(placement);
                        }
                        build_reply_effect(&ctx, "OK")
                    }
                    Err(e) => {
                        self.deferred_placements.remove(&image_id);
                        build_reply_effect(&ctx, &format!("ENOMEM: {e}"))
                    }
                }
            }
            Err(ImageDecodeError::Reply(msg)) => {
                self.deferred_placements.remove(&image_id);
                build_reply_effect(&ctx, &msg)
            }
            Err(ImageDecodeError::Panicked { message }) => {
                self.deferred_placements.remove(&image_id);
                build_reply_effect(&ctx, &format!("EINVAL: internal decode failure: {message}"))
            }
            Err(ImageDecodeError::EnqueueOverflow) => {
                self.deferred_placements.remove(&image_id);
                build_reply_effect(&ctx, "ENOMEM: image decode queue full")
            }
            Err(ImageDecodeError::EnqueueWorkerDead) => {
                self.deferred_placements.remove(&image_id);
                build_reply_effect(&ctx, "EINVAL: image worker unavailable")
            }
        };

        // Resolve the sequencer slot with the materialized reply (skip
        // emission if reply_effect is None — quiet level suppressed it).
        self.resolve_pending_reply(seq, reply_effect);
        self.flush_pending_replies_mut();
    }

}

/// Build a kitty reply Effect from a `KittyReplyContext` + message body,
/// honoring `quiet`. Returns `None` when the reply is suppressed.
fn build_reply_effect(ctx: &KittyReplyContext, msg: &str) -> Option<crate::effect::Effect> {
    use crate::effect::{Effect, PtyEffect, PtyWriteKind};
    use std::fmt::Write as _;
    if ctx.quiet >= 2 {
        return None;
    }
    if ctx.quiet >= 1 && msg == "OK" {
        return None;
    }
    let head = match (ctx.image_id, ctx.image_number) {
        (0, Some(n)) => format!("I={n}"),
        (id, Some(n)) => format!("i={id},I={n}"),
        (id, None) => format!("i={id}"),
    };
    let mut qualifiers = String::new();
    if let Some(pid) = ctx.placement_id {
        let _ = write!(qualifiers, ",p={pid}");
    }
    if let Some(frame) = ctx.frame_num {
        let _ = write!(qualifiers, ",r={frame}");
    }
    let response = format!("\x1b_G{head}{qualifiers};{msg}\x1b\\");
    Some(Effect::Pty(PtyEffect::Write {
        bytes: response.into_bytes(),
        kind: PtyWriteKind::ImageProtocolReply,
    }))
}

/// Decoded-image + cell-pixel dimensions resolved at apply time. Bundles
/// the four scalars previously passed individually to keep
/// `make_placement_from_params` under the 5-argument clippy threshold.
struct DecodedDims {
    img_w: u32,
    img_h: u32,
    cell_w: u32,
    cell_h: u32,
}

/// Build an `ImagePlacement` from the worker-pipeline `PlacementParams`.
/// Resolves cells from `display_cols`/`display_rows` when explicit, or
/// computes from decoded image dims + cell pixel dims when implicit.
/// Matches `kitty_create_placement`'s sizing logic so async placements
/// behave identically to sync placements.
fn make_placement_from_params(
    image_id: crate::image::ImageId,
    params: &crate::image::worker_pipeline::PlacementParams,
    dims: &DecodedDims,
) -> crate::image::ImagePlacement {
    use crate::grid::StableRowIndex;
    use crate::image::PlacementSizing;
    let &DecodedDims {
        img_w,
        img_h,
        cell_w,
        cell_h,
    } = dims;
    let explicit_cells = params.display_cols.is_some() || params.display_rows.is_some();
    let cols = params
        .display_cols
        .unwrap_or_else(|| if img_w > 0 { img_w.div_ceil(cell_w) } else { 1 })
        as usize;
    let rows = params
        .display_rows
        .unwrap_or_else(|| if img_h > 0 { img_h.div_ceil(cell_h) } else { 1 })
        as usize;
    let sizing = if explicit_cells {
        PlacementSizing::CellCount
    } else {
        PlacementSizing::FixedPixels {
            width: cols as u32 * cell_w,
            height: rows as u32 * cell_h,
        }
    };
    crate::image::ImagePlacement {
        image_id,
        placement_id: params.placement_id,
        source_x: params.source_x,
        source_y: params.source_y,
        source_w: params.source_w,
        source_h: params.source_h,
        cell_col: params.cursor_col as usize,
        cell_row: StableRowIndex(u64::from(params.cursor_row)),
        cols,
        rows,
        z_index: params.z_index,
        cell_x_offset: 0,
        cell_y_offset: 0,
        sizing,
    }
}

impl<S: EffectSink> Term<S> {
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
