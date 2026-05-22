//! Worker-thread image decode pipeline — cross-crate seam.
//!
//! `oriterm_mux::pane::io_thread::image_worker::ImageWorker` calls
//! `run_image_decode` from a worker thread; the IO thread later drains
//! results and calls `Term::apply_decoded_image`. `oriterm_core` owns the
//! decode work + the public types; `oriterm_mux` owns the thread + channel
//! orchestration.
//!
//! See: bug-tracker/plans/BUG-06-088/section-02-fix-consensus.md
//! See: bug-tracker/plans/BUG-06-088/section-05-implementation.md

use crate::image::ImageSource;
use crate::image::decode::rgb_to_rgba;
use crate::image::kitty::KittyTransmission;

/// Request to decode one image off the IO thread.
///
/// Carries the payload (moved, not borrowed) + format / size / compression
/// parameters + reply correlation context. Constructed inside `Term`'s kitty
/// dispatch arm (Transmit / Frame); consumed by the worker thread which
/// produces a matching `ImageDecodeResult` and pushes it back via the IO
/// thread's drain pipeline.
#[derive(Debug, Clone)]
pub struct ImageDecodeRequest {
    /// Per-kitty-command sequence id. Term mints these monotonically so the
    /// reply sequencer can flush replies in command order even when async
    /// (worker) and synchronous (Query / Place / Delete) commands interleave.
    pub sequence_id: u64,
    /// Resolved image id for cache insertion.
    pub image_id: u32,
    /// Raw payload bytes (post-base64-decode, pre-zlib-inflate).
    pub payload: Vec<u8>,
    /// Format code (`f=`): 24=RGB, 32=RGBA, 100=PNG.
    pub format: u32,
    /// Source pixel width (kitty `s=`).
    pub width: u32,
    /// Source pixel height (kitty `v=`).
    pub height: u32,
    /// Compression flag (`o=`): `Some(b'z')` for zlib, `None` for raw.
    pub compression: Option<u8>,
    /// `ImageCache::max_single_image_bytes()` snapshot at enqueue time.
    pub max_bytes: usize,
    /// Reply correlation echo data — sent back unchanged in the result so
    /// `Term::apply_decoded_image` can emit the kitty reply with the
    /// original `image_id` / `image_number` / `placement_id` / `quiet` level.
    pub reply_ctx: DecodeReplyContext,
    /// Optional kitty `I=` number for reply echo.
    pub image_number: Option<u32>,
    /// When `Some`, the request is from `a=T` (transmit-and-place). The
    /// placement is applied on the IO thread immediately after the decoded
    /// image lands in the cache, via the same drain step.
    pub placement: Option<PlacementParams>,
    /// Source classification for `ImageData.source` after store. Worker
    /// never opens filesystem paths; this is always `Direct` for now.
    pub source: ImageSource,
    /// Marker for which transmission mode the original command used. Gates
    /// the worker path: `Direct` enqueues; `File` / `TempFile` / `SharedMemory`
    /// stay on the IO thread via `kitty_store_from_file` and never reach
    /// this enum.
    pub transmission: KittyTransmission,
}

/// Worker-decode outcome, applied by `Term::apply_decoded_image` on the IO thread.
///
/// Carries the original `reply_ctx` + `placement` so apply can emit
/// the kitty reply and create the deferred placement without re-deriving them.
#[derive(Debug, Clone)]
pub struct ImageDecodeResult {
    /// Mirrors `ImageDecodeRequest.sequence_id`. Term's reply sequencer
    /// matches this against the `PendingReply::Pending { seq, .. }` entry
    /// and replaces it with `Ready { seq, effect: Option<Effect> }`.
    pub sequence_id: u64,
    /// Mirrors `ImageDecodeRequest.image_id`.
    pub image_id: u32,
    /// Decoded RGBA buffer + dimensions, OR a structured error.
    pub decoded: Result<DecodedImage, ImageDecodeError>,
    /// Mirrors `ImageDecodeRequest.reply_ctx`.
    pub reply_ctx: DecodeReplyContext,
    /// Mirrors `ImageDecodeRequest.placement`.
    pub placement: Option<PlacementParams>,
    /// Mirrors `ImageDecodeRequest.payload.len()`. The IO thread uses this
    /// to decrement `pending_request_bytes` after applying the result
    /// (decrement happens on apply, NOT on worker decode completion, so the
    /// budget stays charged end-to-end through the worker → IO-thread handoff).
    pub payload_bytes: usize,
}

/// Successfully decoded image — RGBA pixel buffer + decoded dimensions.
///
/// `width` / `height` are the post-decode dimensions, which may differ from
/// the request's source `width` / `height` for `f=100` (PNG) where the
/// decoded size comes from the PNG header. For `f=24` / `f=32`, post-decode
/// dimensions equal the request dimensions.
#[derive(Debug, Clone)]
pub struct DecodedImage {
    pub rgba_bytes: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub source: ImageSource,
}

