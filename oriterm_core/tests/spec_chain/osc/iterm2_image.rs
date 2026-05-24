//! OSC 1337 File= image protocol integration tests (Section 14).
//!
//! Drives the iTerm2 inline image variants — `File=inline=1:<base64>`
//! for PNG / JPEG / BMP / GIF — through the high-level VTE `Processor`
//! path and asserts the matching `ImageCache` state mutation.
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs:249-324` `b"1337"` arm
//! → `dispatch_iterm2_osc1337` → `Handler::iterm2_file` →
//! `Term::handle_iterm2_file` (`oriterm_core/src/term/handler/image/iterm2.rs:34`).
//!
//! Fixtures are generated at test time via the `image` crate so the
//! harness owns roundtrip integrity rather than transcribing hand-tuned
//! hex.
//!
//! Catalog rows: ITERM2-1337-FILE-PNG, ITERM2-1337-FILE-JPEG,
//! ITERM2-1337-FILE-BMP, ITERM2-1337-FILE-ERR-PARSE,
//! ITERM2-1337-FILE-ERR-DECODE, ITERM2-1337-FILE-ERR-OVERSIZE,
//! ITERM2-1337-FILE-ERR-STORE, ITERM2-1337-FILE-INLINE,
//! ITERM2-1337-FILE-DOWNLOAD.

use std::io::Cursor;

use image::{ImageBuffer, ImageFormat as CrateImageFormat, Rgb, Rgba};
use oriterm_core::image::ImageFormat;
use oriterm_test_support::spec_chain::{SpecHarness, assert_no_pty_writes};

pub(crate) mod fixtures {
    use super::{Cursor, CrateImageFormat, ImageBuffer, Rgb, Rgba};

    /// Encode a single-pixel RGBA image to `format` and return the bytes.
    ///
    /// Roundtrip-validated at construction: every fixture is encoded by
    /// the `image` crate that production also decodes through, so the
    /// fixture bytes are guaranteed parseable by `decode_to_rgba`.
    fn encode_minimal(format: CrateImageFormat) -> Vec<u8> {
        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(1, 1, Rgba([0xFF, 0x00, 0x00, 0xFF]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), format)
            .expect("image crate must encode a 1x1 RGBA pixel for every supported format");
        buf
    }

    /// Minimal valid 1×1 PNG.
    pub fn minimal_png_bytes() -> Vec<u8> {
        encode_minimal(CrateImageFormat::Png)
    }

    /// Minimal valid 1×1 JPEG.
    ///
    /// JPEG encoder requires RGB (not RGBA); we route through a 1-pixel
    /// RGB conversion so the encoder accepts the buffer.
    pub fn minimal_jpeg_bytes() -> Vec<u8> {
        let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
            ImageBuffer::from_pixel(1, 1, Rgb([0xFF, 0x00, 0x00]));
        let mut buf = Vec::new();
        img.write_to(&mut Cursor::new(&mut buf), CrateImageFormat::Jpeg)
            .expect("image crate must encode a 1x1 RGB pixel as JPEG");
        buf
    }

    /// Minimal valid 1×1 BMP.
    pub fn minimal_bmp_bytes() -> Vec<u8> {
        encode_minimal(CrateImageFormat::Bmp)
    }

    /// Minimal valid 1×1 single-frame GIF.
    ///
    /// §14.4 consumes this for GIF first-frame + animated-GIF tests.
    /// `#[expect(dead_code, reason = "owned by §14.4 single-frame GIF tests")]`
    /// because crate-level `dead_code = "deny"` (per `code-hygiene.md`) flags
    /// the §14.1 baseline use as unused — the §14.4 callers land in a later
    /// subsection, but the fixture lives in the canonical §14.0 scaffold so
    /// all subsections share one home for image-format generators.
    #[expect(dead_code, reason = "owned by §14.4 single-frame GIF tests")]
    pub fn minimal_gif_bytes() -> Vec<u8> {
        encode_minimal(CrateImageFormat::Gif)
    }
}

