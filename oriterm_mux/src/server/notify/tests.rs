//! Tests for notification → IPC PDU routing.

use std::collections::HashMap;
use std::sync::Arc;

use oriterm_core::ClipboardType;

use crate::mux_event::MuxNotification;
use crate::pane::Pane;
use crate::{MuxPdu, PaneId};

use super::{TargetClients, notification_to_pdu};

fn empty_panes() -> HashMap<PaneId, Pane> {
    HashMap::new()
}

// Notifications that produce Some

/// `PaneOutput` is intercepted before reaching `notification_to_pdu` — returns `None`.
#[test]
fn pane_output_returns_none() {
    let pid = PaneId::from_raw(1);
    let notif = MuxNotification::PaneOutput(pid);
    assert!(notification_to_pdu(&notif, &empty_panes()).is_none());
}

/// `PaneClosed` → `NotifyPaneExited` routed to pane subscribers.
#[test]
fn pane_closed_to_notify_exited() {
    let pid = PaneId::from_raw(2);
    let notif = MuxNotification::PaneClosed {
        pane_id: pid,
        exit_code: 0,
    };
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(
        pdu,
        MuxPdu::NotifyPaneExited {
            pane_id: pid,
            exit_code: 0,
        }
    );
}

/// `PaneMetadataChanged` with pane not in map → empty title string.
#[test]
fn pane_metadata_changed_missing_pane() {
    let pid = PaneId::from_raw(3);
    let notif = MuxNotification::PaneMetadataChanged(pid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(
        pdu,
        MuxPdu::NotifyPaneMetadataChanged {
            pane_id: pid,
            title: String::new(),
        }
    );
}

/// `PaneBell` → `NotifyPaneBell` routed to pane subscribers.
#[test]
fn pane_bell_to_notify() {
    let pid = PaneId::from_raw(4);
    let notif = MuxNotification::PaneBell(pid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(pdu, MuxPdu::NotifyPaneBell { pane_id: pid });
}

/// `CommandComplete` → `NotifyCommandComplete` with duration in milliseconds.
#[test]
fn command_complete_to_notify() {
    let pid = PaneId::from_raw(5);
    let notif = MuxNotification::CommandComplete {
        pane_id: pid,
        duration: std::time::Duration::from_millis(1234),
    };
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(
        pdu,
        MuxPdu::NotifyCommandComplete {
            pane_id: pid,
            duration_ms: 1234,
        }
    );
}

/// `ClipboardStore` → `NotifyClipboardStore` with wire clipboard type.
#[test]
fn clipboard_store_to_notify() {
    let pid = PaneId::from_raw(6);
    let notif = MuxNotification::ClipboardStore {
        pane_id: pid,
        clipboard_type: ClipboardType::Clipboard,
        text: "hello".into(),
    };
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(
        pdu,
        MuxPdu::NotifyClipboardStore {
            pane_id: pid,
            clipboard_type: 0,
            text: "hello".into(),
        }
    );
}

/// `ClipboardStore` with Selection clipboard type maps to wire value 1.
#[test]
fn clipboard_store_selection_type() {
    let pid = PaneId::from_raw(7);
    let notif = MuxNotification::ClipboardStore {
        pane_id: pid,
        clipboard_type: ClipboardType::Selection,
        text: "sel".into(),
    };
    let (_, pdu) = notification_to_pdu(&notif, &empty_panes()).unwrap();

    assert_eq!(
        pdu,
        MuxPdu::NotifyClipboardStore {
            pane_id: pid,
            clipboard_type: 1,
            text: "sel".into(),
        }
    );
}

/// `ClipboardLoad` → `NotifyClipboardLoad` (formatter dropped at wire boundary).
#[test]
fn clipboard_load_to_notify() {
    let pid = PaneId::from_raw(8);
    let notif = MuxNotification::ClipboardLoad {
        pane_id: pid,
        clipboard_type: ClipboardType::Clipboard,
        formatter: Arc::new(|s| s.to_string()),
    };
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(
        pdu,
        MuxPdu::NotifyClipboardLoad {
            pane_id: pid,
            clipboard_type: 0,
        }
    );
}
