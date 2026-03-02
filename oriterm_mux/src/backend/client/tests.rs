//! Tests for MuxClient stub.

use super::MuxClient;
use crate::backend::MuxBackend;

/// `MuxClient` implements `MuxBackend` (compile check via object safety).
#[test]
fn object_safe() {
    let client = MuxClient::new();
    let _boxed: Box<dyn MuxBackend> = Box::new(client);
}

/// `pane()` returns `None` in client mode.
#[test]
fn pane_returns_none() {
    let client = MuxClient::new();
    assert!(client.pane(crate::PaneId::from_raw(1)).is_none());
}

/// `pane_mut()` returns `None` in client mode.
#[test]
fn pane_mut_returns_none() {
    let mut client = MuxClient::new();
    assert!(client.pane_mut(crate::PaneId::from_raw(1)).is_none());
}

/// `drain_notifications` returns empty initially.
#[test]
fn drain_empty() {
    let mut client = MuxClient::new();
    let mut buf = Vec::new();
    client.drain_notifications(&mut buf);
    assert!(buf.is_empty());
}

/// `poll_events` is a no-op and doesn't panic.
#[test]
fn poll_events_noop() {
    let mut client = MuxClient::new();
    client.poll_events();
}
