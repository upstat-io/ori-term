use portable_pty::{CommandBuilder, ExitStatus};

use super::super::parser::default_parser;
use super::super::spec::{
    MenuStep, PhaseSpec, ScenarioSpec, unverified_anchor, unverified_menu_key,
};
use super::phase::phase_capture_loop;
use super::{LiveSession, ScenarioRunner, ScreenFacts, assert_no_unverified_sentinels};
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

// ============================================================
// 05.0.b Phase-capture framework extension — unit tests.
//
// Two test families:
//
// 1. SENTINEL DETECTION — pure data scans through
//    `assert_no_unverified_sentinels` (and via `run_at` /
//    `run_phase_at` to prove the gate fires BEFORE PTY spawn).
//    These tests work on hosts without tack/tic installed because
//    the panic happens before any TerminfoEnv::compile or
//    spawn_tack call.
//
// 2. `phase_capture_loop` BEHAVIOR — real PtySession spawned
//    against a small shell script (Unix) or cmd.exe pause loop
//    (Windows) to exercise the loop's "anchor present" /
//    "deadline expires" branches without depending on tack.
//
// The 05.1 sub-section adds end-to-end tests against real tack
// for each modes-family cap. 05.0.b proves the PRIMITIVE.
// ============================================================

/// Helper for catch-unwind tests: extract a panic message string
/// from a `Box<dyn Any + Send>` payload, handling both `String`
/// and `&'static str` payload shapes (Rust panics use both).
fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else {
        String::from("<non-string panic payload>")
    }
}

// ----- Family 1: phase_spec construction -----

#[test]
fn phase_spec_construction_compiles() {
    // SEMANTIC PIN: this test exists so the PhaseSpec type and
    // every one of its public fields is referenced from at least
    // one #[test], proving the spec is exhaustively constructible
    // outside the tack_framework::scenarios::* modules. A future
    // PR that accidentally renames a field, makes a field private,
    // or removes a field would break THIS test in addition to
    // any specific consumer.
    const _SPEC: PhaseSpec = PhaseSpec {
        id: "construction_check",
        screen_id: "construction_check",
        menu_path: &[MenuStep::new(b"n", "anchor")],
        phase_setup_anchor: "setup",
        phase_trigger: b"n",
        phase_anchor: "(am)",
        phase_timeout_ms: 5_000,
        quit_path: None,
        parser: default_parser,
    };
    assert_eq!(_SPEC.id, "construction_check");
    assert_eq!(_SPEC.phase_anchor, "(am)");
    assert_eq!(_SPEC.menu_path.len(), 1);
}

// ----- Family 2: assert_no_unverified_sentinels (pure helper) -----

#[test]
fn assert_no_unverified_sentinels_passes_on_clean_inputs() {
    // Negative pin: a fully valid spec must NOT panic. Without
    // this test, a future refactor that accidentally always-panics
    // would break every other test silently while making this
    // helper non-load-bearing.
    let menu_path = [MenuStep::new(b"n", "anchor1")];
    assert_no_unverified_sentinels("clean_test", &menu_path, Some(b"x"), &["anchor2"]);
}

#[test]
#[should_panic(expected = "menu_path[0].send is the unverified-menu-key sentinel")]
fn assert_no_unverified_sentinels_panics_on_menu_step_send_sentinel() {
    let menu_path = [MenuStep {
        send: unverified_menu_key(),
        wait_for: "real_anchor",
        or_wait_for: &[],
    }];
    assert_no_unverified_sentinels("send_sentinel", &menu_path, None, &[]);
}

#[test]
#[should_panic(expected = "menu_path[0].wait_for is the unverified-anchor sentinel")]
fn assert_no_unverified_sentinels_panics_on_menu_step_wait_for_sentinel() {
    let menu_path = [MenuStep {
        send: b"n",
        wait_for: unverified_anchor(),
        or_wait_for: &[],
    }];
    assert_no_unverified_sentinels("wait_for_sentinel", &menu_path, None, &[]);
}

