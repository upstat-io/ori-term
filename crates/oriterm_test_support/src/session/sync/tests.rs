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

#[test]
fn pty_session_drain_writes_osc_responses_back() {
    // SEMANTIC PIN for Section 06.0.c: `drain_blocking` must flush the
    // `PtyResponder::osc_responses` queue back through `self.writer`
    // after each VTE advance, exactly the same way it already flushes
    // `take_responses` (DA/DSR path). Proof-of-work: spawn a stdin-echo
    // child in raw mode, forge an OSC 10 query into the child's stdin,
    // and observe that after `drain_blocking` finishes,
    // `palette().color(10)` equals `PtyResponder`'s pinned TEST_COLOR
    // (0xabcdef).
    //
    // Why palette-inspection proves the round-trip. The only path that
    // sets palette[10] to 0xabcdef is: (1) child echoes query bytes back
    // through PTY read → (2) VTE parses OSC 10 query → (3) Term fires
    // `Event::ColorRequest(10, formatter)` → (4) PtyResponder calls
    // `formatter(TEST_COLOR)` and buffers the canonical response into
    // `osc_responses` → (5) `drain_blocking`'s `write_osc_responses_back`
    // writes that response through `self.writer` → (6) child echoes
    // response bytes back → (7) VTE parses the response as an OSC 10
    // *set* and calls `palette.set_indexed(10, TEST_COLOR)`. Any broken
    // link in that chain leaves palette[10] at its default.
    //
    // **Raw mode is load-bearing.** The PTY line discipline defaults
    // to ICANON + ECHOCTL, which echoes `\x1b` as the visible two-byte
    // sequence `^[` — that is NOT a valid OSC query for VTE, so the
    // round-trip silently fails. `stty raw -echo` disables canonical
    // mode and terminal-driver echo; `cat` then provides byte-exact
    // echo from stdin to stdout.
    //
    // **Platform scope.** The raw-mode `stty` + `cat` pipeline is
    // POSIX-specific. Windows ``ConPTY`` has a different
    // line-discipline model with no direct `stty raw` equivalent, and
    // the responder's OSC dispatch is platform-independent Rust, so
    // the Windows path is covered by the sibling unit tests in
    // `pty_responder/tests.rs` (which exercise `Term<PtyResponder>`
    // directly, no PTY). The test function itself exists on every
    // platform — the cross-platform `#[test] fn` existence rule from
    // tack-conformance section 02.3 is preserved; only the body skips
    // on non-Unix hosts with a loud `eprintln!`.
    #[cfg(not(unix))]
    {
        eprintln!(
            "pty_session_drain_writes_osc_responses_back: skipping on \
             non-unix host — raw-mode PTY echo pipeline is POSIX-specific; \
             Windows coverage lives in pty_responder sibling unit tests"
        );
        return;
    }

    #[cfg(unix)]
    {
        // `stty raw -echo` disables ICANON + ECHO + ECHOCTL so the PTY
        // driver no longer echoes control bytes as `^X` sequences; the
        // subsequent `cat` then echoes every input byte byte-exact.
        // `echo STTY-READY` after `stty` is the synchronisation marker
        // — the test blocks on that marker before sending the OSC
        // query, eliminating the race between `stty` applying and
        // `send_raw` firing.
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "stty raw -echo; printf 'STTY-READY\\n'; exec cat"]);
        c.env("TERM", "xterm-256color");
        let mut session = PtySession::spawn(c, 80, 24);

        // Wait until `stty` has applied and `printf` has fired. Any
        // content arriving after the marker is guaranteed raw-echo
        // territory.
        session.wait_for("STTY-READY", 5_000);

        // OSC 10 query/set targets the foreground color entry in the
        // palette, which lives at `NamedColor::Foreground as usize`
        // (= 256). Checking ANSI index 10 (bright green) would always
        // succeed trivially because OSC 10 never touches it.
        const FG_INDEX: usize = 256;

        // Baseline sanity: palette[FG] must NOT be TEST_COLOR before
        // the round-trip fires. Defends against a future change to
        // Theme::default that accidentally picks 0xabcdef as the
        // default — the test would otherwise pass trivially with no
        // OSC handling at all.
        let baseline = session.term().palette().color(FG_INDEX);
        assert_ne!(
            (baseline.r, baseline.g, baseline.b),
            (0xab, 0xcd, 0xef),
            "theme default collided with the pinned TEST_COLOR — the \
             round-trip assertion would be vacuous"
        );

        // Forge the OSC 10 query into the child's stdin. Raw-mode `cat`
        // echoes it back byte-exact through the PTY reader channel; the
        // drain loop picks it up, feeds it through VTE, and fires
        // ColorRequest. We terminate with `\x1b\\` (ST) rather than BEL
        // so the VTE parser's OSC state machine transitions via the
        // canonical String-Terminator path.
        session.send_raw(b"\x1b]10;?\x1b\\");

        // Drive the round-trip. Each drain_blocking budget is a
        // generous 500 ms upper bound (typical round-trip is <10 ms on
        // a local PTY loop) so a CI runner under load still completes.
        // Four iterations are enough for: (1) query echo arrives →
        // ColorRequest → osc_resp flushed to writer; (2) response echo
        // arrives → palette update; plus slack for kernel pipe
        // scheduling.
        for _ in 0..4 {
            session.drain_blocking(500);
        }

        let captured = session.term().palette().color(FG_INDEX);
        assert_eq!(
            (captured.r, captured.g, captured.b),
            (0xab, 0xcd, 0xef),
            "palette[Foreground] must be TEST_COLOR (0xabcdef) after \
             the OSC 10 query/response round-trip — drain_blocking \
             failed to flush PtyResponder::osc_responses back through \
             the PTY writer. Observed palette[Foreground]: \
             ({:#04x}, {:#04x}, {:#04x})",
            captured.r,
            captured.g,
            captured.b,
        );
    }
}

