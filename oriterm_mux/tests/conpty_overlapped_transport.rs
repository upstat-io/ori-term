//! Windows-gated regression pin for the overlapped-pipe ConPTY/APC
//! transport cure surface: delivers raw `\x1b` bytes intact through
//! the duplex pipe primitive, the sideloaded OpenConsole.exe VT
//! passthrough preserves them, and the PTY reader/writer loops route
//! them to the VTE parser without corruption.
//!
//! Pre-cure (kernel32 ConPTY's VT-input ESC-stripping), inbound `\x1b`
//! bytes were converted to VK_ESCAPE INPUT_RECORDs that got dropped
//! downstream of wsl.exe / the PTY-bridge boundary. The APC frame
//! `\x1b_Gi=1,a=T,f=24,s=1,v=1;AAAA\x1b\\` would arrive at the VTE
//! parser as `_Gi=1,a=T,f=24,s=1,v=1;AAAA\\` (both ESC bytes stripped).
//! The VTE parser would NOT enter APC state and would feed
//! `_Gi=1,a=T,...` as literal text into the grid; image_count would
//! remain 0.
//!
//! Post-cure, the sideloaded OpenConsole's enhanced VT passthrough
//! preserves ESC bytes; image_count rises within the deadline AND no
//! literal `Gi=*` / `a=T` substrings leak to the grid.
//!
//! Defense-in-depth: single bounded poll evaluates both pins per
//! iteration via PtySession::poll_until_or_fail, so neither is hidden
//! behind the other.
//!
//! Interaction coverage (4 PTY/ConPTY interactions):
//! - Test #1: process death (default mode)
//! - Test #2: read buffering (--large mode, 32KB+ payload)
//! - Test #3: multi-frame burst (--multi mode, 20 frames)
//! - Test #4: resize mid-output (--large mode + session.resize)
//!
//! Out-of-scope: write blocking + true backpressure (require helper
//! variants that block on parent drainage; to be filed at Phase 5
//! close-out per the bug plan's /improve-tooling retrospective —
//! current implementation is unidirectional child→parent).
//!
//! Regression: BUG-07-050 — Windows ConPTY/APC cure path has no
//! automated end-to-end test pin.
//! See: bug-tracker/plans/BUG-07-050/00-overview.md

#![cfg(target_os = "windows")]

use oriterm_test_support::PtySession;
use portable_pty::CommandBuilder;

// Shared SSOT for payload bytes + forbid-output tokens.
// The helper-binary payloads AND the forbid-output token list MUST
// come from one source so they cannot drift independently — colocating
// them closes the drift class.
#[path = "helpers/apc_payload.rs"]
mod apc_payload;

/// Test #1 (process-death interaction, TDD-2): spawns apc_emitter
/// (default mode) which writes a kitty graphics TRANSMIT APC frame on
/// stdout then exits; asserts the frame reaches term.image_cache() via
/// the overlapped-pipe ConPTY transport AND does NOT leak as literal
/// text to the grid.
///
/// Regression: BUG-07-050 — Windows ConPTY/APC cure path has no
/// automated end-to-end test pin.
/// See: bug-tracker/plans/BUG-07-050/00-overview.md
#[test]
fn conpty_apc_byte_sequence_via_rust_helper_emits_image() {
    let helper = env!("CARGO_BIN_EXE_apc_emitter");
    let cmd = CommandBuilder::new(helper);
    let mut session = PtySession::spawn(cmd, 80, 24);

    let status = session.wait_for_child_exit(5_000);
    assert!(
        status.success(),
        "apc_emitter exited non-zero: {status:?}"
    );

    session
        .poll_until_or_fail(
            5_000,
            |s| s.term().image_cache().image_count() == 1,
            |s| forbid_token_check(s),
        )
        .expect("ConPTY/APC transport cure regression");
}

/// Test #2 (read-buffering interaction, TDD-2b): apc_emitter --large
/// emits a 32KB+ raw APC frame (128×64 RGBA, ~43KB base64-encoded),
/// exercising multi-chunk reads through the overlapped duplex pipe.
/// Asserts image_count == 1 with a forbid-output early-fail check.
///
/// Regression: BUG-07-050 TDD-2b.
#[test]
fn conpty_apc_large_payload_across_buffer_reads_emits_image() {
    let helper = env!("CARGO_BIN_EXE_apc_emitter");
    let mut cmd = CommandBuilder::new(helper);
    cmd.arg("--large");
    let mut session = PtySession::spawn(cmd, 80, 24);

    let status = session.wait_for_child_exit(5_000);
    assert!(
        status.success(),
        "apc_emitter --large exited non-zero: {status:?}"
    );

    session
        .poll_until_or_fail(
            5_000,
            |s| s.term().image_cache().image_count() == 1,
            |s| forbid_token_check(s),
        )
        .expect("read-buffering interaction failed");
}

