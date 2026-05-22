//! Shared SSOT for kitty APC payload bytes + forbid-output token list.
//! Included via `#[path = "helpers/apc_payload.rs"] mod apc_payload;`
//! from both:
//!   - `oriterm_mux/tests/helpers/apc_emitter.rs` (helper bin)
//!   - `oriterm_mux/tests/conpty_overlapped_transport.rs` (test)
//!
//! The two locations cannot share via a normal `mod` declaration
//! because the helper bin and the integration test are separate
//! compilation units. `#[path]` is the standard Cargo pattern for
//! cross-CU test-helper sharing.
//!
//! SSOT rationale: forbid-output tokens MUST be derived from the same
//! payload definitions the helper bin emits. If the helper changes its
//! payload tokens (e.g., `Gi=1` → `Gi=42`), the forbid-output check in
//! `conpty_overlapped_transport.rs` MUST update automatically. This
//! module is that automatic link — colocating payload bytes and
//! forbid-token list closes the drift class.
//!
//! The `#[allow(dead_code)]` on individual items is structurally
//! required: each consumer (helper bin vs integration test) uses a
//! SUBSET of the module's exports. From the helper bin's view,
//! `forbid_tokens` is unused; from the test's view, `emit_*` may be
//! used only via the bin's runtime invocation (not at the test's
//! compile-time link layer). Without the allow, `-D dead_code` rejects
//! each CU's view of the unused subset.

#![allow(dead_code)]

use std::io::Write;

/// Tokens that MUST NOT appear in the parent's `grid_text` after
/// successful transport. Derived from the payload definitions `emit_*`
/// produce; if any of these substrings appear in `grid_text`, the
/// pre-cure `ConPTY` ESC-stripping regression is occurring (APC bytes
/// leaked as literal text into the grid).
pub fn forbid_tokens() -> Vec<&'static str> {
    vec![
        // Image IDs the helper emits (1..=20 for emit_multi; 1 for emit_default + emit_large)
        "Gi=1", "Gi=2", "Gi=3", "Gi=4", "Gi=5",
        "Gi=6", "Gi=7", "Gi=8", "Gi=9", "Gi=10",
        "Gi=11", "Gi=12", "Gi=13", "Gi=14", "Gi=15",
        "Gi=16", "Gi=17", "Gi=18", "Gi=19", "Gi=20",
        // Action token (kitty TRANSMIT) — emit_* all use a=T
        "a=T",
        // Format tokens — f=24 (RGB) for default + multi; f=32 (RGBA) for large
        "f=24", "f=32",
    ]
}

/// `emit_default` — single small kitty `a=T` (transmit) frame: 1×1 RGB
/// pixel, base64 `"AAAA"`. Bumps `image_count` from 0 to 1.
pub fn emit_default(out: &mut impl Write) {
    let payload = b"\x1b_Gi=1,a=T,f=24,s=1,v=1;AAAA\x1b\\";
    out.write_all(payload).expect("write default APC frame");
}

/// `emit_large` — 128×64 RGBA frame = 32,768 bytes raw; GUARANTEED to
/// exceed canonical Windows `ConPTY` ~16KB read buffer. Forces
/// multi-chunk reads. Bumps `image_count` to EXACTLY 1.
pub fn emit_large(out: &mut impl Write) {
    let dims = (128u16, 64u16);
    let raw_len = (dims.0 as usize) * (dims.1 as usize) * 4;
    assert!(
        raw_len >= 32 * 1024,
        "large payload must exceed canonical 16KB ConPTY buffer to prove multi-chunk reads"
    );
    let raw = vec![0xAAu8; raw_len];
    let b64 = base64_encode(&raw);
    let header = format!("\x1b_Gi=1,a=T,f=32,s={},v={};", dims.0, dims.1);
    out.write_all(header.as_bytes()).expect("write large header");
    out.write_all(b64.as_bytes()).expect("write large payload");
    out.write_all(b"\x1b\\").expect("write large terminator");
}

/// `emit_multi` — 20 back-to-back TRANSMIT frames with image IDs 1..=20.
/// Exercises multi-frame parsing under sustained writer load.
/// Bumps `image_count` to EXACTLY 20.
pub fn emit_multi(out: &mut impl Write) {
    for id in 1u32..=20 {
        let payload = format!("\x1b_Gi={id},a=T,f=24,s=1,v=1;AAAA\x1b\\");
        out.write_all(payload.as_bytes())
            .expect("write multi APC frame");
    }
}

// Inline base64 (no external dep — keeps helper module minimal).
fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16)
            | ((data[i + 1] as u32) << 8)
            | (data[i + 2] as u32);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push(TABLE[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = (data[i] as u32) << 16;
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(TABLE[((n >> 18) & 63) as usize] as char);
        out.push(TABLE[((n >> 12) & 63) as usize] as char);
        out.push(TABLE[((n >> 6) & 63) as usize] as char);
        out.push('=');
    } else {
        // rem == 0: no padding required; the while-loop above consumed
        // every byte cleanly.
    }
    out
}