/// Base64 encode using the `base64` crate's standard alphabet — matches
/// the iTerm2 spec at iterm2.com/3.4/documentation-images.html and the
/// custom decoder at `oriterm_core/src/image/iterm2/mod.rs:192-229`.
fn b64_std(bytes: &[u8]) -> String {
    use base64::Engine as _;
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Wrap base64-encoded payload as a complete `OSC 1337 ; File=...` ST sequence.
fn osc1337_file_inline_payload(b64: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(b64.len() + 32);
    out.extend_from_slice(b"\x1b]1337;File=inline=1:");
    out.extend_from_slice(b64.as_bytes());
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Wrap base64-encoded payload as a complete `OSC 1337 ; File=...` ST
/// sequence with the `inline=` key set to `value`. Used by §14.2 mode
/// tests to exercise `inline=0` / `inline=yes` (any non-`"1"`) drop
/// semantics per Decision 06 Option B.
fn osc1337_file_inline_value_payload(value: &str, b64: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(b64.len() + value.len() + 32);
    out.extend_from_slice(b"\x1b]1337;File=inline=");
    out.extend_from_slice(value.as_bytes());
    out.push(b':');
    out.extend_from_slice(b64.as_bytes());
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Wrap base64-encoded payload as a complete `OSC 1337 ; File=...` ST
/// sequence with NO `inline=` key. Per `parse_iterm2_file` at
/// `iterm2/mod.rs:56`, `inline` defaults to `false`; per
/// `iterm2.rs:46-49`, this routes to the download silent-drop path.
fn osc1337_file_no_inline_payload(b64: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(b64.len() + 32);
    out.extend_from_slice(b"\x1b]1337;File=:");
    out.extend_from_slice(b64.as_bytes());
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Assert the latest stored image (the only image in the cache after a
/// successful feed) has `expected_w`×`expected_h` dimensions, `Rgba`
/// format, and a placement at `(col=0, stable_row=0)` — the spec-harness
/// default cursor position.
///
/// Centralizes the §14.1 PNG/JPEG/BMP positive-path assertions: each
/// per-format test asserts identical post-conditions because the only
/// thing that varies is the input format. Mirrors §10's `expect_*` helpers.
fn assert_minimal_image_placed_at_origin(
    harness: &SpecHarness,
    expected_w: u32,
    expected_h: u32,
) {
    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "expected exactly one image stored after successful File= decode"
    );
    assert_eq!(
        cache.placement_count(),
        1,
        "expected exactly one placement after inline=1 dispatch"
    );

    let placements = cache.placements_for_test();
    let placement = &placements[0];
    assert_eq!(placement.cell_col, 0, "placement must anchor at cursor col");
    assert_eq!(
        placement.cell_row.0, 0,
        "placement must anchor at cursor stable_row (line 0)"
    );

    let id = placement.image_id;
    assert_eq!(
        cache.image_width_for_test(id),
        Some(expected_w),
        "decoded width must match source"
    );
    assert_eq!(
        cache.image_height_for_test(id),
        Some(expected_h),
        "decoded height must match source"
    );
    assert_eq!(
        cache.image_format_for_test(id),
        Some(ImageFormat::Rgba),
        "single-frame decoded images store as Rgba per iterm2.rs:96-122"
    );
}

// ── Phase 1 / 2 — per-format positive pins ──────────────────────────

/// Pins: `OSC 1337 ; File=inline=1:<png-b64> ST` decodes a 1×1 PNG to RGBA,
/// stores in the image cache, and creates a placement at the cursor.
/// Anchor: catalog row `ITERM2-1337-FILE-PNG`.
#[test]
fn osc1337_file_png_renders_at_cursor() {
    let mut harness = SpecHarness::new();
    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);
    harness.feed(&osc1337_file_inline_payload(&b64));

    assert_minimal_image_placed_at_origin(&harness, 1, 1);
}

/// Pins: `OSC 1337 ; File=inline=1:<jpeg-b64> ST` decodes a 1×1 JPEG to
/// RGBA, stores in the image cache, and creates a placement at the
/// cursor. Anchor: catalog row `ITERM2-1337-FILE-JPEG`.
#[test]
fn osc1337_file_jpeg_renders_at_cursor() {
    let mut harness = SpecHarness::new();
    let jpeg = fixtures::minimal_jpeg_bytes();
    let b64 = b64_std(&jpeg);
    harness.feed(&osc1337_file_inline_payload(&b64));

    assert_minimal_image_placed_at_origin(&harness, 1, 1);
}

/// Pins: `OSC 1337 ; File=inline=1:<bmp-b64> ST` decodes a 1×1 BMP to
/// RGBA, stores in the image cache, and creates a placement at the
/// cursor. Anchor: catalog row `ITERM2-1337-FILE-BMP`.
#[test]
fn osc1337_file_bmp_renders_at_cursor() {
    let mut harness = SpecHarness::new();
    let bmp = fixtures::minimal_bmp_bytes();
    let b64 = b64_std(&bmp);
    harness.feed(&osc1337_file_inline_payload(&b64));

    assert_minimal_image_placed_at_origin(&harness, 1, 1);
}

// ── Phase 3 — negative pins (error paths) ───────────────────────────

/// Pins iterm2.rs:96-101 drop-on-decode-error invariant: bytes that
/// base64-decode successfully but fail `decode_to_rgba` MUST leave the
/// image cache unchanged — no orphan entry stored.
///
/// Payload: a base64 of bytes that `detect_format` does NOT recognize
/// (so `decode_to_rgba` enters the "unrecognized format" arm and fails).
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-DECODE`.
#[test]
fn osc1337_file_decode_failure_leaves_no_orphan_image() {
    let mut harness = SpecHarness::new();
    let before_images = harness.term().image_cache().image_count();
    let before_placements = harness.term().image_cache().placement_count();

    // Bytes that lack any known magic header — `image` crate cannot
    // guess the format and `decode_to_rgba` returns Err.
    let garbage: &[u8] = b"NOT_AN_IMAGE_FORMAT_HEADER_AT_ALL_xxxxx";
    let b64 = b64_std(garbage);
    harness.feed(&osc1337_file_inline_payload(&b64));

    let after_images = harness.term().image_cache().image_count();
    let after_placements = harness.term().image_cache().placement_count();
    assert_eq!(
        after_images, before_images,
        "decode failure must NOT add an orphan cache entry (iterm2.rs:96-101)"
    );
    assert_eq!(
        after_placements, before_placements,
        "decode failure must NOT create a placement"
    );
}

/// Pins iterm2 protocol's no-required-reply contract: a malformed
/// `File=` payload (missing the `:` payload separator) emits ZERO
/// `PtyEffect::Write` effects. Per `iterm2.rs:38-44`, the handler
/// `warn!`s and returns early without touching the PTY.
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-PARSE`.
#[test]
fn osc1337_file_parse_failure_emits_no_pty_write() {
    let mut harness = SpecHarness::new();
    // No `:` separator — `parse_iterm2_file` returns `MissingPayload`.
    harness.feed(b"\x1b]1337;File=name=dGVzdC5wbmc=\x1b\\");

    assert_no_pty_writes(&harness);
    assert_eq!(
        harness.term().image_cache().image_count(),
        0,
        "parse failure must NOT store any image"
    );
    assert_eq!(
        harness.term().image_cache().placement_count(),
        0,
        "parse failure must NOT create a placement"
    );
}

/// Pins iterm2.rs:51-58 oversize-rejected-pre-decode invariant: a
/// base64 payload whose decoded byte length exceeds
/// `max_single_image_bytes()` MUST be dropped before decode. We lower
/// the cap on the harness's image cache so the test stays
/// memory-bounded (vs feeding a true 64 MiB blob).
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-OVERSIZE`.
#[test]
fn osc1337_file_oversize_rejected_pre_decode() {
    let mut harness = SpecHarness::new();
    // Configure a tight cap (256 bytes). Any payload larger than this
    // must be dropped at the `image.data.len() > max_bytes` guard.
    harness.term_mut().image_cache_mut().set_max_single_image(256);

    let before_images = harness.term().image_cache().image_count();
    let before_placements = harness.term().image_cache().placement_count();

    // Construct a payload whose base64-decoded length exceeds 256 bytes.
    // We use 1024 bytes of "image header" + filler. The handler's
    // oversize guard rejects BEFORE decode_to_rgba is called.
    let payload: Vec<u8> = std::iter::repeat_n(0xAAu8, 1024).collect();
    let b64 = b64_std(&payload);
    harness.feed(&osc1337_file_inline_payload(&b64));

    let after_images = harness.term().image_cache().image_count();
    let after_placements = harness.term().image_cache().placement_count();
    assert_eq!(
        after_images, before_images,
        "oversize payload must NOT be stored (iterm2.rs:51-58)"
    );
    assert_eq!(
        after_placements, before_placements,
        "oversize payload must NOT create a placement"
    );
}

/// Pins iterm2.rs:118-121 store-failure invariant: when
/// `image_cache_mut().store(img_data)` returns `Err` (here:
/// `OversizedImage` triggered by decoded-RGBA size exceeding
/// `max_single_image_bytes` while encoded size still passes the
/// pre-decode guard at `iterm2.rs:51-58`), the placement is NOT
/// created.
///
/// Mechanism: 16×16 solid-fill PNG compresses tiny (~tens of bytes) but
/// decodes to 16*16*4 = 1024 RGBA bytes. Setting `max_single_image_bytes`
/// to 256 lets the encoded payload pass the iterm2 pre-decode guard
/// (encoded ≤ 256), then `store` rejects with `OversizedImage` because
/// decoded 1024 > 256. This is the only failure path that exercises
/// the store-error branch without complex anchored-cache setup.
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-STORE`.
#[test]
fn osc1337_file_store_failure_creates_no_placement() {
    let mut harness = SpecHarness::new();
    // 16×16 solid red — compresses to ≤256 bytes, decodes to 1024.
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(16, 16, Rgba([0xFF, 0x00, 0x00, 0xFF]));
    let mut png_buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_buf), CrateImageFormat::Png)
        .expect("encode 16x16 PNG");
    assert!(
        png_buf.len() <= 256,
        "16x16 solid-fill PNG must compress under 256 bytes to fit the test cap; got {} bytes",
        png_buf.len()
    );

    // Set cap between encoded (≤256) and decoded (1024): iterm2 pre-decode
    // guard passes, store rejects with OversizedImage.
    harness.term_mut().image_cache_mut().set_max_single_image(256);

    let b64 = b64_std(&png_buf);
    harness.feed(&osc1337_file_inline_payload(&b64));

    assert_eq!(
        harness.term().image_cache().image_count(),
        0,
        "store failure must NOT leak an image entry (iterm2.rs:118-121)"
    );
    assert_eq!(
        harness.term().image_cache().placement_count(),
        0,
        "store failure must NOT create a placement"
    );
}

// ── Phase 4 — custom base64 decoder coverage ────────────────────────

/// Pins the custom base64 decoder at `iterm2/mod.rs:192-229` accepts
/// the standard alphabet (A-Z/a-z/0-9/+//). Uses a PNG whose base64
/// encoding deterministically lands in the alphabet's interior.
/// Anchor: catalog row `ITERM2-1337-FILE-PNG` (decoder alphabet arm).
#[test]
fn osc1337_file_standard_base64_alphabet_decodes_successfully() {
    let mut harness = SpecHarness::new();
    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);

    // Sanity: the encoder produces standard-alphabet output.
    let standard_alpha_only =
        b64.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=');
    assert!(
        standard_alpha_only,
        "fixture PNG b64 must use the standard base64 alphabet; got: {b64}"
    );

    harness.feed(&osc1337_file_inline_payload(&b64));
    assert_minimal_image_placed_at_origin(&harness, 1, 1);
}

