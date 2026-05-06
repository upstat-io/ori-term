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

use super::super::handle::PENDING_RESIZE_NONE;

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
    // Effect-router harness keeps unbounded `cmd_tx` / `byte_tx` and
    // dummy wake / exit channels so it tests effect-routing logic
    // without coupling to 's bounded-cmd_tx / atomic-resize
    // wiring (per §05 Step 5 test-harness exception). Leak the
    // auxiliary tx ends so receivers stay open for the lifetime of
    // the test — prevents spurious EOF from firing select! arms.
    let (_wake_tx, io_wake_rx) = crossbeam_channel::bounded::<()>(1);
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
        io_wake_rx,
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
        last_animation_deadline: None,
        pending_resize: Arc::new(AtomicU64::new(PENDING_RESIZE_NONE)),
        shrink_call_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        start_barrier: None,
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
        PtyWriteKind::ChecksumReport,
        PtyWriteKind::GraphicsAttributeReport,
        PtyWriteKind::Answerback,
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
/// PRECEDING `DesktopNotification` effects in the SAME drain, AND
/// emits the dedicated `ClearPendingDesktopNotifications` MuxEvent so
/// downstream staging buffers can purge cross-batch leftovers.
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

    let events: Vec<_> = std::iter::from_fn(|| mux_rx.try_recv().ok()).collect();
    assert_eq!(
        events.len(),
        1,
        "preceding notifications must be suppressed"
    );
    assert!(
        matches!(&events[0], MuxEvent::ClearPendingDesktopNotifications(_)),
        "the clear marker MUST surface as ClearPendingDesktopNotifications so \
 downstream staging buffers can purge cross-batch entries; got {:?}",
        events[0]
    );
}

/// Blind-spot §7 contract pin (corrected post-TPR-01-F1):
/// `ClearPendingNotifications` collapses ONLY preceding
/// `DesktopNotification` effects in the same batch — notifications
/// that follow the clear marker survive. Pinned by the canonical
/// sequence from the plan body: `[Notif1, Notif2, Clear, Notif3]` →
/// `[Clear, Notif3]`.
#[test]
fn clear_pending_notifications_collapses_preceding_only() {
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
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::DesktopNotification {
            source: NotificationSource::Osc777,
            title: "C".into(),
            body: "c".into(),
        }));
    t.drain_effects_into_mux_events();

    let events: Vec<_> = std::iter::from_fn(|| mux_rx.try_recv().ok()).collect();
    assert_eq!(
        events.len(),
        2,
        "expected ClearPendingDesktopNotifications + the post-clear notification; got {events:?}"
    );
    assert!(
        matches!(&events[0], MuxEvent::ClearPendingDesktopNotifications(_)),
        "first event must be the clear; got {:?}",
        events[0]
    );
    match &events[1] {
        MuxEvent::DesktopNotification {
            source: NotificationSource::Osc777,
            title,
            ..
        } if title == "C" => {}
        other => {
            panic!("second event must be the post-clear DesktopNotification(C); got {other:?}")
        }
    }
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

/// Regression: BUG-11-049 — `PollResult` variants must stay exhaustive in
/// `response_poll::poll_pending_responses`. An added variant without a
/// consumer-match arm would silently fall through the catch-all.
/// See: bug-tracker/plans/completed/BUG-11-049/00-overview.md
#[test]
fn poll_result_variants_all_constructible() {
    let bell = Effect::Host(HostEffect::Bell);
    let ready = PollResult::Ready(bell);
    let pending = PollResult::Pending;
    let cancelled = PollResult::Cancelled;

    assert!(matches!(ready, PollResult::Ready(_)));
    assert!(matches!(pending, PollResult::Pending));
    assert!(matches!(cancelled, PollResult::Cancelled));

    // All three discriminants must be distinct — a 4th variant would
    // cause the consumer match in poll_pending_responses to go stale.
    let dis = std::mem::discriminant;
    assert_ne!(dis(&ready), dis(&pending));
    assert_ne!(dis(&pending), dis(&cancelled));
    assert_ne!(dis(&cancelled), dis(&ready));
}

