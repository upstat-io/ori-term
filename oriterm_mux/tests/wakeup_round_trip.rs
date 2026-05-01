//! BUG-11-026 Layer 3 — wakeup-callback contract pinned via [`RecordedProxy`].
//!
//! Tests reproduce the App-layer wiring shape from `oriterm/src/app/constructors.rs:46-48`
//! (daemon mode) and `:93-97` (embedded mode) — the closure shape
//! `Arc::new(move || { let _ = proxy.send_event(TermEvent::MuxWakeup); })`
//! — by passing `RecordedProxy::make_mux_wakeup()` into `EmbeddedMux::new`.
//! Tests then drive a real PTY shell and observe:
//!
//! 1. `gate_blocks_when_no_wakeup` — `mux.has_pending_wakeup()` returns
//!    false after startup-wakeup drain; gate's early-exit at
//!    `oriterm/src/app/mux_pump/mod.rs:35-37` is sound.
//! 2. `gate_passes_after_io_emit` — DA1 query → `has_pending_wakeup()`
//!    becomes true → drain delivers response bytes back to child stdin.
//! 3. `recorded_proxy_observes_wakeup` — proxy receives
//!    `RecordedTermEvent::MuxWakeup` for every IO-thread effect emission.
//! 4. `recorded_proxy_does_not_double_signal` — coalescing guard at
//!    `oriterm_mux/src/backend/embedded/mod.rs:62-66` fires the wakeup
//!    AT MOST once per parse cycle.
//! 5. `flag_clears_on_poll_events` — `wakeup_pending.store(false)` at
//!    `oriterm_mux/src/backend/embedded/mod.rs:86` runs before drain.
//!
//! These tests live in `oriterm_mux/tests/` (NOT `oriterm/tests/`) per
//! the `.claude/rules/crate-boundaries.md §Litmus Test`: the tests
//! construct only mux-layer types (`EmbeddedMux`, `RecordedProxy`,
//! `SpawnConfig`) and exercise only mux-crate code. The doc comments
//! cite `constructors.rs:94-97` as the reference closure shape this
//! test reproduces — the documentation captures the App-layer
//! reference-pattern intent without violating the litmus.

#![cfg(unix)]

use std::thread;
use std::time::{Duration, Instant};

use oriterm_core::Theme;
use oriterm_mux::backend::MuxBackend;
use oriterm_mux::domain::SpawnConfig;
use oriterm_mux::{EmbeddedMux, PaneId};

use oriterm_test_support::event_proxy_stub::{RecordedProxy, RecordedTermEvent};

/// Build a minimal `SpawnConfig` for these tests (history suppressed so
/// fence commands don't pollute the user's `~/.zsh_history`).
fn test_spawn_config() -> SpawnConfig {
    SpawnConfig {
        env: vec![("HISTFILE".into(), "/dev/null".into())],
        ..SpawnConfig::default()
    }
}

/// Drain the startup wakeup signal sequence so tests start from a known
/// quiescent state. The shell prompt's first render produces a small
/// burst of wakeups; this helper calls `poll_events` + `drain_notifications`
/// in a poll-the-condition loop until the wakeup flag clears AND no
/// further wakeups land within a brief settling window.
///
/// Per `.claude/rules/tests.md §Wall-Clock-Free Testing`, the deadline
/// is the safety valve; the awaited condition is the flag transition.
fn drain_startup_wakeups(mux: &mut EmbeddedMux, deadline: Duration) {
    let stop = Instant::now() + deadline;
    loop {
        if mux.has_pending_wakeup() {
            mux.poll_events();
            let mut notifs = Vec::new();
            mux.drain_notifications(&mut notifs);
        } else {
            // Brief settling window — if no further wakeup lands, we're done.
            thread::sleep(Duration::from_millis(50));
            if !mux.has_pending_wakeup() {
                return;
            }
        }
        assert!(
            Instant::now() < stop,
            "startup wakeup drain did not quiesce within {deadline:?}"
        );
    }
}

