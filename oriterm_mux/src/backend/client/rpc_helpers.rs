//! Long-body RPC helpers for `MuxClient`.
//!
//! Extracted from `client/rpc_methods.rs` to keep that file under the
//! 500-line budget. Each helper is a `pub(super) fn` inherent method on
//! `MuxClient`; the trait impl in `rpc_methods.rs` delegates as 1-line
//! calls per the impl-hygiene "mod.rs dispatches, leaf files implement"
//! pattern.

use std::io;

use oriterm_core::Theme;

use crate::backend::client::MuxClient;
use crate::backend::{HostReply, MuxBackend};
use crate::domain::SpawnConfig;
use crate::in_process::ClosePaneResult;
use crate::mux_event::MuxNotification;
use crate::protocol::{HostReplyPayload, theme_to_wire};
use crate::{MuxPdu, PaneId};

impl MuxClient {
    /// Drain pending wire notifications from the daemon, scanning `PaneOutput`
    /// arms to mark dirty for the next render. Bell-state notifications are
    /// NOT mutated here — the App's `MuxNotification::PaneBell` arm owns
    /// the focus-gated `set_bell` decision.
    pub(super) fn poll_events_impl(&mut self) {
        if let Some(transport) = &self.transport {
            transport.clear_wakeup_pending();
            transport.poll_notifications(&mut self.notifications);
        }

        for notif in &self.notifications {
            if let MuxNotification::PaneOutput(pane_id) = notif {
                self.dirty_panes.insert(*pane_id);
            }
        }
    }

    /// Issue a `SpawnPane` RPC, subscribe to the new pane on success,
    /// and roll the subscription back via `close_pane` on subscribe
    /// failure (to avoid stranding panes in the daemon).
    pub(super) fn spawn_pane_impl(
        &mut self,
        config: &SpawnConfig,
        theme: Theme,
    ) -> io::Result<PaneId> {
        let pdu = MuxPdu::SpawnPane {
            shell: config.shell.clone(),
            cwd: config.cwd.as_ref().map(|p| p.display().to_string()),
            theme: theme_to_wire(theme).map(str::to_owned),
        };

        match self.rpc(pdu)? {
            MuxPdu::SpawnPaneResponse { pane_id } => {
                if let Err(e) = self.subscribe(pane_id) {
                    self.close_pane(pane_id);
                    return Err(e);
                }
                log::info!("daemon spawned pane {pane_id}");
                Ok(pane_id)
            }
            other => Err(io::Error::other(format!(
                "spawn_pane: unexpected response: {other:?}"
            ))),
        }
    }

    /// Issue a `ClosePane` RPC and remove the cached snapshot on
    /// `PaneClosedAck`. Logs and returns `NotFound` on unexpected
    /// response / RPC error to match the in-process backend's
    /// fail-soft semantics.
    pub(super) fn close_pane_impl(&mut self, pane_id: PaneId) -> ClosePaneResult {
        match self.rpc(MuxPdu::ClosePane { pane_id }) {
            Ok(MuxPdu::PaneClosedAck) => {
                self.remove_snapshot(pane_id);
                ClosePaneResult::PaneRemoved
            }
            Ok(other) => {
                log::error!("close_pane: unexpected response: {other:?}");
                ClosePaneResult::NotFound
            }
            Err(e) => {
                log::error!("close_pane: RPC failed: {e}");
                ClosePaneResult::NotFound
            }
        }
    }

    /// Package a [`HostReply`] (clipboard text / palette color) into a
    /// `ReplyHostRequest` PDU and fire it back to the daemon. Looks up
    /// the pending entry by token slot to recover the daemon-allocated
    /// `request_id` that must be echoed for correlation.
    pub(super) fn fulfill_host_request_impl(
        &mut self,
        _pane_id: PaneId,
        reply: HostReply,
    ) -> io::Result<()> {
        let transport = self.transport.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "not connected to daemon")
        })?;
        let (slot_id, payload) = match reply {
            HostReply::ClipboardLoad { token, text } => {
                (token.slot_id(), HostReplyPayload::ClipboardLoad { text })
            }
            HostReply::ColorQuery { token, color } => (
                token.slot_id(),
                HostReplyPayload::ColorQuery {
                    rgb: [color.r, color.g, color.b],
                },
            ),
        };
        let Some(pending) = transport.take_pending_reply(slot_id) else {
            log::warn!(
                "fulfill_host_request: no pending reply for slot_id {slot_id:?} \
                 (token already replied or never registered)"
            );
            return Ok(());
        };
        let request_id = pending.request_id();
        transport.try_fire_and_forget(MuxPdu::ReplyHostRequest {
            request_id,
            payload,
        })
    }
}
