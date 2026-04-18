//! Notification routing from mux events to IPC push messages.
//!
//! Converts [`MuxNotification`] variants into [`MuxPdu`] notifications and
//! identifies which clients should receive them.

use std::collections::HashMap;

use oriterm_core::ClipboardType;

use crate::pane::Pane;
use crate::{MuxNotification, MuxPdu, PaneId};

/// Which clients should receive a notification.
pub enum TargetClients {
    /// All clients subscribed to a specific pane.
    PaneSubscribers(PaneId),
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

        MuxNotification::ClipboardLoad {
            pane_id,
            clipboard_type,
            ..
        } => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyClipboardLoad {
                pane_id: *pane_id,
                clipboard_type: clipboard_type_to_wire(*clipboard_type),
            },
        )),

        // PaneOutput is intercepted by drain_mux_events before reaching
        // this function. NewTab comes from IPC dispatch, not PTY events.
        // DesktopNotification / ClearPendingDesktopNotifications / HostClipboardLoad /
        // HostColorQuery are added in effect-cutover 01.1; the daemon-wire
        // translation lives in `plans/effect-cutover/section-01-migrate-mux-consumer.md §01.4`
        // (filed as a tracked bug — fulfillment replies require a request-ID
        // protocol; the notification-out side is filed separately if needed).
        // ResponseToken-carrying notifications cannot cross the IPC boundary
        // today (process-local `Arc<Mutex<Option<T>>>`). Clients in embedded
        // mode receive them via `in_process::event_pump`; daemon mode drops
        // them here with a logged warning until the reply-PDU design lands.
        MuxNotification::PaneOutput(_)
        | MuxNotification::NewTab
        | MuxNotification::DesktopNotification { .. }
        | MuxNotification::ClearPendingDesktopNotifications(_)
        | MuxNotification::HostClipboardLoad { .. }
        | MuxNotification::HostColorQuery { .. } => None,
    }
}

#[cfg(test)]
mod tests;
