//! Tests for daemon-mode host-request dispatch (BUG-11-011).

use std::collections::HashMap;

use oriterm_core::effect::{ClipboardSelection, ResponseToken};

use crate::id::{ClientId, HostRequestId, IdAllocator, PaneId};
use crate::mux_event::MuxNotification;
use crate::protocol::{MuxPdu, WireClipboardSelection};
use crate::server::notify::TargetClients;

use super::{HostRequestDispatchCtx, PendingHostReplyKind, host_request_to_pdu};

/// `host_request_to_pdu` allocates a request id and packages OSC parameters
/// into a `NotifyHostClipboardLoad` PDU targeted at the chosen responder.
#[test]
fn host_request_to_pdu_clipboard_packages_token_and_allocates_id() {
    let pane_id = PaneId::from_raw(1);
    let responder = ClientId::from_raw(7);
    let token = ResponseToken::<String>::new();
    let notif = MuxNotification::HostClipboardLoad {
        pane_id,
        selection: ClipboardSelection::Primary,
        clipboard_char: b'p',
        terminator: "\x1b\\".into(),
        reply: token,
    };

    let mut alloc = IdAllocator::<HostRequestId>::new();
    let mut ctx = HostRequestDispatchCtx {
        request_alloc: &mut alloc,
        responder,
    };
    let dispatch =
        host_request_to_pdu(notif, &mut ctx).expect("clipboard host-request must dispatch");

    assert_eq!(dispatch.request_id, HostRequestId::from_raw(1));
    assert!(
        matches!(dispatch.target, TargetClients::SinglePaneSubscriber(p, c) if p == pane_id && c == responder)
    );
    assert!(matches!(
        dispatch.pdu,
        MuxPdu::NotifyHostClipboardLoad {
            request_id: 1,
            selection: WireClipboardSelection::Primary,
            clipboard_char: b'p',
            ..
        }
    ));
    assert_eq!(dispatch.pending.responder, responder);
    assert!(matches!(
        dispatch.pending.kind,
        PendingHostReplyKind::Clipboard(_)
    ));
}

#[test]
fn host_request_to_pdu_color_query_carries_index_and_prefix() {
    let pane_id = PaneId::from_raw(2);
    let token = ResponseToken::<oriterm_core::color::Rgb>::new();
    let notif = MuxNotification::HostColorQuery {
        pane_id,
        prefix: "10".into(),
        index: 256,
        terminator: "\x07".into(),
        reply: token,
    };
    let mut alloc = IdAllocator::<HostRequestId>::new();
    let mut ctx = HostRequestDispatchCtx {
        request_alloc: &mut alloc,
        responder: ClientId::from_raw(3),
    };
    let dispatch = host_request_to_pdu(notif, &mut ctx).expect("color host-request must dispatch");
    match dispatch.pdu {
        MuxPdu::NotifyHostColorQuery {
            prefix,
            index,
            terminator,
            ..
        } => {
            assert_eq!(prefix, "10");
            assert_eq!(index, 256);
            assert_eq!(terminator, "\x07");
        }
        other => panic!("expected NotifyHostColorQuery, got {other:?}"),
    }
    assert!(matches!(
        dispatch.pending.kind,
        PendingHostReplyKind::Color(_)
    ));
}

#[test]
fn host_request_to_pdu_returns_none_for_non_host_request_variants() {
    let mut alloc = IdAllocator::<HostRequestId>::new();
    let mut ctx = HostRequestDispatchCtx {
        request_alloc: &mut alloc,
        responder: ClientId::from_raw(1),
    };
    let pane_id = PaneId::from_raw(5);
    assert!(host_request_to_pdu(MuxNotification::PaneOutput(pane_id), &mut ctx).is_none());
    assert!(host_request_to_pdu(MuxNotification::PaneBell(pane_id), &mut ctx).is_none());
}

#[test]
fn select_responder_returns_none_when_no_subscribers() {
    let subscriptions: HashMap<PaneId, Vec<ClientId>> = HashMap::new();
    let connections: HashMap<ClientId, super::ClientConnection> = HashMap::new();
    assert!(super::select_responder(&subscriptions, &connections, PaneId::from_raw(1)).is_none());
}

// `pick_lowest_priority` is the pure core of `select_responder`. Testing
// against this signature exercises the priority comparison + tie-break logic
// without requiring `ClientConnection` construction (which needs an
// `IpcStream`). End-to-end priority routing is covered indirectly by the
// daemon e2e tests in `oriterm_mux/tests/e2e.rs` exercising the full
// request → notify → reply pipeline.

/// Lowest priority value wins — focused (priority 0) beats hidden (priority 2).
/// Regression for BUG-11-011 §03 cell `select_responder picks lowest-priority u8 (focused = 0)`.
#[test]
fn pick_lowest_priority_picks_focused_client() {
    let focused = ClientId::from_raw(2);
    let hidden = ClientId::from_raw(1);
    // Focused has higher ClientId but lower priority → still wins.
    let candidates = [(hidden, 2u8), (focused, 0u8)];
    assert_eq!(super::pick_lowest_priority(&candidates), Some(focused));
}

/// Equal-priority ties broken deterministically by ClientId numerical order
/// (lower raw value wins). Regression for BUG-11-011 §03 cell
/// `select_responder ties broken by ClientId numerical order (determinism)`.
#[test]
fn pick_lowest_priority_breaks_ties_by_client_id_numerical_order() {
    let early = ClientId::from_raw(1);
    let late = ClientId::from_raw(7);
    // Same priority — earlier ClientId wins.
    let candidates = [(late, 1u8), (early, 1u8)];
    assert_eq!(super::pick_lowest_priority(&candidates), Some(early));
}

/// Disconnected clients (priority `u8::MAX` from `select_responder`'s fallback)
/// MUST lose to any connected client, regardless of ClientId order.
#[test]
fn pick_lowest_priority_excludes_disconnected_clients() {
    let connected = ClientId::from_raw(99);
    let disconnected = ClientId::from_raw(1); // earlier id but disconnected
    let candidates = [(disconnected, u8::MAX), (connected, 2u8)];
    assert_eq!(super::pick_lowest_priority(&candidates), Some(connected));
}

/// Empty candidate set returns `None`.
#[test]
fn pick_lowest_priority_returns_none_for_empty_candidates() {
    assert_eq!(super::pick_lowest_priority(&[]), None);
}

/// Three-client scenario across the full priority range — focused beats
/// visible beats hidden.
#[test]
fn pick_lowest_priority_three_client_full_range() {
    let focused = ClientId::from_raw(3);
    let visible = ClientId::from_raw(1);
    let hidden = ClientId::from_raw(2);
    let candidates = [(visible, 1u8), (hidden, 2u8), (focused, 0u8)];
    assert_eq!(super::pick_lowest_priority(&candidates), Some(focused));
}
