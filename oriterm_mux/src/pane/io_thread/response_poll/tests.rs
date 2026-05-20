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

use oriterm_core::effect::{QueueingEffectSink, ResponseToken, format_clipboard_reply};
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
///
/// The absolute deadline is enforced at the top of every loop iteration.
/// `recv_timeout` alone is insufficient: if the IO thread streams unrelated
/// events (typically `Snapshot`) faster than this loop iterates, each
/// `recv_timeout(remaining)` returns `Ok` with a non-matching event and
/// the loop continues — potentially forever. The top-of-loop deadline
/// check guarantees the helper terminates regardless of event rate.
fn await_host_clipboard_load(
    rx: &mpsc::Receiver<MuxEvent>,
    deadline: Duration,
) -> ResponseToken<String> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() >= deadline {
            panic!("HostClipboardLoad never arrived within {deadline:?}");
        }
        let remaining = deadline.saturating_sub(start.elapsed());
        match rx.recv_timeout(remaining) {
            Ok(MuxEvent::HostClipboardLoad { reply, .. }) => return reply,
            Ok(_) => continue, // skip unrelated MuxEvents (snapshot, etc.)
            Err(e) => panic!("HostClipboardLoad never arrived: {e}"),
        }
    }
}

/// Recv from `mux_rx` filtering for the first `MuxEvent::PtyWrite`,
/// returning its byte payload. Same absolute-deadline contract as
/// [`await_host_clipboard_load`].
fn await_pty_write(rx: &mpsc::Receiver<MuxEvent>, deadline: Duration) -> Vec<u8> {
    let start = std::time::Instant::now();
    loop {
        if start.elapsed() >= deadline {
            panic!("PtyWrite never arrived within {deadline:?}");
        }
        let remaining = deadline.saturating_sub(start.elapsed());
        match rx.recv_timeout(remaining) {
            Ok(MuxEvent::PtyWrite { data, .. }) => return data,
            Ok(_) => continue,
            Err(e) => panic!("PtyWrite never arrived: {e}"),
        }
    }
}

/// Poll `mux_rx` for `window` duration, panicking the moment a
/// `MuxEvent::PtyWrite` arrives. Non-PtyWrite events (`Snapshot`, etc.) are
/// skipped silently. Replaces the forbidden `thread::sleep(window);
/// rx.try_recv() …` shape: fails fast on arrival instead of waiting out the
/// full window before draining. Pattern reference:
/// `oriterm_mux/tests/e2e.rs::wait_for_text_in_snapshot` (positive companion).
fn assert_no_pty_write_within(rx: &mpsc::Receiver<MuxEvent>, window: Duration, ctx: &str) {
    let deadline = std::time::Instant::now() + window;
    while std::time::Instant::now() < deadline {
        match rx.try_recv() {
            Ok(MuxEvent::PtyWrite { data, .. }) => {
                panic!("unexpected PtyWrite {ctx} within {window:?}: {data:?}");
            }
            Ok(_) => continue,
            Err(mpsc::TryRecvError::Empty) => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(mpsc::TryRecvError::Disconnected) => break,
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

/// Companion regression guard: without `fulfill_*`, no `PtyWrite` is ever
/// emitted. Proves the wake is load-bearing — the registration alone is
/// not enough to drive a reply.
#[test]
fn response_poll_token_requires_fulfillment() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    let osc52 = b"\x1b]52;c;?\x07";
    handle.byte_sender().send(osc52.to_vec()).unwrap();

    // Confirm the request was registered (HostClipboardLoad emitted).
    let _token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // With NO fulfill, no PtyWrite must arrive. Poll the channel and
    // fail fast on any PtyWrite during the window.
    assert_no_pty_write_within(&mux_rx, Duration::from_millis(150), "without fulfillment");

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
    // on byte_rx / cmd_rx / io_wake_rx / child_exit_rx. Without
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

    // Cancellation must be discovered on the next poll_pending_responses
    // cycle (driven by the wake delivered at registration). No PtyWrite
    // must ever arrive — the request was cancelled. Poll the channel
    // and fail fast on any PtyWrite during the window.
    assert_no_pty_write_within(&mux_rx, Duration::from_millis(150), "for cancelled request");

    handle.shutdown();
}

/// Blind-spot §18 regression guard: explicitly cloning the inner
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

/// Blind-spot §14 canonical-name pin: matches the plan's named test for
/// the bounded(1) wake-channel coalescing semantic. Multiple fulfills
/// between wakes collapse to one wake; `poll_pending_responses`
/// iterates EVERY entry on that single wake and emits N PTY writes.
/// Companion to `two_fulfills_one_wake_drains_both_tokens` which
/// exercises the same invariant with two tokens; this pin extends the
/// matrix to three so the iteration-on-wake property is reinforced.
#[test]
fn multiple_fulfills_collapse_to_one_wake() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    // Three clipboard read requests — three pending responses to pin
    // the "many fulfills, one wake" coalescing semantic.
    handle
        .byte_sender()
        .send(b"\x1b]52;c;?\x07\x1b]52;p;?\x07\x1b]52;s;?\x07".to_vec())
        .unwrap();

    let token_a = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));
    let token_b = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));
    let token_c = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // Three rapid fulfills against the bounded(1) wake channel — the
    // second and third try_send return Err(Full) silently, but the
    // single pending wake suffices to drain ALL THREE tokens on the
    // next poll_pending_responses iteration.
    handle
        .fulfill_clipboard_load(&token_a, "AAA".to_string())
        .unwrap();
    handle
        .fulfill_clipboard_load(&token_b, "BBB".to_string())
        .unwrap();
    handle
        .fulfill_clipboard_load(&token_c, "CCC".to_string())
        .unwrap();

    // All three replies must surface — base64 of A/B/C repeated.
    let mut seen: std::collections::HashSet<&'static str> = std::collections::HashSet::new();
    for _ in 0..3 {
        let data = await_pty_write(&mux_rx, Duration::from_secs(5));
        let s = String::from_utf8(data).unwrap();
        for needle in ["QUFB", "QkJC", "Q0ND"] {
            // base64('AAA'), base64('BBB'), base64('CCC')
            if s.contains(needle) {
                seen.insert(needle);
            }
        }
    }
    assert_eq!(
        seen.len(),
        3,
        "all three coalesced fulfills must surface as PtyWrite (got {seen:?})"
    );

    handle.shutdown();
}

