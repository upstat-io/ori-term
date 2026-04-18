//! Critical router coverage for effect-cutover 01.1.
//!
//! These tests pin the load-bearing invariants of the effect router:
//! byte-exact PTY writes, wakeup contract parity, fulfillment
//! round-trip, cancellation detection, and intra-batch
//! `ClearPendingNotifications` collapse.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use crossbeam_channel::Receiver;

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{
    ClipboardSelection, Effect, HostEffect, HostRequest, NotificationSource, PollResult,
    PresentationEffect, PtyEffect, PtyWriteKind, QueueingEffectSink, ResponseToken,
    SyncAbortReason, UiEffect,
};
use oriterm_core::{Term, TermMode, Theme};

use super::super::{PaneIoThread, SnapshotDoubleBuffer};
use crate::PaneId;
use crate::mux_event::MuxEvent;
use crate::pty::spawn::ExitStatus;

/// Build a `PaneIoThread<QueueingEffectSink>` with a counting wakeup and
/// captured mux output so router semantics can be asserted directly.
fn make_router_harness() -> (
    PaneIoThread<QueueingEffectSink>,
    mpsc::Receiver<MuxEvent>,
    Arc<AtomicU64>,
) {
    let wakeup_count = Arc::new(AtomicU64::new(0));
    let wake_clone = Arc::clone(&wakeup_count);
    let (mux_tx, mux_rx) = mpsc::channel::<MuxEvent>();
    let (_cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<super::super::PaneIoCommand>();
    let (_byte_tx, byte_rx) = crossbeam_channel::unbounded::<Vec<u8>>();
    let (_exit_tx, child_exit_rx): (_, Receiver<ExitStatus>) =
        crossbeam_channel::bounded::<ExitStatus>(1);
    let (_wake_tx, response_wake_rx) = crossbeam_channel::bounded::<()>(1);
    // Leak the auxiliary tx ends so receivers stay open for the lifetime
    // of the test — prevents spurious EOF from firing select! arms.
    std::mem::forget(_cmd_tx);
    std::mem::forget(_byte_tx);
    std::mem::forget(_exit_tx);
    std::mem::forget(_wake_tx);

    let term = Term::new(24, 80, 1000, Theme::default(), QueueingEffectSink::new());
    let thread = PaneIoThread {
        terminal: term,
        pane_id: PaneId::from_raw(7),
        mux_tx,
        child_exit_rx,
        pending_child_exit: None,
        response_wake_rx,
        cmd_rx,
        byte_rx,
        shutdown: Arc::new(AtomicBool::new(false)),
        wakeup: Arc::new(move || {
            wake_clone.fetch_add(1, Ordering::Release);
        }),
        processor: vte::ansi::Processor::new(),
        raw_parser: vte::Parser::new(),
        mode_cache: Arc::new(AtomicU64::new(TermMode::default().bits())),
        double_buffer: SnapshotDoubleBuffer::new(),
        snapshot_buf: Default::default(),
        grid_dirty: Arc::new(AtomicBool::new(false)),
        pty_control: None,
        adopted_signal: None,
        last_pty_size: (24u32 << 16) | 80u32,
        search: None,
        selection_dirty: Arc::new(AtomicBool::new(false)),
        pending_responses: Vec::new(),
        effects_buf: Vec::new(),
    };
    (thread, mux_rx, wakeup_count)
}

/// Blind-spot §15: `MuxEvent::PtyWrite::data` is `Vec<u8>` — non-UTF-8
/// bytes survive the effect→MuxEvent boundary byte-exact.
#[test]
fn pty_write_preserves_non_utf8_bytes() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    let bytes = vec![0x90, 0xFF, 0x00, 0x80, 0xC0];
    t.terminal.effect_sink().push(Effect::Pty(PtyEffect::Write {
        bytes: bytes.clone(),
        kind: PtyWriteKind::Other,
    }));
    t.drain_effects_into_mux_events();

    match mux_rx.recv().expect("expected PtyWrite event") {
        MuxEvent::PtyWrite { data, pane_id } => {
            assert_eq!(pane_id, PaneId::from_raw(7));
            assert_eq!(
                data, bytes,
                "bytes must survive byte-exact — no lossy UTF-8 downgrade"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Blind-spot §15 companion: every `PtyWriteKind` preserves bytes.
#[test]
fn pty_write_all_kinds_preserve_bytes() {
    let kinds = [
        PtyWriteKind::DeviceAttribute,
        PtyWriteKind::CursorReport,
        PtyWriteKind::DeviceStatus,
        PtyWriteKind::ModeReport,
        PtyWriteKind::StatusString,
        PtyWriteKind::ImageProtocolReply,
        PtyWriteKind::MouseEvent,
        PtyWriteKind::KeyboardEvent,
        PtyWriteKind::FocusEvent,
        PtyWriteKind::Other,
    ];
    for kind in kinds {
        let (mut t, mux_rx, _wake) = make_router_harness();
        let bytes = vec![0x90, 0xFF, 0xC0];
        t.terminal.effect_sink().push(Effect::Pty(PtyEffect::Write {
            bytes: bytes.clone(),
            kind,
        }));
        t.drain_effects_into_mux_events();
        match mux_rx.recv_timeout(Duration::from_millis(100)).unwrap() {
            MuxEvent::PtyWrite { data, .. } => assert_eq!(data, bytes, "kind={kind:?}"),
            other => panic!("kind={kind:?}: expected PtyWrite, got {other:?}"),
        }
    }
}

/// Blind-spot §16: every `MuxEvent` emission also fires the wakeup
/// callback so the winit loop observes the new event.
#[test]
fn router_fires_wakeup_after_every_mux_event() {
    let (mut t, mux_rx, wake) = make_router_harness();
    for effect in [
        Effect::Host(HostEffect::Bell),
        Effect::Host(HostEffect::TitleSet {
            value: Some("x".into()),
        }),
        Effect::Host(HostEffect::CwdSet { cwd: "/y".into() }),
        Effect::Host(HostEffect::ClipboardStore {
            selection: ClipboardSelection::Clipboard,
            data: "z".into(),
        }),
        Effect::Host(HostEffect::CommandComplete {
            duration: Duration::from_secs(1),
        }),
    ] {
        t.terminal.effect_sink().push(effect);
    }
    t.drain_effects_into_mux_events();

    assert_eq!(
        wake.load(Ordering::Acquire),
        5,
        "wakeup must fire once per MuxEvent"
    );
    assert_eq!(
        mux_rx.iter().take(5).count(),
        5,
        "five MuxEvents must reach the receiver"
    );
}

/// Blind-spot §16 companion: UI-only effects fire wakeup without
/// emitting a `MuxEvent`.
#[test]
fn router_fires_wakeup_for_ui_effects_without_mux_event() {
    let (mut t, mux_rx, wake) = make_router_harness();
    t.terminal
        .effect_sink()
        .push(Effect::Ui(UiEffect::CursorBlinkChanged { enabled: true }));
    t.terminal
        .effect_sink()
        .push(Effect::Ui(UiEffect::MouseCursorDirty));
    t.drain_effects_into_mux_events();
    assert_eq!(wake.load(Ordering::Acquire), 2, "two wakeups expected");
    assert!(
        mux_rx.try_recv().is_err(),
        "UI-only effects do not produce MuxEvents"
    );
}

/// Round-trip: `HostRequest` routes to `MuxEvent::HostClipboardLoad`
/// AND registers a pending response; fulfilling the token produces the
/// base64-encoded OSC 52 reply as a `MuxEvent::PtyWrite`.
#[test]
fn host_clipboard_load_roundtrip_emits_pty_write() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    let token: ResponseToken<String> = ResponseToken::new();
    t.terminal
        .effect_sink()
        .push(Effect::HostRequest(HostRequest::ClipboardLoad {
            selection: ClipboardSelection::Clipboard,
            clipboard_char: b'c',
            terminator: "\x1b\\".into(),
            reply: token.clone(),
        }));
    t.drain_effects_into_mux_events();

    // First event: HostClipboardLoad notification to main thread.
    match mux_rx.recv().expect("HostClipboardLoad expected") {
        MuxEvent::HostClipboardLoad { selection, .. } => {
            assert_eq!(selection, ClipboardSelection::Clipboard);
        }
        other => panic!("expected HostClipboardLoad, got {other:?}"),
    }

    // Pending response was registered.
    assert_eq!(t.pending_responses.len(), 1);

    // Main thread fulfills the token.
    token
        .fulfill("hello".into())
        .expect("fresh fulfill succeeds");

    // IO thread drains the poll and emits PtyWrite.
    t.poll_pending_responses();
    t.drain_effects_into_mux_events();

    match mux_rx.recv().expect("PtyWrite expected") {
        MuxEvent::PtyWrite { data, .. } => {
            let s = String::from_utf8(data).expect("ASCII");
            assert!(
                s.contains("aGVsbG8="),
                "base64('hello') should appear in OSC 52 reply: got {s:?}"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
    assert!(t.pending_responses.is_empty(), "fulfilled entry removed");
}

/// Blind-spot §13: consumer drops handle without fulfilling → pending
/// entry removed without emitting a PTY reply.
#[test]
fn pending_response_cancelled_when_consumer_handle_dropped() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    let token: ResponseToken<String> = ResponseToken::new();
    t.terminal
        .effect_sink()
        .push(Effect::HostRequest(HostRequest::ClipboardLoad {
            selection: ClipboardSelection::Clipboard,
            clipboard_char: b'c',
            terminator: "\x1b\\".into(),
            reply: token.clone(),
        }));
    t.drain_effects_into_mux_events();
    // Drain the HostClipboardLoad MuxEvent — it carried an extra Arc
    // clone which we want to drop so only the test's `token` handle and
    // the PendingResponse's capture remain.
    let _ = mux_rx.recv().unwrap();
    assert_eq!(t.pending_responses.len(), 1);

    // Drop the consumer's only remaining handle.
    drop(token);

    // Two poll ticks (first observes strong_count transition).
    t.poll_pending_responses();
    t.poll_pending_responses();

    assert!(
        t.pending_responses.is_empty(),
        "cancelled entry must be removed without PTY reply"
    );
    assert!(
        mux_rx.try_recv().is_err(),
        "no PtyWrite emitted for cancelled response"
    );
}

/// Ordering: effects drain in push order end-to-end.
#[test]
fn drain_preserves_push_order_end_to_end() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::Bell));
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::TitleSet {
            value: Some("A".into()),
        }));
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::CwdSet { cwd: "/x".into() }));
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::Bell));
    t.drain_effects_into_mux_events();

    let events: Vec<_> = std::iter::from_fn(|| mux_rx.try_recv().ok()).collect();
    assert_eq!(events.len(), 4);
    assert!(matches!(&events[0], MuxEvent::PaneBell(_)));
    assert!(matches!(&events[1], MuxEvent::PaneTitleChanged { .. }));
    assert!(matches!(&events[2], MuxEvent::PaneCwdChanged { .. }));
    assert!(matches!(&events[3], MuxEvent::PaneBell(_)));
}

