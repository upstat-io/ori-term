//! Per-binary kitty `spec_chain` helpers.
//!
//! APC framing, base64 encoding, and the `rgba_4x4_red` constructor live in
//! `oriterm_test_support::spec_chain::kitty_fixtures` as the cross-binary
//! SSOT; this module re-exports those and owns the `rgb_4x4_red`, `png_1x1_red`,
//! temp-dir, and effect-sink observation primitives that are scoped to the
//! kitty test binary.

use oriterm_core::effect::PtyWriteKind;
pub(super) use oriterm_test_support::spec_chain::TempDirGuard;
pub(super) use oriterm_test_support::spec_chain::kitty_fixtures::{b64, kitty_apc, rgba_4x4_red};
pub(super) use oriterm_test_support::spec_chain::sixel_fixtures::placement_count;
use oriterm_test_support::spec_chain::{
    SpecHarness, count_exact_pty_writes, pty_write_concat, pty_write_contains,
};

/// 4×4 opaque-red raw RGB payload — 48 bytes (matches `s=4,v=4,f=24`).
pub(super) fn rgb_4x4_red() -> Vec<u8> {
    let mut v = Vec::with_capacity(48);
    for _ in 0..16 {
        v.extend_from_slice(&[255, 0, 0]);
    }
    v
}

/// Minimal 1×1 red PNG for the `f=100` success path. Only available when
/// the `image-protocol` cargo feature is on (default) — without it the
/// `image` crate is not linked.
#[cfg(feature = "image-protocol")]
pub(super) fn png_1x1_red() -> Vec<u8> {
    let img = image::RgbaImage::from_pixel(1, 1, image::Rgba([255, 0, 0, 255]));
    let mut buf = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut buf), image::ImageFormat::Png)
        .expect("png encode");
    buf
}

/// RAII tmp directory for per-test fixtures. The caller MUST hold the
/// guard for the full test lifetime — `Drop` cleans up on every exit
/// path including panic unwinding, which manual `fs::remove_*` tails
/// cannot do.
pub(super) fn tmp_dir(suffix: &str) -> TempDirGuard {
    TempDirGuard::new(suffix)
}

/// Concatenation of every `ImageProtocolReply` write the harness saw
/// so the assertion messages can show the full transcript on failure.
pub(super) fn reply_bytes(h: &SpecHarness) -> Vec<u8> {
    pty_write_concat(h, PtyWriteKind::ImageProtocolReply)
}

/// True iff an `ImageProtocolReply` write contained `needle` bytes.
pub(super) fn reply_contains(h: &SpecHarness, needle: &[u8]) -> bool {
    pty_write_contains(h, PtyWriteKind::ImageProtocolReply, needle)
}

/// Count how many `ImageProtocolReply` writes matched `expected` byte-for-byte.
/// Used to distinguish "second command produced its own reply" from "first
/// command's reply was already in the transcript" (per the §13.1 review round 0
/// finding that multi-step tests were satisfied by aggregated transcripts).
pub(super) fn count_replies_exact(h: &SpecHarness, expected: &[u8]) -> usize {
    count_exact_pty_writes(h, PtyWriteKind::ImageProtocolReply, expected)
}

pub(super) fn ok_reply_for(id: u32) -> Vec<u8> {
    format!("\x1b_Gi={id};OK\x1b\\").into_bytes()
}

/// Build a solid-color RGBA payload of `w × h` pixels (`w*h*4` bytes).
pub(super) fn rgba_solid(w: u32, h: u32, r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
    let pixels = (w as usize) * (h as usize);
    let mut v = Vec::with_capacity(pixels * 4);
    for _ in 0..pixels {
        v.extend_from_slice(&[r, g, b, a]);
    }
    v
}

/// Assert frame `frame_num` (1-based) of `image_id` byte-equals `expected`.
///
/// On mismatch prints the index of the first differing byte + ±8 bytes of
/// context so the failure diagnostic is actionable. Used by the kitty
/// c=/r= dispatch matrix for byte-exact pin assertions — hand-computed
/// goldens only, NEVER derived from production blend kernels.
pub(super) fn assert_frame_eq(
    cache: &oriterm_core::image::ImageCache,
    image_id: oriterm_core::image::ImageId,
    frame_num: u32,
    expected: &[u8],
) {
    let actual = cache
        .frame_bytes_for_test(image_id, frame_num)
        .unwrap_or_else(|| panic!("frame_bytes_for_test({image_id:?}, {frame_num}) returned None"));
    if actual.as_slice() == expected {
        return;
    }
    let first_diff = actual
        .iter()
        .zip(expected.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| actual.len().min(expected.len()));
    let lo = first_diff.saturating_sub(8);
    let hi_actual = (first_diff + 8).min(actual.len());
    let hi_expected = (first_diff + 8).min(expected.len());
    panic!(
        "frame {frame_num} bytes differ at index {first_diff} \
         (actual.len={}, expected.len={}):\n  actual[{lo}..{hi_actual}] = {:?}\n\
         expected[{lo}..{hi_expected}] = {:?}",
        actual.len(),
        expected.len(),
        &actual[lo..hi_actual],
        &expected[lo..hi_expected],
    );
}

/// Assert an EINVAL reply was emitted for `image_id` containing `msg_fragment`.
pub(super) fn assert_einval_reply(h: &SpecHarness, image_id: u32, msg_fragment: &str) {
    let prefix = format!("\x1b_Gi={image_id};EINVAL");
    assert!(
        reply_contains(h, prefix.as_bytes()),
        "expected EINVAL reply for i={image_id} — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(h)),
    );
    assert!(
        reply_contains(h, msg_fragment.as_bytes()),
        "expected EINVAL reply to contain {msg_fragment:?} — transcript: {:?}",
        String::from_utf8_lossy(&reply_bytes(h)),
    );
}