#[test]
#[should_panic(expected = "menu_path[0].or_wait_for[0] is the unverified-anchor sentinel")]
fn assert_no_unverified_sentinels_panics_on_or_wait_for_sentinel() {
    // The MenuStep::or_wait_for field is &'static [&'static str], so
    // the alternates array must live in static storage. A bare
    // `&[unverified_anchor()]` is a temporary that can't satisfy
    // 'static.
    const ALTS: &[&str] = &[unverified_anchor()];
    let menu_path = [MenuStep {
        send: b"n",
        wait_for: "primary",
        or_wait_for: ALTS,
    }];
    assert_no_unverified_sentinels("or_wait_for_sentinel", &menu_path, None, &[]);
}

#[test]
#[should_panic(expected = "phase_trigger is the unverified-menu-key sentinel")]
fn assert_no_unverified_sentinels_panics_on_phase_trigger_sentinel() {
    assert_no_unverified_sentinels(
        "phase_trigger_sentinel",
        &[],
        Some(unverified_menu_key()),
        &[],
    );
}

#[test]
#[should_panic(expected = "anchor is the unverified-anchor sentinel")]
fn assert_no_unverified_sentinels_panics_on_extra_anchor_sentinel() {
    assert_no_unverified_sentinels("anchor_sentinel", &[], None, &[unverified_anchor()]);
}

// ----- Family 3: sentinel detection through `run_at` / `run_phase_at`
// (proves the gate fires BEFORE PTY spawn / TerminfoEnv::compile,
// works on hosts without tack/tic) -----

const SENTINEL_SEND_SCENARIO_SPEC: ScenarioSpec = ScenarioSpec {
    id: "test_sentinel_send_run_at",
    screen_id: "test_sentinel_send_run_at",
    menu_path: &[MenuStep {
        send: unverified_menu_key(),
        wait_for: "real_anchor",
        or_wait_for: &[],
    }],
    ready_anchor: "real_anchor",
    quit_path: None,
    parser: default_parser,
};

const SENTINEL_READY_ANCHOR_SCENARIO_SPEC: ScenarioSpec = ScenarioSpec {
    id: "test_sentinel_ready_anchor_run_at",
    screen_id: "test_sentinel_ready_anchor_run_at",
    menu_path: &[MenuStep::new(b"n", "real_anchor")],
    ready_anchor: unverified_anchor(),
    quit_path: None,
    parser: default_parser,
};

const SENTINEL_PHASE_TRIGGER_SPEC: PhaseSpec = PhaseSpec {
    id: "test_sentinel_phase_trigger",
    screen_id: "test_sentinel_phase_trigger",
    menu_path: &[MenuStep::new(b"n", "anchor")],
    phase_setup_anchor: "setup",
    phase_trigger: unverified_menu_key(),
    phase_anchor: "(am)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: default_parser,
};

const SENTINEL_PHASE_SETUP_ANCHOR_SPEC: PhaseSpec = PhaseSpec {
    id: "test_sentinel_phase_setup_anchor",
    screen_id: "test_sentinel_phase_setup_anchor",
    menu_path: &[MenuStep::new(b"n", "anchor")],
    phase_setup_anchor: unverified_anchor(),
    phase_trigger: b"n",
    phase_anchor: "(am)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: default_parser,
};

const SENTINEL_PHASE_ANCHOR_SPEC: PhaseSpec = PhaseSpec {
    id: "test_sentinel_phase_anchor",
    screen_id: "test_sentinel_phase_anchor",
    menu_path: &[MenuStep::new(b"n", "anchor")],
    phase_setup_anchor: "setup",
    phase_trigger: b"n",
    phase_anchor: unverified_anchor(),
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: default_parser,
};

