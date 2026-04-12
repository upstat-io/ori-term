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
fn host_visual_bell_constructs() {
    let effect = Effect::Host(HostEffect::VisualBell);
    assert!(matches!(effect, Effect::Host(HostEffect::VisualBell)));
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

    token.fulfill("hello".to_owned());
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
fn presentation_begin_sync_constructs() {
    let effect = Effect::Presentation(PresentationEffect::Begin);
    assert!(matches!(
        effect,
        Effect::Presentation(PresentationEffect::Begin)
    ));
}

#[test]
fn presentation_commit_sync_constructs() {
    let effect = Effect::Presentation(PresentationEffect::Commit { snapshot_seqno: 42 });
    assert!(matches!(
        effect,
        Effect::Presentation(PresentationEffect::Commit { snapshot_seqno: 42 })
    ));
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
    assert_ne!(
        SyncAbortReason::Timeout,
        SyncAbortReason::MaxBufferBytesExceeded
    );
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

#[test]
fn pending_response_returns_none_when_unfulfilled() {
    let token = ResponseToken::<String>::new();
    let mut pr = PendingResponse::new(Box::new(move || {
        let text = token.take()?;
        Some(Effect::Pty(PtyEffect::Write {
            bytes: text.into_bytes(),
            kind: PtyWriteKind::Other,
        }))
    }));
    assert!(pr.poll().is_none());
    assert!(pr.poll().is_none());
}

#[test]
fn pending_response_returns_effect_when_fulfilled() {
    let token = ResponseToken::<String>::new();
    let token_clone = token.clone();
    let mut pr = PendingResponse::new(Box::new(move || {
        let text = token.take()?;
        Some(Effect::Pty(PtyEffect::Write {
            bytes: text.into_bytes(),
            kind: PtyWriteKind::Other,
        }))
    }));

    token_clone.fulfill("hello".to_owned());
    let effect = pr.poll();
    assert!(effect.is_some());
    match effect.unwrap() {
        Effect::Pty(PtyEffect::Write { bytes, kind }) => {
            assert_eq!(bytes, b"hello");
            assert_eq!(kind, PtyWriteKind::Other);
        }
        other => panic!("expected Pty(Write), got {other:?}"),
    }
}

#[test]
fn pending_response_returns_none_after_drain() {
    let token = ResponseToken::<String>::new();
    let token_clone = token.clone();
    let mut pr = PendingResponse::new(Box::new(move || {
        let text = token.take()?;
        Some(Effect::Pty(PtyEffect::Write {
            bytes: text.into_bytes(),
            kind: PtyWriteKind::Other,
        }))
    }));

    token_clone.fulfill("data".to_owned());
    assert!(pr.poll().is_some());
    // After draining, subsequent polls return None.
    assert!(pr.poll().is_none());
}