/// Blind-spot §5: drain happens INSIDE `handle_bytes` (per chunk), not
/// at the end of `handle_bytes_chunked`. Pinned architecturally —
/// `mod.rs` MUST contain the drain call inside the per-chunk function so
/// a 1 MB forwarded read does not accumulate ~16 chunks of effects.
#[test]
fn drain_call_lives_inside_handle_bytes_per_chunk() {
    let source = include_str!("../mod.rs");

    // The drain call must appear inside `fn handle_bytes` (the per-chunk
    // function), not only inside `fn handle_bytes_chunked` (the outer
    // loop).
    let handle_bytes_start = source
        .find("fn handle_bytes(&mut self, bytes: &[u8])")
        .expect("fn handle_bytes signature must exist verbatim");

    // Find the matching closing brace by matching braces.
    let after_sig = &source[handle_bytes_start..];
    let body_open = after_sig.find('{').expect("handle_bytes must have body");
    let mut depth: usize = 0;
    let mut end_offset = 0;
    for (i, c) in after_sig[body_open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end_offset = body_open + i;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &after_sig[body_open..=end_offset];
    assert!(
        body.contains("self.drain_effects_into_mux_events()"),
        "drain MUST be called inside handle_bytes per blind-spot §5; \
 body did not contain the drain call:\n{body}"
    );
}

/// Blind-spot §16: every `MuxEvent` emission flows through
/// `send_mux_event` (the wakeup-pairing helper), not direct
/// `self.mux_tx.send(..)`. Enforced via grep against the router source,
/// excluding doc/line comments.
#[test]
fn router_routes_mux_events_only_via_send_mux_event() {
    let source = include_str!("mod.rs");
    // Count non-comment lines that contain the direct send pattern.
    // `///` doc lines and `//` line comments don't count — they describe
    // the contract, they don't bypass it.
    let occurrences = source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("///") && !trimmed.starts_with("//")
        })
        .filter(|line| line.contains("self.mux_tx.send("))
        .count();
    assert_eq!(
        occurrences, 1,
        "router must route MuxEvents only through send_mux_event; \
 unexpected `self.mux_tx.send(..)` site bypasses the wakeup pair"
    );
}

/// Blind-spot §18: every staging-buffer site that carries
/// `MuxNotification::HostClipboardLoad` / `HostColorQuery` MUST move
/// the variant, not clone it — clones defeat `Arc::strong_count`-based
/// cancellation detection in `PendingResponse::poll`.
#[test]
fn no_cloned_host_clipboard_load_notification_in_staging() {
    // Files that buffer or forward `MuxNotification` between threads /
    // processes. Each MUST use move semantics (`Vec::drain`,
    // `mem::replace`, match-move) for HostClipboardLoad / HostColorQuery.
    let staging_files: &[(&str, &str)] = &[
        (
            "oriterm_mux/src/backend/client/mod.rs",
            include_str!("../../../backend/client/mod.rs"),
        ),
        (
            "oriterm_mux/src/backend/client/notification.rs",
            include_str!("../../../backend/client/notification.rs"),
        ),
        (
            "oriterm_mux/src/server/mod.rs",
            include_str!("../../../server/mod.rs"),
        ),
    ];
    for (path, body) in staging_files {
        for forbidden in [
            "MuxNotification::HostClipboardLoad",
            "MuxNotification::HostColorQuery",
        ] {
            for line in body.lines() {
                if line.contains(forbidden) && line.contains(".clone()") {
                    panic!(
                        "{path}: forbidden clone of {forbidden} detected — \
 move semantics required (Vec::drain / mem::replace / match-move):\n {line}"
                    );
                }
            }
        }
    }
}

/// preservation pin (§03 test 19): after the router-arm
/// split removes `HostEffect::VisualBell` and routes
/// `HostEffect::AudioRequest` to a dedicated log-only arm,
/// `AudioRequest` MUST NOT emit any `MuxEvent` — stays open
/// and the audio-pipeline producer-side gap remains tracked.
///
/// Regression: split combined VisualBell|AudioRequest|PrintRequest arm
/// See: bug-tracker/plans//00-overview.md
#[test]
fn audio_request_remains_log_only() {
    use oriterm_core::effect::{AudioKind, AudioRequest as AudioReq};

    let (mut t, mux_rx, _wake) = make_router_harness();
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::AudioRequest(AudioReq {
            kind: AudioKind::Tone,
            volume: 4,
            duration_ms: 100,
            note: 12,
        })));
    t.drain_effects_into_mux_events();

    assert!(
        mux_rx.try_recv().is_err(),
        "AudioRequest must not emit a MuxEvent \
 the router's split arm preserves the pre-fix log-only behavior"
    );
}

/// preservation pin (§03 test 20): after the router-arm
/// split, `HostEffect::PrintRequest` MUST NOT emit any `MuxEvent` —
/// stays open.
///
/// Regression: split combined VisualBell|AudioRequest|PrintRequest arm
/// See: bug-tracker/plans//00-overview.md
#[test]
fn print_request_remains_log_only() {
    use oriterm_core::effect::{PrintKind, PrintRequest as PrintReq};

    let (mut t, mux_rx, _wake) = make_router_harness();
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::PrintRequest(PrintReq {
            kind: PrintKind::Screen,
            data: b"hello".to_vec(),
        })));
    t.drain_effects_into_mux_events();

    assert!(
        mux_rx.try_recv().is_err(),
        "PrintRequest must not emit a MuxEvent \
 the router's split arm preserves the pre-fix log-only behavior"
    );
}

