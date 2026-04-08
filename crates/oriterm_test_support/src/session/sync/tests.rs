use std::time::{Duration, Instant};

use portable_pty::CommandBuilder;

use crate::session::PtySession;

/// Spawn a silent long-lived child suitable for timeout/bounded-poll
/// pin tests. Two-arm cross-platform: Unix `/bin/sh -c "sleep 10"`,
/// Windows `cmd.exe /C "pause > NUL"` (blocks until killed on both
/// arms).
///
/// Windows note: we use `pause`, an in-process `cmd.exe` builtin,
/// rather than spawning a real subprocess like `ping.exe`. Two
/// reasons:
///
/// 1. **Grandchild orphan avoidance.** Wrapping a real subprocess
///    in `cmd.exe /C "child …"` makes the wrapper the immediate
///    child and the real subprocess a grandchild attached to the
///    `ConPTY`. When `PtySession::drop` terminates `cmd.exe`, the
///    grandchild becomes orphaned and remains attached to the
///    pseudoconsole as a still-alive console client.
///    `ClosePseudoConsole` (called when `_master` drops) then
///    blocks waiting for the orphaned grandchild to release the
///    HPCON.
/// 2. **No shared kernel resources.** `ping.exe` (and similar
///    network-touching helpers) contend on Windows ICMP loopback
///    rate limits when many tests run in parallel, ballooning
///    per-test wall-clock from <1 s to 10+ s. `pause` is a pure
///    user-mode busy-loop on console input — no network, no file
///    I/O, no inherited resource contention.
///
/// `pause` consumes one byte from stdin and exits. None of the
/// silent-long-lived consumers write to the child after spawn,
/// so `pause` blocks for the full test duration; the
/// `Drop`-driven `TerminateProcess` is the only termination
/// path.
fn spawn_silent_long_lived() -> PtySession {
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "sleep 10"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "pause > NUL"]);
        c.env("TERM", "xterm-256color");
        c
    };
    PtySession::spawn(cmd, 80, 24)
}

#[test]
fn pty_session_drains_simple_output() {
    // Portable PTY drain smoke test. portable-pty owns `ConPTY` on
    // Windows, so the same `PtySession` spawn path works on every
    // platform. Two-arm shell selection — `/bin/sh` on Unix and
    // `cmd.exe` on Windows — is the cross-platform idiom for "run a
    // one-liner in the platform shell." This replaces the previous
    // `#[cfg(unix)]`-gated test (BUG-07-008) so Windows gets real
    // `ConPTY` drain coverage instead of a no-op skip. The
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

#[test]
fn pty_session_wait_for_with_context_uses_custom_message() {
    // Spawn a silent long-lived child so wait_for_with_context can't
    // ever match its needle. The custom ctx closure embeds a unique
    // tag that the panic payload MUST contain — this is the semantic
    // pin proving the closure was actually called and its output
    // was used as the panic message (a regression that ignored ctx
    // and used a hard-coded message would not contain the tag).
    let mut session = spawn_silent_long_lived();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.wait_for_with_context("never_printed", 100, |g| format!("CUSTOM_TAG: {g}"));
    }));
    let payload = result.expect_err("expected wait_for_with_context to panic");
    let msg = if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        String::from("<non-string panic payload>")
    };
    assert!(
        msg.contains("CUSTOM_TAG"),
        "expected panic payload to contain CUSTOM_TAG, got: {msg}"
    );
}

#[test]
fn pty_session_wait_for_with_context_bounded_poll_invariant() {
    // Bounded-poll SEMANTIC PIN for the wait_for_with_context consumer
    // of poll_until. With a 500 ms deadline, no match, and the silent
    // long-lived child producing no output, drain_blocking(50) returns
    // 0 every iteration and the 10 ms idle sleep keeps wall-clock
    // close to the deadline. A regression that removes the idle sleep
    // would not affect wall-clock (deadline still fires at ~500 ms)
    // but it would burn CPU. The cleaner upper bound is the
    // post-deadline cleanup window: poll_until returns None
    // immediately after the deadline check, then panic! and grid_text
    // are fast — total wall-clock should be in [500 ms, 1500 ms]
    // even on a slow CI runner.
    let mut session = spawn_silent_long_lived();
    let start = Instant::now();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        session.wait_for_with_context("never", 500, |_| String::from("timeout"));
    }));
    let elapsed = start.elapsed();
    assert!(result.is_err(), "expected timeout panic");
    assert!(
        elapsed >= Duration::from_millis(500),
        "deadline honored: expected ≥500 ms, got {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(1500),
        "bounded-poll wall-clock: expected <1500 ms, got {elapsed:?}"
    );
}

#[test]
fn pty_session_wait_for_any_returns_some_zero_when_primary_matches() {
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "printf marker_primary"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "echo marker_primary"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    let idx = session.wait_for_any(&["marker_primary", "marker_alt"], 3_000);
    assert_eq!(idx, Some(0), "expected primary match at index 0");
}

