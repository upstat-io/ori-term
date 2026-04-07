use super::{tool_available, vttest_available};

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

#[cfg(unix)]
#[test]
fn pty_session_drains_simple_output() {
    use portable_pty::CommandBuilder;

    use super::PtySession;

    // `printf` is part of the POSIX shell builtins on every Unix; the
    // standalone binary lives at /usr/bin/printf on Linux/macOS. We use
    // /bin/sh -c so PATH lookup works either way.
    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.arg("-c");
    cmd.arg("printf hello");
    cmd.env("TERM", "xterm-256color");

    let mut session = PtySession::spawn(cmd, 80, 24);
    // Give the child time to write its output and exit.
    session.wait(500);

    let text = session.grid_text();
    assert!(
        text.contains("hello"),
        "expected grid to contain 'hello', got:\n{text}"
    );
}