/// Effect-cutover §01.N regression guard: `ClearPendingNotifications` does
/// NOT retroactively collapse `DesktopNotification` effects that
/// landed in an EARLIER drain batch. Cross-batch staging-buffer
/// purging is the responsibility of `mux_pump`'s
/// `purge_pending_desktop_notifications` (and parallel daemon-side
/// staging) — the router itself is intra-batch only.
#[test]
fn clear_pending_notifications_does_not_retro_collapse_across_drains() {
    let (mut t, mux_rx, _wake) = make_router_harness();

    // Batch 1: emit a notification, drain.
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::DesktopNotification {
            source: NotificationSource::Osc9,
            title: "BatchOne".into(),
            body: "first".into(),
        }));
    t.drain_effects_into_mux_events();

    // Batch 1 produced the notification — confirm it reached mux_rx.
    let mut saw_notif = false;
    while let Ok(event) = mux_rx.try_recv() {
        if let MuxEvent::DesktopNotification { title, .. } = &event {
            if title == "BatchOne" {
                saw_notif = true;
            }
        }
    }
    assert!(
        saw_notif,
        "batch 1 DesktopNotification must reach mux_rx before any clear"
    );

    // Batch 2: emit ONLY the clear marker.
    t.terminal
        .effect_sink()
        .push(Effect::Host(HostEffect::ClearPendingNotifications));
    t.drain_effects_into_mux_events();

    // The router emits the cross-batch ClearPendingDesktopNotifications
    // MuxEvent so downstream staging can purge — but it does NOT reach
    // back into batch 1's already-emitted notification (which is
    // irretrievable from the router side; that's mux_pump's job via
    // purge_pending_desktop_notifications).
    let mut saw_clear = false;
    while let Ok(event) = mux_rx.try_recv() {
        if matches!(event, MuxEvent::ClearPendingDesktopNotifications(_)) {
            saw_clear = true;
        }
    }
    assert!(
        saw_clear,
        "batch 2 ClearPendingNotifications must surface as a MuxEvent for downstream staging purge"
    );
}

// --- — DA/DSR/CSI 18t/DECRQM byte-parse → MuxEvent round-trip ---
//
// These tests pin the byte-parse → effect-emit → router → MuxEvent leg for
// every response kind handled in oriterm_core/src/term/handler/status.rs.
// Each test calls handle_bytes() with the canonical query bytes and asserts
// mux_rx receives MuxEvent::PtyWrite with the byte-exact response.
// See bug-tracker/plans/completed/.

