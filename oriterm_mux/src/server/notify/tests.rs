//! Tests for notification → IPC PDU routing.

use std::collections::HashMap;
use std::sync::Arc;

use oriterm_core::ClipboardType;

use crate::mux_event::MuxNotification;
use crate::pane::Pane;
use crate::registry::SessionRegistry;
use crate::session::{MuxTab, MuxWindow};
use crate::{MuxPdu, PaneId, TabId, WindowId};

use super::{TargetClients, notification_to_pdu};

fn empty_panes() -> HashMap<PaneId, Pane> {
    HashMap::new()
}

fn empty_session() -> SessionRegistry {
    SessionRegistry::new()
}

// -- Notifications that produce Some --

/// `PaneDirty` → `NotifyPaneOutput` routed to pane subscribers.
#[test]
fn pane_dirty_to_notify_output() {
    let pid = PaneId::from_raw(1);
    let notif = MuxNotification::PaneDirty(pid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes(), &empty_session()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(pdu, MuxPdu::NotifyPaneOutput { pane_id: pid });
}

/// `PaneClosed` → `NotifyPaneExited` routed to pane subscribers.
#[test]
fn pane_closed_to_notify_exited() {
    let pid = PaneId::from_raw(2);
    let notif = MuxNotification::PaneClosed(pid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes(), &empty_session()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(pdu, MuxPdu::NotifyPaneExited { pane_id: pid });
}

/// `PaneTitleChanged` with pane not in map → empty title string.
#[test]
fn pane_title_changed_missing_pane() {
    let pid = PaneId::from_raw(3);
    let notif = MuxNotification::PaneTitleChanged(pid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes(), &empty_session()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(
        pdu,
        MuxPdu::NotifyPaneTitleChanged {
            pane_id: pid,
            title: String::new(),
        }
    );
}

/// `Alert` → `NotifyPaneBell` routed to pane subscribers.
#[test]
fn alert_to_notify_bell() {
    let pid = PaneId::from_raw(4);
    let notif = MuxNotification::Alert(pid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes(), &empty_session()).unwrap();

    assert!(matches!(target, TargetClients::PaneSubscribers(id) if id == pid));
    assert_eq!(pdu, MuxPdu::NotifyPaneBell { pane_id: pid });
}

/// `WindowTabsChanged` → `NotifyWindowTabsChanged` routed to window client.
#[test]
fn window_tabs_changed_to_notify() {
    let wid = WindowId::from_raw(5);
    let notif = MuxNotification::WindowTabsChanged(wid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes(), &empty_session()).unwrap();

    assert!(matches!(target, TargetClients::WindowClient(id) if id == wid));
    assert_eq!(pdu, MuxPdu::NotifyWindowTabsChanged { window_id: wid });
}

/// `TabLayoutChanged` with tab in session → `NotifyTabLayoutChanged`.
#[test]
fn tab_layout_changed_with_tab() {
    let tid = TabId::from_raw(10);
    let pid = PaneId::from_raw(20);
    let wid = WindowId::from_raw(30);

    let mut session = SessionRegistry::new();
    session.add_tab(MuxTab::new(tid, pid));
    let mut win = MuxWindow::new(wid);
    win.add_tab(tid);
    session.add_window(win);

    let notif = MuxNotification::TabLayoutChanged(tid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes(), &session).unwrap();

    assert!(matches!(target, TargetClients::WindowClient(id) if id == wid));
    assert!(matches!(pdu, MuxPdu::NotifyTabLayoutChanged { tab_id, .. } if tab_id == tid));
}

/// `TabLayoutChanged` without tab in session → `None`.
#[test]
fn tab_layout_changed_missing_tab() {
    let notif = MuxNotification::TabLayoutChanged(TabId::from_raw(1));
    assert!(notification_to_pdu(&notif, &empty_panes(), &empty_session()).is_none());
}

/// `FloatingPaneChanged` with tab in session → `NotifyTabLayoutChanged`.
#[test]
fn floating_pane_changed_with_tab() {
    let tid = TabId::from_raw(11);
    let pid = PaneId::from_raw(21);
    let wid = WindowId::from_raw(31);

    let mut session = SessionRegistry::new();
    session.add_tab(MuxTab::new(tid, pid));
    let mut win = MuxWindow::new(wid);
    win.add_tab(tid);
    session.add_window(win);

    let notif = MuxNotification::FloatingPaneChanged(tid);
    let (target, pdu) = notification_to_pdu(&notif, &empty_panes(), &session).unwrap();

    assert!(matches!(target, TargetClients::WindowClient(id) if id == wid));
    assert!(matches!(pdu, MuxPdu::NotifyTabLayoutChanged { tab_id, .. } if tab_id == tid));
}

/// `FloatingPaneChanged` without tab in session → `None`.
#[test]
fn floating_pane_changed_missing_tab() {
    let notif = MuxNotification::FloatingPaneChanged(TabId::from_raw(1));
    assert!(notification_to_pdu(&notif, &empty_panes(), &empty_session()).is_none());
}

// -- Notifications that return None --

/// `CommandComplete` is not pushed over IPC.
#[test]
fn command_complete_returns_none() {
    let notif = MuxNotification::CommandComplete {
        pane_id: PaneId::from_raw(1),
        duration: std::time::Duration::from_secs(5),
    };
    assert!(notification_to_pdu(&notif, &empty_panes(), &empty_session()).is_none());
}

/// `WindowClosed` is not pushed over IPC.
#[test]
fn window_closed_returns_none() {
    let notif = MuxNotification::WindowClosed(WindowId::from_raw(1));
    assert!(notification_to_pdu(&notif, &empty_panes(), &empty_session()).is_none());
}

/// `LastWindowClosed` is not pushed over IPC.
#[test]
fn last_window_closed_returns_none() {
    let notif = MuxNotification::LastWindowClosed;
    assert!(notification_to_pdu(&notif, &empty_panes(), &empty_session()).is_none());
}

/// `ClipboardStore` is not pushed over IPC.
#[test]
fn clipboard_store_returns_none() {
    let notif = MuxNotification::ClipboardStore {
        pane_id: PaneId::from_raw(1),
        clipboard_type: ClipboardType::Clipboard,
        text: "hello".into(),
    };
    assert!(notification_to_pdu(&notif, &empty_panes(), &empty_session()).is_none());
}

/// `ClipboardLoad` is not pushed over IPC.
#[test]
fn clipboard_load_returns_none() {
    let notif = MuxNotification::ClipboardLoad {
        pane_id: PaneId::from_raw(1),
        clipboard_type: ClipboardType::Clipboard,
        formatter: Arc::new(|s| s.to_string()),
    };
    assert!(notification_to_pdu(&notif, &empty_panes(), &empty_session()).is_none());
}
