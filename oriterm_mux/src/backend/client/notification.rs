//! Convert daemon push PDUs to [`MuxNotification`]s.
//!
//! The reader thread calls [`pdu_to_notification`] for PDUs that are not
//! handled directly in the reader loop. `NotifyPaneOutput` and
//! `NotifyPaneSnapshot` are intercepted in the reader loop (stored in
//! the `pushed_snapshots` shared map) and never reach this function.

use std::sync::Arc;
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

/// Build an OSC 52 response formatter for a clipboard type.
///
/// The formatter base64-encodes the clipboard text and wraps it in the
/// standard OSC 52 response with BEL terminator. This is used when the
/// original formatter closure was lost over the IPC boundary.
fn osc52_response_formatter(
    clipboard_type: ClipboardType,
) -> Arc<dyn Fn(&str) -> String + Send + Sync> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as B64;

    let letter = match clipboard_type {
        ClipboardType::Clipboard => 'c',
        ClipboardType::Selection => 's',
    };
    Arc::new(move |text: &str| {
        let encoded = B64.encode(text.as_bytes());
        format!("\x1b]52;{letter};{encoded}\x07")
    })
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
        MuxPdu::NotifyClipboardLoad {
            pane_id,
            clipboard_type,
        } => {
            let ct = wire_to_clipboard_type(clipboard_type);
            Some(MuxNotification::ClipboardLoad {
                pane_id,
                clipboard_type: ct,
                formatter: osc52_response_formatter(ct),
            })
        }
        other => {
            log::debug!("unexpected notification PDU: {other:?}");
            None
        }
    }
}
