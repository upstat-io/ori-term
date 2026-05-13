//! Capture helper — runs `notcurses-info` via `PtySession` and serializes the
//! full input byte stream to `plans/spec-conformance/captures/notcurses-info-full.cap`.
//!
//! Env-var-gated so it runs only when explicitly requested:
//!
//! ```
//! ORITERM_CAPTURE_NOTCURSES_INFO=1 cargo test -p oriterm_core --test notcurses_info_full_capture
//! ```
//!
//! Without the env var the test exits as a no-op `SKIP`. With the env var set,
//! it spawns `notcurses-info`, waits for child exit, and writes the byte stream
//! (every byte the child wrote to the PTY) to the wrapper-resident captures
//! directory.
//!
//! The resulting `.cap` is consumed by
//! `oriterm/src/gpu/visual_regression/spec_chain/pilots/notcurses_info_visual.rs`
//! via `include_bytes!()` so the visual-regression test stays deterministic on
//! every run after the capture is committed.

#![cfg(unix)]

use oriterm_test_support::{PtySession, notcurses_info_available, tool_available};
use portable_pty::CommandBuilder;

#[test]
fn capture_notcurses_info_full_stream() {
    if std::env::var("ORITERM_CAPTURE_NOTCURSES_INFO").is_err() {
        eprintln!("SKIP: ORITERM_CAPTURE_NOTCURSES_INFO not set");
        return;
    }
    if !notcurses_info_available() {
        eprintln!("SKIP: notcurses-info not installed");
        return;
    }
    if !tool_available("infocmp", "-V") {
        eprintln!("SKIP: ncurses tooling (infocmp) not available");
        return;
    }

    let Some(captures) = oriterm_test_support::paths::captures_dir() else {
        eprintln!("SKIP: wrapper-repo captures dir not discoverable");
        return;
    };

    let mut cmd = CommandBuilder::new("notcurses-info");
    cmd.env("TERM", "xterm-256color");

    let mut session = PtySession::spawn(cmd, 142, 54);

    let status = session.wait_for_child_exit(10_000);
    assert!(
        status.success(),
        "notcurses-info exited unsuccessfully: {status:?}"
    );

    let bytes = session.input_bytes();
    let replies = session.reply_bytes();
    let out_path = captures.join("notcurses-info-full.cap");
    let replies_path = captures.join("notcurses-info-full.replies.cap");

    std::fs::write(&out_path, bytes).expect("write capture file");
    std::fs::write(&replies_path, replies).expect("write replies file");

    eprintln!(
        "captured {} input bytes -> {}",
        bytes.len(),
        out_path.display()
    );
    eprintln!(
        "captured {} reply bytes -> {}",
        replies.len(),
        replies_path.display()
    );
}