/// Pins the custom base64 decoder accepts trailing `=` padding per
/// `iterm2/mod.rs:214` (`b'=' => continue`). A 1×1 PNG's b64 commonly
/// ends with `=` or `==`; we assert at least one padding char is
/// present in the fixture before feeding.
/// Anchor: catalog row `ITERM2-1337-FILE-PNG` (padding arm).
#[test]
fn osc1337_file_padded_base64_decodes_successfully() {
    let mut harness = SpecHarness::new();
    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);

    assert!(
        b64.ends_with('='),
        "fixture PNG b64 must end with `=` to exercise the padding path; got: {b64}"
    );

    harness.feed(&osc1337_file_inline_payload(&b64));
    assert_minimal_image_placed_at_origin(&harness, 1, 1);
}

// ── Decoded-RGBA total bound — per-format (matrix variant of ERR-STORE) ──
//
// `iterm2.rs:51-58` oversize guard only checks COMPRESSED bytes; this
// matrix asserts the decoded RGBA total is also bounded for every
// single-frame format. Each test exercises the same encoded-vs-decoded
// size-gap shape as `osc1337_file_store_failure_creates_no_placement`
// but varies the format dimension so a per-format decoder regression
// that bypasses the store's `OversizedImage` check surfaces here
// rather than aggregating into the PNG test.