/// Regression: DA1 (CSI c) emits VT420-class device attributes.
/// See: bug-tracker/plans/completed/section-03-tdd-matrix.md
#[test]
fn da1_byte_parse_emits_pty_write_response() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[c");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DA1 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(data, b"\x1b[?64;6;4c", "DA1 response bytes mismatch");
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: DA3 (CSI = c) emits DCS unit-ID response.
#[test]
fn da3_byte_parse_emits_pty_write_response() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[=c");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DA3 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"\x1bP!|00000000\x1b\\",
                "DA3 response bytes mismatch"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: DSR 5 (CSI 5 n) emits terminal-OK status.
#[test]
fn dsr5_byte_parse_emits_pty_write_response() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[5n");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DSR 5 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(data, b"\x1b[0n", "DSR 5 response bytes mismatch");
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: DSR 6 (CSI 6 n) at default cursor reports (1,1).
#[test]
fn dsr6_byte_parse_at_default_cursor_emits_position_one_one() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[6n");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DSR 6 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"\x1b[1;1R",
                "DSR 6 default-cursor response bytes mismatch"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: CSI 18t at default 24x80 grid reports `\x1b[8;24;80t`.
#[test]
fn csi_18t_byte_parse_at_default_grid_emits_size_24_80() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[18t");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for CSI 18t response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"\x1b[8;24;80t",
                "CSI 18t default-grid response bytes mismatch"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: DA2 (CSI > c) emits versioned response prefix `\x1b[>0;` + version + `;1c`.
#[test]
fn da2_byte_parse_emits_pty_write_response_with_version() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[>c");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DA2 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert!(
                data.starts_with(b"\x1b[>0;"),
                "DA2 response must start with \\x1b[>0;, got {data:?}"
            );
            assert!(
                data.ends_with(b";1c"),
                "DA2 response must end with;1c, got {data:?}"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: DECRQM SET (mode 25 cursor visible) reports value 1.
#[test]
fn decrqm_set_byte_parse_emits_value_one() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[?25$p");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DECRQM mode 25 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"\x1b[?25;1$y",
                "DECRQM mode 25 default response bytes mismatch"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: DECRQM RESET (mode 1049 alt screen off by default) reports value 2.
#[test]
fn decrqm_reset_byte_parse_emits_value_two() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[?1049$p");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DECRQM mode 1049 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"\x1b[?1049;2$y",
                "DECRQM mode 1049 default response bytes mismatch"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: XTVERSION (CSI > q) emits DCS terminal-version response.
///
/// Joins the cluster covering every response kind handled in
/// `oriterm_core/src/term/handler/status.rs`. XTVERSION's reply pipeline:
/// raw bytes → vte CSI dispatch arm `('q', [b'>'])` (Ps=0 gate) →
/// `Handler::xtversion()` → `Term::status_xtversion()` → `effect_sink` →
/// `drain_effects_into_mux_events` → `MuxEvent::PtyWrite`.
///
/// See: bug-tracker/plans//section-03-tdd-matrix.md
#[test]
fn xtversion_byte_parse_emits_pty_write_response() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[>q");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for XTVERSION response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            let s = String::from_utf8_lossy(&data);
            assert!(
                s.starts_with("\x1bP>|oriterm("),
                "XTVERSION reply must begin with DCS > | oriterm( prefix, got: {s}"
            );
            assert!(
                s.ends_with("\x1b\\"),
                "XTVERSION reply must end with ST, got: {s}"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: split-chunk XTVERSION still emits exactly one response.
///
/// Pins that the parser handles partial-byte input correctly: feeding
/// `\x1b[>` then `q` as separate `handle_bytes` calls must produce the
/// same single PtyWrite as the single-chunk case.
#[test]
fn xtversion_split_chunk_byte_parse_emits_pty_write_response() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[>");
    t.handle_bytes(b"q");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite after split-chunk XTVERSION parse");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert!(
                String::from_utf8_lossy(&data).contains("oriterm"),
                "XTVERSION split-chunk reply must contain 'oriterm'"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
    // Regression guard: handle_bytes() is synchronous — any second event would
    // already be queued by the time we reach this assertion. try_recv() is
    // wall-clock-free §Wall-Clock-Free Testing
    // (no `recv_timeout` deadline; deterministic against scheduler jitter).
    assert!(
        mux_rx.try_recv().is_err(),
        "XTVERSION must produce exactly one PtyWrite even on split-chunk input"
    );
}

/// Regression: DECRQM unknown mode reports value 0 (unrecognized).
#[test]
fn decrqm_unknown_mode_emits_value_zero() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.handle_bytes(b"\x1b[?9999$p");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for DECRQM unknown-mode response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"\x1b[?9999;0$y",
                "DECRQM unknown-mode response bytes mismatch"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: XTSMGRAPHICS query had no reply path.
///
/// Pins the full pipeline: raw bytes → vte parser → CSI dispatch arm
/// `('S', [b'?'])` → `Handler::graphics_attribute` → `Term`'s
/// `status_graphics_attribute` helper → `effect_sink` →
/// `drain_effects_into_mux_events` → `MuxEvent::PtyWrite`.
///
/// Without the dispatch arm + Handler method + helper + sync points,
/// no PtyWrite event reaches the mux.
#[test]
fn xtsmgraphics_byte_parse_emits_pty_write_response() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    // notcurses startup repro: XTSMGRAPHICS Pi=1 Pa=1 (read color registers).
    t.handle_bytes(b"\x1b[?1;1;0S");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for XTSMGRAPHICS Pi=1 Pa=1 response");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"\x1b[?1;0;256S",
                "XTSMGRAPHICS Pi=1 Pa=1 response bytes mismatch"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}

/// Regression: BUG-08-006 — ENQ (`0x05`) byte-parse through router emits
/// `MuxEvent::PtyWrite` carrying the configured answerback bytes.
/// Distinguishes the kinds-array test (which proves preservation given an
/// Answerback effect) from the real-byte-parse path (which proves the
/// dispatch chain produces the effect when ENQ byte arrives at the mux).
/// See: bug-tracker/plans/BUG-08-006/section-03-tdd-matrix.md
#[test]
fn enq_byte_through_router_emits_pty_write_with_answerback_bytes() {
    let (mut t, mux_rx, _wake) = make_router_harness();
    t.terminal.set_answerback(b"oriterm-X".to_vec());
    t.handle_bytes(b"\x05");
    let event = mux_rx
        .recv_timeout(Duration::from_millis(100))
        .expect("expected MuxEvent::PtyWrite for ENQ Answerback");
    match event {
        MuxEvent::PtyWrite { data, .. } => {
            assert_eq!(
                data, b"oriterm-X",
                "ENQ must emit configured answerback bytes through the router"
            );
        }
        other => panic!("expected PtyWrite, got {other:?}"),
    }
}
