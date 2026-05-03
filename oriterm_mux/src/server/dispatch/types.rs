//! Types shared across the dispatch submodule.

use std::collections::HashMap;
use std::sync::Arc;

use crate::id::HostRequestId;
use crate::in_process::InProcessMux;
use crate::pane::Pane;
use crate::{MuxPdu, PaneId};

use super::super::host_request::PendingHostReply;
use super::super::snapshot::SnapshotCache;

/// Side effects returned from [`super::dispatch_request`].
///
/// Moves PDU-internal routing decisions out of the caller and into the
/// dispatch function. The caller reads named fields instead of inspecting
/// the raw PDU.
pub(in crate::server) struct DispatchResult {
    /// Response PDU to send back to the client.
    pub response: Option<MuxPdu>,
    /// PDU to broadcast to all OTHER connected clients (excludes sender).
    pub broadcast: Option<MuxPdu>,
    /// Whether the request changed subscription state (Subscribe/Unsubscribe).
    pub sub_changed: bool,
    /// Pane that was unsubscribed (for `pending_push` cleanup).
    pub unsubscribed_pane: Option<PaneId>,
}

/// Shared context for request dispatch.
///
/// Groups the server-owned state that `dispatch_request` needs. Avoids
/// threading 6+ scratch buffers as individual parameters.
pub(in crate::server) struct DispatchContext<'a> {
    pub mux: &'a mut InProcessMux,
    pub panes: &'a mut HashMap<PaneId, Pane>,
    pub wakeup: &'a Arc<dyn Fn() + Send + Sync>,
    pub closed_panes: &'a mut Vec<PaneId>,
    pub snapshot_cache: &'a mut SnapshotCache,
    #[allow(
        dead_code,
        reason = "read by clients.rs; dispatch no longer populates after IO thread migration"
    )]
    pub immediate_push: &'a mut Vec<PaneId>,
    /// Pending host-request tokens (BUG-11-011). Threaded into dispatch so
    /// the `ReplyHostRequest` arm can fulfill the matching token + the
    /// `Unsubscribe` arm can drop entries the unsubscribing client owned.
    pub pending_host_replies: &'a mut HashMap<HostRequestId, PendingHostReply>,
}

impl DispatchContext<'_> {
    /// Take a pending host-reply entry, validating that the caller is the
    /// expected responder. Returns `None` (and logs a warn) when the
    /// `request_id` is unknown or the responder mismatch — both surface
    /// as routing bugs (the daemon should never route a request to client
    /// A and accept a reply from client B).
    pub(in crate::server) fn take_validated_pending_host_reply(
        &mut self,
        request_id: HostRequestId,
        responder: crate::id::ClientId,
    ) -> Option<PendingHostReply> {
        let entry = self.pending_host_replies.get(&request_id)?;
        if entry.responder != responder {
            log::warn!(
                "ReplyHostRequest: {request_id} routed to {} but reply came from {responder} (drop)",
                entry.responder
            );
            return None;
        }
        self.pending_host_replies.remove(&request_id)
    }
}