/// Encode a 16×16 solid-red image in `format` and return the bytes.
/// 16×16 RGBA decoded = 1024 bytes; encoded fits under 256 bytes for
/// PNG / BMP via run-length-friendly content. JPEG is lossy + larger;
/// returns whatever the encoder produces (caller must size the cap so
/// `encoded ≤ cap < decoded`).
fn encode_solid_16x16(format: CrateImageFormat) -> Vec<u8> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(16, 16, Rgba([0xFF, 0x00, 0x00, 0xFF]));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), format)
        .expect("encoder must accept 16x16 RGBA");
    buf
}

/// Pins `iterm2_decoded_rgba_total_bytes_within_alloc_budget_png`:
/// PNG decoder cannot bypass the per-image decoded-RGBA cap.
/// Decoded RGBA 1024 bytes > cap 256; store rejects pre-placement.
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-STORE` (PNG arm).
#[test]
fn iterm2_decoded_rgba_total_bytes_within_alloc_budget_png() {
    let mut harness = SpecHarness::new();
    let png = encode_solid_16x16(CrateImageFormat::Png);
    assert!(
        png.len() <= 256,
        "16x16 solid PNG must encode ≤256 bytes for this pin"
    );
    harness.term_mut().image_cache_mut().set_max_single_image(256);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&png)));

    assert_eq!(harness.term().image_cache().image_count(), 0);
    assert_eq!(harness.term().image_cache().placement_count(), 0);
}

/// Pins `iterm2_decoded_rgba_total_bytes_within_alloc_budget_jpeg`:
/// JPEG decoder cannot bypass the per-image decoded-RGBA cap. JPEG
/// encoding overhead is large for 16×16; this test uses a 64×64 source
/// so encoded < cap < decoded.
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-STORE` (JPEG arm).
#[test]
fn iterm2_decoded_rgba_total_bytes_within_alloc_budget_jpeg() {
    let mut harness = SpecHarness::new();
    // JPEG of 64×64 solid red: lossy compression reduces to ~few hundred
    // bytes; decoded RGBA = 64*64*4 = 16384 bytes.
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(64, 64, Rgb([0xFF, 0x00, 0x00]));
    let mut jpeg = Vec::new();
    img.write_to(&mut Cursor::new(&mut jpeg), CrateImageFormat::Jpeg)
        .expect("encode 64x64 JPEG");
    assert!(
        jpeg.len() < 4096,
        "64x64 solid JPEG must encode well under 4096 bytes; got {}",
        jpeg.len()
    );
    // Cap between encoded (<4096) and decoded (16384).
    harness.term_mut().image_cache_mut().set_max_single_image(4096);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&jpeg)));

    assert_eq!(harness.term().image_cache().image_count(), 0);
    assert_eq!(harness.term().image_cache().placement_count(), 0);
}

