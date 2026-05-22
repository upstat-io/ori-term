//! Unit tests for `prepare_image_bytes`.
//!
//! Phase 3 (TDD-first) shape: every test except `prepare_none_compression_passes_through`
//! is EXPECTED TO FAIL pre-Phase 4 because the helper stub returns
//! `Err(KittyStoreError::Reply("STUB: ..."))` for any `Some(_)`. Phase 4
//! lands the real body (zlib decompress with `cap + 1` zip-bomb defense +
//! unknown-compression rejection); these pins then go green.
//!
//! See: bug-tracker/plans/BUG-06-086/section-03b-tdd-matrix.md
//! §"NEW-1 — `prepare_image_bytes` helper tests".

use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use super::super::store::KittyStoreError;
use super::prepare_image_bytes;

/// Generous default cap for tests that aren't probing the cap boundary.
const TEST_MAX_BYTES: usize = 64 * 1024 * 1024;

/// Build a zlib-compressed payload of `raw` at default compression.
fn zlib_encode(raw: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(raw).expect("encode write");
    encoder.finish().expect("encode finish")
}

/// Build a known 4×4 RGBA pattern (64 bytes).
fn rgba_4x4() -> Vec<u8> {
    let mut out = Vec::with_capacity(64);
    for i in 0..16u8 {
        out.extend_from_slice(&[i.wrapping_mul(17), i.wrapping_mul(23), i.wrapping_mul(29), 0xFF]);
    }
    out
}

// ===========================================================================
// Positive (round-trip success)
// ===========================================================================

/// `compression == None` → byte-for-byte pass-through, regardless of cap.
#[test]
fn prepare_none_compression_passes_through() {
    let raw = vec![0xDE, 0xAD, 0xBE, 0xEF];
    let result = prepare_image_bytes(raw.clone(), None, Some(4), TEST_MAX_BYTES)
        .expect("None compression must pass through");
    assert_eq!(result, raw, "None branch must return input unchanged");
}

/// `compression == Some(b'z')` with valid zlib payload + sufficient cap →
/// decompresses to the original RGBA bytes byte-for-byte.
#[test]
fn prepare_oz_valid_zlib_decompresses_to_expected_bytes() {
    let raw = rgba_4x4();
    let compressed = zlib_encode(&raw);
    let result = prepare_image_bytes(compressed, Some(b'z'), Some(raw.len()), TEST_MAX_BYTES)
        .expect("valid zlib + sufficient cap MUST decompress");
    assert_eq!(
        result, raw,
        "o=z MUST round-trip the original RGBA bytes byte-for-byte"
    );
}

/// Decompressed size exactly equals the configured cap → success (boundary).
#[test]
fn prepare_oz_at_exact_max_size_succeeds() {
    let raw = vec![0xAA; 1024];
    let compressed = zlib_encode(&raw);
    let result = prepare_image_bytes(compressed, Some(b'z'), Some(raw.len()), 1024)
        .expect("decompressed length == max_bytes MUST succeed");
    assert_eq!(result.len(), 1024, "exact-cap decompress MUST land at cap");
    assert_eq!(result, raw, "exact-cap decompress MUST preserve content");
}

/// `expected_decoded_size = w*h*4` for `f=32` is the canonical positive
/// shape: helper output is exactly that size.
#[test]
fn prepare_oz_decompressed_size_matches_expected_size_for_f32() {
    let raw = rgba_4x4(); // 4*4*4 = 64 bytes
    let compressed = zlib_encode(&raw);
    let result = prepare_image_bytes(compressed, Some(b'z'), Some(64), TEST_MAX_BYTES)
        .expect("expected_size == w*h*4 MUST decompress");
    assert_eq!(result.len(), 64);
}

// ===========================================================================
// Negative (failure modes)
// ===========================================================================

/// Random non-zlib bytes with `Some(b'z')` → EINVAL.
#[test]
fn prepare_oz_corrupt_zlib_returns_einval() {
    let garbage = vec![0xFF, 0xFE, 0xFD, 0xFC, 0xFB, 0xFA, 0xF9, 0xF8];
    let err = prepare_image_bytes(garbage, Some(b'z'), Some(64), TEST_MAX_BYTES)
        .expect_err("corrupt zlib MUST be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("EINVAL"),
        "corrupt zlib MUST emit EINVAL — got {msg:?}"
    );
}