#[test]
fn drain_until_returns_none_immediately_on_channel_disconnect() {
    // SEMANTIC PIN for TPR-05-003: drain_until's contract is that
    // channel closure (the reader thread has hung up because the
    // child exited or the PTY closed) returns None IMMEDIATELY,
    // not after burning the full timeout budget.
    //
    // The earlier draft used `let Ok(chunk) = ... else { continue; }`
    // which collapsed both Timeout and Disconnected into "loop
    // again," so a child that exited before the phase anchor
    // appeared would burn the full 5 s deadline before reporting
    // a (misleading) timeout. The fix distinguishes the two
    // RecvTimeoutError variants and returns None on Disconnected.
    //
    // Two-arm cross-platform: spawn a child that prints something
    // (so the PTY isn't immediately empty) then exits cleanly.
    // The reader thread observes EOF, closes the channel; the
    // next drain_until recv hits Disconnected.
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "echo HELLO; exit 0"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "echo HELLO"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);

    // Wait briefly so the child exits and the reader thread closes
    // the channel before drain_until starts. 200 ms is plenty for
    // an `echo + exit 0` to complete on any host.
    std::thread::sleep(Duration::from_millis(200));

    let started = Instant::now();
    let captured = session.drain_until("__NEVER_APPEARS__", 64 * 1024, 5_000);
    let elapsed = started.elapsed();

    assert!(
        captured.is_none(),
        "drain_until must return None when the needle never appears \
         and the channel disconnects"
    );
    // The disconnect-fast-return is a 1 s upper bound (very
    // generous — the actual return should be sub-100 ms). The
    // important property is "well below the 5 s timeout budget"
    // so a regression to the old `continue` semantics is caught
    // unambiguously.
    assert!(
        elapsed < Duration::from_millis(1_000),
        "drain_until must return immediately on channel disconnect, \
         not burn the full 5 s timeout: elapsed {elapsed:?} \
         (TPR-05-003 regression — recv_timeout's Disconnected \
         variant is being treated like Timeout instead of returning \
         None)"
    );
}
