//! Server-push snapshot logic.
//!
//! Proactively pushes [`PaneSnapshot`]s to clients that advertised
//! [`CAP_SNAPSHOT_PUSH`]. Push rate is throttled to ~60fps (16ms interval).
//! Clients above the write high-water mark are deferred to a trailing-edge
//! flush that retries once their buffer drains.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::id::ClientId;
use crate::protocol::messages::CAP_SNAPSHOT_PUSH;
use crate::{MuxPdu, PaneId, PaneSnapshot};

use super::connection::ClientConnection;
use super::snapshot;

/// Minimum interval between snapshot pushes for the same pane.
pub const SNAPSHOT_PUSH_INTERVAL: Duration = Duration::from_millis(16);

/// Write buffer threshold — skip push entirely above this.
const WRITE_HIGH_WATER: usize = 512 * 1024;

/// Whether enough time has passed since the last push for this pane.
pub fn should_push(now: Instant, last_push: Option<Instant>, interval: Duration) -> bool {
    last_push.is_none_or(|t| now.duration_since(t) >= interval)
}

/// Push a snapshot to all capable subscribers for a pane, respecting
/// backpressure. Returns the set of client IDs that were deferred
/// (above high-water).
///
/// Clients without `CAP_SNAPSHOT_PUSH` receive a bare `NotifyPaneOutput`
/// instead and are never added to the deferred set.
pub fn push_snapshot_to_subscribers(
    pane_id: PaneId,
    snapshot: &PaneSnapshot,
    subscribers: &[ClientId],
    connections: &mut HashMap<ClientId, ClientConnection>,
    deferred: &mut HashSet<ClientId>,
) {
    let push_pdu = MuxPdu::NotifyPaneSnapshot {
        pane_id,
        snapshot: snapshot.clone(),
    };
    let bare_pdu = MuxPdu::NotifyPaneOutput { pane_id };

    for &cid in subscribers {
        let Some(conn) = connections.get_mut(&cid) else {
            continue;
        };
        if !conn.has_capability(CAP_SNAPSHOT_PUSH) {
            // Legacy client — send bare notification (pre-existing path).
            let _ = conn.queue_frame(0, &bare_pdu);
            continue;
        }
        if conn.pending_write_bytes() > WRITE_HIGH_WATER {
            // Backpressured — defer to trailing-edge flush.
            deferred.insert(cid);
            continue;
        }
        let _ = conn.queue_frame(0, &push_pdu);
    }
}

/// Add all capable subscribers to the deferred set for trailing-edge retry.
pub fn defer_all_subscribers(
    pane_id: PaneId,
    subscribers: &[ClientId],
    connections: &HashMap<ClientId, ClientConnection>,
    pending_push: &mut HashMap<PaneId, HashSet<ClientId>>,
) {
    let deferred = pending_push.entry(pane_id).or_default();
    for &cid in subscribers {
        if let Some(conn) = connections.get(&cid) {
            if conn.has_capability(CAP_SNAPSHOT_PUSH) {
                deferred.insert(cid);
            }
        }
    }
}

/// Send bare `NotifyPaneOutput` to subscribers without `CAP_SNAPSHOT_PUSH`.
///
/// Non-capable clients need bare dirty notifications regardless of throttle
/// state so they can trigger RPC-based snapshot refresh.
fn notify_bare_to_non_capable(
    pane_id: PaneId,
    subscribers: &[ClientId],
    connections: &mut HashMap<ClientId, ClientConnection>,
) {
    let bare_pdu = MuxPdu::NotifyPaneOutput { pane_id };
    for &cid in subscribers {
        if let Some(conn) = connections.get_mut(&cid) {
            if !conn.has_capability(CAP_SNAPSHOT_PUSH) {
                let _ = conn.queue_frame(0, &bare_pdu);
            }
        }
    }
}

