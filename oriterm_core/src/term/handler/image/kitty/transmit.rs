//! Kitty graphics `a=t` (transmit) and `a=T` (transmit + place) actions.

use log::warn;

use crate::effect::Effect;
use crate::effect::sink::EffectSink;
use crate::image::ImageId;
use crate::image::kitty::{KittyCommand, KittyTransmission};
use crate::image::worker_pipeline::{
    DecodeReplyContext, ImageDecodeRequest, PlacementParams,
};
use crate::term::Term;

use super::{KittyReplyContext, KittyStoreParams};

impl<S: EffectSink> Term<S> {
    /// Transmit: upload image data (possibly chunked).
    ///
    /// `Direct` transmissions emit `Effect::ImageDecode` (consumed by the
    /// IO-thread `ImageWorker` in `oriterm_mux`); `File` / `TempFile` /
    /// `SharedMemory` transmissions stay synchronous via `kitty_store_image`
    /// (filesystem I/O isn't worker-eligible).
    /// See: bug-tracker/plans/BUG-06-088/section-05-implementation.md
    pub(super) fn kitty_transmit(&mut self, cmd: KittyCommand) {
        if cmd.more_data {
            self.kitty_accumulate_chunk(cmd);
            return;
        }

        let (image_id, mut merged) = self.kitty_finalize_payload(cmd);
        let ctx = KittyReplyContext::from_cmd(&merged).with_image_id(image_id);
        let unicode_placeholder = merged.unicode_placeholder;
        let params = KittyStoreParams::from_merged(image_id, &mut merged);

        // File-backed transmissions stay synchronous (filesystem I/O).
        if !matches!(params.transmission, KittyTransmission::Direct) {
            if let Err(err) = self.kitty_store_image(params) {
                let msg = err.to_string();
                warn!("kitty transmit failed: {msg}");
                self.kitty_respond(&ctx, &msg);
                return;
            }
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
            return;
        }

        // Direct transmission: emit Effect::ImageDecode for off-thread decode.
        // U=1 anchor still happens synchronously since it's bookkeeping, not
        // decoding (image data isn't stored until apply_decoded_image lands).
        if unicode_placeholder {
            let id = ImageId::from_raw(image_id);
            let grid = match (merged.display_cols, merged.display_rows) {
                (Some(cols), Some(rows)) => Some((cols, rows)),
                _ => None,
            };
            self.image_cache_mut()
                .anchor_placeholder_with_grid(id, grid);
        }
        let Some(seq) = self.current_command_seq() else {
            // Called outside handle_kitty_graphics (e.g., test fixture).
            // Fall back to synchronous store so the reply still emits.
            if let Err(err) = self.kitty_store_image(params) {
                let msg = err.to_string();
                self.kitty_respond(&ctx, &msg);
                return;
            }
            self.kitty_respond(&ctx, "OK");
            return;
        };
        let req = build_decode_request(seq, image_id, params, &merged, None);
        self.record_decode_enqueued(image_id, seq);
        self.push_pending_reply(seq, image_id);
        self.effect_sink().push(Effect::ImageDecode(req));
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

        // File-backed transmissions stay synchronous (filesystem I/O).
        if !matches!(params.transmission, KittyTransmission::Direct) {
            if let Err(err) = self.kitty_store_image(params) {
                let msg = err.to_string();
                warn!("kitty transmit+place failed: {msg}");
                self.kitty_respond(&ctx, &msg);
                return;
            }
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
            return;
        }

        // Direct transmission with placement.
        if merged.unicode_placeholder {
            let id = ImageId::from_raw(image_id);
            let grid = match (merged.display_cols, merged.display_rows) {
                (Some(cols), Some(rows)) => Some((cols, rows)),
                _ => None,
            };
            self.image_cache_mut()
                .anchor_placeholder_with_grid(id, grid);
            // U=1 has no explicit placement (placeholder cells provide it);
            // emit without PlacementParams.
            let Some(seq) = self.current_command_seq() else {
                if let Err(err) = self.kitty_store_image(params) {
                    self.kitty_respond(&ctx, &err.to_string());
                    return;
                }
                self.kitty_respond(&ctx, "OK");
                return;
            };
            let req = build_decode_request(seq, image_id, params, &merged, None);
            self.record_decode_enqueued(image_id, seq);
            self.push_pending_reply(seq, image_id);
            self.effect_sink().push(Effect::ImageDecode(req));
            return;
        }
        // Standard a=T — emit with placement params so apply_decoded_image
        // creates the placement in the same drain step as the store.
        let Some(seq) = self.current_command_seq() else {
            if let Err(err) = self.kitty_store_image(params) {
                self.kitty_respond(&ctx, &err.to_string());
                return;
            }
            self.kitty_create_placement(image_id, &merged);
            self.kitty_respond(&ctx, "OK");
            return;
        };
        // Capture cursor position at dispatch time so the placement reflects
        // the state the program intended, not the cursor state at apply time
        // (which may have moved by then).
        let cursor_col = self.grid().cursor().col().0 as u32;
        let cursor_row = self.grid().cursor().line() as u32;
        let placement = Some(build_placement_params_from_cmd(&merged, cursor_col, cursor_row));
        let req = build_decode_request(seq, image_id, params, &merged, placement);
        self.record_decode_enqueued(image_id, seq);
        self.push_pending_reply(seq, image_id);
        self.effect_sink().push(Effect::ImageDecode(req));
        // Cursor-advance MUST happen at dispatch time (not at decode-apply
        // time) so subsequent text writes in the same parse chunk land in
        // the cells past the image. Use source dims as a proxy for decoded
        // dims (matches for f=24/f=32; f=100 PNG defers to apply-time
        // best-effort). Mirrors kitty_create_placement's cursor logic.
        if !merged.no_cursor_move {
            advance_cursor_past_placement(self, &merged);
        }
    }
}

