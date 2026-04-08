use std::time::{Duration, Instant};

use portable_pty::CommandBuilder;

use crate::session::PtySession;

#[cfg(unix)]
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
    /// Lives in `teardown/tests.rs` (not production `mod.rs`) because
    /// it is test scaffolding whose only caller is this file. `tests`
    /// is a descendant module of `teardown`, which is a child of
    /// `session`, so this inherent method can freely access
    /// `PtySession`'s private `rx` field.
    ///
    /// `#[cfg(unix)]` because the only caller,
    /// `pty_session_wait_for_child_exit_bounded_poll_invariant`, is
    /// itself `#[cfg(unix)]` — exposing this helper on Windows would
    /// be dead code under `-D dead-code`.
    fn force_close_rx_for_test(&mut self) {
        let (_tx, rx) = std::sync::mpsc::channel();
        // Dropping `_tx` closes the channel. Any subsequent
        // `recv_timeout` on `rx` returns `Err(Disconnected)`
        // immediately, which is exactly the hot-spin scenario the
        // 10 ms idle sleep in `poll_until` defends against.
        self.rx = rx;
    }
}

/// Deterministic pin for the bounded-poll invariant of
/// [`PtySession::wait_for_child_exit`] (see 03.R TPR-03-004).
///
/// Simulates the "reader thread EOF but `try_wait()` still returns
/// `Ok(None)`" race window by force-closing the PTY reader channel
/// BEFORE calling the poll loop. A lingering `sleep 0.5` child keeps
/// the process alive for ~500 ms; with the 10 ms idle sleep in
/// `poll_until` honored on the empty-drain branch, the inner loop
/// runs ~50 iterations before `try_wait` observes termination.
/// Without the sleep, the loop burns tens of thousands of iterations
/// in the same wall-clock window, which this assertion catches loud.
/// The iteration counter is injected via `wait_for_child_exit_inner`'s
/// `on_iter` callback so the production signature stays clean.
///
/// Unix-only: the bounded-poll behavior is a portable timing
/// invariant shared by both `ConPTY` and Unix PTY paths — the poll
/// body is platform-agnostic. A cross-platform reproduction would
/// need `cmd /C timeout` or `ping -n` shims with coarser timing; the
/// Windows `ConPTY` path for the SAME code is already exercised by
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
    // 10 ms idle sleep from `poll_until` and reintroduced the
    // busy-loop regression.
    assert!(
        iters < 500,
        "bounded-poll invariant violated: {iters} iterations observed \
         for a 500 ms lingering child (expected ~50 with 10 ms \
         anti-hot-spin sleep; > 500 indicates the idle sleep was removed)"
    );
}

/// Spawn a child that exits cleanly the moment any single byte is
/// read on stdin, with the requested exit code.
///
/// Tack reads input in raw mode (one byte at a time, no line
/// buffering), so `quit_tack` sends a bare `q` rather than `q\n`.
/// The test child must match: it has to exit on a single byte,
/// NOT wait for a newline. PTYs default to cooked (canonical)
/// mode where `read()` blocks until a newline arrives, so we
/// invoke `stty -icanon min 1 -echo` to disable line buffering
/// (and echo so the PTY doesn't echo `q` back into the grid).
///
/// **Race-free synchronization.** Without a barrier, the
/// caller's first `q` byte can race with the child's `stty` call
/// — if the byte arrives while the PTY is still in cooked mode,
/// it gets buffered there waiting for a newline and `head -c 1`
/// blocks forever. We print a `__READY__` marker AFTER stty
/// completes and have the spawn helper `wait_for` that marker
/// before returning. By the time the helper returns, the child
/// is guaranteed to be in raw mode and blocked at `head -c 1`.
///
/// Windows: `pause` reads one console keystroke directly without
/// requiring Enter, so no stty equivalent is needed.
fn spawn_quit_on_keystroke(exit_code: i32) -> PtySession {
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        let script = format!(
            "stty -icanon min 1 -echo; echo __READY__; head -c 1 > /dev/null; exit {exit_code}"
        );
        c.args(["-c", &script]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let _ = exit_code;
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "pause > NUL"]);
        c.env("TERM", "xterm-256color");
        c
    };
    // `mut` is required on Unix so `wait_for` can drive the reader;
    // on Windows the call is cfg-gated out, so the binding is only
    // read — `#[cfg_attr(not(unix), expect(unused_mut, ...))]` keeps
    // the single source-form line while satisfying `-D unused-mut`.
    #[cfg_attr(
        not(unix),
        expect(
            unused_mut,
            reason = "session.wait_for() below is unix-only; on windows the binding is never mutated"
        )
    )]
    let mut session = PtySession::spawn(cmd, 80, 24);
    #[cfg(unix)]
    session.wait_for("__READY__", 5_000);
    session
}

