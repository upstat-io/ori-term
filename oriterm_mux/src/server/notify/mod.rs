//! Notification routing from mux events to IPC push messages.
//!
//! Converts [`MuxNotification`] variants into [`MuxPdu`] notifications and
//! identifies which clients should receive them.

use std::collections::HashMap;

use crate::pane::Pane;
use crate::{MuxNotification, MuxPdu, PaneId};

/// Which clients should receive a notification.
pub enum TargetClients {
    /// All clients subscribed to a specific pane.
    PaneSubscribers(PaneId),
}

/// Convert a mux notification into a target + PDU pair for IPC dispatch.
///
/// Returns `None` for notifications that aren't pushed over IPC (clipboard
/// operations, command completions, etc.).
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

        // PaneOutput is intercepted by drain_mux_events before reaching
        // this function — included here for match exhaustiveness.
        MuxNotification::PaneOutput(_)
        | MuxNotification::CommandComplete { .. }
        | MuxNotification::ClipboardStore { .. }
        | MuxNotification::ClipboardLoad { .. } => None,
    }
}

#[cfg(test)]
mod tests;
