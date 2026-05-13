//! Server-push snapshot logic.
//!
//! Proactively pushes [`PaneSnapshot`]s to clients that advertised
//! [`CAP_SNAPSHOT_PUSH`]. Push rate is throttled to ~250fps (4ms interval).
//! Clients above the write high-water mark are deferred to a trailing-edge
//! flush that retries once their buffer drains.

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use oriterm_core::ImageId;

use crate::id::ClientId;
use crate::pane::Pane;
use crate::protocol::messages::CAP_SNAPSHOT_PUSH;
use crate::protocol::snapshot::WireImageData;
use crate::{MuxPdu, PaneId, PaneSnapshot};

use super::connection::ClientConnection;
use super::snapshot::SnapshotCache;

/// Minimum interval between snapshot pushes for focused panes (priority 0).
///
/// Set low (4ms / 250fps) so the daemon's push throttle never gates
/// interactive typing. The client's own frame budget (16ms) is the
/// authoritative render cadence — a second unsynchronized 16ms gate
/// here creates visible stutter from 0-32ms beat-frequency jitter.
pub const SNAPSHOT_PUSH_INTERVAL: Duration = Duration::from_millis(4);

/// Push interval for visible but unfocused panes (priority 1, ~60fps).
pub const VISIBLE_PUSH_INTERVAL: Duration = Duration::from_millis(16);

/// Push interval for hidden panes (priority 2+, low overhead).
pub const HIDDEN_PUSH_INTERVAL: Duration = Duration::from_millis(100);

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

/// Map a priority value to its push interval.
pub(super) fn interval_for_priority(priority: u8) -> Duration {
    match priority {
        0 => SNAPSHOT_PUSH_INTERVAL,
        1 => VISIBLE_PUSH_INTERVAL,
        _ => HIDDEN_PUSH_INTERVAL,
    }
}

/// Compute the effective push interval for a pane across all subscribers.
///
/// Uses the highest priority (lowest number) among all subscribers to
/// determine the interval. Returns `SNAPSHOT_PUSH_INTERVAL` (4ms) if no
/// subscribers have an explicit priority set.
pub(super) fn effective_push_interval(
    pane_id: PaneId,
    subscribers: &[ClientId],
    connections: &HashMap<ClientId, ClientConnection>,
) -> Duration {
    let min_priority = subscribers
        .iter()
        .filter_map(|cid| connections.get(cid))
        .map(|conn| conn.pane_priority(pane_id))
        .min()
        .unwrap_or(0);
    interval_for_priority(min_priority)
}

/// Whether enough time has passed since the last push for this pane.
pub(super) fn should_push(now: Instant, last_push: Option<Instant>, interval: Duration) -> bool {
    last_push.is_none_or(|t| now.duration_since(t) >= interval)
}

/// Shared state needed by per-client snapshot dispatch — bundled together so
/// dispatch helpers can take a single broker instead of two separate `&mut`
/// references, keeping caller signatures under the clippy 5-arg limit.
pub(super) struct PushBroker<'a> {
    pub connections: &'a mut HashMap<ClientId, ClientConnection>,
    pub snapshot_cache: &'a mut SnapshotCache,
}

/// Deferred `sent_images` mutations to apply ONLY after a queue succeeds.
///
/// `project_per_client_pure` produces this side-table; callers apply via
/// `apply_to(conn)` AFTER `conn.queue_frame(...)` returns `Ok(())`. Failed
/// queues drop the mutations and the trailing-edge flush retries against
/// the latest snapshot.
/// See: bug-tracker/plans/BUG-06-072/
pub(super) struct PendingImageMutations {
    /// When `Some(pane_id)`, clear that pane's `sent_images` before applying
    /// `mark_sent` (set when `images_dirty == true` — server is resending
    /// every visible ID).
    pub clear_pane: Option<PaneId>,
    /// `(PaneId, ImageId)` pairs to mark as sent (one per referenced placement
    /// in the projected snapshot). Empty when no placements were projected.
    pub mark_sent: Vec<(PaneId, ImageId)>,
}

impl PendingImageMutations {
    /// Apply the deferred mutations onto a connection. Caller invokes this
    /// AFTER the corresponding `queue_frame` succeeds; a failed queue MUST
    /// NOT apply the mutations.
    pub(super) fn apply_to(&self, conn: &mut ClientConnection) {
        if let Some(pane_id) = self.clear_pane {
            conn.clear_sent_images(pane_id);
        }
        for (pane_id, image_id) in &self.mark_sent {
            conn.mark_image_sent(*pane_id, *image_id);
        }
    }
}

