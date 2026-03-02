//! Tests for EmbeddedMux backend.

use std::sync::Arc;

use super::EmbeddedMux;
use crate::backend::MuxBackend;

/// No-op wakeup for tests (no event loop to wake).
fn test_wakeup() -> Arc<dyn Fn() + Send + Sync> {
    Arc::new(|| {})
}

/// `EmbeddedMux` implements `MuxBackend` (compile check via object safety).
#[test]
fn object_safe() {
    let mux = EmbeddedMux::new(test_wakeup());
    let _boxed: Box<dyn MuxBackend> = Box::new(mux);
}

/// `create_window` returns a valid window ID.
#[test]
fn create_window() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let wid = mux.create_window();
    assert!(mux.session().get_window(wid).is_some());
}

/// `drain_notifications` returns empty when nothing has happened.
#[test]
fn drain_empty() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let mut buf = Vec::new();
    mux.drain_notifications(&mut buf);
    assert!(buf.is_empty());
}

/// `discard_notifications` clears pending notifications.
#[test]
fn discard_notifications() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let _ = mux.create_window();
    // Window creation doesn't emit notifications, but discard shouldn't panic.
    mux.discard_notifications();
}

/// `close_window` on an empty window returns empty pane list.
#[test]
fn close_empty_window() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let wid = mux.create_window();
    let panes = mux.close_window(wid);
    assert!(panes.is_empty());
}

/// `active_tab_id` returns `None` for empty window.
#[test]
fn empty_window_no_active_tab() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    let wid = mux.create_window();
    assert!(mux.active_tab_id(wid).is_none());
}

/// `pane()` returns `None` for nonexistent pane ID.
#[test]
fn pane_not_found() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(mux.pane(crate::PaneId::from_raw(999)).is_none());
}

/// `is_last_pane` returns `false` when no panes exist.
#[test]
fn no_panes_not_last() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(!mux.is_last_pane(crate::PaneId::from_raw(1)));
}

/// `poll_events` with no pending events doesn't panic.
#[test]
fn poll_events_empty() {
    let mut mux = EmbeddedMux::new(test_wakeup());
    mux.poll_events();
}

/// `event_tx` returns `Some` in embedded mode.
#[test]
fn event_tx_available() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(mux.event_tx().is_some());
}

/// `pane_ids` returns empty initially.
#[test]
fn pane_ids_empty() {
    let mux = EmbeddedMux::new(test_wakeup());
    assert!(mux.pane_ids().is_empty());
}
