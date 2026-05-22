//! Image-bytes preparation helper — single SSOT for decompression +
//! unknown-compression rejection, applied at all three kitty payload entry
//! points (`kitty_store_image` direct transmits, `kitty_frame` animation
//! frame transmits, `kitty_store_from_file` file-backed transmits).
//!
//! Also hosts the extracted free-fn form of `kitty_decode_pixels` (previously
//! an assoc fn on `impl<S> Term<S>`) so `crate::image::worker_pipeline::run_image_decode`
//! can invoke it without coupling to `Term`.

use std::io::Read;

use flate2::read::ZlibDecoder;

use crate::image::{decode_to_rgba, rgb_to_rgba};

use super::store::KittyStoreError;

/// Decompress (or pass through) image bytes ahead of `kitty_decode_pixels`.
///
/// - `compression == None` → return `Ok(raw)` byte-for-byte.
/// - `compression == Some(b'z')` → zlib-decompress with an incremental
///   `cap + 1` zip-bomb defense. Bounded by `min(expected_decoded_size,
///   max_bytes)` when `expected_decoded_size` is `Some`; otherwise by
///   `max_bytes` alone.
/// - `compression == Some(other)` → reject with
///   `EINVAL: unsupported compression o=<char>`.
///
/// `expected_decoded_size` lets the helper pre-size the output buffer for
/// `f=24`/`f=32` raw-pixel transmits where `w * h * channels` is known up
/// front; `None` (e.g., `f=100` PNG) falls back to `max_bytes` as the bound.
pub(crate) fn prepare_image_bytes(
    raw: Vec<u8>,
    compression: Option<u8>,
    expected_decoded_size: Option<usize>,
    max_bytes: usize,
) -> Result<Vec<u8>, KittyStoreError> {
    let Some(code) = compression else {
        return Ok(raw);
    };

    if code != b'z' {
        return Err(KittyStoreError::Reply(format!(
            "EINVAL: unsupported compression o={}",
            code as char
        )));
    }

    // Cap = min(expected, max_bytes) when expected is Some; else max_bytes.
    // expected can lie (caller's `w*h*channels` arithmetic) — always clamp.
    let cap = expected_decoded_size.map_or(max_bytes, |n| n.min(max_bytes));

    // Read up to cap+1 bytes — landing at cap is success, hitting cap+1
    // means the stream is oversized and we abort with EBIG. The +1 makes
    // the boundary precise: a payload that decompresses to exactly `cap`
    // bytes succeeds; anything larger gets caught after reading just one
    // extra byte (no unbounded allocation regardless of the zlib bomb's
    // claimed expansion ratio).
    let limit = cap.saturating_add(1);
    let mut decoder = ZlibDecoder::new(&*raw).take(limit as u64);
    let mut out = Vec::with_capacity(cap.min(64 * 1024));

    decoder
        .read_to_end(&mut out)
        .map_err(|e| KittyStoreError::Reply(format!("EINVAL: zlib decode failed: {e}")))?;

    if out.len() > cap {
        return Err(KittyStoreError::Reply(
            "EBIG: decompressed payload exceeds max image size".to_string(),
        ));
    }

    // Strict size check when expected_decoded_size is known up-front
    // (raw-pixel formats f=24 / f=32). flate2's ZlibDecoder is permissive
    // on truncated streams that happen to end at a DEFLATE block boundary —
    // it returns partial data instead of erroring. Catch that here: if the
    // caller pre-computed the exact decompressed size and the actual output
    // doesn't match, the stream was either truncated or the caller lied
    // about dimensions; both are EINVAL.
    if let Some(expected) = expected_decoded_size
        && out.len() != expected.min(max_bytes)
    {
        return Err(KittyStoreError::Reply(format!(
            "EINVAL: zlib decode produced {} bytes != expected {expected}",
            out.len()
        )));
    }

    Ok(out)
}

/// Decode pixel data from format code to RGBA.
///
/// Extracted from `impl<S: EffectSink> Term<S>` (formerly
/// `Term::kitty_decode_pixels`) so the worker-thread runner at
/// `crate::image::worker_pipeline::run_image_decode` can invoke it without
/// coupling to `Term`. f=32 (raw RGBA) is a size-check + ownership transfer
/// (fast); f=24 (RGB→RGBA) is a memory expansion (mid); f=100 (PNG) is
/// decompression (slow under large transmits). Sustained >= 5ms per call
/// points to inline-decode IO-thread saturation under graphics-heavy
/// workloads — the very symptom the worker-pipeline architecture cures.
///
/// See: bug-tracker/plans/BUG-06-088/
pub(crate) fn kitty_decode_pixels(
    payload: Vec<u8>,
    format: u32,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    let decode_start = std::time::Instant::now();
    let payload_len = payload.len();
    let result = kitty_decode_pixels_inner(payload, format, width, height);
    let elapsed_us = decode_start.elapsed().as_micros();
    if elapsed_us >= 5_000 {
        log::info!(
            target: "oriterm_core::term::handler::image::kitty::decode_pixels",
            "format={format} payload_bytes={payload_len} width={width} height={height} duration_us={elapsed_us}"
        );
    }
    result
}

fn kitty_decode_pixels_inner(
    payload: Vec<u8>,
    format: u32,
    width: u32,
    height: u32,
) -> Result<(Vec<u8>, u32, u32), String> {
    match format {
        32 => {
            if width == 0 || height == 0 {
                return Err("EINVAL: missing s= or v= for raw RGBA".to_string());
            }
            let expected = (width as usize)
                .checked_mul(height as usize)
                .and_then(|wh| wh.checked_mul(4))
                .ok_or_else(|| format!("EINVAL: RGBA dimensions {width}x{height} overflow usize"))?;
            if payload.len() != expected {
                return Err(format!(
                    "EINVAL: RGBA payload size {} != expected {expected}",
                    payload.len(),
                ));
            }
            Ok((payload, width, height))
        }
        24 => {
            if width == 0 || height == 0 {
                return Err("EINVAL: missing s= or v= for raw RGB".to_string());
            }
            let _expected = (width as usize)
                .checked_mul(height as usize)
                .and_then(|wh| wh.checked_mul(3))
                .ok_or_else(|| format!("EINVAL: RGB dimensions {width}x{height} overflow usize"))?;
            rgb_to_rgba(&payload)
                .map(|rgba| (rgba, width, height))
                .ok_or_else(|| "EINVAL: RGB payload length not a multiple of 3".to_string())
        }
        100 => decode_to_rgba(&payload).map_err(|e| format!("EINVAL: PNG decode failed: {e}")),
        _ => Err(format!("EINVAL: unsupported format f={format}")),
    }
}

#[cfg(test)]
mod tests;
