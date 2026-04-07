use portable_pty::CommandBuilder;

use super::{PtySession, infocmp_available, tic_available, tool_available, vttest_available};

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
fn pty_session_drains_simple_output() {
    // Portable PTY drain smoke test. portable-pty owns ConPTY on
    // Windows, so the same `PtySession` spawn path works on every
    // platform. Two-arm shell selection — `/bin/sh` on Unix and
    // `cmd.exe` on Windows — is the cross-platform idiom for "run a
    // one-liner in the platform shell." This replaces the previous
    // `#[cfg(unix)]`-gated test (BUG-07-008) so Windows gets real
    // ConPTY drain coverage instead of a no-op skip. The
    // `#[cfg(unix)] / #[cfg(windows)]` block INSIDE the `#[test] fn`
    // is the cross-platform idiom this codebase uses; the OUTER
    // `#[cfg(unix)] #[test]` form is the antipattern banned by
    // tack-conformance section 02.3 because the test function then
    // does not even EXIST on Windows.
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "printf hello"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "echo hello"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    session.wait_for("hello", 5_000);
    let text = session.grid_text();
    assert!(
        text.contains("hello"),
        "expected drained output to contain 'hello', got:\n{text}"
    );
}