#[test]
fn pty_session_quit_tack_returns_status_when_child_exits() {
    // Two-arm cross-platform pattern. The child exits the moment
    // it reads the first byte (`q`), so `quit_tack(5)` returns
    // `Some(status)` on the first iteration. Verifies the
    // success-path control flow (`send_raw` + drain + `try_wait`
    // + early return) works on both `ConPTY` (Windows) and Unix
    // PTY paths.
    let mut session = spawn_quit_on_keystroke(0);
    let status = session.quit_tack(5);
    assert!(status.success(), "expected clean exit, got {status:?}");
}

#[test]
fn pty_session_quit_tack_exits_early_when_child_dies_after_first_q() {
    // SEMANTIC PIN: this proves the q-loop returns the moment
    // `try_wait()` observes exit, NOT after exhausting
    // `max_iterations`. A regression that removed the `try_wait`
    // early-exit (e.g., always looped `max_iterations` times)
    // would push wall-clock to 5 × (200 ms drain + 10 ms idle) ≈
    // 1050 ms. The < 1000 ms upper bound is the canary.
    let mut session = spawn_quit_on_keystroke(0);
    let start = Instant::now();
    let status = session.quit_tack(5);
    let elapsed = start.elapsed();
    assert!(status.success(), "expected clean exit, got {status:?}");
    assert!(
        elapsed < Duration::from_millis(1000),
        "early-exit invariant violated: quit_tack(5) returned in {elapsed:?} \
         (expected <1000 ms — child dies on first `q`; >1000 ms means \
         the loop ignored try_wait and exhausted max_iterations)"
    );
}

#[cfg(unix)]
#[test]
fn pty_session_quit_tack_panics_on_max_iterations() {
    // Unix-only: spawn a child that ignores keystrokes forever and
    // verify quit_tack panics with the max-iterations message after
    // exhausting its budget. Drop reaps the runaway child via kill.
    //
    // Windows coverage gap acknowledgment: a child that truly
    // ignores `q` forever is hard to construct portably in
    // `ConPTY`. The happy-path `quit_tack` tests above exercise the
    // `ConPTY` q-loop body, and the panic body is platform-agnostic
    // (`assert!`/`format!`/`grid_text()`), so the Unix-only panic
    // pin is sufficient coverage of the overflow path.
    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.args(["-c", "while true; do sleep 0.1; done"]);
    cmd.env("TERM", "xterm-256color");
    let mut session = PtySession::spawn(cmd, 80, 24);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.quit_tack(2);
    }));
    let payload = result.expect_err("expected quit_tack to panic on max iterations");
    let msg = if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        String::from("<non-string panic payload>")
    };
    // After max_iterations q's, quit_tack falls through to
    // wait_for_child_exit which polls for the actual exit and
    // panics with `"child did not exit within 2000ms"` if the
    // child is still alive — same panic site, same diagnostic
    // grid in the message.
    assert!(
        msg.contains("child did not exit within"),
        "expected wait_for_child_exit timeout panic, got: {msg}"
    );
    // Drop runs at end of scope and reaps the runaway child.
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
    // 10 ms idle sleep is accidentally dropped in a future refactor,
    // this test still passes (the exit is observed in the first
    // iteration) but the `test-all.sh` wall-clock budget regresses —
    // that wall clock IS the canary.
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