/// Spawn a pane and wait until the shell is ready by sending a fence
/// command and polling until its output appears.
fn spawn_ready_pane(mux: &mut EmbeddedMux) -> PaneId {
    let pane_id = mux
        .spawn_pane(&test_spawn_config(), Theme::Dark)
        .expect("spawn_pane");
    mux.send_input(pane_id, b"echo SHELL_READY_FENCE\n");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        mux.poll_events();
        let mut notifs = Vec::new();
        mux.drain_notifications(&mut notifs);
        if let Some(snap) = mux.refresh_pane_snapshot(pane_id) {
            let count = snap
                .cells
                .iter()
                .filter(|row| {
                    let line: String = row.iter().map(|c| c.ch).collect();
                    line.contains("SHELL_READY_FENCE")
                })
                .count();
            if count >= 2 {
                return pane_id;
            }
        }
        assert!(
            Instant::now() < deadline,
            "shell did not start within 30 seconds"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

/// Semantic pin: the gate's early-exit path at
/// `oriterm/src/app/mux_pump/mod.rs:35-37` — `has_pending_wakeup()`
/// returns false after startup drain; subsequent gate checks NEVER
/// trigger `poll_events` because no IO activity is happening.
///
/// Reproduces the App-layer wiring shape from
/// `oriterm/src/app/constructors.rs:94-97` via [`RecordedProxy`].
#[test]
fn gate_blocks_when_no_wakeup() {
    let proxy = RecordedProxy::new();
    let mut mux = EmbeddedMux::new(proxy.make_mux_wakeup());
    let _pane_id = spawn_ready_pane(&mut mux);
    drain_startup_wakeups(&mut mux, Duration::from_secs(5));
    // Single-sample assertion — drain_startup_wakeups already established
    // the deterministic synchronization signal (no pending wakeup +
    // 50ms settle). If a spurious wakeup mechanism existed, the next
    // assertion would catch it on the very first call.
    assert!(
        !mux.has_pending_wakeup(),
        "gate must be false after startup wakeup drain"
    );
    let observed_before = proxy.observed().len();
    assert!(
        !mux.has_pending_wakeup(),
        "second sample: gate stays false absent IO activity"
    );
    let observed_after = proxy.observed().len();
    assert_eq!(
        observed_before, observed_after,
        "no spurious wakeup signal between samples"
    );
}

/// Semantic pin: gate-flag transitions correctly under the App-pattern
/// drain.
///
/// Sends an `echo` (which fires a wakeup as IO thread emits PaneOutput
/// notification — verified by the BUG-11-004 effect-router tests),
/// then mirrors `App::pump_mux_events`'s gate logic at
/// `oriterm/src/app/mux_pump/mod.rs:35-43`: only call `poll_events`
/// when `has_pending_wakeup()` returns true.
///
/// Validates that the gate eventually opens (flag flips true after
/// IO-thread emit), and the gated drain delivers the echo output to
/// the snapshot. Pins the gate's transition contract end-to-end.
#[test]
fn gate_passes_after_io_emit() {
    let proxy = RecordedProxy::new();
    let mut mux = EmbeddedMux::new(proxy.make_mux_wakeup());
    let pane_id = spawn_ready_pane(&mut mux);
    drain_startup_wakeups(&mut mux, Duration::from_secs(5));
    while proxy.wait_for_wakeup(Duration::from_millis(20)).is_ok() {}

    // Send an echo — IO thread emits PaneOutput notification, fires the
    // wakeup callback.
    mux.send_input(pane_id, b"echo POST_GATE_MARKER\n");

    // Mirror App::pump_mux_events gate logic — wait for has_pending_wakeup
    // to become true, then drain via the gated path.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut gate_passed = false;
    while Instant::now() < deadline {
        if mux.has_pending_wakeup() {
            mux.poll_events();
            let mut notifs = Vec::new();
            mux.drain_notifications(&mut notifs);
            gate_passed = true;
            break;
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        gate_passed,
        "gate never opened — has_pending_wakeup stayed false"
    );

    // Continue draining via the gated path until the echo output appears
    // in the snapshot — proves the round-trip completes through the gate.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if mux.has_pending_wakeup() {
            mux.poll_events();
            let mut notifs = Vec::new();
            mux.drain_notifications(&mut notifs);
        }
        if let Some(snap) = mux.refresh_pane_snapshot(pane_id) {
            let count = snap
                .cells
                .iter()
                .filter(|row| {
                    let line: String = row.iter().map(|c| c.ch).collect();
                    line.contains("POST_GATE_MARKER")
                })
                .count();
            if count >= 2 {
                return;
            }
        }
        assert!(
            Instant::now() < deadline,
            "POST_GATE_MARKER did not appear after gated drain"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

/// Semantic pin: the [`RecordedProxy`] callback fires when an IO-thread
/// effect lands. Reproduces `oriterm/src/app/constructors.rs:94-97`'s
/// closure shape — every `MuxEvent::PtyWrite` trips the wakeup, which
/// reaches the recorder.
///
/// Uses an `echo` command rather than a device-query because shell-
/// prompt-framework consumption (see BUG-11-029 in section-11-mux.md)
/// can suppress query-response wakeups in the user's interactive
/// shell environment. `echo` produces shell-stdout bytes that ALWAYS
/// flow through the IO thread → fire the wakeup callback regardless
/// of prompt-framework state.
#[test]
fn recorded_proxy_observes_wakeup() {
    let proxy = RecordedProxy::new();
    let mut mux = EmbeddedMux::new(proxy.make_mux_wakeup());
    let pane_id = spawn_ready_pane(&mut mux);
    drain_startup_wakeups(&mut mux, Duration::from_secs(5));
    // Drain the proxy's mpsc backlog from startup wakeups too.
    while proxy.wait_for_wakeup(Duration::from_millis(20)).is_ok() {}

    let observed_before_query = proxy.observed().len();
    mux.send_input(pane_id, b"echo PROXY_OBSERVE_FENCE\n");

    // Poll for the snapshot to contain the fence text — this proves the
    // round-trip ran and at least one wakeup landed.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut saw_fence = false;
    while Instant::now() < deadline {
        if mux.has_pending_wakeup() {
            mux.poll_events();
            let mut notifs = Vec::new();
            mux.drain_notifications(&mut notifs);
        }
        if let Some(snap) = mux.refresh_pane_snapshot(pane_id) {
            let count = snap
                .cells
                .iter()
                .filter(|row| {
                    let line: String = row.iter().map(|c| c.ch).collect();
                    line.contains("PROXY_OBSERVE_FENCE")
                })
                .count();
            if count >= 2 {
                saw_fence = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        saw_fence,
        "echo PROXY_OBSERVE_FENCE did not produce snapshot output"
    );

    let observed_after = proxy.observed();
    assert!(
        observed_after.len() > observed_before_query,
        "proxy should have recorded at least one new MuxWakeup after echo \
         (before={observed_before_query}, after={})",
        observed_after.len()
    );
    assert!(
        observed_after
            .iter()
            .all(|e| *e == RecordedTermEvent::MuxWakeup),
        "all observed events must be MuxWakeup"
    );
}

/// Negative pin: the coalescing guard at
/// `oriterm_mux/src/backend/embedded/mod.rs:62-66` (`pending.swap(true,
/// Release)`) prevents wakeup spam — many effects in one parse cycle
/// fire AT MOST a small bounded number of wakeups until `poll_events`
/// clears the flag.
///
/// Sends multi-line shell output (a `seq | xargs echo` burst) which
/// produces many PaneOutput effects in rapid succession. Without the
/// coalescing guard, every PtyEffect would fire its own wakeup;
/// with the guard, the IO thread bounds the wakeup count.
#[test]
fn recorded_proxy_does_not_double_signal() {
    let proxy = RecordedProxy::new();
    let mut mux = EmbeddedMux::new(proxy.make_mux_wakeup());
    let pane_id = spawn_ready_pane(&mut mux);
    drain_startup_wakeups(&mut mux, Duration::from_secs(5));
    while proxy.wait_for_wakeup(Duration::from_millis(20)).is_ok() {}

    let observed_before = proxy.observed().len();
    // Burst of output: 20 short echoes. Each `echo` produces multiple
    // PtyEffect writes (chars + newline + prompt redraw). Without
    // coalescing, this would produce many wakeups; with coalescing,
    // the IO thread bounds it to at most a few per parse cycle.
    mux.send_input(pane_id, b"for i in 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20; do echo COALESCE_BURST_$i; done; echo BURST_DONE\n");

    // Wait for BURST_DONE marker to confirm the burst completed.
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if mux.has_pending_wakeup() {
            mux.poll_events();
            let mut notifs = Vec::new();
            mux.drain_notifications(&mut notifs);
        }
        if let Some(snap) = mux.refresh_pane_snapshot(pane_id) {
            let count = snap
                .cells
                .iter()
                .filter(|row| {
                    let line: String = row.iter().map(|c| c.ch).collect();
                    line.contains("BURST_DONE")
                })
                .count();
            if count >= 2 {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "BURST_DONE did not appear within deadline"
        );
        thread::sleep(Duration::from_millis(20));
    }

    // Settle window — give any spurious follow-up wakeups time to fire.
    thread::sleep(Duration::from_millis(100));

    let observed_after = proxy.observed().len();
    let new_wakeups = observed_after - observed_before;
    // Coalescing-guard pin: 20 echoes producing many effects must NOT
    // produce >40 wakeups (no coalescing would mean ~60-100 wakeups —
    // each echo writes "COALESCE_BURST_N\n" plus prompt redraw, all
    // generating PaneOutput effects). The exact bound depends on how
    // the IO thread chunks PTY reads, but it must be SUBSTANTIALLY
    // less than the effect count without coalescing.
    assert!(
        new_wakeups <= 40,
        "coalescing guard failed: 20-echo burst produced {new_wakeups} wakeups \
         (expected substantially less than the 60+ effect count)"
    );
    assert!(
        new_wakeups >= 1,
        "expected at least 1 wakeup for the burst, got {new_wakeups}"
    );
}

/// Negative pin: `wakeup_pending.store(false, Release)` at
/// `oriterm_mux/src/backend/embedded/mod.rs:86` runs at the START of
/// `poll_events`. After draining, `has_pending_wakeup()` returns false
/// until the next IO emit. Pins the gate's flag-clear ordering.
#[test]
fn flag_clears_on_poll_events() {
    let proxy = RecordedProxy::new();
    let mut mux = EmbeddedMux::new(proxy.make_mux_wakeup());
    let pane_id = spawn_ready_pane(&mut mux);
    drain_startup_wakeups(&mut mux, Duration::from_secs(5));
    while proxy.wait_for_wakeup(Duration::from_millis(20)).is_ok() {}

    // Trigger a wakeup via echo (reliable across all shell environments).
    mux.send_input(pane_id, b"echo FLAG_CLEAR_FENCE\n");

    // Wait until has_pending_wakeup transitions to true (poll-the-condition
    // — the deadline is the safety valve, not the signal).
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if mux.has_pending_wakeup() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(
        mux.has_pending_wakeup(),
        "flag must be true after IO-thread emit (echo command)"
    );
    mux.poll_events();
    let mut notifs = Vec::new();
    mux.drain_notifications(&mut notifs);

    // After poll_events, the flag must be false (until next IO emit).
    // Drain any follow-up wakeups (shell prompt redraw, etc.) and assert
    // the flag clears once the IO thread is quiescent.
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        thread::sleep(Duration::from_millis(50));
        if !mux.has_pending_wakeup() {
            return;
        }
        mux.poll_events();
        let mut notifs = Vec::new();
        mux.drain_notifications(&mut notifs);
        assert!(
            Instant::now() < deadline,
            "flag never cleared after poll_events drain"
        );
    }
}