/// Error variants surfaced by the worker pipeline OR by enqueue rejection on the IO thread.
///
/// `Term::apply_decoded_image` formats each variant into the appropriate
/// kitty reply string — keeping kitty protocol formatting inside
/// `oriterm_core` per the layer-boundary discipline.
#[derive(Debug, Clone)]
pub enum ImageDecodeError {
    /// Decoder-side error from `prepare_image_bytes` (zlib invalid, unknown
    /// compression, EBIG) or `kitty_decode_pixels` (format unsupported,
    /// dimension mismatch). Message is pre-formatted as the kitty reply
    /// body (e.g. `"EINVAL: zlib decode failed: ..."`).
    Reply(String),
    /// Worker thread panicked inside `catch_unwind` while decoding this
    /// request. Other requests continue on the same worker thread.
    Panicked { message: String },
    /// IO thread side: `ImageWorker::enqueue` rejected because the pending
    /// payload-byte budget would overflow (`MAX_PENDING_BYTES`).
    /// `apply_decoded_image` formats as `"ENOMEM: image decode queue full"`.
    EnqueueOverflow,
    /// IO thread side: worker thread died before request landed.
    /// `apply_decoded_image` formats as `"EINVAL: image worker unavailable"`.
    EnqueueWorkerDead,
}

/// Public re-shape of the kitty-private `KittyReplyContext` — carries the
/// fields needed to emit the eventual reply without exposing the kitty
/// handler's internal type across the crate boundary.
///
/// `Term::handle_kitty_graphics` constructs this from a `KittyCommand` via
/// `From<&KittyCommand>`; `Term::apply_decoded_image` reconstructs a
/// `KittyReplyContext` via `From<&DecodeReplyContext>` to call `kitty_respond`.
#[derive(Debug, Clone, Copy, Default)]
pub struct DecodeReplyContext {
    pub image_id: u32,
    pub image_number: Option<u32>,
    pub placement_id: Option<u32>,
    pub frame_num: Option<u32>,
    pub quiet: u8,
}

/// Placement metadata for `a=T` (transmit-and-place) or deferred Place commands.
///
/// Cursor / cell snapshot is captured at the dispatch site (IO thread)
/// before enqueue, so the placement geometry reflects the state at the
/// moment the program emitted the command — not the state at apply time.
#[derive(Debug, Clone)]
pub struct PlacementParams {
    pub placement_id: Option<u32>,
    pub cursor_col: u32,
    pub cursor_row: u32,
    pub z_index: i32,
    pub source_x: u32,
    pub source_y: u32,
    pub source_w: u32,
    pub source_h: u32,
    pub display_cols: Option<u32>,
    pub display_rows: Option<u32>,
}

/// Worker-thread entry point: decode one image and produce a result.
///
/// Pure function — no side effects on `Term` state. Called by
/// `oriterm_mux::pane::io_thread::image_worker::worker::run` for each
/// inbound `ImageDecodeRequest`. The worker wraps this call in
/// `std::panic::catch_unwind` so a panic in `kitty_decode_pixels` or
/// `prepare_image_bytes` becomes `ImageDecodeError::Panicked` rather
/// than killing the worker thread.
pub fn run_image_decode(req: ImageDecodeRequest) -> ImageDecodeResult {
    let payload_bytes = req.payload.len();
    let expected_size = expected_decoded_size_for_format(req.format, req.width, req.height);
    let decoded = decode_payload(
        req.payload,
        req.compression,
        expected_size,
        req.max_bytes,
        req.format,
        req.width,
        req.height,
        req.source,
    );
    ImageDecodeResult {
        sequence_id: req.sequence_id,
        image_id: req.image_id,
        decoded,
        reply_ctx: req.reply_ctx,
        placement: req.placement,
        payload_bytes,
    }
}

/// Compute the expected post-decode payload size for raw-pixel formats
/// (`f=24` → `w*h*3` decompresses to `w*h*4` RGBA; `f=32` → `w*h*4`).
/// Returns `None` for `f=100` (PNG) where decoded size is not derivable
/// from the `s=`/`v=` control fields alone.
fn expected_decoded_size_for_format(format: u32, width: u32, height: u32) -> Option<usize> {
    let channels: usize = match format {
        24 => 3,
        32 => 4,
        _ => return None,
    };
    Some(
        (width as usize)
            .checked_mul(height as usize)
            .and_then(|wh| wh.checked_mul(channels))
            .unwrap_or(usize::MAX),
    )
}

#[allow(clippy::too_many_arguments)]
fn decode_payload(
    payload: Vec<u8>,
    compression: Option<u8>,
    expected_size: Option<usize>,
    max_bytes: usize,
    format: u32,
    width: u32,
    height: u32,
    source: ImageSource,
) -> Result<DecodedImage, ImageDecodeError> {
    let prepared = crate::term::handler::image::kitty::prepare_image_bytes(
        payload,
        compression,
        expected_size,
        max_bytes,
    )
    .map_err(|e| ImageDecodeError::Reply(e.to_string()))?;
    // Delegate to the canonical synchronous decoder so error message
    // shapes (e.g. "EINVAL: RGBA payload size N != expected M") stay
    // single-sourced — both the sync legacy fallback path and the worker
    // path produce byte-identical replies.
    let (rgba_bytes, decoded_w, decoded_h) =
        crate::term::handler::image::kitty::prepare::kitty_decode_pixels(
            prepared, format, width, height,
        )
        .map_err(ImageDecodeError::Reply)?;
    let _ = &rgb_to_rgba; // suppress unused import; helper used inside kitty_decode_pixels
    Ok(DecodedImage {
        rgba_bytes,
        width: decoded_w,
        height: decoded_h,
        source,
    })
}

