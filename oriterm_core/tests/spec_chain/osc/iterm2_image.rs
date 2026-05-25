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
use std::time::{Duration, Instant};

use image::{ImageBuffer, ImageFormat as CrateImageFormat, Rgb, Rgba};
use oriterm_core::image::{ImageFormat, decode_gif_frames};
use oriterm_test_support::spec_chain::{SpecHarness, assert_no_pty_writes};

pub(crate) mod fixtures {
    use super::{CrateImageFormat, Cursor, ImageBuffer, Rgb, Rgba};

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
    /// `decode_gif_frames` returns `None` for this (frame count ≤ 1), so the
    /// iterm2 handler routes it through the static `decode_to_rgba` path.
    pub fn minimal_gif_bytes() -> Vec<u8> {
        encode_minimal(CrateImageFormat::Gif)
    }

    /// Build an `frame_count`-frame animated GIF of `w`×`h` solid frames,
    /// each with a distinct shade and a `delay_ms` per-frame delay.
    ///
    /// `Delay::from_numer_denom_ms(delay_ms, 1)` round-trips through the
    /// GIF centisecond field: the encoder stores `delay_ms / 10`
    /// centiseconds and the decoder reconstructs `centiseconds * 10` ms.
    /// `delay_ms` multiples of 10 round-trip exactly (100 → 10 cs → 100 ms).
    pub fn animated_gif_bytes(frame_count: u32, w: u32, h: u32, delay_ms: u32) -> Vec<u8> {
        encode_gif(
            frame_count,
            w,
            h,
            delay_ms,
            image::codecs::gif::Repeat::Infinite,
        )
    }

    /// Build an `frame_count`-frame GIF with a FINITE NETSCAPE2.0 loop count
    /// of `loops`. Used to pin that `decode_gif_frames` reads the real loop
    /// block (vs. the infinite-loop default).
    pub fn finite_loop_gif_bytes(frame_count: u32, w: u32, h: u32, loops: u16) -> Vec<u8> {
        encode_gif(
            frame_count,
            w,
            h,
            100,
            image::codecs::gif::Repeat::Finite(loops),
        )
    }