const SENTINEL_PHASE_MENU_SEND_SPEC: PhaseSpec = PhaseSpec {
    id: "test_sentinel_phase_menu_send",
    screen_id: "test_sentinel_phase_menu_send",
    menu_path: &[MenuStep {
        send: unverified_menu_key(),
        wait_for: "anchor",
        or_wait_for: &[],
    }],
    phase_setup_anchor: "setup",
    phase_trigger: b"n",
    phase_anchor: "(am)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: default_parser,
};

const SENTINEL_PHASE_MENU_WAIT_FOR_SPEC: PhaseSpec = PhaseSpec {
    id: "test_sentinel_phase_menu_wait_for",
    screen_id: "test_sentinel_phase_menu_wait_for",
    menu_path: &[MenuStep {
        send: b"n",
        wait_for: unverified_anchor(),
        or_wait_for: &[],
    }],
    phase_setup_anchor: "setup",
    phase_trigger: b"n",
    phase_anchor: "(am)",
    phase_timeout_ms: 5_000,
    quit_path: None,
    parser: default_parser,
};

/// Drive a sentinel-detection assertion through `run_at` and
/// confirm the panic message references "unverified" — proving the
/// gate fired BEFORE any PTY spawn. Works without tack/tic.
fn run_at_panic_message(spec: &ScenarioSpec) -> String {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = ScenarioRunner::run_at(spec, 80, 24);
    }));
    panic_message(result.expect_err("expected run_at to panic on sentinel spec"))
}

fn run_phase_panic_message(spec: &PhaseSpec) -> String {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = ScenarioRunner::run_phase(spec);
    }));
    panic_message(result.expect_err("expected run_phase to panic on sentinel spec"))
}

#[test]
fn run_at_panics_when_menu_step_send_is_sentinel() {
    let msg = run_at_panic_message(&SENTINEL_SEND_SCENARIO_SPEC);
    assert!(
        msg.contains("unverified-menu-key sentinel"),
        "expected sentinel panic, got: {msg}"
    );
    assert!(
        msg.contains("test_sentinel_send_run_at"),
        "expected scenario id in panic, got: {msg}"
    );
}

#[test]
fn run_at_panics_when_ready_anchor_is_sentinel() {
    let msg = run_at_panic_message(&SENTINEL_READY_ANCHOR_SCENARIO_SPEC);
    assert!(
        msg.contains("unverified-anchor sentinel"),
        "expected sentinel panic, got: {msg}"
    );
    assert!(
        msg.contains("test_sentinel_ready_anchor_run_at"),
        "expected scenario id in panic, got: {msg}"
    );
}

#[test]
fn run_phase_panics_when_phase_trigger_is_sentinel() {
    let msg = run_phase_panic_message(&SENTINEL_PHASE_TRIGGER_SPEC);
    assert!(
        msg.contains("phase_trigger is the unverified-menu-key sentinel"),
        "expected phase_trigger sentinel panic, got: {msg}"
    );
    assert!(msg.contains("test_sentinel_phase_trigger"), "got: {msg}");
}

#[test]
fn run_phase_panics_when_phase_setup_anchor_is_sentinel() {
    let msg = run_phase_panic_message(&SENTINEL_PHASE_SETUP_ANCHOR_SPEC);
    assert!(
        msg.contains("anchor is the unverified-anchor sentinel"),
        "expected anchor sentinel panic, got: {msg}"
    );
}

#[test]
fn run_phase_panics_when_phase_anchor_is_sentinel() {
    let msg = run_phase_panic_message(&SENTINEL_PHASE_ANCHOR_SPEC);
    assert!(
        msg.contains("anchor is the unverified-anchor sentinel"),
        "expected anchor sentinel panic, got: {msg}"
    );
}

#[test]
fn run_phase_panics_when_menu_step_send_is_sentinel() {
    let msg = run_phase_panic_message(&SENTINEL_PHASE_MENU_SEND_SPEC);
    assert!(
        msg.contains("menu_path[0].send is the unverified-menu-key sentinel"),
        "expected menu_path send sentinel panic, got: {msg}"
    );
}