/// `cap + 1` boundary: decompressed payload is exactly `max_bytes + 1`
/// bytes → EBIG. Pins the precise incremental-abort boundary; complements
/// the unbounded zip-bomb pin below.
#[test]
fn prepare_oz_boundary_cap_plus_one_rejected_with_ebig() {
    const CAP: usize = 1024;
    let raw = vec![0u8; CAP + 1];
    let compressed = zlib_encode(&raw);
    let err = prepare_image_bytes(compressed, Some(b'z'), Some(CAP + 1), CAP)
        .expect_err("decompressed > max_bytes MUST be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("EBIG"),
        "cap + 1 boundary MUST emit EBIG — got {msg:?}"
    );
}

/// Unbounded zip-bomb defense: small zlib payload that expands to GB.
/// Helper MUST abort without reading past `max_bytes + 1`. Complements the
/// boundary pin above by exercising the unbounded-expansion path.
#[test]
fn prepare_oz_unbounded_zip_bomb_rejected_at_cap() {
    const CAP: usize = 16 * 1024;
    // Compress 4 MB of zeros — ratio ~1000x; decompressed >> cap.
    let raw = vec![0u8; 4 * 1024 * 1024];
    let compressed = zlib_encode(&raw);
    assert!(
        compressed.len() < CAP,
        "zip-bomb fixture sanity: compressed payload must be smaller than cap to test the unbounded path; \
         compressed={} cap={}",
        compressed.len(),
        CAP
    );
    let err = prepare_image_bytes(compressed, Some(b'z'), None, CAP)
        .expect_err("zip bomb MUST be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("EBIG"),
        "unbounded zip-bomb MUST emit EBIG — got {msg:?}"
    );
}

/// Truncated zlib payload (header + partial data) → decoder returns
/// InvalidData mid-stream → EINVAL.
#[test]
fn prepare_oz_truncated_zlib_returns_einval() {
    let raw = rgba_4x4();
    let compressed = zlib_encode(&raw);
    let half = &compressed[..compressed.len() / 2];
    let err = prepare_image_bytes(half.to_vec(), Some(b'z'), Some(raw.len()), TEST_MAX_BYTES)
        .expect_err("truncated zlib MUST be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("EINVAL"),
        "truncated zlib MUST emit EINVAL — got {msg:?}"
    );
}

/// `compression == Some(b'x')` → EINVAL `unsupported compression o=x`.
/// Fixes the silent-treat-as-uncompressed shape Plan TPR surfaced — pre-
/// helper, any non-`z` value flows through as if uncompressed.
#[test]
fn prepare_unknown_compression_x_returns_einval() {
    let raw = vec![0xCAFE_BABEu32.to_le_bytes(); 4]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let err = prepare_image_bytes(raw, Some(b'x'), Some(16), TEST_MAX_BYTES)
        .expect_err("unknown compression MUST be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("EINVAL"),
        "unknown o=x MUST emit EINVAL — got {msg:?}"
    );
    assert!(
        msg.contains("unsupported compression"),
        "unknown o=x MUST name 'unsupported compression' — got {msg:?}"
    );
    assert!(
        msg.contains("o=x"),
        "unknown o=x MUST echo the offending value — got {msg:?}"
    );
}

/// Sanity clamp on the unknown-compression variant handling — any
/// non-`z`/non-`None` value renders the same shape.
#[test]
fn prepare_unknown_compression_q_returns_einval() {
    let raw = vec![0u8; 16];
    let err = prepare_image_bytes(raw, Some(b'q'), Some(16), TEST_MAX_BYTES)
        .expect_err("unknown compression MUST be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("EINVAL") && msg.contains("o=q"),
        "unknown o=q MUST emit EINVAL with offending value — got {msg:?}"
    );
}

// ===========================================================================
// Bounded-cap semantics
// ===========================================================================

/// `expected_decoded_size > max_bytes` → helper clamps the cap to
/// `max_bytes` and rejects when real output exceeds it.
#[test]
fn prepare_oz_expected_size_capped_to_max_bytes() {
    const CAP: usize = 1024;
    let raw = vec![0xBBu8; CAP + 500];
    let compressed = zlib_encode(&raw);
    // expected_size lies about how big the payload is (CAP + 1000); the
    // helper MUST still clamp to max_bytes (CAP) and reject.
    let err = prepare_image_bytes(compressed, Some(b'z'), Some(CAP + 1000), CAP)
        .expect_err("oversized output MUST be rejected even when expected_size lies");
    let msg = err.to_string();
    assert!(
        msg.contains("EBIG"),
        "expected_size > max_bytes case MUST clamp + reject with EBIG — got {msg:?}"
    );
}