/// Build and push (or defer) a snapshot for a single pane.
///
/// If the throttle interval has elapsed, builds a snapshot and pushes it
/// to all capable subscribers (with backpressure deferral). Non-capable
/// clients receive a bare `NotifyPaneOutput`.
///
/// If throttled, defers all capable subscribers to `pending_push` for
/// trailing-edge retry and sends bare notifications to non-capable clients.
#[allow(clippy::too_many_arguments)]
pub fn push_or_defer_pane(
    now: Instant,
    pane_id: PaneId,
    last_snapshot_push: &mut HashMap<PaneId, Instant>,
    subscriptions: &HashMap<PaneId, Vec<ClientId>>,
    connections: &mut HashMap<ClientId, ClientConnection>,
    panes: &HashMap<PaneId, crate::pane::Pane>,
    snapshot_cache: &mut HashMap<PaneId, PaneSnapshot>,
    render_buf: &mut oriterm_core::RenderableContent,
    pending_push: &mut HashMap<PaneId, HashSet<ClientId>>,
    scratch: &mut Vec<ClientId>,
) {
    let Some(subs) = subscriptions.get(&pane_id) else {
        return;
    };
    scratch.clear();
    scratch.extend_from_slice(subs);

    if should_push(
        now,
        last_snapshot_push.get(&pane_id).copied(),
        SNAPSHOT_PUSH_INTERVAL,
    ) {
        if let Some(pane) = panes.get(&pane_id) {
            let cached = snapshot_cache.entry(pane_id).or_default();
            snapshot::build_snapshot_into(pane, cached, render_buf);
            let deferred = pending_push.entry(pane_id).or_default();
            push_snapshot_to_subscribers(pane_id, cached, scratch, connections, deferred);
            last_snapshot_push.insert(pane_id, now);
        }
    } else {
        defer_all_subscribers(pane_id, scratch, connections, pending_push);
        notify_bare_to_non_capable(pane_id, scratch, connections);
    }
}

/// Trailing-edge flush: retry deferred pushes for panes whose throttle
/// interval has elapsed.
///
/// For each pane in `pending_push`:
/// 1. Prune stale clients (disconnected, unsubscribed, no capability).
/// 2. If set is empty after pruning, remove entry and skip.
/// 3. If no client is below high-water, skip snapshot build.
/// 4. Otherwise, build snapshot and push to sendable clients.
#[allow(clippy::too_many_arguments)]
pub fn trailing_edge_flush(
    now: Instant,
    pending_push: &mut HashMap<PaneId, HashSet<ClientId>>,
    last_snapshot_push: &mut HashMap<PaneId, Instant>,
    subscriptions: &HashMap<PaneId, Vec<ClientId>>,
    connections: &mut HashMap<ClientId, ClientConnection>,
    panes: &HashMap<PaneId, crate::pane::Pane>,
    snapshot_cache: &mut HashMap<PaneId, PaneSnapshot>,
    render_buf: &mut oriterm_core::RenderableContent,
) {
    // Collect pane IDs to process (can't iterate and mutate simultaneously).
    let pane_ids: Vec<PaneId> = pending_push.keys().copied().collect();

    for pane_id in pane_ids {
        if !should_push(
            now,
            last_snapshot_push.get(&pane_id).copied(),
            SNAPSHOT_PUSH_INTERVAL,
        ) {
            continue;
        }

        let Some(deferred) = pending_push.get_mut(&pane_id) else {
            continue;
        };

        // Prune stale clients.
        let subs = subscriptions.get(&pane_id);
        deferred.retain(|cid| {
            let sub_list = subs.is_some_and(|s| s.contains(cid));
            let connected = connections
                .get(cid)
                .is_some_and(|c| c.has_capability(CAP_SNAPSHOT_PUSH));
            sub_list && connected
        });

        if deferred.is_empty() {
            pending_push.remove(&pane_id);
            continue;
        }

        // Check if any client is below high-water.
        let any_sendable = deferred.iter().any(|cid| {
            connections
                .get(cid)
                .is_some_and(|c| c.pending_write_bytes() <= WRITE_HIGH_WATER)
        });
        if !any_sendable {
            continue; // All above high-water — skip snapshot build.
        }

        // Build snapshot.
        let Some(pane) = panes.get(&pane_id) else {
            pending_push.remove(&pane_id);
            continue;
        };
        let cached = snapshot_cache.entry(pane_id).or_default();
        snapshot::build_snapshot_into(pane, cached, render_buf);

        let push_pdu = MuxPdu::NotifyPaneSnapshot {
            pane_id,
            snapshot: cached.clone(),
        };

        // Push to sendable clients, keep deferred for the rest.
        let mut served = Vec::new();
        for &cid in deferred.iter() {
            let Some(conn) = connections.get_mut(&cid) else {
                served.push(cid);
                continue;
            };
            if conn.pending_write_bytes() <= WRITE_HIGH_WATER {
                let _ = conn.queue_frame(0, &push_pdu);
                served.push(cid);
            }
        }
        for cid in served {
            deferred.remove(&cid);
        }
        if deferred.is_empty() {
            pending_push.remove(&pane_id);
        }

        last_snapshot_push.insert(pane_id, now);
    }
}

#[cfg(test)]
mod tests;
