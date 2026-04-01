//! Tests for IoThreadEventProxy.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use oriterm_core::{Event, EventListener};

use super::IoThreadEventProxy;

/// `Wakeup` sets the `grid_dirty` flag.
#[test]
fn wakeup_sets_grid_dirty() {
    let dirty = Arc::new(AtomicBool::new(false));
    let proxy = IoThreadEventProxy::new(Arc::clone(&dirty), true);

    proxy.send_event(Event::Wakeup);
    assert!(dirty.load(Ordering::Acquire));
}

/// Title events are suppressed when `suppress_metadata` is true.
#[test]
fn suppresses_title_event() {
    let dirty = Arc::new(AtomicBool::new(false));
    let proxy = IoThreadEventProxy::new(Arc::clone(&dirty), true);

    proxy.send_event(Event::Title("test".into()));
    // Grid dirty should NOT be set by a non-Wakeup event.
    assert!(!dirty.load(Ordering::Acquire));
}

/// PtyWrite events are suppressed (prevents duplicate DA responses).
#[test]
fn suppresses_pty_write() {
    let dirty = Arc::new(AtomicBool::new(false));
    let proxy = IoThreadEventProxy::new(Arc::clone(&dirty), true);

    proxy.send_event(Event::PtyWrite("data".into()));
    assert!(!dirty.load(Ordering::Acquire));
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
    let dirty = Arc::new(AtomicBool::new(false));
    let proxy = IoThreadEventProxy::new(Arc::clone(&dirty), true);
    assert!(proxy.is_suppressed());

    let proxy2 = IoThreadEventProxy::new(dirty, false);
    assert!(!proxy2.is_suppressed());
}