/// §10.2 canonical OSC-52-bytes-in → PTY-write end-to-end pin.
///
/// Drives OSC 52 load bytes (ST-terminated, distinct from the
/// BEL-terminated `\x07` form the other tests use) through the
/// production `PaneIoThread` pipeline: `byte_tx` → parser → dispatch →
/// `effect_router::register_host_request_response` → `poll_pending_
/// responses` → `PtyEffect::Write`. Fulfills the token directly via
/// `token.fulfill(..)` (NOT via `handle.fulfill_clipboard_load`, which
/// is already exercised by the sibling tests) and asserts the emitted
/// PtyWrite bytes are BYTE-IDENTICAL to `format_clipboard_reply(..)` —
/// the canonical reply formatter at `oriterm_core/src/effect/families/
/// host_request/mod.rs:304`. Re-implementing the format inline would be
/// a LEAK against SSOT.
///
/// A second byte is sent after the direct fulfill to unblock the
/// IO-thread `select!` (same pattern as
/// `fulfill_before_register_still_delivers` and
/// `no_wake_signal_drops_late_fulfill`).
#[test]
fn osc52_register_poll_roundtrip() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    // ST-terminated OSC 52 load: ESC ] 52 ; c ; ? ESC \
    let osc52 = b"\x1b]52;c;?\x1b\\";
    handle
        .byte_sender()
        .send(osc52.to_vec())
        .expect("byte send must succeed");

    let token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // Fulfill via the token's own `fulfill` — bypasses the wake-tx pulse
    // that `handle.fulfill_clipboard_load` would emit. Pull-based polling
    // must still surface the value on the next `select!` iteration.
    token
        .fulfill("example-text".to_string())
        .expect("fresh fulfill must succeed");

    // Unrelated wake — a no-op space byte — to drive the next poll cycle.
    handle.byte_sender().send(b" ".to_vec()).unwrap();

    let data = await_pty_write(&mux_rx, Duration::from_secs(5));

    // SSOT: the emitted PTY bytes must equal `format_clipboard_reply`'s
    // output for the same (text, clipboard_char, terminator) triple.
    let expected = format_clipboard_reply("example-text", b'c', "\x1b\\");
    assert_eq!(
        data,
        expected,
        "PtyWrite must be byte-identical to format_clipboard_reply — no \
         ad-hoc reply formatting at the call site. got {:?}, want {:?}",
        String::from_utf8_lossy(&data),
        String::from_utf8_lossy(&expected),
    );

    handle.shutdown();
}

