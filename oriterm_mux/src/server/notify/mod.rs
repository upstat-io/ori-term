//! Notification routing from mux events to IPC push messages.
//!
//! Converts [`MuxNotification`] variants into [`MuxPdu`] notifications and
//! identifies which clients should receive them.

use std::collections::HashMap;

use oriterm_core::ClipboardType;

use crate::id::ClientId;
use crate::pane::Pane;
use crate::{MuxNotification, MuxPdu, PaneId};

/// Which clients should receive a notification.
pub enum TargetClients {
    /// All clients subscribed to a specific pane.
    PaneSubscribers(PaneId),
    /// One specific subscriber chosen by `select_responder` — used for
 /// daemon-mode `HostRequest` routing () where a single
    /// authoritative responder answers. The `PaneId` is structural — kept
    /// for diagnostic logging / future routing decisions even though the
    /// dispatch path only reads the `ClientId`.
    SinglePaneSubscriber(
        #[allow(dead_code, reason = "structural — read by future routing diagnostics")] PaneId,
        ClientId,
    ),
}

/// Wire representation of [`ClipboardType`]: 0 = Clipboard, 1 = Selection.
fn clipboard_type_to_wire(ct: ClipboardType) -> u8 {
    match ct {
        ClipboardType::Clipboard => 0,
        ClipboardType::Selection => 1,
    }
}

/// Convert a mux notification into a target + PDU pair for IPC dispatch.
///
/// Returns `None` only for `PaneOutput` (intercepted earlier by
/// `drain_mux_events`).
pub fn notification_to_pdu(
    notif: &MuxNotification,
    panes: &HashMap<PaneId, Pane>,
) -> Option<(TargetClients, MuxPdu)> {
    match notif {
        MuxNotification::PaneClosed { pane_id, exit_code } => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyPaneExited {
                pane_id: *pane_id,
                exit_code: *exit_code,
            },
        )),

        MuxNotification::PaneMetadataChanged(pane_id) => {
            let title = panes
                .get(pane_id)
                .map(|p| p.effective_title().to_string())
                .unwrap_or_default();
            Some((
                TargetClients::PaneSubscribers(*pane_id),
                MuxPdu::NotifyPaneMetadataChanged {
                    pane_id: *pane_id,
                    title,
                },
            ))
        }

        MuxNotification::PaneBell(pane_id) => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyPaneBell { pane_id: *pane_id },
        )),

        MuxNotification::PaneUrgencyHint(pane_id) => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyPaneUrgencyHint { pane_id: *pane_id },
        )),

        MuxNotification::CommandComplete { pane_id, duration } => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyCommandComplete {
                pane_id: *pane_id,
                duration_ms: duration.as_millis() as u64,
            },
        )),

        MuxNotification::ClipboardStore {
            pane_id,
            clipboard_type,
            text,
        } => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyClipboardStore {
                pane_id: *pane_id,
                clipboard_type: clipboard_type_to_wire(*clipboard_type),
                text: text.clone(),
            },
        )),

        MuxNotification::DesktopNotification {
            pane_id,
            source,
            title,
            body,
        } => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyDesktopNotification {
                pane_id: *pane_id,
                source: (*source).into(),
                title: title.clone(),
                body: body.clone(),
            },
        )),

        MuxNotification::ClearPendingDesktopNotifications(pane_id) => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyClearPendingDesktopNotifications { pane_id: *pane_id },
        )),

        // PaneOutput is intercepted by drain_mux_events before reaching
        // this function. NewTab comes from IPC dispatch, not PTY events.
        // HostClipboardLoad / HostColorQuery carry process-local
        // `ResponseToken`s and are routed through `host_request::host_request_to_pdu`
        // (allocates a request_id + registers the token) rather than the
        // stateless dispatch path. AnimationDeadlineChanged is in-process
        // only; daemon clients re-derive deadlines from snapshot cadence.
        MuxNotification::PaneOutput(_)
        | MuxNotification::NewTab
        | MuxNotification::HostClipboardLoad { .. }
        | MuxNotification::HostColorQuery { .. }
        | MuxNotification::AnimationDeadlineChanged { .. } => None,
    }
}

#[cfg(test)]
mod tests;
