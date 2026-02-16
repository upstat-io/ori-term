//! Tests for tab identity, event types, EventProxy, Notifier, and Tab.

use std::sync::mpsc;

use winit::event_loop::EventLoopProxy;

use oriterm_core::{Event, EventListener};

use super::{EventProxy, Notifier, Tab, TabId, TermEvent};
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
    let proxy = test_proxy();
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
// Tab (live PTY)
// ---------------------------------------------------------------------------

#[test]
fn tab_spawns_with_live_pty() {
    let proxy = test_proxy();
    let id = TabId::next();

    let tab = Tab::new(id, 24, 80, 1000, proxy).expect("tab creation should succeed");

    assert_eq!(tab.id(), id);
    assert_eq!(tab.title(), "");
    assert!(!tab.has_bell());
}

#[test]
fn tab_terminal_is_accessible() {
    let proxy = test_proxy();
    let id = TabId::next();

    let tab = Tab::new(id, 24, 80, 1000, proxy).expect("tab creation should succeed");

    // Lock the terminal and verify grid dimensions.
    let term = tab.terminal().lock();
    let grid = term.grid();
    assert_eq!(grid.cols(), 80);
    assert_eq!(grid.lines(), 24);
}

#[test]
fn tab_write_input_reaches_pty() {
    let proxy = test_proxy();
    let id = TabId::next();

    let tab = Tab::new(id, 24, 80, 1000, proxy).expect("tab creation should succeed");

    // Should not panic — bytes go through Notifier → channel → PTY writer.
    tab.write_input(b"echo hello\r\n");
}

#[test]
fn tab_resize_sends_to_pty() {
    let proxy = test_proxy();
    let id = TabId::next();

    let tab = Tab::new(id, 24, 80, 1000, proxy).expect("tab creation should succeed");

    // Should not panic — resize goes through Notifier → channel → PTY control.
    tab.resize(40, 120);
}

#[test]
fn tab_bell_state() {
    let proxy = test_proxy();
    let id = TabId::next();

    let mut tab = Tab::new(id, 24, 80, 1000, proxy).expect("tab creation should succeed");

    assert!(!tab.has_bell());
    tab.set_bell();
    assert!(tab.has_bell());
    tab.clear_bell();
    assert!(!tab.has_bell());
}

#[test]
fn tab_title_update() {
    let proxy = test_proxy();
    let id = TabId::next();

    let mut tab = Tab::new(id, 24, 80, 1000, proxy).expect("tab creation should succeed");

    assert_eq!(tab.title(), "");
    tab.set_title("my terminal".into());
    assert_eq!(tab.title(), "my terminal");
}

#[test]
fn tab_drop_is_clean() {
    let proxy = test_proxy();
    let id = TabId::next();

    let tab = Tab::new(id, 24, 80, 1000, proxy).expect("tab creation should succeed");

    // Drop should send Shutdown, kill child, and join reader thread
    // without panicking.
    drop(tab);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get a cloned winit `EventLoopProxy` for tests.
///
/// Winit only allows one `EventLoop` per process. This creates one
/// on first call (leaked to keep the proxy valid) and clones the
/// proxy for each test.
fn test_proxy() -> EventLoopProxy<TermEvent> {
    use std::sync::OnceLock;

    static PROXY: OnceLock<EventLoopProxy<TermEvent>> = OnceLock::new();
    PROXY
        .get_or_init(|| {
            let event_loop = build_event_loop();
            let proxy = event_loop.create_proxy();
            // Leak so the event loop stays alive for the process lifetime.
            std::mem::forget(event_loop);
            proxy
        })
        .clone()
}

/// Build a winit event loop usable from test threads.
///
/// Tests run outside the main thread. winit requires `any_thread(true)`
/// on both Windows and Linux (X11/Wayland) to allow this.
fn build_event_loop() -> winit::event_loop::EventLoop<TermEvent> {
    #[cfg(windows)]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .with_any_thread(true)
            .build()
            .expect("event loop")
    }
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .with_any_thread(true)
            .build()
            .expect("event loop")
    }
    #[cfg(target_os = "macos")]
    {
        winit::event_loop::EventLoop::<TermEvent>::with_user_event()
            .build()
            .expect("event loop")
    }
}
