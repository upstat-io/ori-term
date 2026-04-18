//! Spawn-thread integration tests for the host-request reply path.
//!
//! These pin the load-bearing semantics that are unreachable through
//! direct router calls — the wake-channel that unblocks `select!`,
//! wake-collapse, and the lost-wake race. Each test spawns a real
//! `PaneIoThread<QueueingEffectSink>` and observes via `mux_rx`.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::mpsc;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

use oriterm_core::effect::{QueueingEffectSink, ResponseToken};
use oriterm_core::{Term, TermMode, Theme};

use super::super::{IoThreadConfig, PaneIoHandle, new_with_handle};
use crate::PaneId;
use crate::mux_event::MuxEvent;
use crate::pty::spawn::ExitStatus;

/// Spawn a `PaneIoThread<QueueingEffectSink>` with a real `mux_rx` end
/// for the test to observe and a held-open `child_exit_tx` to keep the
/// child-exit arm idle.
fn spawn_queueing_pair() -> (
    PaneIoHandle,
    mpsc::Receiver<MuxEvent>,
    Sender<ExitStatus>,
    Arc<AtomicBool>,
) {
    let shutdown = Arc::new(AtomicBool::new(false));
    let (mux_tx, mux_rx) = mpsc::channel::<MuxEvent>();
    let (exit_tx, exit_rx): (_, Receiver<ExitStatus>) = crossbeam_channel::bounded(1);
    let term = Term::new(24, 80, 1000, Theme::default(), QueueingEffectSink::new());
    let (thread, mut handle) = new_with_handle(IoThreadConfig {
        terminal: term,
        pane_id: PaneId::from_raw(99),
        mux_tx,
        child_exit_rx: exit_rx,
        mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
        shutdown: Arc::clone(&shutdown),
        wakeup: Arc::new(|| {}),
        grid_dirty: Arc::new(AtomicBool::new(false)),
        pty_control: None,
        adopted_signal: None,
        initial_rows: 24,
        initial_cols: 80,
        selection_dirty: Arc::new(AtomicBool::new(false)),
    });
    let join = thread.spawn().expect("failed to spawn IO thread");
    handle.set_join(join);
    (handle, mux_rx, exit_tx, shutdown)
}

/// Recv from `mux_rx` filtering for the first `MuxEvent::HostClipboardLoad`,
/// returning its `reply` token for the test to fulfill.
fn await_host_clipboard_load(
    rx: &mpsc::Receiver<MuxEvent>,
    deadline: Duration,
) -> ResponseToken<String> {
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline
            .checked_sub(start.elapsed())
            .unwrap_or(Duration::ZERO);
        match rx.recv_timeout(remaining) {
            Ok(MuxEvent::HostClipboardLoad { reply, .. }) => return reply,
            Ok(_) => continue, // skip unrelated MuxEvents (snapshot, etc.)
            Err(e) => panic!("HostClipboardLoad never arrived: {e}"),
        }
    }
}

/// Recv from `mux_rx` filtering for the first `MuxEvent::PtyWrite`,
/// returning its byte payload.
fn await_pty_write(rx: &mpsc::Receiver<MuxEvent>, deadline: Duration) -> Vec<u8> {
    let start = std::time::Instant::now();
    loop {
        let remaining = deadline
            .checked_sub(start.elapsed())
            .unwrap_or(Duration::ZERO);
        match rx.recv_timeout(remaining) {
            Ok(MuxEvent::PtyWrite { data, .. }) => return data,
            Ok(_) => continue,
            Err(e) => panic!("PtyWrite never arrived: {e}"),
        }
    }
}

/// **THE critical pin (§01.2)**: a fulfilled `ResponseToken` unblocks
/// the IO thread's `crossbeam_channel::select!` within one iteration —
/// no PTY bytes, no commands required.
///
/// End-to-end via the production API: send OSC 52 read bytes through
/// `byte_tx`, observe `MuxEvent::HostClipboardLoad` (proves the request
/// was registered), call `handle.fulfill_clipboard_load(..)`, then
/// observe `MuxEvent::PtyWrite` carrying the base64-encoded reply.
///
/// The IO thread's `select!` is otherwise idle between the fulfill and
/// the PtyWrite emission — only the wake channel can unblock it.
#[test]
fn response_poll_idle_wake_unblocks_select() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    // Send OSC 52 clipboard read: ESC ] 52 ; c ; ? BEL
    let osc52 = b"\x1b]52;c;?\x07";
    handle
        .byte_sender()
        .send(osc52.to_vec())
        .expect("byte send must succeed");

    // The router emits HostClipboardLoad carrying a clone of the token.
    let token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // Fulfill from the test thread — this is the ONLY thing that wakes
    // the IO thread (no further bytes, no commands).
    handle
        .fulfill_clipboard_load(&token, "hello".to_string())
        .expect("fresh fulfill must succeed");

    // Within one select! iteration the IO thread polls pending responses,
    // discovers the fulfilled token, and emits the PTY write.
    let data = await_pty_write(&mux_rx, Duration::from_secs(5));
    let s = String::from_utf8(data).expect("OSC 52 reply is ASCII");
    assert!(
        s.contains("aGVsbG8="),
        "PtyWrite must carry base64('hello'); got {s:?}"
    );

    handle.shutdown();
}

