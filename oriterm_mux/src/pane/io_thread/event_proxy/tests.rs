//! Tests for IoThreadEventProxy.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use oriterm_core::{Event, EventListener};

use crate::PaneId;
use crate::mux_event::MuxEvent;

use super::IoThreadEventProxy;

/// Helper: create a proxy with the given suppression flag.
fn make_proxy(
    suppress: bool,
) -> (
    IoThreadEventProxy,
    Arc<AtomicBool>,
    mpsc::Receiver<MuxEvent>,
) {
    let dirty = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(|| {});
    let proxy = IoThreadEventProxy::new(
        Arc::clone(&dirty),
        suppress,
        PaneId::from_raw(1),
        tx,
        wakeup,
    );
    (proxy, dirty, rx)
}

/// `Wakeup` sets the `grid_dirty` flag.
#[test]
fn wakeup_sets_grid_dirty() {
    let (proxy, dirty, _rx) = make_proxy(true);
    proxy.send_event(Event::Wakeup);
    assert!(dirty.load(Ordering::Acquire));
}

/// Title events are suppressed when `suppress_metadata` is true.
#[test]
fn suppresses_title_event() {
    let (proxy, dirty, rx) = make_proxy(true);
    proxy.send_event(Event::Title("test".into()));
    assert!(!dirty.load(Ordering::Acquire));
    // No MuxEvent sent.
    assert!(rx.try_recv().is_err());
}

/// PtyWrite events are suppressed (prevents duplicate DA responses).
#[test]
fn suppresses_pty_write() {
    let (proxy, dirty, rx) = make_proxy(true);
    proxy.send_event(Event::PtyWrite("data".into()));
    assert!(!dirty.load(Ordering::Acquire));
    assert!(rx.try_recv().is_err());
}

/// Static assertion that `IoThreadEventProxy` is `Send`.
#[test]
fn is_send() {
    fn assert_send<T: Send>() {}
    assert_send::<IoThreadEventProxy>();
}

/// Verify the suppression flag is readable.
#[test]
fn suppression_flag_readable() {
    let (proxy_on, _, _) = make_proxy(true);
    assert!(proxy_on.is_suppressed());

    let (proxy_off, _, _) = make_proxy(false);
    assert!(!proxy_off.is_suppressed());
}

/// Unsuppressed proxy forwards title events to the mux channel.
#[test]
fn unsuppressed_forwards_title() {
    let (proxy, _dirty, rx) = make_proxy(false);
    proxy.send_event(Event::Title("hello".into()));
    let event = rx.try_recv().expect("should have received MuxEvent");
    assert!(matches!(event, MuxEvent::PaneTitleChanged { title, .. } if title == "hello"));
}

/// Unsuppressed proxy forwards PtyWrite events.
#[test]
fn unsuppressed_forwards_pty_write() {
    let (proxy, _dirty, rx) = make_proxy(false);
    proxy.send_event(Event::PtyWrite("response".into()));
    let event = rx.try_recv().expect("should have received MuxEvent");
    assert!(matches!(event, MuxEvent::PtyWrite { data, .. } if data == "response"));
}

/// Unsuppressed proxy forwards bell events.
#[test]
fn unsuppressed_forwards_bell() {
    let (proxy, _dirty, rx) = make_proxy(false);
    proxy.send_event(Event::Bell);
    let event = rx.try_recv().expect("should have received MuxEvent");
    assert!(matches!(event, MuxEvent::PaneBell(_)));
}

/// Unsuppressed proxy forwards CWD events.
#[test]
fn unsuppressed_forwards_cwd() {
    let (proxy, _dirty, rx) = make_proxy(false);
    proxy.send_event(Event::Cwd("/home/user".into()));
    let event = rx.try_recv().expect("should have received MuxEvent");
    assert!(matches!(event, MuxEvent::PaneCwdChanged { cwd, .. } if cwd == "/home/user"));
}

/// Unsuppressed proxy forwards all metadata events.
#[test]
fn unsuppressed_forwards_all_metadata() {
    let (proxy, _dirty, rx) = make_proxy(false);

    proxy.send_event(Event::Title("t".into()));
    proxy.send_event(Event::PtyWrite("w".into()));
    proxy.send_event(Event::Bell);

    // All three should arrive.
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_ok());
    assert!(rx.try_recv().is_err());
}

/// Wakeup still sets grid_dirty even when unsuppressed.
#[test]
fn unsuppressed_wakeup_still_sets_dirty() {
    let (proxy, dirty, _rx) = make_proxy(false);
    proxy.send_event(Event::Wakeup);
    assert!(dirty.load(Ordering::Acquire));
}
