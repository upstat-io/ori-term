//! Sibling tests for `session/mod.rs`.
//!
//! `mod.rs` itself owns `PtySession` (constructors, accessors,
//! `send`/`send_raw` write primitives, `Drop`) and the
//! `CONPTY_LIFETIME_LOCK` static. Tests that exercise those
//! members live here. The tool-availability probes
//! (`tool_available` and friends) live in `tools/tests.rs`; the
//! tack version gate lives in `version_gate/tests.rs` — both
//! moved out in the M1 TPR cleanup per the
//! "one sibling tests.rs per source file" rule.

use std::time::{Duration, Instant};

use portable_pty::CommandBuilder;

use super::PtySession;

#[test]
fn pty_session_send_raw_writes_without_quiesce() {
 // Verifies that send_raw is distinct from send: send() bakes
    // in a 300 ms quiesce internally, send_raw() must NOT. The
    // wall-clock assertion is the canary — if a future refactor
    // accidentally rewires send_raw to delegate to send, the elapsed
    // time jumps to ~300 ms and this test fires.
    //
    // Two-arm cross-platform shell: spawn `cat`/`findstr` so the
    // child is alive (PTY writer succeeds) and silent (no output to
    // confound timing). Then send_raw a single byte and measure how
    // long the call takes.
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/cat");
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        // findstr.exe waits for stdin and emits matching lines —
        // /N x just looks for "x" with line numbers; we won't send
        // any "x" so it stays silent and never exits.
        let mut c = CommandBuilder::new("findstr.exe");
        c.args(["/N", "x"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    let start = Instant::now();
    session.send_raw(b"x\n");
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(100),
        "send_raw must skip the 300 ms quiesce: elapsed {elapsed:?} \
         (regression: send_raw was rewired to delegate to send)"
    );
}