/// Advance the cursor past where the placement will land, matching
/// `kitty_create_placement`'s cursor-move behavior so subsequent text
/// writes appear AFTER the image. Source dimensions (`s=`, `v=`) are used
/// as a proxy for decoded image dimensions since the actual image isn't
/// in cache yet at dispatch time.
fn advance_cursor_past_placement<S: EffectSink>(
    term: &mut Term<S>,
    cmd: &KittyCommand,
) {
    use crate::index::Column;
    let cell_w = term.cell_pixel_width.max(1) as u32;
    let cell_h = term.cell_pixel_height.max(1) as u32;
    let cols = cmd.display_cols.unwrap_or_else(|| {
        if cmd.source_width > 0 {
            cmd.source_width.div_ceil(cell_w)
        } else {
            1
        }
    }) as usize;
    let rows = cmd.display_rows.unwrap_or_else(|| {
        if cmd.source_height > 0 {
            cmd.source_height.div_ceil(cell_h)
        } else {
            1
        }
    }) as usize;
    let grid = term.grid_mut();
    for _ in 0..rows.saturating_sub(1) {
        grid.linefeed();
    }
    let max_col = grid.cols();
    let current_col = grid.cursor().col().0;
    let new_col = current_col.saturating_add(cols).min(max_col);
    grid.cursor_mut().set_col(Column(new_col));
}

/// Build an `ImageDecodeRequest` from a finalized `KittyStoreParams` +
/// command + optional placement. Used by both `a=t` and `a=T` Direct paths.
fn build_decode_request(
    seq: u64,
    image_id: u32,
    params: KittyStoreParams,
    cmd: &KittyCommand,
    placement: Option<PlacementParams>,
) -> ImageDecodeRequest {
    let max_bytes = 50 * 1024 * 1024; // matches ImageCache default; refined by snapshot at apply time
    ImageDecodeRequest {
        sequence_id: seq,
        image_id,
        payload: params.payload,
        format: params.format,
        width: params.width,
        height: params.height,
        compression: params.compression,
        max_bytes,
        reply_ctx: DecodeReplyContext {
            image_id,
            image_number: cmd.image_number,
            placement_id: cmd.placement_id,
            frame_num: None,
            quiet: cmd.quiet,
        },
        image_number: cmd.image_number,
        placement,
        source: crate::image::ImageSource::Direct,
        transmission: KittyTransmission::Direct,
    }
}

/// Build `PlacementParams` from a `KittyCommand` for `a=T` requests.
fn build_placement_params_from_cmd(
    cmd: &KittyCommand,
    cursor_col: u32,
    cursor_row: u32,
) -> PlacementParams {
    PlacementParams {
        placement_id: cmd.placement_id,
        cursor_col,
        cursor_row,
        z_index: cmd.z_index,
        source_x: cmd.source_x,
        source_y: cmd.source_y,
        source_w: cmd.source_width,
        source_h: cmd.source_height,
        display_cols: cmd.display_cols,
        display_rows: cmd.display_rows,
    }
}