#[test]
fn run_phase_panics_when_menu_step_wait_for_is_sentinel() {
    let msg = run_phase_panic_message(&SENTINEL_PHASE_MENU_WAIT_FOR_SPEC);
    assert!(
        msg.contains("menu_path[0].wait_for is the unverified-anchor sentinel"),
        "expected menu_path wait_for sentinel panic, got: {msg}"
    );
}

// ----- Family 4: phase_capture_loop behavior (real PTY) -----

/// Spawn a child that prints `marker` to its PTY stdout, then
/// blocks until a single byte is received on stdin (so the child
/// stays alive long enough for the test to capture).
///
/// On Unix uses `/bin/sh -c "...; head -c 1 > /dev/null"`. On
/// Windows uses `cmd.exe /C "echo MARKER & pause > NUL"`. The
/// helper is exposed in both cfg branches so the test bodies stay
/// platform-agnostic.
fn spawn_marker_then_pause(marker: &str) -> PtySession {
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        let script = format!("printf '%s\\n' {marker:?}; head -c 1 > /dev/null");
        c.args(["-c", &script]);
        c.env("TERM", "xterm-256color");
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = CommandBuilder::new("cmd.exe");
        let script = format!("echo {marker} & pause > NUL");
        c.args(["/C", &script]);
        c.env("TERM", "xterm-256color");
        c
    };
    PtySession::spawn(cmd, 80, 24)
}

/// Spawn a child that prints nothing and just blocks waiting for a
/// byte on stdin. Used to drive the `phase_capture_loop` deadline
/// branch — the loop will never see its anchor and must return
/// `None` after the timeout.
fn spawn_silent_pause() -> PtySession {
    #[cfg(unix)]
    let cmd = {
        let mut c = CommandBuilder::new("/bin/sh");
        c.args(["-c", "head -c 1 > /dev/null"]);
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
fn phase_capture_loop_returns_when_anchor_present() {
    let mut session = spawn_marker_then_pause("__PHASE_AM_MARKER__");
    let captured = phase_capture_loop(&mut session, "__PHASE_AM_MARKER__", 5_000);
    assert!(
        captured.is_some(),
        "phase_capture_loop should return Some when the anchor is in the grid"
    );
    let grid = captured.expect("captured Some above");
    assert!(
        grid.contains("__PHASE_AM_MARKER__"),
        "captured grid must contain the anchor; got:\n{grid}"
    );
    // Best-effort cleanup; Drop reaps the child even on error.
    session.send_raw(b"x");
}

#[test]
fn phase_capture_loop_returns_none_on_timeout() {
    let mut session = spawn_silent_pause();
    let started = std::time::Instant::now();
    let captured = phase_capture_loop(&mut session, "__NEVER_APPEARS__", 200);
    let elapsed = started.elapsed();
    assert!(
        captured.is_none(),
        "phase_capture_loop should return None when the anchor never appears"
    );
    assert!(
        elapsed >= std::time::Duration::from_millis(200),
        "phase_capture_loop must honor the deadline lower bound; elapsed: {elapsed:?}"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(2_000),
        "phase_capture_loop must not run away past the deadline; elapsed: {elapsed:?}"
    );
    session.send_raw(b"x");
}

// ----- Family 5: defaults pin -----

#[test]
fn phase_default_timeout_ms_matches_documented_value() {
    // SEMANTIC PIN for the default phase timeout. PhaseSpec
    // documentation references this value; if it changes here, all
    // downstream PhaseSpec consumers (and the rustdoc on
    // `phase_timeout_ms`) need to update in lockstep. The pin
    // catches drift between the constant and its callers.
    use super::phase::PHASE_DEFAULT_TIMEOUT_MS;
    assert_eq!(PHASE_DEFAULT_TIMEOUT_MS, 5_000);
}
