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
