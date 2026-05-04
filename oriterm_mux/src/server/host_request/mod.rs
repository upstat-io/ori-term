//! Daemon-mode `HostRequest` dispatch — allocate request IDs, select a
//! single responder, package OSC parameters into wire PDUs ().
//!
//! Two notification variants carry process-local `ResponseToken`s:
//! `HostClipboardLoad` and `HostColorQuery`. Neither token can cross IPC,
//! so the daemon allocates a server-monotonic `HostRequestId`, packages
//! the OSC parameters into a `Notify…` PDU, and stores the token in
//! `MuxServer::pending_host_replies` until the responding client returns
//! a matching `ReplyHostRequest`. The reply payload re-fulfills the
//! token, which the IO thread observes via its existing `response_poll`
//! loop and writes the formatted OSC reply back to the pane's PTY.

use std::collections::HashMap;

use oriterm_core::color::Rgb;
use oriterm_core::effect::ResponseToken;

use crate::id::{ClientId, HostRequestId, IdAllocator, PaneId};
use crate::mux_event::MuxNotification;
use crate::protocol::{MuxPdu, WireClipboardSelection};
use crate::server::ClientConnection;
use crate::server::notify::TargetClients;

/// Daemon-side bookkeeping for an in-flight host request.
pub(crate) struct PendingHostReply {
    /// Pane that originated the request.
    pub pane_id: PaneId,
    /// Client picked by `select_responder` to answer this request.
    pub responder: ClientId,
    /// Typed token to fulfill on reply.
    pub kind: PendingHostReplyKind,
}

/// Per-kind token storage for `PendingHostReply`.
pub(crate) enum PendingHostReplyKind {
    /// OSC 52 clipboard load — fulfilled with the read text.
    Clipboard(ResponseToken<String>),
    /// OSC color query — fulfilled with the resolved RGB triple.
    Color(ResponseToken<Rgb>),
}

/// Output of `host_request_to_pdu` — bundles the routing target, the wire
/// PDU, the allocated request id, and the bookkeeping entry to register
/// in `pending_host_replies` AFTER successful queueing.
pub(crate) struct HostRequestDispatch {
    /// Single-responder routing target.
    pub target: TargetClients,
    /// Wire PDU to queue to the responder.
    pub pdu: MuxPdu,
    /// Allocated server-monotonic request id.
    pub request_id: HostRequestId,
    /// Token + responder bookkeeping to store on success.
    pub pending: PendingHostReply,
}

/// Mutable context for `host_request_to_pdu`. Threads the id allocator and
/// the chosen responder through without re-borrowing `MuxServer`.
pub(crate) struct HostRequestDispatchCtx<'a> {
    /// Server's `HostRequestId` allocator.
    pub request_alloc: &'a mut IdAllocator<HostRequestId>,
    /// Responder client id chosen by `select_responder`.
    pub responder: ClientId,
}

/// Convert a host-request `MuxNotification` into a wire PDU + bookkeeping.
///
/// Allocates a `HostRequestId`, packages the OSC parameters into a
/// `Notify…` PDU, and bundles the token into a `PendingHostReply` for
/// the caller to insert into `pending_host_replies` AFTER successful
/// queueing (-002 round 1 finding: rolling back a register-then-fail
/// would leak a token entry for a request the client never receives).
///
/// Returns `None` for non-host-request notifications — those flow through
/// the stateless `notify::notification_to_pdu` path instead.
pub(crate) fn host_request_to_pdu(
    notif: MuxNotification,
    ctx: &mut HostRequestDispatchCtx<'_>,
) -> Option<HostRequestDispatch> {
    match notif {
        MuxNotification::HostClipboardLoad {
            pane_id,
            selection,
            clipboard_char,
            terminator,
            reply,
        } => {
            let request_id = ctx.request_alloc.alloc();
            let pdu = MuxPdu::NotifyHostClipboardLoad {
                pane_id,
                request_id: request_id.raw(),
                selection: WireClipboardSelection::from(selection),
                clipboard_char,
                terminator,
            };
            Some(HostRequestDispatch {
                target: TargetClients::SinglePaneSubscriber(pane_id, ctx.responder),
                pdu,
                request_id,
                pending: PendingHostReply {
                    pane_id,
                    responder: ctx.responder,
                    kind: PendingHostReplyKind::Clipboard(reply),
                },
            })
        }
        MuxNotification::HostColorQuery {
            pane_id,
            prefix,
            index,
            terminator,
            reply,
        } => {
            let request_id = ctx.request_alloc.alloc();
            // Palette index is bounded at 270 entries (256 ANSI + 10 dynamic
            // + 4 OSC 4 reachable extras) per `oriterm_core::Palette`, so
            // `usize → u32` is lossless on every supported target. The
            // `debug_assert!` documents + enforces the invariant; a release
            // build produces a direct cast (no fabricated `u32::MAX`
 // saturation:lossy-fallback`).
            debug_assert!(
                index < 270,
                "color index {index} exceeds palette bound (270); cast to u32 would lose data"
            );
            let pdu = MuxPdu::NotifyHostColorQuery {
                pane_id,
                request_id: request_id.raw(),
                prefix,
                index: index as u32,
                terminator,
            };
            Some(HostRequestDispatch {
                target: TargetClients::SinglePaneSubscriber(pane_id, ctx.responder),
                pdu,
                request_id,
                pending: PendingHostReply {
                    pane_id,
                    responder: ctx.responder,
                    kind: PendingHostReplyKind::Color(reply),
                },
            })
        }
        _ => None,
    }
}

/// Pure priority-comparison core for `select_responder`.
///
/// Lowest priority value wins (`0` = focused beats `2` = hidden); ties are
/// broken deterministically by `ClientId` numerical order. Returns `None`
/// when `candidates` is empty. Pure-function shape so unit tests can
/// exercise priority comparison + tie-break without constructing
/// `ClientConnection` (which requires an `IpcStream`).
pub(crate) fn pick_lowest_priority(candidates: &[(ClientId, u8)]) -> Option<ClientId> {
    candidates
        .iter()
        .copied()
        .min_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.raw().cmp(&b.0.raw())))
        .map(|(cid, _)| cid)
}

/// Pick the highest-priority subscribed client for a pane.
///
/// Builds the candidate list from `subscriptions[pane_id]` (priorities looked
/// up in `connections`; disconnected clients fall back to `u8::MAX` so they
/// always lose to live ones), then delegates to `pick_lowest_priority`.
/// Returns `None` when no client is subscribed to the pane.
pub(crate) fn select_responder(
    subscriptions: &HashMap<PaneId, Vec<ClientId>>,
    connections: &HashMap<ClientId, ClientConnection>,
    pane_id: PaneId,
) -> Option<ClientId> {
    let subs = subscriptions.get(&pane_id)?;
    let candidates: Vec<(ClientId, u8)> = subs
        .iter()
        .map(|&cid| {
            let priority = connections
                .get(&cid)
                .map_or(u8::MAX, |c| c.pane_priority(pane_id));
            (cid, priority)
        })
        .collect();
    pick_lowest_priority(&candidates)
}

#[cfg(test)]
mod tests;
