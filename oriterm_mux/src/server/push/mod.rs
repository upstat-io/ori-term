//! Server-push snapshot logic.
//!
//! Proactively pushes [`PaneSnapshot`]s to clients that advertised
//! [`CAP_SNAPSHOT_PUSH`]. Push rate is throttled to ~250fps (4ms interval).
//! Clients above the write high-water mark are deferred to a trailing-edge
//! flush that retries once their buffer drains.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::id::ClientId;
use crate::pane::Pane;
use crate::protocol::messages::CAP_SNAPSHOT_PUSH;
use crate::{MuxPdu, PaneId, PaneSnapshot};

use super::connection::ClientConnection;
use super::snapshot::SnapshotCache;

/// Minimum interval between snapshot pushes for the same pane.
///
/// Set low (4ms / 250fps) so the daemon's push throttle never gates
/// interactive typing. The client's own frame budget (16ms) is the
/// authoritative render cadence — a second unsynchronized 16ms gate
/// here creates visible stutter from 0-32ms beat-frequency jitter.
pub const SNAPSHOT_PUSH_INTERVAL: Duration = Duration::from_millis(4);

/// Write buffer threshold — skip push entirely above this.
const WRITE_HIGH_WATER: usize = 512 * 1024;

/// Shared context for push operations.
///
/// Groups the server-owned state that push functions need. Avoids
/// threading 7+ scratch buffers as individual parameters.
pub(super) struct PushContext<'a> {
    pub last_snapshot_push: &'a mut HashMap<PaneId, Instant>,
    pub subscriptions: &'a HashMap<PaneId, Vec<ClientId>>,
    pub connections: &'a mut HashMap<ClientId, ClientConnection>,
    pub panes: &'a HashMap<PaneId, Pane>,
    pub snapshot_cache: &'a mut SnapshotCache,
    pub pending_push: &'a mut HashMap<PaneId, HashSet<ClientId>>,
    pub scratch: &'a mut Vec<ClientId>,
    pub scratch_panes: &'a mut Vec<PaneId>,
}

/// Whether enough time has passed since the last push for this pane.
pub(super) fn should_push(now: Instant, last_push: Option<Instant>, interval: Duration) -> bool {
    last_push.is_none_or(|t| now.duration_since(t) >= interval)
}

/// Push a snapshot to all capable subscribers for a pane, respecting
/// backpressure. Returns the set of client IDs that were deferred
/// (above high-water).
///
/// Clients without `CAP_SNAPSHOT_PUSH` receive a bare `NotifyPaneOutput`
/// instead and are never added to the deferred set.
pub(super) fn push_snapshot_to_subscribers(
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
            if let Err(e) = conn.queue_frame(0, &bare_pdu) {
                log::warn!("push bare to {cid} failed: {e}");
            }
            continue;
        }
        if conn.pending_write_bytes() > WRITE_HIGH_WATER {
            // Backpressured — defer to trailing-edge flush.
            deferred.insert(cid);
            continue;
        }
        if let Err(e) = conn.queue_frame(0, &push_pdu) {
            log::warn!("push snapshot to {cid} failed: {e}");
        }
    }
}

/// Add all capable subscribers to the deferred set for trailing-edge retry.
pub(super) fn defer_all_subscribers(
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
                if let Err(e) = conn.queue_frame(0, &bare_pdu) {
                    log::warn!("push bare to {cid} failed: {e}");
                }
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
pub fn push_or_defer_pane(ctx: &mut PushContext<'_>, now: Instant, pane_id: PaneId) {
    let Some(subs) = ctx.subscriptions.get(&pane_id) else {
        return;
    };
    ctx.scratch.clear();
    ctx.scratch.extend_from_slice(subs);

    if should_push(
        now,
        ctx.last_snapshot_push.get(&pane_id).copied(),
        SNAPSHOT_PUSH_INTERVAL,
    ) {
        if let Some(pane) = ctx.panes.get(&pane_id) {
            let snap = ctx.snapshot_cache.build_clone(pane_id, pane);
            let deferred = ctx.pending_push.entry(pane_id).or_default();
            push_snapshot_to_subscribers(pane_id, &snap, ctx.scratch, ctx.connections, deferred);
            ctx.last_snapshot_push.insert(pane_id, now);
        }
    } else {
        defer_all_subscribers(pane_id, ctx.scratch, ctx.connections, ctx.pending_push);
        notify_bare_to_non_capable(pane_id, ctx.scratch, ctx.connections);
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
pub fn trailing_edge_flush(ctx: &mut PushContext<'_>, now: Instant) {
    // Collect pane IDs into scratch buffer (can't iterate and mutate simultaneously).
    ctx.scratch_panes.clear();
    ctx.scratch_panes.extend(ctx.pending_push.keys().copied());

    for i in 0..ctx.scratch_panes.len() {
        let pane_id = ctx.scratch_panes[i];
        if !should_push(
            now,
            ctx.last_snapshot_push.get(&pane_id).copied(),
            SNAPSHOT_PUSH_INTERVAL,
        ) {
            continue;
        }

        let Some(deferred) = ctx.pending_push.get_mut(&pane_id) else {
            continue;
        };

        // Prune stale clients.
        let subs = ctx.subscriptions.get(&pane_id);
        deferred.retain(|cid| {
            let sub_list = subs.is_some_and(|s| s.contains(cid));
            let connected = ctx
                .connections
                .get(cid)
                .is_some_and(|c| c.has_capability(CAP_SNAPSHOT_PUSH));
            sub_list && connected
        });

        if deferred.is_empty() {
            ctx.pending_push.remove(&pane_id);
            continue;
        }

        // Check if any client is below high-water.
        let any_sendable = deferred.iter().any(|cid| {
            ctx.connections
                .get(cid)
                .is_some_and(|c| c.pending_write_bytes() <= WRITE_HIGH_WATER)
        });
        if !any_sendable {
            continue; // All above high-water — skip snapshot build.
        }

        // Build snapshot.
        let Some(pane) = ctx.panes.get(&pane_id) else {
            ctx.pending_push.remove(&pane_id);
            continue;
        };
        let snap = ctx.snapshot_cache.build_clone(pane_id, pane);

        let push_pdu = MuxPdu::NotifyPaneSnapshot {
            pane_id,
            snapshot: snap,
        };

        // Push to sendable clients, keep deferred for the rest.
        // Reuse scratch buffer for tracking served clients.
        ctx.scratch.clear();
        for &cid in deferred.iter() {
            let Some(conn) = ctx.connections.get_mut(&cid) else {
                ctx.scratch.push(cid);
                continue;
            };
            if conn.pending_write_bytes() <= WRITE_HIGH_WATER {
                if let Err(e) = conn.queue_frame(0, &push_pdu) {
                    log::warn!("trailing push to {cid} failed: {e}");
                }
                ctx.scratch.push(cid);
            }
        }
        for &cid in ctx.scratch.iter() {
            deferred.remove(&cid);
        }
        if deferred.is_empty() {
            ctx.pending_push.remove(&pane_id);
        }

        ctx.last_snapshot_push.insert(pane_id, now);
    }
}

#[cfg(test)]
mod tests;