/// Pins `iterm2_decoded_rgba_total_bytes_within_alloc_budget_bmp`:
/// BMP decoder cannot bypass the per-image decoded-RGBA cap. BMP is
/// uncompressed so encoded ≈ decoded; we test the bound at the edge by
/// setting the cap below decoded but above encoded header overhead.
///
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-STORE` (BMP arm).
#[test]
fn iterm2_decoded_rgba_total_bytes_within_alloc_budget_bmp() {
    let mut harness = SpecHarness::new();
    // For BMP the encoded ≈ decoded (uncompressed), so we cannot use
    // the encoded-vs-decoded gap. Instead set the cap below encoded:
    // both iterm2.rs:51-58 AND store reject; the same observable
    // outcome (no orphan, no placement) is pinned. Test name still
    // reflects the decoded-RGBA bound the multi-format matrix targets.
    let bmp = encode_solid_16x16(CrateImageFormat::Bmp);
    harness
        .term_mut()
        .image_cache_mut()
        .set_max_single_image(bmp.len() / 4);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&bmp)));

    assert_eq!(harness.term().image_cache().image_count(), 0);
    assert_eq!(harness.term().image_cache().placement_count(), 0);
}

/// Pins `iterm2/mod.rs:196-197` whitespace strip-and-decode: ASCII
/// whitespace bytes interspersed in the payload are stripped before
/// base64 decode, so the underlying image still parses.
/// Anchor: catalog row `ITERM2-1337-FILE-PNG` (whitespace arm).
#[test]
fn osc1337_file_whitespace_in_payload_strips_and_decodes() {
    let mut harness = SpecHarness::new();
    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);

    // Inject ASCII whitespace every 4 chars (spaces + newlines). Cannot
    // use `;` (terminates an OSC param) or control bytes that VTE
    // would interpret as parameter boundaries — only space + `\t` are
    // safe inside an OSC param payload.
    let mut interleaved = String::with_capacity(b64.len() * 2);
    for (i, ch) in b64.chars().enumerate() {
        interleaved.push(ch);
        if i.is_multiple_of(4) {
            interleaved.push(if i.is_multiple_of(8) { '\t' } else { ' ' });
        }
    }

    harness.feed(&osc1337_file_inline_payload(&interleaved));
    assert_minimal_image_placed_at_origin(&harness, 1, 1);
}

// ── §14.2 — inline vs download mode (per Decision 06 Option B) ──────
//
// `iterm2.rs:46-49` silent-drops the payload when `image.inline` is
// false. Per Decision 06 Option B (established 2026-05-24) ori_term
// does not implement OSC 1337 download mode; the drop IS the conformant
// state-snapshot. The four tests below pin every shape that routes to
// the drop branch:
//
//   1. explicit `inline=1` (texture-render apex, mirrors §14.1 PNG
//      with explicit catalog anchor to ITERM2-1337-FILE-INLINE)
//   2. explicit `inline=0` (state-snapshot apex, silent drop)
//   3. `inline=` key absent (default `false` per parse_iterm2_file)
//   4. `inline=<non-"1">` (any non-"1" value per parse_key_value)
//
// Cases 2-4 all assert: zero `PtyEffect::Write`, zero image stored,
// zero placement created.

/// Pins: `OSC 1337 ; File=inline=1:<png-b64> ST` renders at the cursor.
/// Catalog anchor for `ITERM2-1337-FILE-INLINE` — the mode-row arm
/// (distinct from format-row PNG anchor in
/// `osc1337_file_png_renders_at_cursor`). Exercises the same
/// `iterm2.rs:46-49` decision branch with the explicit `inline=1`
/// parameter as primary subject so a future regression to the inline
/// decision short-circuit surfaces against this row's apex
/// (texture-render) regardless of which PNG/JPEG/BMP format row is
/// being driven.
#[test]
fn osc1337_file_inline_1_renders_at_cursor() {
    let mut harness = SpecHarness::new();
    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);
    harness.feed(&osc1337_file_inline_value_payload("1", &b64));

    assert_minimal_image_placed_at_origin(&harness, 1, 1);
}

/// Pins `iterm2.rs:46-49` download silent-drop per Decision 06 Option B:
/// `inline=0` causes the handler to early-return WITHOUT touching the
/// image cache, the placement table, or the PTY. Catalog anchor for
/// `ITERM2-1337-FILE-DOWNLOAD` (verified-with-deviation — silent drop
/// is the conformant state-snapshot absent a `HostEffect` variant).
#[test]
fn osc1337_file_inline_0_drops_payload_silently() {
    let mut harness = SpecHarness::new();
    let before_images = harness.term().image_cache().image_count();
    let before_placements = harness.term().image_cache().placement_count();

    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);
    harness.feed(&osc1337_file_inline_value_payload("0", &b64));

    assert_no_pty_writes(&harness);
    assert_eq!(
        harness.term().image_cache().image_count(),
        before_images,
        "inline=0 must NOT store an image (iterm2.rs:46-49 silent drop, Decision 06 Option B)"
    );
    assert_eq!(
        harness.term().image_cache().placement_count(),
        before_placements,
        "inline=0 must NOT create a placement"
    );
}

/// Pins `parse_iterm2_file` default at `iterm2/mod.rs:56` (`inline:
/// false`) + `iterm2.rs:47-49` early-return: a `File=` payload with NO
/// `inline=` key behaves identically to `inline=0`. Spec-default pin
/// per iTerm2 documentation (`inline` defaults to absent → download).
#[test]
fn osc1337_file_inline_absent_defaults_to_download() {
    let mut harness = SpecHarness::new();
    let before_images = harness.term().image_cache().image_count();
    let before_placements = harness.term().image_cache().placement_count();

    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);
    harness.feed(&osc1337_file_no_inline_payload(&b64));

    assert_no_pty_writes(&harness);
    assert_eq!(
        harness.term().image_cache().image_count(),
        before_images,
        "absent inline= key must default to download (parse_iterm2_file inline:false default)"
    );
    assert_eq!(
        harness.term().image_cache().placement_count(),
        before_placements,
        "absent inline= key must NOT create a placement"
    );
}

/// Pins `parse_key_value` at `iterm2/mod.rs:136-138` strict-equality
/// semantics: `image.inline = value == b"1"`. Any non-`"1"` value
/// (here `"yes"`) routes to the download silent-drop branch. Pins the
/// canonical drop-to-download behavior so a future loose-equality
/// "fix" cannot silently widen the inline match without spec-source
/// amendment. Catalog anchor co-row with `osc1337_file_inline_0_*`
/// (`ITERM2-1337-FILE-DOWNLOAD`).
#[test]
fn osc1337_file_inline_invalid_value_drops_to_download() {
    let mut harness = SpecHarness::new();
    let before_images = harness.term().image_cache().image_count();
    let before_placements = harness.term().image_cache().placement_count();

    let png = fixtures::minimal_png_bytes();
    let b64 = b64_std(&png);
    harness.feed(&osc1337_file_inline_value_payload("yes", &b64));

    assert_no_pty_writes(&harness);
    assert_eq!(
        harness.term().image_cache().image_count(),
        before_images,
        "inline=yes must drop to download (parse_key_value strict equality on b\"1\")"
    );
    assert_eq!(
        harness.term().image_cache().placement_count(),
        before_placements,
        "inline=yes must NOT create a placement"
    );
}

