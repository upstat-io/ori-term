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

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use std::io::Write;

// === Single source of truth for payload tokens ===
//
// emit_* and forbid_tokens BOTH derive from these constants. Changing
// ACTION_TRANSMIT here changes both the helper's emitted bytes AND the
// forbid-substring check — they cannot drift.

/// Image-ID prefix used by every emit_* payload (`Gi=N`).
const ID_PREFIX: &str = "Gi=";
/// Kitty action token — TRANSMIT (`a=T`).
const ACTION_TRANSMIT: &str = "a=T";
/// Kitty format token for RGB payloads (`f=24`).
const FORMAT_RGB: &str = "f=24";
/// Kitty format token for RGBA payloads (`f=32`).
const FORMAT_RGBA: &str = "f=32";
/// Image ID `emit_multi` cycles through: 1..=`MAX_MULTI_ID`.
const MAX_MULTI_ID: u32 = 20;

/// Substrings that MUST NOT appear in the parent's `grid_text` after
/// successful transport. If any appear, the pre-cure `ConPTY`
/// ESC-stripping regression is occurring (APC bytes leaked as literal
/// text into the grid).
///
/// Derived from the same constants `emit_*` use to build payloads — the
/// forbid check and the helper payloads cannot drift independently.
/// Using `Gi=` (prefix only) covers every image ID `emit_multi` cycles
/// through without enumerating each one.
pub fn forbid_tokens() -> &'static [&'static str] {
    &[ID_PREFIX, ACTION_TRANSMIT, FORMAT_RGB, FORMAT_RGBA]
}

/// `emit_default` — single small kitty TRANSMIT frame: 1×1 RGB pixel,
/// base64 `"AAAA"`. Bumps `image_count` from 0 to 1.
pub fn emit_default(out: &mut impl Write) {
    let payload = format!("\x1b_{ID_PREFIX}1,{ACTION_TRANSMIT},{FORMAT_RGB},s=1,v=1;AAAA\x1b\\");
    out.write_all(payload.as_bytes())
        .expect("write default APC frame");
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
    let b64 = BASE64_STANDARD.encode(&raw);
    let header = format!(
        "\x1b_{ID_PREFIX}1,{ACTION_TRANSMIT},{FORMAT_RGBA},s={},v={};",
        dims.0, dims.1
    );
    out.write_all(header.as_bytes())
        .expect("write large header");
    out.write_all(b64.as_bytes()).expect("write large payload");
    out.write_all(b"\x1b\\").expect("write large terminator");
}

/// `emit_multi` — `MAX_MULTI_ID` back-to-back TRANSMIT frames with image
/// IDs 1..=`MAX_MULTI_ID`. Exercises multi-frame parsing under sustained
/// writer load. Bumps `image_count` to EXACTLY `MAX_MULTI_ID`.
pub fn emit_multi(out: &mut impl Write) {
    for id in 1u32..=MAX_MULTI_ID {
        let payload =
            format!("\x1b_{ID_PREFIX}{id},{ACTION_TRANSMIT},{FORMAT_RGB},s=1,v=1;AAAA\x1b\\");
        out.write_all(payload.as_bytes())
            .expect("write multi APC frame");
    }
}

/// Number of frames `emit_multi` emits. Test assertions check
/// `image_count == multi_count()` for the multi-frame interaction.
pub fn multi_count() -> u32 {
    MAX_MULTI_ID
}

/// Cross-compilation-unit usage anchor — call once from each consumer's
/// entry path.
///
/// Both consumers (`apc_emitter.rs` and `conpty_overlapped_transport.rs`)
/// only use a SUBSET of this module's exports — the helper binary calls
/// `emit_*` and never `forbid_tokens`; the integration test calls
/// `forbid_tokens` and never `emit_*` (it spawns the binary instead).
/// Without this anchor, each CU's `-D dead_code` lint would reject the
/// unused subset.
///
/// `apc_emitter.rs main()` calls this once at startup; the integration
/// test calls it once from a `#[test]` shim. Both invocations are pure
/// noise at runtime (function-pointer references, no actual call into
/// the referenced items) but satisfy the dead-code analyzer from each
/// CU's perspective. This pattern replaces a prior module-level
/// `#![allow(dead_code)]` that would have silenced the lint instead of
/// addressing the underlying cross-CU sharing constraint.
pub fn dead_code_anchor() {
    let _ = (
        forbid_tokens as fn() -> &'static [&'static str],
        emit_default as fn(&mut std::io::Stdout),
        emit_large as fn(&mut std::io::Stdout),
        emit_multi as fn(&mut std::io::Stdout),
        multi_count as fn() -> u32,
    );
}
