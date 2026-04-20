//! Convert daemon push PDUs to [`MuxNotification`]s.
//!
//! The reader thread calls [`pdu_to_notification`] for PDUs that are not
//! handled directly in the reader loop. `NotifyPaneOutput` and
//! `NotifyPaneSnapshot` are intercepted in the reader loop (stored in
//! the `pushed_snapshots` shared map) and never reach this function.

use std::time::Duration;

use oriterm_core::ClipboardType;

use crate::mux_event::MuxNotification;
use crate::protocol::MuxPdu;

/// Wire clipboard type → [`ClipboardType`]: 0 = Clipboard, 1 = Selection.
fn wire_to_clipboard_type(wire: u8) -> ClipboardType {
    match wire {
        1 => ClipboardType::Selection,
        _ => ClipboardType::Clipboard,
    }
}

/// Convert a daemon push PDU into a [`MuxNotification`].
///
/// Returns `None` for PDUs that have no direct notification equivalent
/// (logged at debug level).
///
/// Note: `NotifyPaneOutput` and `NotifyPaneSnapshot` are handled directly
/// in the reader loop and should never reach this function.
pub(super) fn pdu_to_notification(pdu: MuxPdu) -> Option<MuxNotification> {
    match pdu {
        MuxPdu::NotifyPaneExited { pane_id, exit_code } => {
            Some(MuxNotification::PaneClosed { pane_id, exit_code })
        }
        MuxPdu::NotifyPaneMetadataChanged { pane_id, .. } => {
            Some(MuxNotification::PaneMetadataChanged(pane_id))
        }
        MuxPdu::NotifyPaneBell { pane_id } => Some(MuxNotification::PaneBell(pane_id)),
        MuxPdu::NotifyCommandComplete {
            pane_id,
            duration_ms,
        } => Some(MuxNotification::CommandComplete {
            pane_id,
            duration: Duration::from_millis(duration_ms),
        }),
        MuxPdu::NotifyClipboardStore {
            pane_id,
            clipboard_type,
            text,
        } => Some(MuxNotification::ClipboardStore {
            pane_id,
            clipboard_type: wire_to_clipboard_type(clipboard_type),
            text,
        }),
        MuxPdu::NotifyClipboardLoad { pane_id, .. } => {
            // Daemon-mode HostRequest replies require a request-ID +
            // reply-PDU design that has not landed yet (tracked in
            // bug-tracker BUG-11-11). The legacy `MuxNotification::ClipboardLoad`
            // closure-carrier was deleted in effect-cutover §01.3 and there
            // is no in-process equivalent for daemon clients to drive a reply.
            // Drop with a logged warning until BUG-11-11 is implemented.
            log::warn!(
                "daemon-mode OSC 52 clipboard load (pane {pane_id}) dropped — \
                 BUG-11-11 (HostRequest IPC) not yet implemented"
            );
            None
        }
        MuxPdu::NotifyNewTab => Some(MuxNotification::NewTab),
        other => {
            log::debug!("unexpected notification PDU: {other:?}");
            None
        }
    }
}
