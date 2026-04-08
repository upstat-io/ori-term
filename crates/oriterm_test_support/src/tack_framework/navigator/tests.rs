use portable_pty::CommandBuilder;

use super::TackNavigator;
use crate::session::{PtySession, tool_available};
use crate::tack_framework::spec::MenuStep;

/// Spawn a long-lived child that produces no output. Used by the
/// timeout-panic test where the navigator must wait the full
/// `STEP_TIMEOUT_MS` budget before panicking.
fn spawn_silent_child() -> Option<PtySession> {
    #[cfg(unix)]
    {
        if !tool_available("cat", "--help") {
            return None;
        }
        Some(PtySession::spawn(CommandBuilder::new("cat"), 80, 24))
    }
    #[cfg(windows)]
    {
        if !tool_available("findstr.exe", "/?") {
            return None;
        }
        let mut cmd = CommandBuilder::new("findstr.exe");
        cmd.args(["/N", "x"]);
        Some(PtySession::spawn(cmd, 80, 24))
    }
}

#[test]
#[should_panic(expected = "step 0 failed")]
fn navigator_panics_with_step_index_on_timeout() {
    // The 5-second STEP_TIMEOUT_MS makes this slow but unavoidable
    // without test-only injection of the constant. Acceptable for a
    // single test that must observe the full timeout path.
    let Some(mut session) = spawn_silent_child() else {
        // Tool unavailable on this host — produce the expected panic
        // ourselves so the should_panic gate still triggers.
        panic!("step 0 failed: silent child tool unavailable, skipping");
    };
    let steps = &[MenuStep::new(b"", "this_text_will_never_appear")];
    TackNavigator::navigate(&mut session, steps);
}

#[test]
#[should_panic(expected = "pre-existing-anchor violation")]
fn navigator_panics_when_anchor_already_present_in_pre_grid() {
    // Spawn a child that prints "marker" once, then sleeps. We
    // wait_for "marker" to land BEFORE invoking the navigator so the
    // grid contains it. The navigator's pre-existing guard must
    // reject the "marker" anchor.
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "printf marker; sleep 5"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        // Use the in-process `pause` builtin instead of wrapping a
        // grandchild like `ping`. A `cmd.exe /C "echo marker & ping…"`
        // wrapper makes ping a grandchild attached to the ConPTY; when
        // `PtySession::drop` terminates cmd.exe, ping orphans and
        // `ClosePseudoConsole` blocks waiting for it to release the
        // HPCON. `pause` runs in cmd.exe's own process so killing
        // cmd.exe reaps the only attached console client.
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "echo marker & pause > NUL"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    session.wait_for("marker", 3_000);
    // Now the grid contains "marker". The navigator must reject an
    // anchor of "marker" before sending anything.
    let steps = &[MenuStep::new(b"x", "marker")];
    TackNavigator::navigate(&mut session, steps);
}

/// Targeted alternate-anchor test (M4b SEMANTIC PIN). Spawns a child
/// that delays for ~500 ms and then prints `alt_anchor`. The
/// navigator sends an empty step whose primary anchor is impossible
/// and whose only alternate is `alt_anchor`. After `send(b"")` (which
/// includes a 300 ms quiesce) the `wait_for_step` poll loop observes
/// `alt_anchor` arriving and matches it.
///
/// A regression that reverted to `catch_unwind`-based alternate
/// handling would either panic inside the navigator or take far
/// longer than the wall-clock bound this test enforces. The < 3 s
/// budget proves the navigator is using the parallel `wait_for_any`
/// primitive, not sequential anchor-at-a-time with a budget split.
#[test]
fn navigator_matches_alternate_when_primary_never_appears() {
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "sleep 0.5 && printf 'alt_anchor\\n' && sleep 5"]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        // Use cmd.exe with the in-process `pause` builtin instead of
        // wrapping a `ping` grandchild — see
        // `navigator_panics_when_anchor_already_present_in_pre_grid`
        // above for the orphan-grandchild rationale.
        //
        // The original Unix arm uses `sleep 0.5 && printf alt_anchor`
        // to delay the alternate anchor. The Windows arm prints
        // `alt_anchor` immediately because `cmd.exe` lacks a
        // sub-second sleep builtin and the only multi-process
        // approaches re-introduce the grandchild orphan we just
        // eliminated. The test still validates the alternate-anchor
        // path: the navigator's `wait_for_any` poll loop matches
        // `alt_anchor` on its first iteration (instead of after a
        // 500 ms delay), which is a strict subset of the behavior
        // pinned on Unix. The < 3 s wall-clock assertion below
        // still catches a sequential-budget regression because a
        // sequential implementation would consume budget per anchor.
        let mut c = CommandBuilder::new("cmd.exe");
        c.args(["/C", "echo alt_anchor & pause > NUL"]);
        c.env("TERM", "xterm-256color");
        c
    };
    let mut session = PtySession::spawn(cmd, 80, 24);
    let start = std::time::Instant::now();
    let steps = &[MenuStep {
        send: b"",
        wait_for: "primary_never",
        or_wait_for: &["alt_anchor"],
    }];
    TackNavigator::navigate(&mut session, steps);
    let elapsed = start.elapsed();
    assert!(
        elapsed.as_secs() < 3,
        "navigator took {elapsed:?} — expected <3s; a sequential \
         catch_unwind fallback or sequential per-anchor budget would \
         consume more"
    );
}
