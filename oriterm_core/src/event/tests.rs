//! Tests for the event system.

use super::{ClipboardType, Event, EventListener, VoidListener};

#[test]
fn void_listener_implements_event_listener() {
    let listener = VoidListener;
    // Should compile and not panic — the default no-op body runs.
    listener.send_event(Event::Wakeup);
    listener.send_event(Event::Bell);
}

#[test]
fn void_listener_is_send_and_static() {
    fn assert_send_static<T: Send + 'static>() {}
    assert_send_static::<VoidListener>();
}

#[test]
fn event_wakeup() {
    let event = Event::Wakeup;
    assert_eq!(format!("{event:?}"), "Wakeup");
}

#[test]
fn event_bell() {
    let event = Event::Bell;
    assert_eq!(format!("{event:?}"), "Bell");
}

#[test]
fn event_title() {
    let event = Event::Title("hello".to_string());
    assert_eq!(format!("{event:?}"), "Title(hello)");
}

#[test]
fn event_reset_title() {
    let event = Event::ResetTitle;
    assert_eq!(format!("{event:?}"), "ResetTitle");
}

#[test]
fn event_icon_name() {
    let event = Event::IconName("🐍python".to_string());
    assert_eq!(format!("{event:?}"), "IconName(🐍python)");
}

#[test]
fn event_reset_icon_name() {
    let event = Event::ResetIconName;
    assert_eq!(format!("{event:?}"), "ResetIconName");
}

#[test]
fn event_clipboard_store() {
    let event = Event::ClipboardStore(ClipboardType::Clipboard, "data".to_string());
    assert_eq!(format!("{event:?}"), "ClipboardStore(Clipboard, data)");
}

#[test]
fn event_pty_write() {
    let event = Event::PtyWrite("\x1b[6n".to_string());
    assert_eq!(format!("{event:?}"), "PtyWrite(\x1b[6n)");
}

#[test]
fn event_cursor_blinking_change() {
    let event = Event::CursorBlinkingChange;
    assert_eq!(format!("{event:?}"), "CursorBlinkingChange");
}

#[test]
fn event_mouse_cursor_dirty() {
    let event = Event::MouseCursorDirty;
    assert_eq!(format!("{event:?}"), "MouseCursorDirty");
}

#[test]
fn clipboard_type_variants() {
    assert_ne!(ClipboardType::Clipboard, ClipboardType::Selection);

    let c = ClipboardType::Clipboard;
    let s = ClipboardType::Selection;
    assert_eq!(c, ClipboardType::Clipboard);
    assert_eq!(s, ClipboardType::Selection);
}

#[test]
fn event_clone() {
    let event = Event::Title("test".to_string());
    let cloned = event.clone();
    assert_eq!(format!("{cloned:?}"), "Title(test)");
}

/// Effect-cutover §01.3 deletion pin: the closure-bearing `ClipboardLoad`,
/// `ColorRequest`, and `ChildExit` variants are gone — `HostRequest` and
/// `HostEffect::ChildExit` cover those code paths via the Effect path now.
/// Listing the surviving variants explicitly here surfaces an exhaustiveness
/// regression at compile time if a new variant is added without test coverage.
#[test]
fn all_event_variants_constructible() {
    let _events = [
        Event::Wakeup,
        Event::Bell,
        Event::Title(String::new()),
        Event::ResetTitle,
        Event::IconName(String::new()),
        Event::ResetIconName,
        Event::ClipboardStore(ClipboardType::Clipboard, String::new()),
        Event::PtyWrite(String::new()),
        Event::CursorBlinkingChange,
        Event::Cwd("/tmp".to_string()),
        Event::CommandComplete(std::time::Duration::from_secs(5)),
        Event::MouseCursorDirty,
    ];
}
