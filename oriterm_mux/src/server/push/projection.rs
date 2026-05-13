//! Per-client snapshot projection — pure projection helpers + deferred
//! `sent_images` mutation side-table.
//!
//! Extracted from `push/mod.rs` to keep file size under the 500-line cap
//! and to colocate the projection-specific contract in one place. Both the
//! push-side `project_and_queue_per_client` (in `push/mod.rs`) and the
//! dispatch RPC handlers (Subscribe / `GetPaneSnapshot`) call into
//! [`project_per_client_pure`] for the projection skeleton and apply the
//! returned [`PendingImageMutations`] post-queue-success.
//!
//! See: bug-tracker/plans/BUG-06-072/

use std::collections::HashSet;

use oriterm_core::ImageId;

use crate::id::ClientId;
use crate::protocol::snapshot::WireImageData;
use crate::{PaneId, PaneSnapshot};

use super::super::connection::ClientConnection;
use super::super::snapshot::SnapshotCache;

/// Deferred `sent_images` mutations to apply ONLY after a queue succeeds.
///
/// [`project_per_client_pure`] produces this side-table; callers apply via
/// [`Self::apply_to`] AFTER `conn.queue_frame(...)` returns `Ok(())`. Failed
/// queues drop the mutations and the trailing-edge flush retries against
/// the latest snapshot.
/// See: bug-tracker/plans/BUG-06-072/
pub(in crate::server) struct PendingImageMutations {
    /// When `Some(pane_id)`, clear that pane's `sent_images` before applying
    /// `mark_sent` (set when `images_dirty == true` — server is resending
    /// every visible ID).
    pub clear_pane: Option<PaneId>,
    /// `(PaneId, ImageId)` pairs to mark as sent (one per referenced placement
    /// in the projected snapshot, deduplicated). Empty when no placements
    /// were projected.
    pub mark_sent: Vec<(PaneId, ImageId)>,
}

impl PendingImageMutations {
    /// Apply the deferred mutations onto a connection. Caller invokes this
    /// AFTER the corresponding `queue_frame` succeeds; a failed queue MUST
    /// NOT apply the mutations.
    pub(in crate::server) fn apply_to(&self, conn: &mut ClientConnection) {
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
/// caller applies the returned [`PendingImageMutations`] AFTER successfully
/// queuing the projected snapshot. This pre-queue-mutation deferral is the
/// success-only contract: a failed queue MUST NOT leave stale
/// `sent_images` state that the trailing-edge flush would otherwise skip.
///
/// Single SSOT for the per-client projection skeleton across both call
/// sites — `project_and_queue_per_client` (push side — applies mutations
/// inline on queue success) AND the dispatch RPC handlers (Subscribe /
/// `GetPaneSnapshot` — return mutations via `DispatchResult` for the
/// caller to apply after `conn.queue_frame` on the response succeeds).
/// See: bug-tracker/plans/BUG-06-072/
pub(in crate::server) fn project_per_client_pure(
    pane_id: PaneId,
    snapshot: &PaneSnapshot,
    cid: Option<ClientId>,
    conn: &ClientConnection,
    snapshot_cache: &mut SnapshotCache,
) -> (PaneSnapshot, PendingImageMutations) {
    // Dedupe IDs via HashSet — if a snapshot has N placements referencing the
    // same `ImageId` (same image shown in multiple positions), we only need
    // to ship its pixel data ONCE on the wire and mark it sent once.
    // Without dedup, `image_data` would clone the bytes N times per frame.
    let mut needed_seen: HashSet<ImageId> = HashSet::new();
    let needed_ids: Vec<ImageId> = snapshot
        .images
        .iter()
        .map(|wp| ImageId::from_raw(wp.image_id))
        // When images_dirty=true, the post-queue mutations clear sent_images
        // FIRST, so every visible ID needs to be in the wire payload AND in
        // the post-queue mark_sent list. Don't filter by has_sent_image when
        // dirty — we're rebuilding the client's cache from scratch.
        .filter(|id| snapshot.images_dirty || !conn.has_sent_image(pane_id, *id))
        .filter(|id| needed_seen.insert(*id))
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
    // Dedupe mark_sent the same way — N references to the same image only
    // need one `mark_image_sent` (insert into HashSet is idempotent, but
    // dedupe-at-source avoids the redundant calls entirely).
    let mut mark_seen: HashSet<ImageId> = HashSet::new();
    let mark_sent: Vec<(PaneId, ImageId)> = snapshot
        .images
        .iter()
        .map(|wp| ImageId::from_raw(wp.image_id))
        .filter(|id| mark_seen.insert(*id))
        .map(|id| (pane_id, id))
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