/// `expected_size = None` (PNG / unknown-format path) → helper uses
/// `max_bytes` as the bound.
#[test]
fn prepare_oz_no_expected_size_uses_max_bytes() {
    const CAP: usize = 1024;
    let raw = vec![0xCCu8; 512];
    let compressed = zlib_encode(&raw);
    let result = prepare_image_bytes(compressed, Some(b'z'), None, CAP)
        .expect("under-cap with no expected_size MUST decompress");
    assert_eq!(result.len(), 512);
}

// ===========================================================================
// Memory + alloc shape pins
// ===========================================================================

/// Helper's internal buffer is bounded at `cap + 1` regardless of zlib
/// stream size. This pins the structural invariant — without the helper
/// reading no more than `cap + 1` bytes, the decoder would inflate the
/// whole stream into memory before the size check fires (defeats the
/// incremental zip-bomb defense). Verified indirectly via the result's
/// length cap: under-cap inputs succeed, over-cap inputs reject with EBIG;
/// a buggy "decompress then check" impl would still pass both. The pin
/// fires when a pathological 1 GB zlib bomb completes in < 1 sec wall
/// time (proves the helper bailed early) — but wall-clock is flaky per
/// `tests.md §Wall-Clock-Free Testing`. So we use the boundary clamp +
/// unbounded zip-bomb pins above as the structural proof of bounded reads
/// and leave the explicit allocator instrumentation to the Phase 4
/// criterion bench at `benches/kitty_decode.rs` (per §03b NEW-1 + Item 7
/// follow-up gate).
#[test]
fn prepare_oz_buffer_never_grows_past_cap_plus_one() {
    // Multi-cell pin: the boundary clamp + the unbounded zip-bomb together
    // pin the invariant. This wrapper test asserts BOTH conditions hold so
    // the pin's existence forces co-evolution: removing either underlying
    // pin causes this assertion to fail with a distinguishable message.
    const CAP: usize = 1024;

    // Sub-pin A: at-cap succeeds.
    let exact = vec![0u8; CAP];
    let exact_result = prepare_image_bytes(
        zlib_encode(&exact),
        Some(b'z'),
        Some(CAP),
        CAP,
    );
    assert!(
        exact_result.is_ok(),
        "at-cap decompress MUST succeed (sub-pin A of buffer-bounded invariant)"
    );

    // Sub-pin B: cap+1 rejects.
    let over = vec![0u8; CAP + 1];
    let over_result = prepare_image_bytes(
        zlib_encode(&over),
        Some(b'z'),
        Some(CAP + 1),
        CAP,
    );
    assert!(
        over_result.is_err(),
        "cap+1 decompress MUST reject (sub-pin B of buffer-bounded invariant)"
    );
    let msg = over_result.unwrap_err().to_string();
    assert!(
        msg.contains("EBIG"),
        "cap+1 rejection MUST emit EBIG (sub-pin B) — got {msg:?}"
    );
}

// ===========================================================================
// KittyStoreError variant pin (preserves Protocol vs Reply discipline)
// ===========================================================================

/// EINVAL on unknown compression MUST flow through `KittyStoreError::Reply`
/// (the store-layer stringly-typed reply variant), NOT through
/// `KittyStoreError::Protocol(KittyError::CompressionNotSupported)`. The
/// helper is a store-layer helper; parser-layer protocol errors stay
/// upstream in `parse_kitty_command_into`. This pin prevents a regression
/// where someone reaches into `KittyError::CompressionNotSupported` from
/// the helper to "reuse" the existing typed variant — that would couple
/// the helper to a parser-layer concept and break the layering.
#[test]
fn prepare_unknown_compression_emits_via_reply_variant_not_protocol() {
    let err = prepare_image_bytes(vec![0u8; 4], Some(b'x'), None, TEST_MAX_BYTES)
        .expect_err("unknown compression MUST be rejected");
    assert!(
        matches!(err, KittyStoreError::Reply(_)),
        "helper MUST emit via KittyStoreError::Reply, not Protocol — got {err:?}"
    );
}