    fn encode_gif(
        frame_count: u32,
        w: u32,
        h: u32,
        delay_ms: u32,
        repeat: image::codecs::gif::Repeat,
    ) -> Vec<u8> {
        use image::codecs::gif::GifEncoder;
        use image::{Delay, Frame};

        let mut buf = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut buf);
            encoder.set_repeat(repeat).expect("set GIF repeat");
            for i in 0..frame_count {
                // Distinct per-frame content so frames differ and the LZW
                // stream carries real data (truncation lands inside frame
                // bytes for the corrupt-frame fixture).
                let shade = ((i * 90) % 256) as u8;
                let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_fn(w, h, |x, y| {
                    Rgba([
                        shade,
                        (x as u8).wrapping_mul(7),
                        (y as u8).wrapping_mul(11),
                        0xFF,
                    ])
                });
                let frame = Frame::from_parts(img, 0, 0, Delay::from_numer_denom_ms(delay_ms, 1));
                encoder.encode_frame(frame).expect("encode GIF frame");
            }
        }
        buf
    }

    /// Build a multi-frame GIF whose final frame is truncated mid-data so
    /// the GIF decoder errors when it reaches that frame (every earlier
    /// frame decodes cleanly).
    ///
    /// The truncation offset is derived from a `(frame_count - 1)`-frame
    /// GIF: dropping its trailer byte yields the offset where the last
    /// frame begins in the full stream. The prefix relationship is asserted
    /// so the fixture fails loudly if the encoder ever stops producing a
    /// byte-stable frame prefix.
    pub fn corrupt_last_frame_gif_bytes(
        frame_count: u32,
        w: u32,
        h: u32,
        delay_ms: u32,
    ) -> Vec<u8> {
        assert!(frame_count >= 2, "corrupt-frame fixture needs ≥2 frames");
        let full = animated_gif_bytes(frame_count, w, h, delay_ms);
        let prefix = animated_gif_bytes(frame_count - 1, w, h, delay_ms);
        let prefix_without_trailer = &prefix[..prefix.len() - 1];
        assert!(
            full.starts_with(prefix_without_trailer),
            "GIF encoder must produce a byte-stable frame prefix for the truncation fixture"
        );
        // A few bytes into the final frame's descriptor — enough to make
        // the decoder read a partial frame and error, not stop cleanly.
        let cut = (prefix.len() - 1 + 4).min(full.len());
        full[..cut].to_vec()
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
fn assert_minimal_image_placed_at_origin(harness: &SpecHarness, expected_w: u32, expected_h: u32) {
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

// Phase 1 / 2 — per-format positive pins

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

// Phase 3 — negative pins (error paths)

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
/// `max_single_image_bytes()` MUST be dropped before decode. The cap on
/// the harness's image cache is lowered so the test stays
/// memory-bounded (vs feeding a true 64 MiB blob).
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-OVERSIZE`.
#[test]
fn osc1337_file_oversize_rejected_pre_decode() {
    let mut harness = SpecHarness::new();
    // Configure a tight cap (256 bytes). Any payload larger than this
    // must be dropped at the `image.data.len() > max_bytes` guard.
    harness
        .term_mut()
        .image_cache_mut()
        .set_max_single_image(256);

    let before_images = harness.term().image_cache().image_count();
    let before_placements = harness.term().image_cache().placement_count();

    // Construct a payload whose base64-decoded length exceeds 256 bytes:
    // 1024 bytes of "image header" + filler. The handler's oversize guard
    // rejects BEFORE decode_to_rgba is called.
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

/// Pins iterm2.rs store-failure invariant: when
/// `image_cache_mut().store(img_data)` returns `Err`, the placement is
/// NOT created. Exercises the `MemoryLimitExceeded` store-error path —
/// the genuine store failure the decode-time bound cannot preempt.
///
/// Mechanism: feed a tiny image A first (stored + placed, so its id is
/// un-evictable). Then tighten `memory_limit` below `A + B` and feed a
/// second image B that decodes fine (well under `max_single_image_bytes`)
/// but cannot be stored — the eviction loop cannot reclaim the placed A,
/// so `store(B)` returns `MemoryLimitExceeded`. B must add NO image and NO
/// placement (the cache stays at A's single entry). (Single-image
/// `OversizedImage` is now rejected earlier at decode via the
/// `decode_to_rgba` pre-allocation bound — see
/// `iterm2_decoded_rgba_total_bytes_*`.)
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-STORE`.
#[test]
fn osc1337_file_store_failure_creates_no_placement() {
    let mut harness = SpecHarness::new();

    // Image A: 1×1 PNG (4 decoded bytes) — stored + placed at the cursor.
    harness.feed(&osc1337_file_inline_payload(&b64_std(
        &fixtures::minimal_png_bytes(),
    )));
    assert_eq!(
        harness.term().image_cache().image_count(),
        1,
        "image A must store and place"
    );
    assert_eq!(harness.term().image_cache().placement_count(), 1);

    // Tighten memory_limit below A(4) + B(1024) so B cannot fit and the
    // placed A cannot be evicted to make room.
    harness.term_mut().image_cache_mut().set_memory_limit(500);

    // Image B: 16×16 solid red — decodes to 1024 bytes (under the default
    // max_single_image_bytes, so decode passes), but store fails with
    // MemoryLimitExceeded.
    let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
        ImageBuffer::from_pixel(16, 16, Rgba([0xFF, 0x00, 0x00, 0xFF]));
    let mut png_b = Vec::new();
    img.write_to(&mut Cursor::new(&mut png_b), CrateImageFormat::Png)
        .expect("encode 16x16 PNG");
    harness.feed(&osc1337_file_inline_payload(&b64_std(&png_b)));

    assert_eq!(
        harness.term().image_cache().image_count(),
        1,
        "store failure (MemoryLimitExceeded) must NOT add image B (cache stays at A)"
    );
    assert_eq!(
        harness.term().image_cache().placement_count(),
        1,
        "store failure must NOT create a placement for B"
    );
}

// Phase 4 — custom base64 decoder coverage

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
    let standard_alpha_only = b64
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=');
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

// Decoded-RGBA total bound — per-format (matrix variant of ERR-STORE)
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
/// PNG decoder cannot bypass the per-image decoded-RGBA cap. Decoded RGBA
/// 1024 bytes > cap 256; the `decode_to_rgba` pre-allocation bound rejects
/// it at decode time (before any placement), so no orphan image is stored.
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-STORE` (PNG arm).
#[test]
fn iterm2_decoded_rgba_total_bytes_within_alloc_budget_png() {
    let mut harness = SpecHarness::new();
    let png = encode_solid_16x16(CrateImageFormat::Png);
    assert!(
        png.len() <= 256,
        "16x16 solid PNG must encode ≤256 bytes for this pin"
    );
    harness
        .term_mut()
        .image_cache_mut()
        .set_max_single_image(256);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&png)));

    assert_eq!(harness.term().image_cache().image_count(), 0);
    assert_eq!(harness.term().image_cache().placement_count(), 0);
}

/// Pins `iterm2_decoded_rgba_total_bytes_within_alloc_budget_jpeg`:
/// JPEG decoder cannot bypass the per-image decoded-RGBA cap. JPEG
/// encoding overhead is large for 16×16; this test uses a 64×64 source
/// so encoded < cap < decoded. The `decode_to_rgba` pre-allocation bound
/// rejects the 16384-byte decode against the 4096 cap at decode time.
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
    harness
        .term_mut()
        .image_cache_mut()
        .set_max_single_image(4096);
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
    // the encoded-vs-decoded gap. Setting the cap below encoded makes the
    // iterm2.rs compressed-size pre-guard AND the decode pre-allocation
    // bound both reject; the same observable outcome (no orphan, no
    // placement) is pinned. Test name reflects the decoded-RGBA bound.
    let bmp = encode_solid_16x16(CrateImageFormat::Bmp);
    harness
        .term_mut()
        .image_cache_mut()
        .set_max_single_image(bmp.len() / 4);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&bmp)));

    assert_eq!(harness.term().image_cache().image_count(), 0);
    assert_eq!(harness.term().image_cache().placement_count(), 0);
}

/// Pins the `decode_to_rgba` decompression-bomb bound directly: a
/// solid-fill PNG of large dimensions compresses tiny (passes any
/// compressed-size pre-guard) but decodes to `w*h*4` bytes. With a budget
/// below the decoded size, `decode_to_rgba` must reject at DECODE time
/// (`ImageReader` `max_alloc` limit), not allocate the full buffer first.
/// 2048×2048 decodes to 16 MiB (script-verified:
/// `python3 -c "print(2048*2048*4)"` = 16777216); a 1 MiB budget rejects.
/// Anchor: catalog row `ITERM2-1337-FILE-ERR-STORE` (static decode bound).
#[test]
fn decode_to_rgba_rejects_decompression_bomb_at_decode_time() {
    use oriterm_core::image::decode_to_rgba;
    // 2048×2048 solid red — compresses small, decodes to 16 MiB.
    let bomb = encode_red_png(2048, 2048);
    assert!(
        decode_to_rgba(&bomb, 1024 * 1024).is_err(),
        "16 MiB decode against a 1 MiB budget must be rejected at decode time"
    );
    // Positive companion: a generous budget decodes the same PNG.
    let decoded =
        decode_to_rgba(&bomb, 64 * 1024 * 1024).expect("same PNG decodes under a generous budget");
    assert_eq!(decoded.1, 2048, "decoded width");
    assert_eq!(decoded.2, 2048, "decoded height");
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

// §14.2 — inline vs download mode (per Decision 06 Option B)
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

// §14.3 — dimension matrix + preserveAspectRatio (Decision 05 dual-gate)
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
    assert_eq!(
        p.cols, 4,
        "auto width on 32x32 image at cell_w=8 should span 4 cols"
    );
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

/// §14.5 integration pin (positive): a font-metric change recomputes an
/// iTerm2-routed `FixedPixels` placement's cell-coverage through the REAL OSC
/// path (the cross-protocol matrix bypasses OSC, constructing placements
/// directly). `preserveAspectRatio=0` (stretch) is
/// required — the spec default `=1` would fit the 1×1 image within the
/// 32×64 bbox → 32×32, giving (4,2) not the intended exact FixedPixels{32,64}.
/// At cell (8,16): cols=ceil(32/8)=4, rows=ceil(64/16)=4; after
/// `set_cell_dimensions(16,8)`: cols=ceil(32/16)=2, rows=ceil(64/8)=8.
/// Anchor: ITERM2-1337-FILE-INLINE (font-metric-change recompute).
#[test]
fn iterm2_font_metric_change_recomputes_fixed_pixel_placement() {
    let mut harness = SpecHarness::new();
    harness.term_mut().set_cell_dimensions(8, 16);
    let png = encode_red_png(1, 1);
    harness.feed(&osc1337_file_with_args(
        "preserveAspectRatio=0;width=32px;height=64px",
        &b64_std(&png),
    ));
    {
        let p = only_placement(&harness);
        assert_eq!(
            (p.cols, p.rows),
            (4, 4),
            "FixedPixels{{32,64}} at cell (8,16) covers (4,4)"
        );
    }
    harness.term_mut().set_cell_dimensions(16, 8);
    let p = only_placement(&harness);
    assert_eq!(
        (p.cols, p.rows),
        (2, 8),
        "set_cell_dimensions(16,8) recomputes the iTerm2 FixedPixels placement to (2,8)"
    );
}

/// §14.5 integration pin (negative): a both-cells iTerm2 spec
/// (`width=3;height=2`) resolves to `CellCount`, which `update_cell_coverage`
/// leaves UNTOUCHED on a font-metric change — pairs with the positive pin to
/// prove the recompute is gated to `FixedPixels`.
/// Anchor: ITERM2-1337-FILE-INLINE (font-metric-change CellCount no-op).
#[test]
fn iterm2_font_metric_change_leaves_cell_count_placement_unchanged() {
    let mut harness = SpecHarness::new();
    harness.term_mut().set_cell_dimensions(8, 16);
    let png = encode_red_png(16, 16);
    harness.feed(&osc1337_file_with_args("width=3;height=2", &b64_std(&png)));
    {
        let p = only_placement(&harness);
        assert_eq!(
            (p.cols, p.rows),
            (3, 2),
            "both-cells spec → CellCount (3,2)"
        );
    }
    harness.term_mut().set_cell_dimensions(16, 8);
    let p = only_placement(&harness);
    assert_eq!(
        (p.cols, p.rows),
        (3, 2),
        "CellCount placement is unchanged by set_cell_dimensions"
    );
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
    assert_eq!(
        p.cols, 40,
        "width=50% of 80-col term at cell_w=8 should span 40 cols"
    );
}

/// Pins `SizeSpec::Auto` on the height axis: no `height=` on a 32×48
/// PNG at default `cell_pixel_height=16` yields rows = ceil(48/16) = 3.
/// Anchor: `ITERM2-1337-FILE-DIM-AUTO` (height axis — pairs with the
/// width pin for dual-axis dimension coverage).
#[test]
fn osc1337_file_height_auto_uses_native_size() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(32, 48);
    harness.feed(&osc1337_file_with_args("", &b64_std(&png)));

    let p = only_placement(&harness);
    // Math: ceil(48 native pixels / 16 cell pixel height) = 3.
    assert_eq!(
        p.rows, 3,
        "auto height on 32x48 image at cell_h=16 should span 3 rows"
    );
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
    assert_eq!(
        p.rows, 6,
        "height=25% of 24-row term at cell_h=16 should span 6 rows"
    );
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

// preserveAspectRatio 4-case matrix
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
    harness.feed(&osc1337_file_with_args(
        "preserveAspectRatio=1",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    // preserve=1 with both auto: native 32x32, cell 8x16 -> 4 cols, 2 rows.
    assert_eq!(
        p.cols, 4,
        "(auto,auto,preserve=1): 32 native px / cell 8 = 4 cols"
    );
    assert_eq!(
        p.rows, 2,
        "(auto,auto,preserve=1): 32 native px / cell 16 = 2 rows"
    );
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
    harness.feed(&osc1337_file_with_args(
        "width=10;preserveAspectRatio=1",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    assert_eq!(p.cols, 10, "explicit width=10 cells");
    // 50 * 80/100 = 40 px / 16 = 2.5 -> ceil = 3 rows.
    assert_eq!(
        p.rows, 3,
        "(explicit,auto,preserve=1): height scaled by ratio 50*80/100=40 px / cell 16 -> 3 rows"
    );
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
    harness.feed(&osc1337_file_with_args(
        "height=4;preserveAspectRatio=1",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    assert_eq!(p.rows, 4, "explicit height=4 cells");
    // 100 * 64/50 = 128 px / cell 8 = 16 cols.
    assert_eq!(
        p.cols, 16,
        "(auto,explicit,preserve=1): width scaled by ratio 100*64/50=128 px / cell 8 -> 16 cols"
    );
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
    assert_eq!(
        p.cols, 10,
        "fit-within-bbox: 100*0.8=80 px / cell 8 = 10 cols"
    );
    assert_eq!(
        p.rows, 3,
        "fit-within-bbox: ceil(50*0.8=40 px / cell 16) = 3 rows"
    );
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

// §14.4 — GIF frame extraction + animation
//
// Pins multi-frame GIF decoding via `decode_gif_frames` +
// `ImageCache::store_animated`, and the iterm2 handler's GIF routing at
// `iterm2.rs:60-92`. Two test layers:
//
//   * Direct `decode_gif_frames` unit pins — the decoder's frame-count
//     gate, corrupt-frame fail-whole-decode contract, decoded-RGBA total
//     budget bound, and frame-0 canvas SSOT.
//   * OSC end-to-end pins — single-frame static fallback, multi-frame
//     animation promotion, corrupt-frame + oversized degradation to the
//     static first frame, zero-delay no-panic, loop-count deviation, and
//     the state-level `advance_animations` frame-transition sequence.
//
// Catalog rows: ITERM2-1337-FILE-GIF, ITERM2-1337-FILE-ANIMATED-GIF.
//
// Plan deviations recorded in §14.4 HISTORY:
//   * `gif_frame_zero_duration_does_not_div_by_zero` (denom=0): the
//     image-0.25 GIF decoder always builds `Delay` with denominator 1
//     (gif.rs:459 `Ratio::new(frame.delay*10, 1)`), so the `checked_div`
//     denom=0 arm is unreachable via real GIFs. The reachable case is
//     numerator=0 (zero-centisecond delay) — pinned by
//     `osc1337_file_gif_zero_delay_frame_decodes_without_panic`.
//   * `gif_canvas_size_from_frame_zero`: the decoder composites every
//     frame onto the logical-screen canvas (gif.rs:388-414), so all
//     `into_frames()` buffers are already canvas-sized and the resize
//     branch at decode.rs guards the external image-crate contract.
//     `decode_gif_frames_uses_frame_zero_canvas_size` pins the observable
//     SSOT (width/height from frame 0; all frames uniformly sized).

/// A generous per-image byte budget for direct `decode_gif_frames` calls
/// where the budget is not the subject under test.
const UNBOUNDED_GIF_BUDGET: usize = usize::MAX;

/// Pins `decode.rs` frame-count gate: a single-frame GIF returns `None`
/// from `decode_gif_frames`, so the iterm2 handler falls through to the
/// static `decode_to_rgba` path. No animation promotion, exactly one
/// static image at the cursor.
/// Anchor: catalog row `ITERM2-1337-FILE-GIF`.
#[test]
fn osc1337_file_single_frame_gif_takes_static_path() {
    let mut harness = SpecHarness::new();
    let gif = fixtures::minimal_gif_bytes();
    // Direct pin: single-frame GIF is not animatable.
    assert!(
        decode_gif_frames(&gif, UNBOUNDED_GIF_BUDGET).is_none(),
        "single-frame GIF must return None from decode_gif_frames (frame count ≤ 1)"
    );

    harness.feed(&osc1337_file_inline_payload(&b64_std(&gif)));

    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "single-frame GIF must store one static image"
    );
    assert_eq!(
        cache.placement_count(),
        1,
        "single-frame GIF must create one placement"
    );
    let id = cache.placements_for_test()[0].image_id;
    assert!(
        !cache.animation_promoted_for_test(id),
        "single-frame GIF must NOT be promoted to an animation"
    );
    assert_eq!(
        cache.total_frames_for_test(id),
        1,
        "single-frame GIF static path stores exactly one frame"
    );
}

/// Pins multi-frame GIF promotion: a 3-frame GIF is stored as an
/// animation via `store_animated` — animation-promoted, 3 frames, 3
/// per-frame gaps, one placement at the cursor.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn osc1337_file_multi_frame_gif_stores_animated() {
    let mut harness = SpecHarness::new();
    let gif = fixtures::animated_gif_bytes(3, 2, 2, 100);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&gif)));

    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "3-frame GIF stores one (animated) image"
    );
    assert_eq!(
        cache.placement_count(),
        1,
        "3-frame GIF creates one placement"
    );
    let id = cache.placements_for_test()[0].image_id;
    assert!(
        cache.animation_promoted_for_test(id),
        "3-frame GIF must be promoted to an animation"
    );
    assert_eq!(
        cache.total_frames_for_test(id),
        3,
        "3-frame GIF retains all 3 frames"
    );

    let snap = cache
        .animation_snapshot(id)
        .expect("animated image must expose an animation snapshot");
    assert_eq!(
        snap.total_frames, 3,
        "snapshot total_frames matches frame count"
    );
    assert_eq!(
        snap.frame_gaps.len(),
        3,
        "snapshot carries one gap per frame"
    );
    assert!(
        snap.frame_gaps
            .iter()
            .all(|g| *g == Duration::from_millis(100)),
        "each 100ms-delay frame round-trips to a 100ms gap; got {:?}",
        snap.frame_gaps
    );
}

/// Pins the fail-whole-decode contract (swallowed-error cure at
/// `decode.rs`): a GIF whose final frame is truncated mid-data makes
/// `decode_gif_frames` return `None` rather than silently dropping the
/// bad frame and returning a partial animation. This FAILS if the decoder
/// reverts to `filter_map(Result::ok)` (which would return `Some` with
/// the surviving frames), so it doubles as the regression guard against
/// that revert.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn decode_gif_frames_fails_whole_decode_on_corrupt_frame() {
    let corrupt = fixtures::corrupt_last_frame_gif_bytes(3, 32, 32, 100);
    assert!(
        decode_gif_frames(&corrupt, UNBOUNDED_GIF_BUDGET).is_none(),
        "a corrupt non-first frame must fail the whole decode (no silent truncation)"
    );
    // Companion positive: the same shape WITHOUT corruption decodes fully.
    let intact = fixtures::animated_gif_bytes(3, 32, 32, 100);
    let frames = decode_gif_frames(&intact, UNBOUNDED_GIF_BUDGET)
        .expect("an intact 3-frame GIF must decode");
    assert_eq!(frames.frames.len(), 3, "intact GIF yields all 3 frames");
}

/// End-to-end pin for the corrupt-frame fallback: feeding a GIF with a
/// truncated final frame through the OSC path degrades to the static
/// first frame (via `decode_to_rgba`) — exactly one static image, no
/// animation promotion.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn osc1337_file_gif_with_corrupt_frame_falls_back_to_first_frame() {
    let mut harness = SpecHarness::new();
    let corrupt = fixtures::corrupt_last_frame_gif_bytes(3, 32, 32, 100);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&corrupt)));

    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "corrupt GIF degrades to the static first frame (one image)"
    );
    assert_eq!(
        cache.placement_count(),
        1,
        "corrupt-GIF fallback creates one placement"
    );
    let id = cache.placements_for_test()[0].image_id;
    assert!(
        !cache.animation_promoted_for_test(id),
        "corrupt GIF must NOT be stored as a (partial) animation"
    );
    assert_eq!(
        cache.total_frames_for_test(id),
        1,
        "fallback stores only the static first frame"
    );
}

/// Pins the reachable zero-delay path in `decode_gif_frames` (the
/// `checked_div(denom)` per-frame delay handling): a GIF whose
/// frames declare a 0-centisecond delay decodes to `Duration::ZERO` gaps
/// without panicking (the `checked_div` guard yields 0ms, not a div-by-
/// zero). Replaces the plan's denom=0 framing — the image-0.25 decoder
/// always sets the `Delay` denominator to 1, so only numerator=0 is
/// reachable from real GIFs.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn osc1337_file_gif_zero_delay_frame_decodes_without_panic() {
    let mut harness = SpecHarness::new();
    let gif = fixtures::animated_gif_bytes(3, 2, 2, 0);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&gif)));

    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "zero-delay GIF still stores one image"
    );
    let id = cache.placements_for_test()[0].image_id;
    assert!(
        cache.animation_promoted_for_test(id),
        "a 3-frame zero-delay GIF is still a multi-frame animation"
    );
    let snap = cache
        .animation_snapshot(id)
        .expect("animated snapshot present");
    assert!(
        snap.frame_gaps.iter().all(|g| *g == Duration::ZERO),
        "0-centisecond delay frames decode to ZERO gaps; got {:?}",
        snap.frame_gaps
    );
}

/// Pins the frame-0 canvas SSOT (the `frames[0].buffer()` width/height
/// read in `decode_gif_frames`): `decode_gif_frames`
/// takes width/height from frame 0, and every decoded frame is uniformly
/// canvas-sized (the image-0.25 decoder composites each frame onto the
/// logical screen, so all `into_frames()` buffers share frame-0
/// dimensions).
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn decode_gif_frames_uses_frame_zero_canvas_size() {
    let gif = fixtures::animated_gif_bytes(3, 4, 6, 100);
    let decoded =
        decode_gif_frames(&gif, UNBOUNDED_GIF_BUDGET).expect("a 3-frame 4x6 GIF must decode");
    assert_eq!(decoded.width, 4, "canvas width comes from frame 0");
    assert_eq!(decoded.height, 6, "canvas height comes from frame 0");
    let canvas_bytes = (4 * 6 * 4) as usize;
    assert!(
        decoded.frames.iter().all(|f| f.len() == canvas_bytes),
        "every frame is canvas-sized (4*6*4 = {canvas_bytes} RGBA bytes)"
    );
}

/// Pins that an infinite-loop GIF (NETSCAPE2.0 `Repeat::Infinite`) decodes
/// to `loop_count == None`. `decode_gif_frames` reads the real loop block
/// via `GifDecoder::loop_count()`; `LoopCount::Infinite` maps to `None`.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn gif_infinite_loop_decodes_to_none_loop_count() {
    let mut harness = SpecHarness::new();
    let gif = fixtures::animated_gif_bytes(3, 2, 2, 100);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&gif)));

    let cache = harness.term().image_cache();
    let id = cache.placements_for_test()[0].image_id;
    let snap = cache
        .animation_snapshot(id)
        .expect("animated snapshot present");
    assert_eq!(
        snap.loop_count, None,
        "Repeat::Infinite GIF maps to loop_count None"
    );
}

/// Pins that a FINITE-loop GIF's NETSCAPE2.0 loop count is read from the
/// stream, not hardcoded. A `Repeat::Finite(3)` GIF must decode to
/// `loop_count == Some(3)`. This FAILS if `decode_gif_frames` reverts to
/// hardcoding `loop_count: None` (the prior false-premise behavior).
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn gif_finite_loop_count_is_read_from_netscape_block() {
    let gif = fixtures::finite_loop_gif_bytes(3, 2, 2, 3);
    let decoded =
        decode_gif_frames(&gif, UNBOUNDED_GIF_BUDGET).expect("3-frame finite-loop GIF decodes");
    assert_eq!(
        decoded.loop_count,
        Some(3),
        "finite NETSCAPE2.0 loop count 3 must be read into loop_count Some(3)"
    );
}

/// Rejects an over-budget decoded-RGBA total (GIF decompression-bomb
/// guard): `decode_gif_frames` returns `None` for a multi-frame GIF whose
/// decoded total exceeds the budget. The `set_limits` pre-allocation bound
/// and/or the cumulative `running_bytes` guard reject it before a full
/// animation is materialized. 3-frame 32×32 = 3 × 4096 = 12288 RGBA bytes;
/// a 5000-byte budget cannot hold it.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn decode_gif_frames_bails_when_decoded_total_exceeds_budget() {
    let gif = fixtures::animated_gif_bytes(3, 32, 32, 100);
    assert!(
        decode_gif_frames(&gif, 5000).is_none(),
        "decoded total 12288 > budget 5000 must bail (decompression-bomb guard)"
    );
}

/// Positive companion to the budget guard: the same 3-frame GIF decodes
/// fully when the budget is generous.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn decode_gif_frames_succeeds_within_budget() {
    let gif = fixtures::animated_gif_bytes(3, 32, 32, 100);
    let decoded = decode_gif_frames(&gif, UNBOUNDED_GIF_BUDGET)
        .expect("3-frame GIF decodes under a generous budget");
    assert_eq!(decoded.frames.len(), 3, "all 3 frames decode within budget");
}

/// Rejects a frame-count decompression bomb: a GIF whose frame count
/// exceeds `MAX_GIF_FRAMES` returns `None` even when every frame is tiny
/// and the cumulative byte total stays under budget. Guards the
/// frame-count ceiling in `decode_gif_frames`.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn decode_gif_frames_bails_when_frame_count_exceeds_cap() {
    use oriterm_core::image::MAX_GIF_FRAMES;
    // One past the cap; 1×1 frames keep the byte total trivially small so
    // ONLY the frame-count ceiling can reject this payload.
    let gif = fixtures::animated_gif_bytes((MAX_GIF_FRAMES + 1) as u32, 1, 1, 100);
    assert!(
        decode_gif_frames(&gif, UNBOUNDED_GIF_BUDGET).is_none(),
        "frame count > MAX_GIF_FRAMES must bail regardless of byte budget"
    );
}

/// End-to-end pin: an animated GIF whose decoded total exceeds the cache's
/// per-image cap degrades to the static first frame rather than being
/// stored as an animation. `max_single_image_bytes = 5000` admits the
/// first frame (4096) but not the 3-frame total (12288), so
/// `decode_gif_frames` bails and the handler falls back to
/// `decode_to_rgba`.
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn osc1337_file_oversized_animation_degrades_to_static_first_frame() {
    let mut harness = SpecHarness::new();
    harness
        .term_mut()
        .image_cache_mut()
        .set_max_single_image(5000);
    let gif = fixtures::animated_gif_bytes(3, 32, 32, 100);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&gif)));

    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "oversized-total GIF degrades to one static first frame"
    );
    let id = cache.placements_for_test()[0].image_id;
    assert!(
        !cache.animation_promoted_for_test(id),
        "oversized-total GIF must NOT be stored as an animation"
    );
    assert_eq!(
        cache.total_frames_for_test(id),
        1,
        "fallback stores the static first frame"
    );
}

/// Pins the animated-GIF `store_animated` failure branch
/// (`iterm2.rs:87-90`): when `store_animated` returns `Err`, no image and
/// no placement are created. Exercises the `MemoryLimitExceeded` path the
/// decode bound cannot preempt — `decode_gif_frames` succeeds (frames fit
/// `max_single_image_bytes`) but the inner `store` of frame 0 cannot
/// reclaim the placed image A to make room. Distinct from
/// `osc1337_file_oversized_animation_degrades_to_static_first_frame`
/// (which bails inside `decode_gif_frames` before `store_animated`).
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn osc1337_animated_gif_store_failure_creates_no_placement() {
    let mut harness = SpecHarness::new();

    // Image A: 1×1 PNG — stored + placed (un-evictable, reachable).
    harness.feed(&osc1337_file_inline_payload(&b64_std(
        &fixtures::minimal_png_bytes(),
    )));
    assert_eq!(
        harness.term().image_cache().image_count(),
        1,
        "image A stored"
    );
    assert_eq!(harness.term().image_cache().placement_count(), 1);

    // Tighten memory_limit so the animated GIF's first-frame store cannot
    // make room (A is placed, so eviction cannot reclaim it).
    harness.term_mut().image_cache_mut().set_memory_limit(100);

    // 3-frame 8×8 GIF: each frame 256 decoded bytes, total 768 — all under
    // the default max_single_image_bytes, so decode_gif_frames succeeds.
    // store_animated's inner store(frame0=256) then trips
    // MemoryLimitExceeded (memory_used 4 + 256 > 100, A un-evictable).
    let gif = fixtures::animated_gif_bytes(3, 8, 8, 100);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&gif)));

    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "store_animated MemoryLimitExceeded must NOT add the animated image (cache stays at A)"
    );
    assert_eq!(
        cache.placement_count(),
        1,
        "store_animated failure must NOT create a placement for the GIF"
    );
    let id = cache.placements_for_test()[0].image_id;
    assert!(
        !cache.animation_promoted_for_test(id),
        "only static image A remains; the failed animated GIF added nothing"
    );
}

/// State-level animation-timing pin: drive `Term::advance_animations` at
/// explicit frame-delay boundaries and assert the stepwise frame
/// sequence, the
/// `pixel_generation` GPU-reupload bump on every transition, and
/// monotonic next-frame deadlines. Clock is injected via the `now`
/// parameter (no wall-clock dependency per `tests.md §Wall-Clock-Free
/// Testing`).
/// Anchor: catalog row `ITERM2-1337-FILE-ANIMATED-GIF`.
#[test]
fn iterm2_animated_gif_advance_animations_drives_frame_transitions() {
    let mut harness = SpecHarness::new();
    // 3-frame GIF, 100ms per frame, infinite loop (loop_count None).
    let gif = fixtures::animated_gif_bytes(3, 2, 2, 100);
    harness.feed(&osc1337_file_inline_payload(&b64_std(&gif)));

    let id = harness.term().image_cache().placements_for_test()[0].image_id;
    let gen0 = harness
        .term()
        .image_cache()
        .image_pixel_generation_for_test(id)
        .expect("animated image present");

    let t0 = Instant::now();
    // Step the call sequence past each 100ms boundary. The first call at
    // t0 only seeds frame_starts (elapsed 0, no transition); each later
    // call crosses one 100ms boundary and advances exactly one frame.
    let offsets_ms = [0u64, 150, 300, 450, 600];
    let expected_frames = [0usize, 1, 2, 0, 1];

    let mut last_deadline: Option<Instant> = None;
    let mut last_gen = gen0;
    let mut last_frame = 0usize;

    for (step, (&off, &want_frame)) in offsets_ms.iter().zip(expected_frames.iter()).enumerate() {
        let now = t0 + Duration::from_millis(off);
        let deadline = harness.term_mut().advance_animations(now);

        let cache = harness.term().image_cache();
        let snap = cache
            .animation_snapshot(id)
            .expect("animated snapshot present");
        assert_eq!(
            snap.current_frame, want_frame,
            "step {step} (+{off}ms): current_frame must be {want_frame}"
        );

        let pixel_gen = cache
            .image_pixel_generation_for_test(id)
            .expect("animated image present");
        if snap.current_frame != last_frame {
            assert!(
                pixel_gen > last_gen,
                "step {step}: pixel_generation must bump on a frame transition ({last_gen} -> {pixel_gen})"
            );
        }
        last_gen = pixel_gen;
        last_frame = snap.current_frame;

        let d = deadline.expect("an active infinite animation always reports a next deadline");
        if let Some(prev) = last_deadline {
            assert!(
                d > prev,
                "step {step} (+{off}ms): next deadline must be strictly increasing"
            );
        }
        last_deadline = Some(d);
    }
}

// §14.5 — iTerm2 + image lifecycle interactions: terminal-bounds clamping

/// Pins that an image taller than the terminal does NOT scroll off the
/// screen. `width=10;height=100;preserveAspectRatio=0` (stretch to 100
/// rows) against a 20-row terminal: without the fix the linefeed loop at
/// `iterm2.rs` scrolls `rows-1` lines and evicts the placement off the
/// top. With the fix `resolve_display_size` clamps the display height to
/// the terminal, so `rows <= term_lines` and the image survives.
/// Anchor: catalog rows ITERM2-1337-FILE-INLINE / ITERM2-1337-FILE-ANIMATED-GIF
/// (lifecycle survival).
#[test]
fn osc1337_image_taller_than_terminal_does_not_scroll_off_screen() {
    let mut harness = SpecHarness::with_size(20, 80);
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args(
        "width=10;height=100;preserveAspectRatio=0",
        &b64_std(&png),
    ));

    let cache = harness.term().image_cache();
    assert_eq!(
        cache.image_count(),
        1,
        "an over-tall image must survive (not scroll off-screen + evict)"
    );
    let p = only_placement(&harness);
    // Exact independent-clamp result (preserveAspectRatio=0): width
    // 10*8=80 px <= 640 -> 10 cols (unchanged); height 100*16=1600 px
    // clamped to 20*16=320 px -> ceil(320/16) = 20 rows == term_lines.
    assert_eq!(p.cols, 10, "width within terminal: 10 cols unchanged");
    assert_eq!(
        p.rows, 20,
        "over-tall height clamps exactly to term_lines (20), not merely <= 20"
    );
}

/// Pins that an explicit width exceeding the terminal width preserves the
/// source aspect ratio. `width=200;preserveAspectRatio=1` against an
/// 80-col terminal with a 100×50 (2:1) source: width 200 cells = 1600px
/// auto-scales height to 800px (2:1), exceeding the terminal (640×384px).
/// `resolve_display_size` must scale BOTH axes down to fit
/// (scale=min(640/1600,384/800)=0.4 -> 640×320px -> 80 cols × 20 rows,
/// still 2:1). The old call-site `display_w.min(term_w)` clamped width to
/// 640 (80 cols) but left height at 800 (50 rows), breaking the ratio.
/// Anchor: ITERM2-1337-FILE-ASPECT-PRESERVE (over-wide arm).
#[test]
fn osc1337_explicit_width_exceeding_terminal_preserves_aspect() {
    let mut harness = SpecHarness::new(); // default 80 cols × 24 lines, cell 8×16
    let png = encode_red_png(100, 50);
    harness.feed(&osc1337_file_with_args(
        "width=200;preserveAspectRatio=1",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    // Fit-within-terminal preserving 2:1: 640×320 px -> 80 cols, 20 rows.
    assert_eq!(p.cols, 80, "over-wide width clamps to 80 cols (term width)");
    assert_eq!(
        p.rows, 20,
        "height scales proportionally with the clamped width (2:1 preserved), NOT left at 50"
    );
}

/// Pins the (explicit, explicit, preserve=1) fit-within-bbox behavior
/// through the full placement path (matrix-context confirmation of the
/// §14.3 unit pin): `width=10;height=5;preserveAspectRatio=1` against a
/// wide 100×50 source fits within the 10×5-cell bbox preserving aspect.
/// Anchor: ITERM2-1337-FILE-ASPECT-PRESERVE (bbox arm, lifecycle path).
#[test]
fn osc1337_preserve_aspect_bbox_fit_holds_in_placement_path() {
    let mut harness = SpecHarness::new();
    let png = encode_red_png(100, 50);
    harness.feed(&osc1337_file_with_args(
        "width=10;height=5;preserveAspectRatio=1",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    // scale = min(80/100, 80/50) = 0.8 -> (80,40) px -> 10 cols, 3 rows.
    assert_eq!(p.cols, 10, "bbox fit: 100*0.8=80 px / cell 8 = 10 cols");
    assert_eq!(p.rows, 3, "bbox fit: ceil(50*0.8=40 px / cell 16) = 3 rows");
}

/// Pins that an attacker-controlled oversize dimension value does NOT
/// overflow (debug panic / release wrap). `width=4000000000` (≈4e9 cells)
/// would overflow `n * cell_size` in u32; `resolve_one_dimension` uses
/// saturating arithmetic and `clamp_to_terminal` bounds the result, so the
/// placement is created with cols clamped to the terminal width and no
/// panic. Guards the untrusted-input arithmetic-safety fix.
/// Anchor: ITERM2-1337-FILE-DIM-CELLS (overflow-safety face).
#[test]
fn osc1337_oversize_dimension_value_does_not_overflow() {
    let mut harness = SpecHarness::new(); // 80 cols × 24 lines
    let png = encode_red_png(32, 32);
    // 4_000_000_000 cells * 8 px/cell overflows u32 without saturation.
    harness.feed(&osc1337_file_with_args(
        "width=4000000000;preserveAspectRatio=0",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    assert!(
        p.cols <= 80,
        "oversize width must clamp to term cols (80), got {}",
        p.cols
    );
    assert!(p.cols >= 1, "placement must keep a non-zero width");
}

/// Pins the `clamp_to_terminal` preserve-aspect HEIGHT-tighter branch
/// (the `else { (term_h, h) }` arm): an auto-width image whose explicit
/// height makes it taller-than-wide and exceeds the terminal must scale
/// BOTH axes down by the height bound, preserving aspect. `height=100`
/// (auto width) on a 32×32 source: within-spec (true,false) arm gives
/// 1600×1600 px (square source); clamp picks height as the tighter bound
/// (640*1600 > 384*1600) → scale=384/1600 → 384×384 px → 48 cols × 24 rows
/// (rows == term_lines, anti-scroll invariant holds). Complements the
/// width-tighter pin `osc1337_explicit_width_exceeding_terminal_preserves_aspect`.
/// Anchor: ITERM2-1337-FILE-ASPECT-PRESERVE (height-tighter clamp arm).
#[test]
fn osc1337_auto_width_over_tall_height_preserves_aspect_via_height_clamp() {
    let mut harness = SpecHarness::new(); // 80 cols × 24 lines, cell 8×16
    let png = encode_red_png(32, 32);
    harness.feed(&osc1337_file_with_args(
        "height=100;preserveAspectRatio=1",
        &b64_std(&png),
    ));

    let p = only_placement(&harness);
    // within-spec 1600×1600 px; clamp height-tighter -> 384×384 px.
    assert_eq!(
        p.cols, 48,
        "height-tighter clamp: 384 px / cell 8 = 48 cols"
    );
    assert_eq!(
        p.rows, 24,
        "height-tighter clamp: 384 px / cell 16 = 24 rows (== term_lines)"
    );
}
