//! Shared dispatch helpers — theme parsing, pane cleanup, subscription removal, large-arm extractions.

use std::collections::HashMap;
use std::time::Duration;

use oriterm_core::{Selection, Theme};

use crate::MuxPdu;
use crate::PaneId;
use crate::id::ClientId;
use crate::pane::Pane;
use crate::pane::io_thread::PaneIoCommand;

/// Drop a pane on a background thread to avoid blocking the server event loop.
///
/// `Pane::drop` signals shutdown and kills the child process, but the field
/// destructors (especially `PtyHandle.child`) can block on Windows/ConPTY
/// cleanup. Spawning a thread ensures the server responds to RPCs promptly.
pub(in crate::server) fn drop_pane_background(pane: Option<Pane>) {
    if let Some(pane) = pane {
        std::thread::spawn(move || drop(pane));
    }
}

/// Parse a wire theme string into a [`Theme`].
///
/// `None` or unrecognized strings default to [`Theme::Dark`].
pub(in crate::server) fn parse_theme(s: Option<&str>) -> Theme {
    match s {
        Some("light") => Theme::Light,
        _ => Theme::Dark,
    }
}

/// Build the `HelloAck` or `Error` PDU for a client `Hello` request.
///
/// Accepts ONLY an exact-equal protocol-version match. The wire format
/// (bincode-encoded `PaneSnapshot` + `MuxPdu` codepoints) is neither
/// forward- nor backward-compatible across major versions: a v1 client
/// against a v2 daemon would silently misdecode every snapshot, and
/// vice versa. The asymmetric `>` check used previously accepted older
/// clients into a newer daemon and produced corrupt rendering; the
/// equality check catches both sides.
pub(in crate::server) fn dispatch_hello(
    client_id: ClientId,
    pid: u32,
    protocol_version: u8,
    features: u64,
) -> MuxPdu {
    if protocol_version == crate::protocol::CURRENT_PROTOCOL_VERSION {
        let server_features = crate::protocol::FEAT_ZSTD;
        let negotiated = features & server_features;
        log::info!(
            "client {client_id} handshake (pid={pid}, v={protocol_version}, features=0x{negotiated:X})",
        );
        MuxPdu::HelloAck {
            client_id,
            protocol_version: crate::protocol::CURRENT_PROTOCOL_VERSION,
            features: negotiated,
        }
    } else {
        log::warn!(
            "client {client_id} version mismatch: client={protocol_version}, server={}",
            crate::protocol::CURRENT_PROTOCOL_VERSION,
        );
        MuxPdu::Error {
            message: format!(
                "version mismatch: server speaks v{}, client wants v{protocol_version}",
                crate::protocol::CURRENT_PROTOCOL_VERSION,
            ),
        }
    }
}

/// Synchronously extract plain text for `selection` from `pane`'s IO thread,
/// or return empty string on missing pane / IO-thread timeout. Wraps the
/// `crossbeam_channel::bounded(1) + send_io_command + recv_timeout(100ms)`
/// pattern used by the daemon-mode `ExtractText` handler.
pub(in crate::server) fn dispatch_extract_text(
    pane: Option<&Pane>,
    selection: Selection,
) -> String {
    let Some(pane) = pane else { return String::new(); };
    let (tx, rx) = crossbeam_channel::bounded(1);
    pane.send_io_command(PaneIoCommand::ExtractText { selection, reply: tx });
    rx.recv_timeout(Duration::from_millis(100))
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Synchronously extract HTML + plain text for `selection` from `pane`'s IO
/// thread, or return `(String::new(), String::new())` on missing pane /
/// IO-thread timeout. Companion to [`dispatch_extract_text`] for the
/// HTML clipboard path.
pub(in crate::server) fn dispatch_extract_html(
    pane: Option<&Pane>,
    selection: Selection,
    font_family: String,
    font_size: f32,
) -> (String, String) {
    let Some(pane) = pane else { return (String::new(), String::new()); };
    let (tx, rx) = crossbeam_channel::bounded(1);
    pane.send_io_command(PaneIoCommand::ExtractHtml {
        selection,
        font_family,
        font_size,
        reply: tx,
    });
    rx.recv_timeout(Duration::from_millis(100))
        .ok()
        .flatten()
        .unwrap_or_else(|| (String::new(), String::new()))
}

/// Remove all pane subscriptions from the global subscriptions map for a
/// disconnecting client.
pub(in crate::server) fn remove_client_subscriptions(
    subscriptions: &mut HashMap<PaneId, Vec<ClientId>>,
    client_id: ClientId,
    subscribed_panes: &std::collections::HashSet<PaneId>,
) {
    for pane_id in subscribed_panes {
        if let Some(subs) = subscriptions.get_mut(pane_id) {
            subs.retain(|&c| c != client_id);
            if subs.is_empty() {
                subscriptions.remove(pane_id);
            }
        }
    }
}