/// Intra-batch collapse: `ClearPendingNotifications` discards
/// preceding `DesktopNotification` effects in the SAME drain.
#[test]
fn clear_pending_notifications_collapses_preceding_in_batch() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::DesktopNotification {
            source: NotificationSource::Osc9,
            title: "A".into(),
            body: "a".into(),
        }));
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::DesktopNotification {
            source: NotificationSource::Osc99,
            title: "B".into(),
            body: "b".into(),
        }));
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::ClearPendingNotifications));
    t.drain_effects_into_mux_events();

    assert!(
        mux_rx.try_recv().is_err(),
        "intra-batch Clear must suppress preceding DesktopNotifications"
    );
}

/// `HostEffect::ChildExit` routes to `MuxEvent::PaneExited` with the
/// code intact — the router is the only post-01.3 production path.
#[test]
fn child_exit_effect_routes_to_pane_exited() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::ChildExit { code: 42 }));
    t.drain_effects_into_mux_events();
    match mux_rx.recv().expect("PaneExited expected") {
        MuxEvent::PaneExited { exit_code, .. } => assert_eq!(exit_code, 42),
        other => panic!("expected PaneExited, got {other:?}"),
    }
}

/// `Presentation` effects are logged, not queued.
#[test]
fn presentation_effects_do_not_produce_mux_events() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.terminal
        .effect_sink()
        .push(Effect::Presentation(PresentationEffect::Begin));
    t.terminal
        .effect_sink()
        .push(Effect::Presentation(PresentationEffect::Commit {
            snapshot_seqno: 1,
        }));
    t.terminal
        .effect_sink()
        .push(Effect::Presentation(PresentationEffect::Abort {
            reason: SyncAbortReason::Timeout,
        }));
    t.drain_effects_into_mux_events();
    assert!(mux_rx.try_recv().is_err());
}

