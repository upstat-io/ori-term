//! Image-bytes preparation helper — single SSOT for decompression +
//! unknown-compression rejection, applied at all three kitty payload entry
//! points (`kitty_store_image` direct transmits, `kitty_frame` animation
//! frame transmits, `kitty_store_from_file` file-backed transmits).
//!
//! Phase 3 stub: only the `None` (uncompressed) branch is wired. The
//! `Some(b'z')` zlib decompression branch + the `Some(other)` unknown-
//! compression rejection branch land in Phase 4 (Item NEW-1).

use super::store::KittyStoreError;

/// Decompress (or pass through) image bytes ahead of `kitty_decode_pixels`.
///
/// - `compression == None` → return `Ok(raw)` byte-for-byte.
/// - `compression == Some(b'z')` → zlib-decompress with an incremental
///   `cap + 1` zip-bomb defense bounded by `max_bytes`. Phase 4 wiring.
/// - `compression == Some(other)` → reject with
///   `EINVAL: unsupported compression o=<char>`. Phase 4 wiring.
///
/// `expected_decoded_size` lets the helper pre-size the output buffer for
/// `f=24`/`f=32` raw-pixel transmits where `w * h * channels` is known up
/// front; `None` (e.g., `f=100` PNG) falls back to `max_bytes` as the bound.
pub(super) fn prepare_image_bytes(
    raw: Vec<u8>,
    compression: Option<u8>,
    _expected_decoded_size: Option<usize>,
    _max_bytes: usize,
) -> Result<Vec<u8>, KittyStoreError> {
    match compression {
        None => Ok(raw),
        Some(c) => Err(KittyStoreError::Reply(format!(
            "STUB: prepare_image_bytes does not yet implement compression handling for o={}",
            c as char
        ))),
    }
}

#[cfg(test)]
mod tests;
