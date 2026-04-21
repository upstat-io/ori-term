//! Shared fixtures + helpers for kitty §13 spec_chain tests.
//!
//! SSOT for the APC framing, base64 encoding, fixed-payload constructors,
//! temp-dir helper, and effect-sink observation primitives consumed by the
//! per-action / per-format / per-transmission sibling files. Helpers are
//! `pub(super)` — scoped to `tests/spec_chain/kitty/` only.

use std::path::PathBuf;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

use oriterm_core::effect::{Effect, PtyEffect, PtyWriteKind};
use oriterm_test_support::spec_chain::SpecHarness;
pub(super) use oriterm_test_support::spec_chain::sixel_fixtures::placement_count;

/// Build the full `\x1b_G<control>;<payload_b64>\x1b\\` APC command.
pub(super) fn kitty_apc(control: &[u8], payload_b64: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(3 + control.len() + 1 + payload_b64.len() + 2);
    out.extend_from_slice(b"\x1b_G");
    out.extend_from_slice(control);
    if !payload_b64.is_empty() {
        out.push(b';');
        out.extend_from_slice(payload_b64.as_bytes());
    }
    out.extend_from_slice(b"\x1b\\");
    out
}

pub(super) fn b64(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// 4×4 opaque-red raw RGBA payload — 64 bytes (matches `s=4,v=4,f=32`).
pub(super) fn rgba_4x4_red() -> Vec<u8> {
    let mut v = Vec::with_capacity(64);
    for _ in 0..16 {
        v.extend_from_slice(&[255, 0, 0, 255]);
    }
    v
}

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

/// A parallel-safe tmp directory under $TMPDIR, keyed by PID + a per-test
/// suffix so concurrent `cargo test` threads don't collide.
pub(super) fn tmp_dir(suffix: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "oriterm_spec13_1_{}_{}",
        std::process::id(),
        suffix
    ));
    std::fs::create_dir_all(&dir).expect("tmp dir create");
    dir
}

/// Concatenation of every `ImageProtocolReply` write the harness saw
/// so the assertion messages can show the full transcript on failure.
pub(super) fn reply_bytes(h: &SpecHarness) -> Vec<u8> {
    let mut out = Vec::new();
    for e in &h.outcome().effects_emitted {
        if let Effect::Pty(PtyEffect::Write {
            kind: PtyWriteKind::ImageProtocolReply,
            bytes,
        }) = e
        {
            out.extend_from_slice(bytes);
        }
    }
    out
}

/// True iff an `ImageProtocolReply` write contained `needle` bytes.
pub(super) fn reply_contains(h: &SpecHarness, needle: &[u8]) -> bool {
    h.outcome().effects_emitted.iter().any(|e| {
        matches!(e, Effect::Pty(PtyEffect::Write {
            kind: PtyWriteKind::ImageProtocolReply,
            bytes,
        }) if bytes.windows(needle.len()).any(|w| w == needle))
    })
}

/// Count how many `ImageProtocolReply` writes matched `expected` byte-for-byte.
/// Used to distinguish "second command produced its own reply" from "first
/// command's reply was already in the transcript" (per the §13.1 TPR round 0
/// finding that multi-step tests were satisfied by aggregated transcripts).
pub(super) fn count_replies_exact(h: &SpecHarness, expected: &[u8]) -> usize {
    h.outcome()
        .effects_emitted
        .iter()
        .filter(|e| {
            matches!(e, Effect::Pty(PtyEffect::Write {
            kind: PtyWriteKind::ImageProtocolReply,
            bytes,
        }) if bytes.as_slice() == expected)
        })
        .count()
}

pub(super) fn ok_reply_for(id: u32) -> Vec<u8> {
    format!("\x1b_Gi={id};OK\x1b\\").into_bytes()
}