#[test]
fn pty_session_wait_for_any_returns_some_alt_when_alternate_matches() {
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "printf marker_alt"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "echo marker_alt"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    let idx = session.wait_for_any(&["marker_primary", "marker_alt"], 3_000);
    assert_eq!(idx, Some(1), "expected alternate match at index 1");
}

#[test]
fn pty_session_wait_for_any_returns_none_on_timeout() {
    // Semantic pin for the non-panicking contract: wait_for_any must
    // return Option::None on timeout, NOT panic. A future refactor
    // that swapped the body for catch_unwind on wait_for_with_context
    // would panic inside the call and this test's assert_eq would
    // never run — the test would fail with a panic instead of an
    // assertion error.
    let mut session = spawn_silent_long_lived();
    let idx = session.wait_for_any(&["never"], 100);
    assert_eq!(idx, None, "expected None on timeout");
}

#[test]
fn pty_session_wait_for_any_prefers_primary_over_alternates_on_tie() {
    // The grid contains BOTH markers in the same line. wait_for_any
    // must return the LOWER index (primary preferred over alternates)
    // even when both anchors match in the same poll iteration.
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "printf 'marker_primary marker_alt'"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "echo marker_primary marker_alt"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    let idx = session.wait_for_any(&["marker_primary", "marker_alt"], 3_000);
    assert_eq!(
        idx,
        Some(0),
        "primary index 0 must win when both anchors match"
    );
}

#[test]
fn pty_session_wait_for_any_empty_slice_returns_none() {
    // Pure-unit fast-path: empty anchors slice returns None without
    // entering the poll loop. The wall-clock assertion proves the
    // fast path doesn't accidentally fall through to a 100 ms timeout
    // wait.
    let mut session = spawn_silent_long_lived();
    let start = Instant::now();
    let idx = session.wait_for_any(&[], 100);
    let elapsed = start.elapsed();
    assert_eq!(idx, None);
    assert!(
        elapsed < Duration::from_millis(50),
        "empty-slice fast path took {elapsed:?} — expected <50 ms"
    );
}

#[test]
fn pty_session_repeated_spawn_drop_cycle_succeeds_on_subsequent_cmd_exe_spawn() {
    // Regression pin for the Windows `ConPTY` HPCON premature-close
    // failure mode. Spawns 5 sequential `PtySession`s with the
    // silent-long-lived child (cmd.exe + ping on Windows, /bin/sh +
    // sleep on Unix), drops each, then spawns a fresh
    // cmd.exe /C exit 0 (or /bin/sh -c "exit 0" on Unix) and asserts
    // the 6th child exits cleanly.
    //
    // Before the fix, the 6th spawn on Windows hangs inside
    // `WaitForSingleObject` (or returns `STATUS_DLL_INIT_FAILED` /
    // 0xC0000142, depending on the host) because the prior 5 spawns
    // prematurely closed their HPCONs while children were still
    // running, leaking console-subsystem DLL state. Per Microsoft's
    // `ClosePseudoConsole` contract: "you should never call
    // ClosePseudoConsole until after the client has exited or the
    // call may hang." `PtySession::spawn` previously dropped
    // `pair.master` at function exit, before the child was reaped;
    // the fix is to hold `pair.master` inside `PtySession` so it
    // outlives the child.
    //
    // Cross-platform: the test exercises the same `PtySession` code
    // path on every platform. On Unix the 6th spawn always succeeds
    // even without the fix (no HPCON contract to violate), but the
    // test still pins the structural invariant that the master is
    // held — a future refactor that removes the `_master` field on
    // the assumption "Unix doesn't need it" would not regress on
    // Unix but would regress on Windows CI. Running this test on
    // Unix gives non-Windows contributors a local sanity check that
    // nothing structural broke.
    for _ in 0..5 {
        let mut s = spawn_silent_long_lived();
        // Touch the session so the IO thread has actually started
        // and the master is fully wired up before drop.
        let _ = s.drain();
        // Drop reaps the child synchronously via PtySession::drop,
        // and then drops the held master so ClosePseudoConsole runs
        // strictly after child exit.
    }

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
    assert!(
        status.success(),
        "expected clean exit on 6th spawn after 5 prior spawn/drop cycles, got {status:?}"
    );
}

#[test]
fn pty_session_wait_for_any_bounded_poll_invariant() {
    // Bounded-poll SEMANTIC PIN for the third poll_until consumer.
    // Mirror of the wait_for_with_context bounded-poll test — pins
    // that the 10 ms idle-sleep discipline is preserved when
    // poll_until is invoked via the wait_for_any predicate shape.
    // Together with pty_session_wait_for_with_context_bounded_poll_invariant
    // and pty_session_wait_for_child_exit_bounded_poll_invariant
    // this completes the three-call-site bounded-poll pin.
    let mut session = spawn_silent_long_lived();
    let start = Instant::now();
    let idx = session.wait_for_any(&["never"], 500);
    let elapsed = start.elapsed();
    assert_eq!(idx, None, "expected None on timeout");
    assert!(
        elapsed >= Duration::from_millis(500),
        "deadline honored: expected ≥500 ms, got {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_millis(1500),
        "bounded-poll wall-clock: expected <1500 ms, got {elapsed:?}"
    );
}
