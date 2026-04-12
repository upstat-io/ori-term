//! Tests for `LegacyEventSink` adapter.

use std::sync::{Arc, Mutex};

use crate::color::Rgb;
use crate::effect::Effect;
use crate::effect::families::{
    ClipboardSelection, HostEffect, HostRequest, NotificationSource, PresentationEffect, PtyEffect,
    PtyWriteKind, ResponseToken, UiEffect,
};
use crate::effect::sink::EffectSink;
use crate::event::{Event, EventListener};

use super::LegacyEventSink;

/// Recording listener that stores all received events.
#[derive(Debug, Clone, Default)]
struct RecordingListener {
    events: Arc<Mutex<Vec<String>>>,
}

impl EventListener for RecordingListener {
    fn send_event(&self, event: Event) {
        self.events
            .lock()
            .expect("RecordingListener mutex poisoned")
            .push(format!("{event:?}"));
    }
}

impl RecordingListener {
    fn events(&self) -> Vec<String> {
        self.events
            .lock()
            .expect("RecordingListener mutex poisoned")
            .clone()
    }
}

#[test]
fn pty_write_routes_to_pty_write_event() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Pty(PtyEffect::Write {
        bytes: b"\x1b[?1;2c".to_vec(),
        kind: PtyWriteKind::DeviceAttribute,
    }));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert!(events[0].starts_with("PtyWrite("));
    assert!(events[0].contains("[?1;2c"));
}

#[test]
fn host_bell_routes_to_bell_event() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Host(HostEffect::Bell));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "Bell");
}

#[test]
fn host_title_set_routes_to_title_event() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Host(HostEffect::TitleSet {
        value: Some("hello".into()),
    }));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "Title(hello)");
}

#[test]
fn host_title_reset_routes_to_reset_title_event() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Host(HostEffect::TitleSet { value: None }));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "ResetTitle");
}

#[test]
fn host_request_clipboard_load_routes_with_reply_token() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    let token = ResponseToken::<String>::new();
    sink.push(Effect::HostRequest(HostRequest::ClipboardLoad {
        selection: ClipboardSelection::Clipboard,
        clipboard_char: b'c',
        terminator: "\x1b\\".into(),
        reply: token.clone(),
    }));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert!(events[0].starts_with("ClipboardLoad(Clipboard)"));

    // Token is not fulfilled until the closure is called by the consumer.
    assert!(!token.is_fulfilled());
}

#[test]
fn host_request_color_query_routes_with_reply_token() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    let token = ResponseToken::<Rgb>::new();
    sink.push(Effect::HostRequest(HostRequest::ColorQuery {
        prefix: "10".into(),
        index: 0,
        terminator: "\x07".into(),
        reply: token.clone(),
    }));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert!(events[0].starts_with("ColorRequest(0)"));

    assert!(!token.is_fulfilled());
}

#[test]
fn desktop_notification_queued_in_adapter() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Host(HostEffect::DesktopNotification {
        source: NotificationSource::Osc99,
        title: "test title".into(),
        body: "test body".into(),
    }));

    let notifications = sink.drain_pending_notifications();
    assert_eq!(notifications.len(), 1);
    assert_eq!(notifications[0].title, "test title");
    assert_eq!(notifications[0].body, "test body");

    // Drain again — should be empty.
    assert!(sink.drain_pending_notifications().is_empty());
}

#[test]
fn desktop_notification_not_forwarded_as_event() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Host(HostEffect::DesktopNotification {
        source: NotificationSource::Osc9,
        title: "t".into(),
        body: "b".into(),
    }));

    // The EventListener should NOT have received any Event.
    assert!(listener.events().is_empty());
}

#[test]
fn clear_pending_notifications_clears_queue() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Host(HostEffect::DesktopNotification {
        source: NotificationSource::Osc99,
        title: "t".into(),
        body: "b".into(),
    }));
    assert_eq!(sink.drain_pending_notifications().len(), 1);

    // Re-add and then clear.
    sink.push(Effect::Host(HostEffect::DesktopNotification {
        source: NotificationSource::Osc777,
        title: "x".into(),
        body: "y".into(),
    }));
    sink.push(Effect::Host(HostEffect::ClearPendingNotifications));
    assert!(sink.drain_pending_notifications().is_empty());
}

#[test]
fn presentation_effects_dropped_silently() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Presentation(PresentationEffect::Begin));
    sink.push(Effect::Presentation(PresentationEffect::Commit {
        snapshot_seqno: 1,
    }));

    assert!(listener.events().is_empty());
}

#[test]
fn visual_bell_dropped_silently() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Host(HostEffect::VisualBell));

    assert!(listener.events().is_empty());
}

#[test]
fn cursor_blink_routes_to_cursor_blinking_change() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Ui(UiEffect::CursorBlinkChanged { enabled: true }));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "CursorBlinkingChange");
}

#[test]
fn mouse_cursor_dirty_routes_correctly() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener.clone());

    sink.push(Effect::Ui(UiEffect::MouseCursorDirty));

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "MouseCursorDirty");
}

#[test]
fn drain_into_is_noop() {
    let listener = RecordingListener::default();
    let sink = LegacyEventSink::new(listener);

    sink.push(Effect::Host(HostEffect::Bell));

    let mut out = Vec::new();
    sink.drain_into(&mut out);
    assert!(out.is_empty());
}