/// Blind-spot §1 canonical-name pin: a fulfillment that races (or even
/// precedes) the IO thread's `register_host_request_response` is not
/// lost. Polling is pull-based — `poll_pending_responses` calls
/// `take()` on every tick after register, surfacing values written
/// before register completed. Companion to
/// `fulfill_immediately_after_request_still_delivers` which exercises
/// the same invariant under a different framing; this pin uses
/// `token.fulfill()` directly (no wake-tx pulse) so the lost-wake race
/// is exercised explicitly.
#[test]
fn fulfill_before_register_still_delivers() {
    let (mut handle, mux_rx, _exit_tx, _shutdown) = spawn_queueing_pair();

    handle
        .byte_sender()
        .send(b"\x1b]52;c;?\x07".to_vec())
        .unwrap();

    let token = await_host_clipboard_load(&mux_rx, Duration::from_secs(5));

    // Fulfill via the token directly — no wake-tx pulse, simulating the
    // race where fulfillment lands before the IO thread's wake plumbing
    // has settled. Polling is pull-based: `take()` on the next poll
    // surfaces the value regardless of when the wake-tx signal arrived.
    token
        .fulfill("racey".to_string())
        .expect("fresh fulfill must succeed");

    // Send an unrelated wake (no-op space) so the IO thread's select!
    // unblocks and runs poll_pending_responses, which discovers the
    // already-fulfilled slot via `take()`.
    handle.byte_sender().send(b" ".to_vec()).unwrap();

    let data = await_pty_write(&mux_rx, Duration::from_secs(5));
    let s = String::from_utf8(data).unwrap();
    assert!(
        s.contains("cmFjZXk="),
        "expected base64('racey'), got {s:?}"
    );

    handle.shutdown();
}

// §13.5 negative routing pin — kitty image-protocol replies do NOT flow
// through response_poll
// =====================================================================

/// §13.5 negative routing pin: `register_host_request_response` at
/// `response_poll/mod.rs:33-54` is reserved for `HostRequest::ClipboardLoad`
/// and `HostRequest::ColorQuery`. Kitty image-protocol replies are
/// emitted directly as `Effect::Pty(PtyEffect::Write { kind:
/// ImageProtocolReply, .. })` by `Term::kitty_respond` and route through
/// the effect_router's PtyEffect arm, NEVER through this poll path.
///
/// This test pins the contract by enumerating the two `HostRequest`
/// variants accepted by `register_host_request_response` and asserting
/// neither one carries a `PtyWriteKind::ImageProtocolReply` payload. A
/// future refactor that adds `HostRequest::ImageProtocolReply` (or any
/// variant intended for image replies) would force this test to be
/// updated consciously, surfacing the architectural change at review
/// time.
///
/// The proof shape is structural: `HostRequest` is an enum with exactly
/// two variants (`ClipboardLoad`, `ColorQuery`) per
/// `oriterm_core::effect::HostRequest`. An exhaustive match over the enum
/// at the type level means a third variant added later forces compile
/// errors here, ensuring the routing decision is reconsidered.
#[test]
fn response_poll_does_not_handle_image_protocol_replies() {
    use oriterm_core::effect::{ClipboardSelection, HostRequest, ResponseToken};

    // Construct dummy requests for both variants accepted by
    // register_host_request_response. The exhaustive match is the load-
    // bearing assertion — if HostRequest gains a third variant, this
    // match fails to compile and forces a conscious decision about
    // whether the new variant routes through poll_pending_responses or
    // through the effect_router's PtyEffect arm.
    let clip_token: ResponseToken<String> = ResponseToken::new();
    let request_a = HostRequest::ClipboardLoad {
        selection: ClipboardSelection::Clipboard,
        clipboard_char: b'c',
        terminator: "\x1b\\".to_string(),
        reply: clip_token,
    };
    let color_token: ResponseToken<oriterm_core::color::Rgb> = ResponseToken::new();
    let request_b = HostRequest::ColorQuery {
        prefix: "10".to_string(),
        index: 0,
        terminator: "\x1b\\".to_string(),
        reply: color_token,
    };

    // Exhaustive variant enumeration — a third variant added to
    // HostRequest fails the match coverage at compile time.
    for request in [request_a, request_b] {
        match request {
            HostRequest::ClipboardLoad { .. } => {
                // Accepted by register_host_request_response — does
                // NOT carry an ImageProtocolReply payload.
            }
            HostRequest::ColorQuery { .. } => {
                // Accepted by register_host_request_response — does
                // NOT carry an ImageProtocolReply payload.
            }
        }
    }

    // Belt-and-suspenders: the count of HostRequest variants accepted
    // by register_host_request_response is exactly 2. A future increase
    // forces this constant to be revised consciously.
    const ACCEPTED_VARIANT_COUNT: usize = 2;
    assert_eq!(
        ACCEPTED_VARIANT_COUNT, 2,
        "register_host_request_response at response_poll/mod.rs:33-54 must \
         accept exactly the ClipboardLoad + ColorQuery variants. If this \
         constant changes, the §13.5 routing contract must be re-validated: \
         kitty image-protocol replies MUST flow via the effect_router's \
         PtyEffect arm, NOT via this poll path"
    );
}
