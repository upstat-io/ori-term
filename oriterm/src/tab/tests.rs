//! Tests for tab identity, event types, EventProxy, and Notifier.

use std::sync::mpsc;

use oriterm_core::{Event, EventListener};

use super::{EventProxy, Notifier, TabId, TermEvent};
use crate::pty::Msg;

// ---------------------------------------------------------------------------
// TabId
// ---------------------------------------------------------------------------

#[test]
fn tab_id_next_generates_unique_ids() {
    let a = TabId::next();
    let b = TabId::next();
    let c = TabId::next();
    assert_ne!(a, b);
    assert_ne!(b, c);
    assert_ne!(a, c);
}

#[test]
fn tab_id_is_copy() {
    let id = TabId::next();
    let copy = id;
    assert_eq!(id, copy);
}

#[test]
fn tab_id_hash_equality() {
    use std::collections::HashSet;

    let a = TabId::next();
    let b = TabId::next();
    let mut set = HashSet::new();
    set.insert(a);
    set.insert(b);
    set.insert(a); // duplicate
    assert_eq!(set.len(), 2);
}

// ---------------------------------------------------------------------------
// TermEvent
// ---------------------------------------------------------------------------

#[test]
fn term_event_terminal_variant() {
    let id = TabId::next();
    let event = TermEvent::Terminal {
        tab_id: id,
        event: Event::Wakeup,
    };

    match event {
        TermEvent::Terminal { tab_id, event } => {
            assert_eq!(tab_id, id);
            assert!(matches!(event, Event::Wakeup));
        }
    }
}

#[test]
fn term_event_debug_format() {
    let id = TabId::next();
    let event = TermEvent::Terminal {
        tab_id: id,
        event: Event::Bell,
    };
    let debug = format!("{event:?}");
    assert!(debug.contains("Terminal"));
    assert!(debug.contains("Bell"));
}

// ---------------------------------------------------------------------------
// EventProxy
// ---------------------------------------------------------------------------

#[test]
fn event_proxy_sends_terminal_event() {
    let event_loop = build_test_event_loop();
    let proxy = event_loop.create_proxy();
    let tab_id = TabId::next();
    let event_proxy = EventProxy::new(proxy, tab_id);

    // Should not panic. The event is queued but there's no receiver
    // processing it — that's fine, send_event silently drops on error.
    event_proxy.send_event(Event::Wakeup);
    event_proxy.send_event(Event::Bell);
    event_proxy.send_event(Event::Title("test".into()));
}

#[test]
fn event_proxy_is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<EventProxy>();
}

// ---------------------------------------------------------------------------
// Notifier
// ---------------------------------------------------------------------------

#[test]
fn notifier_sends_input() {
    let (tx, rx) = mpsc::channel();
    let notifier = Notifier::new(tx);

    notifier.notify(b"hello");

    match rx.recv().expect("should receive") {
        Msg::Input(data) => assert_eq!(data, b"hello"),
        other => panic!("expected Input, got {other:?}"),
    }
}

#[test]
fn notifier_skips_empty_input() {
    let (tx, rx) = mpsc::channel();
    let notifier = Notifier::new(tx);

    notifier.notify(b"");

    // Channel should be empty — empty bytes are not sent.
    assert!(
        rx.try_recv().is_err(),
        "empty input should not produce a message",
    );
}

#[test]
fn notifier_sends_resize() {
    let (tx, rx) = mpsc::channel();
    let notifier = Notifier::new(tx);

    notifier.resize(40, 120);

    match rx.recv().expect("should receive") {
        Msg::Resize { rows, cols } => {
            assert_eq!(rows, 40);
            assert_eq!(cols, 120);
        }
        other => panic!("expected Resize, got {other:?}"),
    }
}

#[test]
fn notifier_sends_shutdown() {
    let (tx, rx) = mpsc::channel();
    let notifier = Notifier::new(tx);

    notifier.shutdown();

    assert!(
        matches!(rx.recv().expect("should receive"), Msg::Shutdown),
        "expected Shutdown message",
    );
}

#[test]
fn notifier_survives_dropped_receiver() {
    let (tx, rx) = mpsc::channel::<Msg>();
    let notifier = Notifier::new(tx);
    drop(rx);

    // Should not panic when receiver is gone.
    notifier.notify(b"orphaned");
    notifier.resize(24, 80);
    notifier.shutdown();
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a winit event loop usable from test threads.
///
/// On Windows, winit requires `with_any_thread(true)` because tests run
/// outside the main thread. Other platforms allow it by default.
fn build_test_event_loop() -> winit::event_loop::EventLoop<TermEvent> {
    #[cfg(windows)]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .with_any_thread(true)
            .build()
            .expect("event loop")
    }
    #[cfg(not(windows))]
    {
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .build()
            .expect("event loop")
    }
}
