//! Image-bytes preparation helper — single SSOT for decompression +
//! unknown-compression rejection, applied at all three kitty payload entry
//! points (`kitty_store_image` direct transmits, `kitty_frame` animation
//! frame transmits, `kitty_store_from_file` file-backed transmits).

use std::io::Read;

use flate2::read::ZlibDecoder;

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
pub(super) fn prepare_image_bytes(
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
    let mut decoder = ZlibDecoder::new(&raw[..]).take(limit as u64);
    let mut out = Vec::with_capacity(cap.min(64 * 1024));

    decoder.read_to_end(&mut out).map_err(|e| {
        KittyStoreError::Reply(format!("EINVAL: zlib decode failed: {e}"))
    })?;

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

#[cfg(test)]
mod tests;