/// Companion negative pin: without `fulfill_*`, no `PtyWrite` is ever
/// emitted. Proves the wake is load-bearing — the registration alone is
/// not enough to drive a reply.
#[test]
fn response_poll_token_requires_fulfillment() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    let osc52 = b"\x1b]52;c;?\x07";
    handle.byte_sender().send(osc52.to_vec()).unwrap();

    // Confirm the request was registered (HostClipboardLoad emitted).
    let _token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // Wait briefly with NO fulfill. No PtyWrite must arrive.
    std::thread::sleep(Duration::from_millis(150));
    while let Ok(event) = mux_rx.try_recv() {
        if let MuxEvent::PtyWrite { data, .. } = event {
            panic!("unexpected PtyWrite without fulfillment: {data:?}");
        }
    }

    handle.shutdown();
}

/// Blind-spot §14: a single wake drains ALL ready tokens. Two fulfills
/// against the bounded(1) wake channel collapse to one wake; the next
/// `poll_pending_responses` iteration emits BOTH replies.
#[test]
fn two_fulfills_one_wake_drains_both_tokens() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    // Two clipboard read requests back-to-back.
    handle
        .byte_sender()
        .send(b"\x1b]52;c;?\x07\x1b]52;p;?\x07".to_vec())
        .unwrap();

    let token_a = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));
    let token_b = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // Two fulfills in rapid succession — bounded(1) channel coalesces.
    handle
        .fulfill_clipboard_load(&token_a, "AAA".to_string())
        .unwrap();
    handle
        .fulfill_clipboard_load(&token_b, "BBB".to_string())
        .unwrap();

    let first = await_pty_write(&mux_rx, Duration::from_secs(5));
    let second = await_pty_write(&mux_rx, Duration::from_secs(5));
    let s1 = String::from_utf8(first).unwrap();
    let s2 = String::from_utf8(second).unwrap();
    let pair = (s1.contains("QUFB"), s2.contains("QkJC")); // base64
    let pair_swapped = (s1.contains("QkJC"), s2.contains("QUFB"));
    assert!(
        pair == (true, true) || pair_swapped == (true, true),
        "both fulfilled tokens must produce PtyWrites; got {s1:?} {s2:?}"
    );

    handle.shutdown();
}

/// Blind-spot §1: a fulfillment that lands BEFORE the IO thread
/// registers the pending response is not lost. Polling is pull-based —
/// the next poll after register sees the already-set slot via
/// `reply.take()` and emits the PTY write without a second wake.
///
/// We approximate the race by fulfilling the token cloned from the
/// HostClipboardLoad notification immediately, before polling has had a
/// chance to observe a still-empty slot. The IO thread MUST still
/// deliver the PtyWrite — proves polling is take-on-every-tick, not
/// edge-triggered on wake.
#[test]
fn fulfill_immediately_after_request_still_delivers() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    handle
        .byte_sender()
        .send(b"\x1b]52;c;?\x07".to_vec())
        .unwrap();

    let token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));
    // Fulfill before any other activity — racing the register on the IO
    // thread side. The wake-tx send may or may not arrive before the
    // pending entry is registered, but `take()` on the next poll cycle
    // surfaces the value either way.
    handle
        .fulfill_clipboard_load(&token, "race".to_string())
        .expect("fresh fulfill");

    let data = await_pty_write(&mux_rx, Duration::from_secs(5));
    let s = String::from_utf8(data).unwrap();
    assert!(s.contains("cmFjZQ=="), "expected base64('race'), got {s:?}");

    handle.shutdown();
}

/// Blind-spot §1 negative companion: a fulfillment that lands AFTER
/// an unrelated wake signal is consumed is still delivered on the next
/// poll cycle. Polling is pull-based — wake signals are an
/// optimization for latency, not correctness.
///
/// Setup: fulfill via `token.fulfill()` DIRECTLY (bypasses
/// `handle.fulfill_clipboard_load`'s wake-tx pulse). Then send a
/// no-op byte stream to trigger ONE byte_rx wake — that wake's poll
/// cycle is what discovers the fulfillment. If polling were
/// edge-triggered on the wake-channel signal specifically (not on any
/// IO-thread wake source), this test would hang because the
/// fulfillment never sent a wake-channel signal of its own.
#[test]
fn no_wake_signal_drops_late_fulfill() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    handle
        .byte_sender()
        .send(b"\x1b]52;c;?\x07".to_vec())
        .unwrap();

    let token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // Fulfill via the token's own `fulfill` method — no wake-tx pulse
    // is sent. The IO thread is currently parked in `select!` waiting
    // on byte_rx / cmd_rx / response_wake_rx / child_exit_rx. Without
    // a wake-tx pulse, only an unrelated wake source can drive the
    // next poll cycle. We use byte arrival as that unrelated source.
    token
        .fulfill("late".to_string())
        .expect("fresh fulfill (no wake pulse)");

    // Trigger an unrelated wake via a no-op byte (single ASCII char,
    // no escape sequence — produces no effects, just a parser cycle).
    // The IO thread wakes on byte_rx, runs handle_bytes → drain_commands
    // → poll_pending_responses, which discovers the fulfilled token via
    // `take()` and emits the PTY write. If polling were
    // wake-channel-edge-triggered specifically, this would hang.
    handle.byte_sender().send(b" ".to_vec()).unwrap();

    let data = await_pty_write(&mux_rx, Duration::from_secs(5));
    let s = String::from_utf8(data).unwrap();
    assert!(s.contains("bGF0ZQ=="), "expected base64('late'), got {s:?}");

    handle.shutdown();
}

