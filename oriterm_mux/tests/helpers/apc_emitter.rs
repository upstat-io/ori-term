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
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map_or("default", String::as_str);
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    match mode {
        "--large" => apc_payload::emit_large(&mut out),
        "--multi" => apc_payload::emit_multi(&mut out),
        _ => apc_payload::emit_default(&mut out),
    }
    out.flush().expect("flush stdout");
}