/// `effects_buf` retains capacity across drain cycles (alloc regression).
#[test]
fn effects_buf_retains_capacity_across_drains() {
    let (mut t, _mux_rx, _wake) = make_router_harness();
    for _ in 0..8 {
        t.terminal
            .effect_sink()
            .push(Effect::Host(HostEffect::Bell));
    }
    t.drain_effects_into_mux_events();
    let cap_after_first = t.effects_buf.capacity();
    assert!(cap_after_first >= 8);

    for _ in 0..4 {
        t.terminal
            .effect_sink()
            .push(Effect::Host(HostEffect::Bell));
    }
    t.drain_effects_into_mux_events();
    let cap_after_second = t.effects_buf.capacity();
    assert!(
        cap_after_second >= cap_after_first,
        "capacity must not shrink between drains"
    );
}

/// Blind-spot §13 companion: token fulfilled before drop → normal
/// completion, not cancellation.
#[test]
fn pending_response_not_cancelled_if_fulfilled_before_drop() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    let token: ResponseToken<String> = ResponseToken::new();
    t.terminal
        .effect_sink()
        .push(Effect::HostRequest(HostRequest::ClipboardLoad {
            selection: ClipboardSelection::Clipboard,
            clipboard_char: b'c',
            terminator: "\x1b\\".into(),
            reply: token.clone(),
        }));
    t.drain_effects_into_mux_events();
    // Drain the HostClipboardLoad carrier.
    let _ = mux_rx.recv().unwrap();

    // Fulfill, then drop.
    token.fulfill("ok".into()).unwrap();
    drop(token);

    t.poll_pending_responses();
    t.drain_effects_into_mux_events();

    // Verify PtyWrite IS emitted — this is a normal completion, not a
    // cancellation (the slot had a value before strong_count dropped).
    let mut saw_pty_write = false;
    while let Ok(event) = mux_rx.try_recv() {
        if matches!(event, MuxEvent::PtyWrite { .. }) {
            saw_pty_write = true;
        }
    }
    assert!(
        saw_pty_write,
        "fulfilled-then-dropped must yield PtyWrite (normal completion)"
    );
    assert!(t.pending_responses.is_empty());
}

/// `PollResult::Cancelled` variant is reachable (regression pin for
/// exhaustive `response_poll::poll_pending_responses` match).
#[test]
fn poll_result_variants_all_constructible() {
    let bell = Effect::Host(HostEffect::Bell);
    let _ready = PollResult::Ready(bell);
    let _pending = PollResult::Pending;
    let _cancelled = PollResult::Cancelled;
}