/// Pure (no-mutation) per-client snapshot projection.
///
/// Reads `conn` to compute `needed_ids` (placements whose pixel data the
/// client hasn't received yet) but does NOT mutate `conn.sent_images` — the
/// caller applies the returned `PendingImageMutations` AFTER successfully
/// queuing the projected snapshot. This pre-queue-mutation deferral is the
/// success-only contract: a failed queue MUST NOT leave stale
/// `sent_images` state that the trailing-edge flush would otherwise skip.
///
/// Single SSOT for the per-client projection skeleton across both call
/// sites — `project_and_queue_per_client` (push side — applies mutations
/// inline on queue success) AND the dispatch RPC handlers (Subscribe /
/// `GetPaneSnapshot` — return mutations via `DispatchResult` for the caller
/// to apply after `conn.queue_frame` on the response succeeds).
/// See: bug-tracker/plans/BUG-06-072/
pub(super) fn project_per_client_pure(
    pane_id: PaneId,
    snapshot: &PaneSnapshot,
    cid: Option<ClientId>,
    conn: &ClientConnection,
    snapshot_cache: &mut SnapshotCache,
) -> (PaneSnapshot, PendingImageMutations) {
    let needed_ids: Vec<ImageId> = snapshot
        .images
        .iter()
        .map(|wp| ImageId::from_raw(wp.image_id))
        // When images_dirty=true, the post-queue mutations clear sent_images
        // FIRST, so every visible ID needs to be in the wire payload AND in
        // the post-queue mark_sent list. Don't filter by has_sent_image when
        // dirty — we're rebuilding the client's cache from scratch.
        .filter(|id| snapshot.images_dirty || !conn.has_sent_image(pane_id, *id))
        .collect();
    let mut projected = snapshot.clone();
    projected.image_data.clear();
    projected.image_data.reserve(needed_ids.len());
    for id in &needed_ids {
        if let Some(arc) = snapshot_cache.image_data(pane_id, *id) {
            projected.image_data.push(WireImageData {
                id: id.as_u32(),
                data: arc.data.to_vec(),
                width: arc.width,
                height: arc.height,
            });
        } else {
            let cid_repr = cid.map_or_else(|| "rpc".to_string(), |c| c.to_string());
            log::warn!(
                "per-client projection: image_data_store miss for ({pane_id}, {id:?}); placement may render blank on client {cid_repr}"
            );
        }
    }
    let mark_sent: Vec<(PaneId, ImageId)> = snapshot
        .images
        .iter()
        .map(|wp| (pane_id, ImageId::from_raw(wp.image_id)))
        .collect();
    let mutations = PendingImageMutations {
        clear_pane: if snapshot.images_dirty {
            Some(pane_id)
        } else {
            None
        },
        mark_sent,
    };
    (projected, mutations)
}

/// Project a per-client `PaneSnapshot` and queue it onto `conn`. On queue
/// success (and ONLY on success) applies the deferred `sent_images`
/// mutations.
///
/// Returns `Err` if the underlying `queue_frame` failed (transport error);
/// `sent_images` is NOT mutated on the error path so the trailing-edge
/// flush retries against the latest snapshot.
/// See: bug-tracker/plans/BUG-06-072/
fn project_and_queue_per_client(
    pane_id: PaneId,
    snapshot: &PaneSnapshot,
    cid: ClientId,
    conn: &mut ClientConnection,
    snapshot_cache: &mut SnapshotCache,
) -> std::io::Result<()> {
    let (projected, mutations) =
        project_per_client_pure(pane_id, snapshot, Some(cid), conn, snapshot_cache);
    let pdu = MuxPdu::NotifyPaneSnapshot {
        pane_id,
        snapshot: projected,
    };
    let result = conn.queue_frame(0, &pdu);
    if result.is_ok() {
        mutations.apply_to(conn);
    }
    result
}

