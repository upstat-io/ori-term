use portable_pty::CommandBuilder;

use super::{
    PtySession, infocmp_available, tack_available, tic_available, tool_available, vttest_available,
};

impl PtySession {
    /// Test-only helper: replace the reader channel with a fresh
    /// closed channel so [`PtySession::drain_blocking`] returns 0
    /// immediately.
    ///
    /// Used exclusively by
    /// `pty_session_wait_for_child_exit_bounded_poll_invariant` to
    /// simulate the "reader EOF but child still alive" race window
    /// without having to precisely time a real PTY close. The reader
    /// thread is still alive and its `tx` still owns the OLD channel;
    /// swapping `self.rx` orphans that channel. The reader thread's
    /// next `tx.send` fails `is_err()` and the thread breaks out of
    /// its loop — no leaks.
    ///
    /// Lives in `tests.rs` (not production `mod.rs`) because it is
    /// test scaffolding whose only caller is this file. `tests` is a
    /// descendant module of `session`, so this inherent method can
    /// freely access `PtySession`'s private `rx` field even though
    /// it's defined outside `mod.rs`.
    fn force_close_rx_for_test(&mut self) {
        let (_tx, rx) = std::sync::mpsc::channel();
        // Dropping `_tx` closes the channel. Any subsequent
        // `recv_timeout` on `rx` returns `Err(Disconnected)`
        // immediately, which is exactly the hot-spin scenario
        // `POLL_SLEEP` defends against.
        self.rx = rx;
    }
}

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

/// Deterministic pin for the bounded-poll invariant of
/// [`PtySession::wait_for_child_exit`] (see 03.R TPR-03-004).
///
/// Simulates the "reader thread EOF but `try_wait()` still returns
/// `Ok(None)`" race window by force-closing the PTY reader channel
/// BEFORE calling the poll loop. A lingering `sleep 0.5` child keeps
/// the process alive for ~500 ms; with the 10 ms `POLL_SLEEP` honored
/// on the `Ok(None)` path, the inner loop runs ~50 iterations before
/// `try_wait` observes termination. Without the sleep, the loop burns
/// tens of thousands of iterations in the same wall-clock window,
/// which this assertion catches loud. The iteration counter is
/// injected via `wait_for_child_exit_inner`'s `on_iter` callback so
/// the production signature stays clean.
///
/// Unix-only: the bounded-poll behavior is a portable timing
/// invariant shared by both ConPTY and Unix PTY paths — the poll
/// body is platform-agnostic. A cross-platform reproduction would
/// need `cmd /C timeout` or `ping -n` shims with coarser timing; the
/// Windows ConPTY path for the SAME code is already exercised by
/// `pty_session_wait_for_child_exit_returns_on_clean_exit`'s two-arm
/// shell test, and pinning the invariant on Unix is sufficient
/// because both platforms share the identical
/// `wait_for_child_exit_inner` body.
#[cfg(unix)]
#[test]
fn pty_session_wait_for_child_exit_bounded_poll_invariant() {
    let mut cmd = CommandBuilder::new("/bin/sh");
    // 500 ms lingering child, no PTY output.
    cmd.args(["-c", "sleep 0.5"]);
    cmd.env("TERM", "xterm-256color");
    let mut session = PtySession::spawn(cmd, 80, 24);

    // Swap the reader channel for a closed one so `drain_blocking`
    // returns 0 instantly on every iteration — the exact scenario
    // the 10 ms anti-hot-spin sleep exists to defend against.
    session.force_close_rx_for_test();

    let mut iters = 0usize;
    let status = session.wait_for_child_exit_inner(10_000, || iters += 1);
    assert!(status.success(), "expected clean exit, got {status:?}");

    // Expected with 10 ms sleep on closed-channel path: ~50 iterations
    // (500 ms / 10 ms). Bound at 500 gives an order-of-magnitude
    // headroom for scheduler jitter while still catching hot-spin
    // (which would produce 10⁴-10⁵ iterations for the same 500 ms
    // child). If this assertion fires, it means someone removed the
    // `thread::sleep(POLL_SLEEP)` line on the `Ok(None)` branch of
    // `wait_for_child_exit_inner` and reintroduced the busy-loop
    // regression.
    assert!(
        iters < 500,
        "bounded-poll invariant violated: {iters} iterations observed \
         for a 500 ms lingering child (expected ~50 with 10 ms \
         anti-hot-spin sleep; > 500 indicates POLL_SLEEP was removed)"
    );
}

#[test]
fn pty_session_wait_for_child_exit_returns_on_clean_exit() {
    // Two-arm cross-platform pattern (same idiom as
    // `pty_session_drains_simple_output`): on Unix `/bin/sh -c "exit 0"`
    // exits cleanly; on Windows `cmd.exe /C "exit 0"` does the same via
    // ConPTY. Both arms exercise the bounded-poll path of
    // `wait_for_child_exit` because a `sh -c "exit 0"` (or `cmd /C
    // exit 0`) child terminates almost immediately after spawn — the
    // loop must tolerate the reader-closed-channel + try_wait-observes
    // -exit race without hot-spinning and without deadlocking. If the
    // 10 ms `Ok(None)`-path sleep is accidentally dropped in a future
    // refactor this test still passes (the exit is observed in the
    // first iteration) but the `test-all.sh` wall-clock budget regresses
    // — that wall clock IS the canary.
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "exit 0"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "exit 0"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    let status = session.wait_for_child_exit(5_000);
    assert!(status.success(), "expected clean exit, got {status:?}");
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