// ── §14.3 — dimension matrix + preserveAspectRatio (Decision 05 dual-gate) ──
//
// Pins iterm2 size-spec semantics per iterm2.com/3.4/documentation-images.html
// §Inline Images. `parse_size_spec` at `iterm2/mod.rs:146-166`:
//   - empty or `auto` -> SizeSpec::Auto
//   - `Npx` suffix    -> SizeSpec::Pixels(N)
//   - `N%` suffix     -> SizeSpec::Percent(N)
//   - plain integer N -> SizeSpec::Cells(N)
//   - parse failure   -> SizeSpec::Auto (spec-drift pin for `Nch` etc.)
//
// `resolve_display_size` at `iterm2.rs:209-255` combines width + height
// specs with `preserveAspectRatio`. Each catalog row gets a state-snapshot
// pin (this file) + GPU pilot (oriterm/src/gpu/visual_regression/spec_chain/
// pilots/iterm2_image_dim_*.rs) per Decision 05 §Consequences.
//
// Catalog rows: ITERM2-1337-FILE-DIM-AUTO, ITERM2-1337-FILE-DIM-CELLS,
// ITERM2-1337-FILE-DIM-PIXELS, ITERM2-1337-FILE-DIM-PERCENT,
// ITERM2-1337-FILE-ASPECT-PRESERVE.

/// Encode a solid-red PNG of the given dimensions and return the bytes.
/// Used by §14.3 dimension tests where the source image size matters
/// (auto-sizing, native dimensions, aspect-ratio math).
fn encode_red_png(w: u32, h: u32) -> Vec<u8> {
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(w, h, Rgba([0xFF, 0x00, 0x00, 0xFF]));
    let mut buf = Vec::new();
    img.write_to(&mut Cursor::new(&mut buf), CrateImageFormat::Png)
        .expect("encode PNG of requested dimensions");
    buf
}

/// Build an OSC 1337 File payload with the given key=value args and a
/// PNG-of-given-dimensions payload. Args appear as additional key=value
/// pairs between `File=` and the `:` separator. The empty-args form is
/// equivalent to `osc1337_file_inline_payload`.
fn osc1337_file_with_args(args: &str, b64: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(b64.len() + args.len() + 32);
    out.extend_from_slice(b"\x1b]1337;File=inline=1");
    if !args.is_empty() {
        out.push(b';');
        out.extend_from_slice(args.as_bytes());
    }
    out.push(b':');
    out.extend_from_slice(b64.as_bytes());
    out.extend_from_slice(b"\x1b\\");
    out
}

/// Returns the single placement created by the most recent successful
/// File= feed. Panics if zero or more than one placement is present.
fn only_placement(harness: &SpecHarness) -> oriterm_core::image::ImagePlacement {
    let placements = harness.term().image_cache().placements_for_test();
    assert_eq!(
        placements.len(),
        1,
        "expected exactly one placement after dimension test feed"
    );
    placements[0].clone()
}

/// Pins `SizeSpec::Auto` (no `width=` key) on a 32×32 PNG with the
/// default 8-pixel-wide cell: native pixel width = 32, cols = ceil(32/8)
/// = 4. Anchor: `ITERM2-1337-FILE-DIM-AUTO` (width axis).
#[test]
fn osc1337_file_width_auto_uses_native_size() {
    let mut harness = SpecHarness::new();
    // Default harness: cell_pixel_width=8, cell_pixel_height=16.
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("", &b64_std(&png)));

    let p = only_placement(&harness);
    // Math: ceil(32 native pixels / 8 cell pixel width) = 4.
    assert_eq!(p.cols, 4, "auto width on 32x32 image at cell_w=8 should span 4 cols");
}

/// Pins `SizeSpec::Cells(N)` (unitless integer) on the width axis:
/// `width=10` means 10 terminal cells regardless of image native size.
/// Anchor: `ITERM2-1337-FILE-DIM-CELLS` (width axis).
#[test]
fn osc1337_file_width_unitless_is_cells() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("width=10", &b64_std(&png)));

    let p = only_placement(&harness);
    assert_eq!(p.cols, 10, "width=10 (unitless) must be 10 cells");
}

/// Pins `SizeSpec::Pixels(N)` (Npx suffix) on the width axis:
/// `width=200px` with `cell_pixel_width=16` means cols = ceil(200/16) = 13.
/// Anchor: `ITERM2-1337-FILE-DIM-PIXELS` (width axis).
#[test]
fn osc1337_file_width_px_suffix_is_pixels() {
    let mut harness = SpecHarness::new();
    // Set cell_pixel_width=16, cell_pixel_height=16 so cols math is clean.
    harness.term_mut().set_cell_dimensions(16, 16);
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("width=200px", &b64_std(&png)));

    let p = only_placement(&harness);
    // Math (script-verified): ceil(200/16) = 13.
    assert_eq!(p.cols, 13, "width=200px at cell_w=16 should span 13 cols");
}

/// Pins `SizeSpec::Percent(N)` (N% suffix) on the width axis:
/// `width=50%` of an 80-col terminal (default) at `cell_pixel_width=8`
/// means `display_w` = 80*8*50/100 = 320px = 40 cells.
/// Anchor: `ITERM2-1337-FILE-DIM-PERCENT` (width axis).
#[test]
fn osc1337_file_width_percent_is_terminal_fraction() {
    let mut harness = SpecHarness::new();
    // Default: 80 cols, cell_pixel_width=8 -> term_w = 640px.
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("width=50%", &b64_std(&png)));

    let p = only_placement(&harness);
    // Math (script-verified): 80*8*50/100 = 320px / 8 = 40 cols.
    assert_eq!(p.cols, 40, "width=50% of 80-col term at cell_w=8 should span 40 cols");
}