/// Push a snapshot to all capable subscribers for a pane, respecting
/// backpressure. Subscribers above the write high-water mark are added to
/// `deferred` for trailing-edge retry.
///
/// Clients without `CAP_SNAPSHOT_PUSH` receive a bare `NotifyPaneOutput`
/// instead and are never added to the deferred set.
pub(super) fn push_snapshot_to_subscribers(
    pane_id: PaneId,
    snapshot: &PaneSnapshot,
    subscribers: &[ClientId],
    deferred: &mut HashSet<ClientId>,
    broker: &mut PushBroker<'_>,
) {
    let connections: &mut HashMap<ClientId, ClientConnection> = &mut *broker.connections;
    let snapshot_cache: &mut SnapshotCache = &mut *broker.snapshot_cache;
    let bare_pdu = MuxPdu::NotifyPaneOutput { pane_id };

    // Steady-state fast path: when `!images_dirty` AND every subscriber has
    // every referenced ImageId in `sent_images[pane_id]`, build ONE shared
    // NotifyPaneSnapshot PDU and queue it across all capable subscribers (the
    // shared-PDU shape from the pre-image-cache codepath). Avoids per-subscriber clone in the
    // common case.
    let fast_path_eligible = !snapshot.images_dirty
        && subscribers.iter().all(|cid| {
            let Some(conn) = connections.get(cid) else {
                return true; // disconnected — irrelevant
            };
            if !conn.has_capability(CAP_SNAPSHOT_PUSH) {
                return true; // bare-PDU path doesn't carry image_data
            }
            snapshot
                .images
                .iter()
                .all(|wp| conn.has_sent_image(pane_id, ImageId::from_raw(wp.image_id)))
        });

    if fast_path_eligible {
        // Shared PDU — placements only, no image_data needed.
        let push_pdu = MuxPdu::NotifyPaneSnapshot {
            pane_id,
            snapshot: snapshot.clone(),
        };
        for &cid in subscribers {
            let Some(conn) = connections.get_mut(&cid) else {
                continue;
            };
            if !conn.has_capability(CAP_SNAPSHOT_PUSH) {
                if let Err(e) = conn.queue_frame(0, &bare_pdu) {
                    log::warn!("push bare to {cid} failed: {e}");
                }
                continue;
            }
            if conn.pending_write_bytes() > WRITE_HIGH_WATER {
                deferred.insert(cid);
                continue;
            }
            if let Err(e) = conn.queue_frame(0, &push_pdu) {
                log::warn!("push snapshot to {cid} failed: {e}");
            }
            // sent_images already contains every referenced ID — no update needed.
        }
        return;
    }

    // Per-client slow path: at least one subscriber needs new image_data OR
    // `images_dirty == true`. Build per-client projection.
    for &cid in subscribers {
        let Some(conn) = connections.get_mut(&cid) else {
            continue;
        };
        if !conn.has_capability(CAP_SNAPSHOT_PUSH) {
            if let Err(e) = conn.queue_frame(0, &bare_pdu) {
                log::warn!("push bare to {cid} failed: {e}");
            }
            continue;
        }
        if conn.pending_write_bytes() > WRITE_HIGH_WATER {
            deferred.insert(cid);
            continue;
        }
        if let Err(e) = project_and_queue_per_client(pane_id, snapshot, cid, conn, snapshot_cache)
        {
            log::warn!("push snapshot to {cid} failed: {e}");
            // Don't mutate sent_images on failure — trailing-edge flush
            // retries against the latest snapshot.
        }
    }
}

/// Propagate `(PaneId, ImageId)` evictions from `SnapshotCache.image_data_store`
/// to every connection's `sent_images`. Called after each `build_clone` /
/// `build_and_take` so the next snapshot referencing an evicted ID re-includes
/// its pixel data (per-client filter will see `!has_sent_image` and project the
/// `WireImageData`).
/// See: bug-tracker/plans/BUG-06-072/
pub(super) fn propagate_image_evictions(
    evicted: &[(PaneId, ImageId)],
    connections: &mut HashMap<ClientId, ClientConnection>,
) {
    if evicted.is_empty() {
        return;
    }
    for conn in connections.values_mut() {
        for (pane_id, image_id) in evicted {
            conn.forget_sent_image(*pane_id, *image_id);
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

    let interval = effective_push_interval(pane_id, ctx.scratch, ctx.connections);
    if should_push(now, ctx.last_snapshot_push.get(&pane_id).copied(), interval) {
        if let Some(pane) = ctx.panes.get(&pane_id) {
            let (snap, evicted) = ctx.snapshot_cache.build_clone(pane_id, pane);
            propagate_image_evictions(&evicted, ctx.connections);
            let deferred = ctx.pending_push.entry(pane_id).or_default();
            let mut broker = PushBroker {
                connections: ctx.connections,
                snapshot_cache: ctx.snapshot_cache,
            };
            push_snapshot_to_subscribers(pane_id, &snap, ctx.scratch, deferred, &mut broker);
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
        let interval = ctx
            .subscriptions
            .get(&pane_id)
            .map_or(SNAPSHOT_PUSH_INTERVAL, |s| {
                effective_push_interval(pane_id, s, ctx.connections)
            });
        if !should_push(now, ctx.last_snapshot_push.get(&pane_id).copied(), interval) {
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
        let (snap, evicted) = ctx.snapshot_cache.build_clone(pane_id, pane);
        propagate_image_evictions(&evicted, ctx.connections);

        // Trailing-edge flush uses the same per-client projection logic as
        // push_snapshot_to_subscribers — collect served clients via the
        // scratch buffer, then drop them from `deferred`.
        ctx.scratch.clear();
        let served_clients: Vec<ClientId> = deferred.iter().copied().collect();
        for cid in served_clients {
            let Some(conn) = ctx.connections.get_mut(&cid) else {
                ctx.scratch.push(cid);
                continue;
            };
            if conn.pending_write_bytes() > WRITE_HIGH_WATER {
                continue;
            }
            match project_and_queue_per_client(pane_id, &snap, cid, conn, ctx.snapshot_cache) {
                Ok(()) => ctx.scratch.push(cid),
                Err(e) => log::warn!("trailing push to {cid} failed: {e}"),
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
