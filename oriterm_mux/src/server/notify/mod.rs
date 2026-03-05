//! Notification routing from mux events to IPC push messages.
//!
//! Converts [`MuxNotification`] variants into [`MuxPdu`] notifications and
//! identifies which clients should receive them.

use std::collections::HashMap;

use crate::pane::Pane;
use crate::registry::SessionRegistry;
use crate::{MuxNotification, MuxPdu, PaneId, WindowId};

/// Which clients should receive a notification.
pub enum TargetClients {
    /// All clients subscribed to a specific pane.
    PaneSubscribers(PaneId),
    /// The client that owns a specific window.
    WindowClient(WindowId),
}

/// Convert a mux notification into a target + PDU pair for IPC dispatch.
///
/// Returns `None` for notifications that aren't pushed over IPC (clipboard
/// operations, command completions, etc.).
pub fn notification_to_pdu(
    notif: &MuxNotification,
    panes: &HashMap<PaneId, Pane>,
    session: &SessionRegistry,
) -> Option<(TargetClients, MuxPdu)> {
    match notif {
        MuxNotification::PaneDirty(pane_id) => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyPaneOutput { pane_id: *pane_id },
        )),

        MuxNotification::PaneClosed(pane_id) => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyPaneExited { pane_id: *pane_id },
        )),

        MuxNotification::PaneTitleChanged(pane_id) => {
            let title = panes
                .get(pane_id)
                .map(|p| p.effective_title().to_string())
                .unwrap_or_default();
            Some((
                TargetClients::PaneSubscribers(*pane_id),
                MuxPdu::NotifyPaneTitleChanged {
                    pane_id: *pane_id,
                    title,
                },
            ))
        }

        MuxNotification::Alert(pane_id) => Some((
            TargetClients::PaneSubscribers(*pane_id),
            MuxPdu::NotifyPaneBell { pane_id: *pane_id },
        )),

        MuxNotification::WindowTabsChanged(window_id) => Some((
            TargetClients::WindowClient(*window_id),
            MuxPdu::NotifyWindowTabsChanged {
                window_id: *window_id,
            },
        )),

        MuxNotification::TabLayoutChanged(tab_id)
        | MuxNotification::FloatingPaneChanged(tab_id) => tab_layout_changed_pdu(*tab_id, session),

        // Notifications not pushed over IPC.
        MuxNotification::CommandComplete { .. }
        | MuxNotification::WindowClosed(_)
        | MuxNotification::LastWindowClosed
        | MuxNotification::ClipboardStore { .. }
        | MuxNotification::ClipboardLoad { .. } => None,
    }
}

/// Build a `NotifyTabLayoutChanged` PDU for a tab.
///
/// Looks up the tab's current layout from the session and determines the
/// owning window for client routing.
fn tab_layout_changed_pdu(
    tab_id: crate::TabId,
    session: &SessionRegistry,
) -> Option<(TargetClients, MuxPdu)> {
    let tab = session.get_tab(tab_id)?;
    let window_id = session.window_for_tab(tab_id)?;
    Some((
        TargetClients::WindowClient(window_id),
        MuxPdu::NotifyTabLayoutChanged {
            tab_id,
            tree: tab.tree().clone(),
            floating: tab.floating().clone(),
            active_pane: tab.active_pane(),
            zoomed_pane: tab.zoomed_pane(),
        },
    ))
}

#[cfg(test)]
mod tests;