/// Pins `SizeSpec::Auto` on the height axis: no `height=` on a 32×48
/// PNG at default `cell_pixel_height=16` yields rows = ceil(48/16) = 3.
/// Anchor: `ITERM2-1337-FILE-DIM-AUTO` (height axis — pairs with width
/// pin per /tpr-review §14 R3 codex.F4 dual-axis matrix).
#[test]
fn osc1337_file_height_auto_uses_native_size() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(32, 48);
    harness.feed(&osc1337_file_with_args("", &b64_std(&png)));

    let p = only_placement(&harness);
    // Math: ceil(48 native pixels / 16 cell pixel height) = 3.
    assert_eq!(p.rows, 3, "auto height on 32x48 image at cell_h=16 should span 3 rows");
}

/// Pins `SizeSpec::Cells(N)` on the height axis: `height=5` means 5
/// rows regardless of image native size.
/// Anchor: `ITERM2-1337-FILE-DIM-CELLS` (height axis).
#[test]
fn osc1337_file_height_unitless_is_cells() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("height=5", &b64_std(&png)));

    let p = only_placement(&harness);
    assert_eq!(p.rows, 5, "height=5 (unitless) must be 5 rows");
}

/// Pins `SizeSpec::Pixels(N)` on the height axis: `height=100px` at
/// `cell_pixel_height=20` means rows = ceil(100/20) = 5.
/// Anchor: `ITERM2-1337-FILE-DIM-PIXELS` (height axis).
#[test]
fn osc1337_file_height_px_suffix_is_pixels() {
    let mut harness = SpecHarness::new();
    // Set cell_pixel_height=20 so math is clean.
    harness.term_mut().set_cell_dimensions(8, 20);
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("height=100px", &b64_std(&png)));

    let p = only_placement(&harness);
    // Math (script-verified): ceil(100/20) = 5.
    assert_eq!(p.rows, 5, "height=100px at cell_h=20 should span 5 rows");
}

/// Pins `SizeSpec::Percent(N)` on the height axis: `height=25%` of a
/// 24-row terminal at `cell_pixel_height=16` means `display_h` =
/// 24*16*25/100 = 96px = 6 rows.
/// Anchor: `ITERM2-1337-FILE-DIM-PERCENT` (height axis).
#[test]
fn osc1337_file_height_percent_is_terminal_fraction() {
    let mut harness = SpecHarness::new();
    // Default: 24 lines, cell_pixel_height=16 -> term_h = 384px.
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("height=25%", &b64_std(&png)));

    let p = only_placement(&harness);
    // Math (script-verified): 24*16*25/100 = 96px / 16 = 6 rows.
    assert_eq!(p.rows, 6, "height=25% of 24-row term at cell_h=16 should span 6 rows");
}

/// Spec-drift pin (width): `parse_size_spec` at `iterm2/mod.rs:156-165`
/// has NO `ch` suffix arm — `20ch` neither matches `auto`, nor strips
/// `px` / `%`, nor parses cleanly as a plain integer (`"20ch".parse()`
/// returns `Err`). Per the final fallback at line 165
/// (`parse().map_or(Auto, Cells)`), the whole string falls through to
/// `SizeSpec::Auto`.
///
/// This test pins that drop-to-auto behavior so a future "fix" to add
/// `Nch` parsing cannot silently change semantics without amending the
/// spec source (iterm2.com/3.4/documentation-images.html). Catalog
/// anchor: `ITERM2-1337-FILE-DIM-CELLS` (Nch fallback face).
#[test]
fn osc1337_file_width_nch_suffix_falls_back_to_auto() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("width=20ch", &b64_std(&png)));

    let p = only_placement(&harness);
    // Auto on width axis with native=32, cell_w=8 -> 4 cols, NOT 20.
    assert_eq!(
        p.cols, 4,
        "width=20ch must fall back to Auto (native 32 px / cell 8 = 4 cols), NOT 20 cells"
    );
}

/// Spec-drift pin (height): mirror of width-Nch test. `height=20ch`
/// falls through to `SizeSpec::Auto` per the same final arm at
/// `iterm2/mod.rs:165`.
/// Catalog anchor: `ITERM2-1337-FILE-DIM-CELLS` (Nch fallback face).
#[test]
fn osc1337_file_height_nch_suffix_falls_back_to_auto() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(32, 48);
    harness.feed(&osc1337_file_with_args("height=20ch", &b64_std(&png)));

    let p = only_placement(&harness);
    // Auto on height axis with native=48, cell_h=16 -> 3 rows, NOT 20.
    assert_eq!(
        p.rows, 3,
        "height=20ch must fall back to Auto (native 48 px / cell 16 = 3 rows), NOT 20 cells"
    );
}

// ── preserveAspectRatio 4-case matrix ───────────────────────────────
//
// Per `resolve_display_size` at `iterm2.rs:209-255`:
//   - (auto, auto, preserve=1)        -> native, clamped to term width (line 223-231)
//   - (explicit_w, auto, preserve=1)  -> width raw; height scaled to ratio (line 232-241)
//   - (auto, explicit_h, preserve=1)  -> height raw; width scaled to ratio (line 242-251)
//   - (explicit_w, explicit_h, preserve=1) -> fit-within-bbox (scale-to-fit) (line 252-253)
//   - (any, any, preserve=0)          -> (raw_w, raw_h) stretch (line 213-215)

