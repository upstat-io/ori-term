use std::time::{Duration, Instant};

use portable_pty::CommandBuilder;

use super::{
    PtySession, infocmp_available, tack_available, tic_available, tool_available, vttest_available,
};

#[test]
fn tool_available_returns_false_for_nonexistent_binary() {
    assert!(!tool_available(
        "definitely_not_a_real_program_xyz_oriterm",
        "--version"
    ));
}

#[test]
fn vttest_available_matches_tool_available() {
    assert_eq!(vttest_available(), tool_available("vttest", "--help"));
}

#[test]
fn tic_available_matches_tool_available() {
    assert_eq!(tic_available(), tool_available("tic", "-V"));
}

#[test]
fn infocmp_available_matches_tool_available() {
    assert_eq!(infocmp_available(), tool_available("infocmp", "-V"));
}

#[test]
fn tack_available_matches_tool_available() {
    assert_eq!(tack_available(), tool_available("tack", "-V"));
}

#[test]
fn pty_session_send_raw_writes_without_quiesce() {
    // SEMANTIC PIN that send_raw is distinct from send: send() bakes
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
