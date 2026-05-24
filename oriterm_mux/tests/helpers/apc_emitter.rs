//! Test helper: emits kitty graphics `TRANSMIT` APC frames on stdout,
//! byte-for-byte. Used by the Windows-gated
//! `conpty_overlapped_transport` regression pin to validate the
//! overlapped-pipe `ConPTY` transport delivers raw `\x1b` bytes intact,
//! without ESC-stripping.
//!
//! Modes (argument-switched, interaction-cell coverage):
//! - `apc_emitter` (default) — emits ONE small TRANSMIT frame; helper
//!   exits immediately (process-death interaction).
//! - `apc_emitter --large` — emits ONE TRANSMIT frame with a 32KB+
//!   base64 RGBA payload, exercising multi-chunk read-buffering through
//!   the overlapped duplex pipe.
//! - `apc_emitter --multi` — emits 20 TRANSMIT frames back-to-back with
//!   image IDs 1-20, exercising multi-frame parsing under sustained
//!   writer load.
//!
//! Payload bytes + forbid-output tokens live in the shared SSOT module
//! `apc_payload.rs`. Both this binary and the integration test
//! `conpty_overlapped_transport.rs` include that module via `#[path]`.
//!
//! Design constraints:
//! - PowerShell `[Console]::Out.Write` would mangle raw APC bytes via
//!   encoding transforms — a Rust helper is byte-for-byte deterministic.
//! - No `thread::sleep` — kernel pipe buffers retain bytes after the
//!   writer exits; the parent reads them at its own pace before
//!   observing EOF.

use std::io::Write;

#[path = "apc_payload.rs"]
mod apc_payload;

fn main() {
    // Cross-CU dead-code anchor — see apc_payload::dead_code_anchor.
    apc_payload::dead_code_anchor();

    let args: Vec<String> = std::env::args().collect();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    // Reject unknown modes with a non-zero exit so a misspelled
    // --large / --multi argument fails LOUDLY in the integration test
    // (otherwise the wait_for_child_exit assertion would treat the
    // default mode's image_count == 1 as "passing" for a test that
    // was supposed to exercise --large or --multi).
    match args.get(1).map(String::as_str) {
        Some("--large") => apc_payload::emit_large(&mut out),
        Some("--multi") => apc_payload::emit_multi(&mut out),
        None => apc_payload::emit_default(&mut out),
        Some(other) => {
            eprintln!(
                "apc_emitter: unknown mode {other:?}; expected --large, --multi, or no argument"
            );
            std::process::exit(2);
        }
    }
    out.flush().expect("flush stdout");
}