/// Test #3 (multi-frame burst interaction — sustained writer load):
/// apc_emitter --multi emits 20 back-to-back TRANSMIT frames with
/// image IDs 1-20. Asserts image_count == 20 (all frames parsed and
/// dispatched). NOT backpressure — the helper does not block on parent
/// drainage; see the test-strategy notes for the naming clarification.
///
/// Regression: BUG-07-050 TDD-2c.
#[test]
fn conpty_apc_multiple_frames_burst_emits_all_images() {
    let helper = env!("CARGO_BIN_EXE_apc_emitter");
    let mut cmd = CommandBuilder::new(helper);
    cmd.arg("--multi");
    let mut session = PtySession::spawn(cmd, 80, 24);

    let status = session.wait_for_child_exit(5_000);
    assert!(
        status.success(),
        "apc_emitter --multi exited non-zero: {status:?}"
    );

    session
        .poll_until_or_fail(
            5_000,
            |s| {
                s.term().image_cache().image_count()
                    == apc_payload::multi_count() as usize
            },
            |s| forbid_token_check(s),
        )
        .expect("multi-frame burst interaction failed");
}

/// Test #4 (resize-mid-output interaction): apc_emitter --large writes
/// a 32KB+ payload over the overlapped pipe; the parent calls
/// session.resize(132, 40) before observing child exit. On Windows
/// ConPTY with a typical ~16KB pipe buffer, the 32KB payload exceeds
/// buffer capacity → the writer naturally BLOCKS on pipe-full → the
/// resize call has a structural window of opportunity (the buffer-
/// blocked write state) to interleave. NOT a strict synchronization
/// guarantee — on a very fast reader the writer may not block; the
/// bug plan documents a follow-up for a proper bidirectional handshake.
/// Asserts post-write state is identical to the unresized case:
/// image_count == 1 AND no forbid-tokens appear in grid_text.
///
/// Regression: BUG-07-050 TDD-2d.
#[test]
fn conpty_apc_resize_mid_output_preserves_payload() {
    let helper = env!("CARGO_BIN_EXE_apc_emitter");
    let mut cmd = CommandBuilder::new(helper);
    cmd.arg("--large");
    let mut session = PtySession::spawn(cmd, 80, 24);

    // Resize between spawn and child-exit observation — mid-flight by
    // construction on typical Windows ConPTY because the 32KB payload
    // exceeds the ~16KB pipe buffer and naturally blocks the writer.
    session
        .resize(132, 40)
        .expect("resize during apc_emitter --large write");

    let status = session.wait_for_child_exit(5_000);
    assert!(
        status.success(),
        "apc_emitter --large (resized) exited non-zero: {status:?}"
    );

    session
        .poll_until_or_fail(
            5_000,
            |s| s.term().image_cache().image_count() == 1,
            |s| forbid_token_check(s),
        )
        .expect("resize-mid-output interaction failed");
}

/// Cross-CU dead-code anchor — see `apc_payload::dead_code_anchor`.
/// Calling it from a `#[test]` shim from THIS compilation unit marks
/// `emit_*` as "used" from the test's perspective, mirroring the
/// `apc_emitter` binary's main() call that marks `forbid_tokens` as
/// "used" from the binary's perspective.
#[test]
fn dead_code_anchor_shim() {
    apc_payload::dead_code_anchor();
}

/// Shared early-fail closure for poll_until_or_fail's forbid-output
/// check. Returns Some(message) when grid_text contains any of the
/// SSOT-defined forbid tokens (rejecting the pre-cure leak-to-grid
/// regression mode).
fn forbid_token_check(session: &PtySession) -> Option<String> {
    let grid = session.grid_text();
    if apc_payload::forbid_tokens()
        .iter()
        .any(|t| grid.contains(t))
    {
        Some(format!(
            "pre-cure ConPTY ESC-stripping regression: APC payload leaked \
             to grid as literal text. Grid:\n{grid}"
        ))
    } else {
        None
    }
}
