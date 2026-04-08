use portable_pty::{CommandBuilder, ExitStatus};

use super::{LiveSession, ScreenFacts};
use crate::session::PtySession;
use crate::session::tic_available;
use crate::terminfo::TerminfoEnv;

#[cfg(test)]
impl LiveSession {
    /// Test-only constructor that wraps a [`PtySession`] in a
    /// [`LiveSession`] with placeholder identity fields and a real
    /// (but irrelevant) [`TerminfoEnv`].
    ///
    /// The terminfo env is real because [`TerminfoEnv`] doesn't
    /// expose a `Default`/stub constructor — its only path is
    /// `compile()` which shells out to `tic`. Tests using this
    /// helper MUST gate on [`tic_available`] and skip cleanly when
    /// `tic` is missing.
    fn new_for_test(
        session: PtySession,
        env: TerminfoEnv,
        quit_path: Option<fn(&mut PtySession) -> ExitStatus>,
    ) -> Self {
        Self {
            session,
            facts: ScreenFacts::default(),
            scenario_id: "test_scenario",
            screen_id: "test_screen",
            cols: 80,
            rows: 24,
            _terminfo: env,
            quit_path,
        }
    }
}

/// Spawn a child that exits cleanly the moment any single byte is
/// read on stdin, with the requested exit code. Matches the
/// raw-mode contract `quit_tack` expects (bare `q`, no newline).
/// See `session/teardown/tests.rs::spawn_quit_on_keystroke` for
/// the `stty -icanon` + `__READY__` race-free synchronization
/// rationale.
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
    let mut session = PtySession::spawn(cmd, 80, 24);
    #[cfg(unix)]
    session.wait_for("__READY__", 5_000);
    session
}

#[test]
fn live_session_finish_asserts_clean_exit_via_quit_tack() {
    // SEMANTIC PIN: this proves `LiveSession::finish` actually
    // exercises `quit_tack` and not a "just drop" shortcut. Without
    // this test, a regression that silently replaces `finish`'s body
    // with a no-op `drop(self)` would pass every other test in
    // 04.0-04.4 because no test directly observes the exit status
    // returned by `finish`.
    //
    // Skips on hosts without `tic` (the placeholder TerminfoEnv
    // requires it).
    if !tic_available() {
        eprintln!(
            "tic not installed, skipping live_session_finish_asserts_clean_exit_via_quit_tack"
        );
        return;
    }
    let env = TerminfoEnv::compile();
    let session = spawn_quit_on_keystroke(0);
    let live = LiveSession::new_for_test(session, env, None);
    let exit = live.finish();
    assert!(
        exit.success(),
        "expected clean exit from LiveSession::finish, got {exit:?}"
    );
}

/// SEMANTIC PIN for the C3 exit-success assertion inside
/// [`LiveSession::finish`]. Wraps a child that exits with code 1
/// after a single input read in a `LiveSession` and asserts that
/// `finish()` panics with a message containing the literal `"tack
/// exited"` (the panic format in `finish`).
///
/// Without this test, a regression that removes the
/// `assert!(exit.success(), ...)` from `finish` would pass silently
/// — this test fires the moment the assertion is gone.
///
/// Unix-only: a clean "exit 1 on single read" child is hard to
/// construct portably in `cmd.exe` without spawning a temp `.cmd`
/// file. The Windows `ConPTY` exit-success path is exercised by
/// `live_session_finish_asserts_clean_exit_via_quit_tack`'s Windows
/// arm above (the happy path), and the panic body is
/// platform-agnostic (`assert!` + `format!` + `grid_text()`), so the
/// Unix-only panic pin is sufficient coverage of the exit-failure
/// path.
#[cfg(unix)]
#[test]
fn live_session_finish_panics_on_non_success_exit() {
    if !tic_available() {
        eprintln!("tic not installed, skipping live_session_finish_panics_on_non_success_exit");
        return;
    }
    let env = TerminfoEnv::compile();
    let session = spawn_quit_on_keystroke(1);
    let live = LiveSession::new_for_test(session, env, None);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        live.finish();
    }));
    let payload = result.expect_err("expected LiveSession::finish to panic on non-success exit");
    let msg = if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        String::from("<non-string panic payload>")
    };
    assert!(
        msg.contains("tack exited"),
        "expected non-success-exit panic to mention 'tack exited', got: {msg}"
    );
}
