//! Basic constructibility tests for all Effect variants.

use super::*;

#[test]
fn pty_write_constructs() {
    let effect = Effect::Pty(PtyEffect::Write {
        bytes: b"\x1b[?1;2c".to_vec(),
        kind: PtyWriteKind::DeviceAttribute,
    });
    assert!(matches!(effect, Effect::Pty(PtyEffect::Write { .. })));
}

#[test]
fn host_bell_constructs() {
    let effect = Effect::Host(HostEffect::Bell);
    assert!(matches!(effect, Effect::Host(HostEffect::Bell)));
}

#[test]
fn host_desktop_notification_constructs() {
    let effect = Effect::Host(HostEffect::DesktopNotification {
        source: NotificationSource::Osc99,
        title: "test".into(),
        body: "body".into(),
    });
    assert!(matches!(
        effect,
        Effect::Host(HostEffect::DesktopNotification { .. })
    ));
}

#[test]
fn host_title_set_constructs() {
    let set = Effect::Host(HostEffect::TitleSet {
        value: Some("title".into()),
    });
    let reset = Effect::Host(HostEffect::TitleSet { value: None });
    assert!(matches!(set, Effect::Host(HostEffect::TitleSet { .. })));
    assert!(matches!(reset, Effect::Host(HostEffect::TitleSet { .. })));
}

#[test]
fn host_clipboard_store_constructs() {
    let effect = Effect::Host(HostEffect::ClipboardStore {
        selection: ClipboardSelection::Clipboard,
        data: "hello".into(),
    });
    assert!(matches!(
        effect,
        Effect::Host(HostEffect::ClipboardStore { .. })
    ));
}

#[test]
fn host_clear_pending_notifications_constructs() {
    let effect = Effect::Host(HostEffect::ClearPendingNotifications);
    assert!(matches!(
        effect,
        Effect::Host(HostEffect::ClearPendingNotifications)
    ));
}

#[test]
fn host_request_clipboard_load_constructs() {
    let token = ResponseToken::<String>::new();
    let effect = Effect::HostRequest(HostRequest::ClipboardLoad {
        selection: ClipboardSelection::Primary,
        clipboard_char: b'p',
        terminator: "\x1b\\".into(),
        reply: token,
    });
    assert!(matches!(
        effect,
        Effect::HostRequest(HostRequest::ClipboardLoad { .. })
    ));
}

#[test]
fn host_request_color_query_constructs() {
    let token = ResponseToken::<crate::color::Rgb>::new();
    let effect = Effect::HostRequest(HostRequest::ColorQuery {
        prefix: "10".into(),
        index: 0,
        terminator: "\x07".into(),
        reply: token,
    });
    assert!(matches!(
        effect,
        Effect::HostRequest(HostRequest::ColorQuery { .. })
    ));
}

#[test]
fn response_token_fulfill_and_take() {
    let token = ResponseToken::new();
    assert!(!token.is_fulfilled());
    assert!(token.take().is_none());

    token.fulfill("hello".to_owned()).unwrap();
    assert!(token.is_fulfilled());

    let value = token.take();
    assert_eq!(value.as_deref(), Some("hello"));
    assert!(!token.is_fulfilled());
}

#[test]
fn response_token_default() {
    let token = ResponseToken::<u32>::default();
    assert!(!token.is_fulfilled());
}

#[test]
fn ui_cursor_blink_constructs() {
    let effect = Effect::Ui(UiEffect::CursorBlinkChanged { enabled: true });
    assert!(matches!(
        effect,
        Effect::Ui(UiEffect::CursorBlinkChanged { enabled: true })
    ));
}

#[test]
fn ui_mouse_cursor_dirty_constructs() {
    let effect = Effect::Ui(UiEffect::MouseCursorDirty);
    assert!(matches!(effect, Effect::Ui(UiEffect::MouseCursorDirty)));
}