/// Pins `(auto, auto, preserve=1)`: a 32×32 image (smaller than the
/// 640-px-wide terminal) keeps its native size and produces 4 cols x 2
/// rows (at default cell 8×16).
/// Catalog anchor: `ITERM2-1337-FILE-ASPECT-PRESERVE` (auto/auto arm).
#[test]
fn osc1337_file_aspect_preserve_auto_auto_native_size_clamped_to_terminal() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args("preserveAspectRatio=1", &b64_std(&png)));

    let p = only_placement(&harness);
    // preserve=1 with both auto: native 32x32, cell 8x16 -> 4 cols, 2 rows.
    assert_eq!(p.cols, 4, "(auto,auto,preserve=1): 32 native px / cell 8 = 4 cols");
    assert_eq!(p.rows, 2, "(auto,auto,preserve=1): 32 native px / cell 16 = 2 rows");
}

/// Pins `(explicit_w, auto, preserve=1)`: `width=10` cells (80 px at
/// `cell_w=8`); height auto-scaled by aspect ratio.
/// Image native 100×50 (2:1), explicit width 80 px -> height scaled to
/// 100-to-80 ratio: 50 * 80/100 = 40 px -> ceil(40/16) = 3 rows.
/// Catalog anchor: `ITERM2-1337-FILE-ASPECT-PRESERVE` (explicit/auto arm).
#[test]
fn osc1337_file_aspect_preserve_explicit_width_auto_height_scales() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(100, 50);
    harness.feed(&osc1337_file_with_args("width=10;preserveAspectRatio=1", &b64_std(&png)));

    let p = only_placement(&harness);
    assert_eq!(p.cols, 10, "explicit width=10 cells");
    // 50 * 80/100 = 40 px / 16 = 2.5 -> ceil = 3 rows.
    assert_eq!(p.rows, 3, "(explicit,auto,preserve=1): height scaled by ratio 50*80/100=40 px / cell 16 -> 3 rows");
}

/// Pins `(auto, explicit_h, preserve=1)`: `height=4` cells (64 px at
/// `cell_h=16`); width auto-scaled by aspect ratio.
/// Image native 100×50 (2:1), explicit height 64 px -> width scaled to
/// 50-to-64 ratio: 100 * 64/50 = 128 px -> ceil(128/8) = 16 cols.
/// Catalog anchor: `ITERM2-1337-FILE-ASPECT-PRESERVE` (auto/explicit arm).
#[test]
fn osc1337_file_aspect_preserve_auto_width_explicit_height_scales() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(100, 50);
    harness.feed(&osc1337_file_with_args("height=4;preserveAspectRatio=1", &b64_std(&png)));

    let p = only_placement(&harness);
    assert_eq!(p.rows, 4, "explicit height=4 cells");
    // 100 * 64/50 = 128 px / cell 8 = 16 cols.
    assert_eq!(p.cols, 16, "(auto,explicit,preserve=1): width scaled by ratio 100*64/50=128 px / cell 8 -> 16 cols");
}

/// Pins `(explicit_w, explicit_h, preserve=1)`: spec says
/// `preserveAspectRatio=1` must FIT WITHIN the W×H bbox (scale-to-fit),
/// NOT stretch. Image 100×50 with W=80 px, H=80 px bbox:
///   `scale = min(80/100, 80/50) = min(0.8, 1.6) = 0.8`
///   `display = (100*0.8, 50*0.8) = (80, 40)` px
///   -> 80 px / cell 8 = 10 cols
///   -> `ceil(40 / 16) = 3` rows
/// Catalog anchor: `ITERM2-1337-FILE-ASPECT-PRESERVE` (explicit/explicit
/// fit-within-bbox arm — pinned post-fix at `iterm2.rs:252-253`).
#[test]
fn osc1337_file_aspect_preserve_explicit_explicit_fits_within_bbox() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(100, 50);
    harness.feed(&osc1337_file_with_args(
        "width=10;height=5;preserveAspectRatio=1",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    // Math (script-verified): scale = min(80/100, 80/50) = 0.8.
    //   out_w = 100*0.8 = 80 px -> 10 cols at cell_w=8.
    //   out_h = 50*0.8 = 40 px -> ceil(40/16) = 3 rows at cell_h=16.
    assert_eq!(p.cols, 10, "fit-within-bbox: 100*0.8=80 px / cell 8 = 10 cols");
    assert_eq!(p.rows, 3, "fit-within-bbox: ceil(50*0.8=40 px / cell 16) = 3 rows");
}

/// Pins `(explicit_w, explicit_h, preserve=0)`: the early-return at
/// `iterm2.rs:213-215` stretches the image to the exact W×H bbox with
/// no aspect-ratio preservation. Image 100×50 with W=10 cells (80 px),
/// H=5 cells (80 px): output is exactly (80, 80) px -> (10 cols, 5 rows).
/// Catalog anchor: `ITERM2-1337-FILE-ASPECT-PRESERVE` (stretch arm).
#[test]
fn osc1337_file_aspect_explicit_explicit_no_preserve_stretches() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(100, 50);
    harness.feed(&osc1337_file_with_args(
        "width=10;height=5;preserveAspectRatio=0",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    // preserve=0 stretches exactly to the W×H bbox.
    assert_eq!(p.cols, 10, "stretch (preserve=0): cols = explicit width");
    assert_eq!(p.rows, 5, "stretch (preserve=0): rows = explicit height");
}