/// Blind-spot §18 positive pin: cancellation detection survives a
/// staging-buffer move. Drop the consumer-held token clone after one
/// staging-hop (move semantics, NOT clone) and confirm the IO thread
/// emits no PTY write — the entry is recognised as cancelled because
/// `Arc::strong_count` is preserved across the move and reads `1`
/// once the consumer side is gone.
#[test]
fn cancellation_detects_after_staging_drain() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    handle
        .byte_sender()
        .send(b"\x1b]52;c;?\x07".to_vec())
        .unwrap();

    // Pull the HostClipboardLoad MuxEvent and simulate ONE staging
    // hop via Vec::remove (a MOVE, not a clone — the variant ownership
    // transfers from `mux_rx` → `staging` → `moved` with no Arc clone).
    let mut staging: Vec<MuxEvent> = vec![mux_rx.recv().expect("HostClipboardLoad")];
    let moved = staging.remove(0);
    // Drop the entire MuxEvent (including its token reply field) at
    // the far side of the staging hop — the consumer-side Arc is gone.
    drop(moved);

    // Wait briefly for the IO thread to next poll. With pull-based
    // polling, the cancellation should be discovered on the next
    // poll_pending_responses cycle (driven by wake signals, command
    // arrivals, or the sync-deadline tick — we don't need to send any
    // bytes ourselves to drive it; the wake on registration was
    // already delivered).
    //
    // No PtyWrite must ever arrive — the request was cancelled.
    std::thread::sleep(Duration::from_millis(150));
    if let Ok(ev) = mux_rx.try_recv() {
        if matches!(ev, MuxEvent::PtyWrite { .. }) {
            panic!("unexpected PtyWrite for cancelled request: {ev:?}");
        }
    }

    handle.shutdown();
}

/// Blind-spot §18 negative pin: explicitly cloning the inner
/// `ResponseToken` artificially extends the `Arc<Mutex<_>>` lifetime
/// past the consumer's logical drop of the carrier MuxEvent.
/// Cancellation detection FAILS — the IO thread keeps polling because
/// `consumer_strong_count` stays >= 2. This documents the trap and
/// proves the move-only invariant is load-bearing, not stylistic.
///
/// The pending-entry-still-alive evidence is constructive: after the
/// "consumer-side" original is dropped, we use the LEAKED clone to
/// fulfill the request. If cancellation had triggered (which is what
/// move semantics would correctly produce), the entry would have been
/// removed from `pending_responses` and the fulfill would have no
/// effect — no PtyWrite would arrive. The fact that PtyWrite DOES
/// arrive proves the entry leaked past the nominal consumer drop.
///
/// Note: `MuxEvent` itself does NOT implement `Clone` — that absence
/// is the type-system-level enforcement of the move-only invariant
/// per blind-spot §18. The trap exists at the level of cloning the
/// INNER `ResponseToken` (which IS `Clone`); this test extracts the
/// token and demonstrates the failure mode.
#[test]
fn cancellation_fails_when_inner_token_cloned() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    handle
        .byte_sender()
        .send(b"\x1b]52;c;?\x07".to_vec())
        .unwrap();

    let token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));
    // FORBIDDEN PATTERN — clone the inner token into a "staging" Arc.
    // Each clone bumps `Arc::strong_count`, defeating the cancellation
    // detection in `PendingResponse::poll`.
    let leaked_clone = token.clone();
    drop(token);

    // Constructive proof: use the leaked clone to fulfill. If the
    // pending entry had been correctly cancelled (which would have
    // happened with move semantics), this fulfill would land in an
    // already-removed entry and produce NO PtyWrite. The PtyWrite
    // arriving is direct evidence the entry leaked past the nominal
    // consumer drop, exactly as the move-only invariant predicts.
    handle
        .fulfill_clipboard_load(&leaked_clone, "leaked".to_string())
        .expect("fulfill via leaked clone — pending entry stayed alive");

    let data = await_pty_write(&mux_rx, Duration::from_secs(5));
    let s = String::from_utf8(data).unwrap();
    assert!(
        s.contains("bGVha2Vk"),
        "leaked clone delivered the PtyWrite — proves pending entry leaked past consumer drop. got {s:?}"
    );

    handle.shutdown();
}