#[test]
fn presentation_abort_sync_constructs() {
    let effect = Effect::Presentation(PresentationEffect::Abort {
        reason: SyncAbortReason::Timeout,
    });
    assert!(matches!(
        effect,
        Effect::Presentation(PresentationEffect::Abort {
            reason: SyncAbortReason::Timeout
        })
    ));
}

#[test]
fn clipboard_selection_variants() {
    assert_ne!(ClipboardSelection::Clipboard, ClipboardSelection::Primary);
    assert_ne!(ClipboardSelection::Primary, ClipboardSelection::Select);
    assert_ne!(ClipboardSelection::Select, ClipboardSelection::Clipboard);
}

#[test]
fn pty_write_kind_equality() {
    assert_eq!(PtyWriteKind::DeviceAttribute, PtyWriteKind::DeviceAttribute);
    assert_ne!(PtyWriteKind::DeviceAttribute, PtyWriteKind::CursorReport);
}

#[test]
fn sync_abort_reason_equality() {
    assert_eq!(SyncAbortReason::Timeout, SyncAbortReason::Timeout);
}

#[test]
fn effect_clone() {
    let effect = Effect::Host(HostEffect::Bell);
    let cloned = effect.clone();
    assert!(matches!(cloned, Effect::Host(HostEffect::Bell)));
}

#[test]
fn effect_debug() {
    let effect = Effect::Host(HostEffect::Bell);
    let debug = format!("{effect:?}");
    assert!(debug.contains("Bell"));
}

fn poll_closure_for_test(token: ResponseToken<String>) -> Box<dyn FnMut() -> PollResult + Send> {
    Box::new(move || {
        if let Some(text) = token.take() {
            return PollResult::Ready(Effect::Pty(PtyEffect::Write {
                bytes: text.into_bytes(),
                kind: PtyWriteKind::Other,
            }));
        }
        if token.consumer_strong_count() <= 1 {
            PollResult::Cancelled
        } else {
            PollResult::Pending
        }
    })
}

#[test]
fn pending_response_returns_pending_when_unfulfilled() {
    let token = ResponseToken::<String>::new();
    // Keep the consumer handle alive so cancellation is not triggered.
    let _consumer = token.clone();
    let mut pr = PendingResponse::new(poll_closure_for_test(token));
    assert!(matches!(pr.poll(), PollResult::Pending));
    assert!(matches!(pr.poll(), PollResult::Pending));
}

#[test]
fn pending_response_returns_ready_when_fulfilled() {
    let token = ResponseToken::<String>::new();
    let consumer = token.clone();
    let mut pr = PendingResponse::new(poll_closure_for_test(token));

    consumer.fulfill("hello".to_owned()).unwrap();
    match pr.poll() {
        PollResult::Ready(Effect::Pty(PtyEffect::Write { bytes, kind })) => {
            assert_eq!(bytes, b"hello");
            assert_eq!(kind, PtyWriteKind::Other);
        }
        other => panic!("expected Ready(Pty(Write)), got {other:?}"),
    }
}

#[test]
fn pending_response_returns_cancelled_when_consumer_dropped() {
    let token = ResponseToken::<String>::new();
    let consumer = token.clone();
    let mut pr = PendingResponse::new(poll_closure_for_test(token));
    drop(consumer);
    assert!(matches!(pr.poll(), PollResult::Cancelled));
}

#[test]
fn pending_response_returns_cancelled_after_drain() {
    let token = ResponseToken::<String>::new();
    let consumer = token.clone();
    let mut pr = PendingResponse::new(poll_closure_for_test(token));

    consumer.fulfill("data".to_owned()).unwrap();
    assert!(matches!(pr.poll(), PollResult::Ready(_)));
    // After draining AND after the consumer handle is dropped, subsequent
    // polls report cancellation.
    drop(consumer);
    assert!(matches!(pr.poll(), PollResult::Cancelled));
}
