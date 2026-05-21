//! Shared kitty graphics-protocol fixtures for `spec_chain` integration
//! tests across crate boundaries.
//!
//! SSOT for APC framing, base64 encoding, and small fixed-payload
//! constructors that previously duplicated across at least four
//! integration-test binaries (`oriterm_core/tests/cross_protocol_rss.rs`,
//! `oriterm_core/tests/spec_chain/kitty/fixtures.rs`,
//! `oriterm_core/tests/spec_chain/sixel/cross_stack_handoff.rs`,
//! `crates/oriterm_test_support/src/spec_chain/recording_handler/tests.rs`).
//!
//! `oriterm_core/src/image/kitty/tests.rs` keeps its own local `kitty_apc`
//! helper — that file is an inner unit-test module under `src/` and cannot
//! depend on `oriterm_test_support` without inverting the dependency edge.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;

/// Build the full `\x1b_G<control>;<payload_b64>\x1b\\` APC command.
///
/// `control` is the parameter list (e.g. `b"a=t,i=1,f=32,s=1,v=1,q=2"` —
/// no trailing `;`). `payload_b64` is the base64-encoded payload — when
/// empty, no `;` separator is emitted (kitty allows zero-payload commands
/// such as `a=q` query or `a=d` delete).
pub fn kitty_apc(control: &[u8], payload_b64: &str) -> Vec<u8> {
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

/// Base64-encode bytes using the STANDARD alphabet — the on-wire form
/// every kitty `t=d` direct-transmit consumer expects.
pub fn b64(data: &[u8]) -> String {
    STANDARD.encode(data)
}

/// 4×4 opaque-red raw RGBA payload — 64 bytes (matches `s=4,v=4,f=32`).
pub fn rgba_4x4_red() -> Vec<u8> {
    let mut v = Vec::with_capacity(64);
    for _ in 0..16 {
        v.extend_from_slice(&[255, 0, 0, 255]);
    }
    v
}
