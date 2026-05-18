//! Snapshot-lifecycle inherent helpers for `EmbeddedMux`.
//!
//! Owns the `snapshot_cache` / `renderable_cache` / `snapshot_dirty` state
//! machine. The trait impl in `embedded/mod.rs` delegates to these
//! `pub(super) fn` helpers as 1-line dispatchers per the
//! impl-hygiene "mod.rs dispatches, leaf files implement" pattern.

use std::sync::atomic::Ordering;
use std::time::Duration;

use crate::backend::embedded::EmbeddedMux;
use crate::mux_event::MuxNotification;
use crate::pane::io_thread::PaneIoCommand;
use crate::server::snapshot::fill_snapshot_from_renderable;
use crate::{PaneId, PaneSnapshot};

/// IO-thread barrier timeout for `sync_pane_snapshot` (ms).
///
/// 500 ms is the documented contract for "guaranteed-fresh or None":
/// IO-thread normally responds in sub-millisecond time; if it doesn't
/// respond before this deadline the snapshot would be stale, so we
/// return None and let the caller decide.
const SNAPSHOT_BARRIER_TIMEOUT: Duration = Duration::from_millis(500);

impl EmbeddedMux {
    /// Poll IO-thread events: reset wakeup-pending flag, drain the in-
    /// process mux, then mark each pane with a fresh IO snapshot as
    /// `snapshot_dirty` and emit `PaneOutput` notifications.
    pub(super) fn poll_events_impl(&mut self) {
        self.wakeup_pending.store(false, Ordering::Release);
        self.mux.poll_events(&mut self.panes);

        // Mark panes dirty when the IO thread has produced a new snapshot.
        // Also emit PaneOutput notifications so the app can schedule redraws,
        // invalidate selections, and track unseen output on background tabs.
        for (&pane_id, pane) in &self.panes {
            if pane.has_io_snapshot() {
                self.snapshot_dirty.insert(pane_id);
                self.mux
                    .push_notification(MuxNotification::PaneOutput(pane_id));
            }
        }
    }

    /// Two-phase pane teardown: drain client-local state (bell, snapshot,
    /// renderable caches, dirty flag), then drop the `Pane` on a
    /// background thread to avoid blocking the event loop with PTY kill
    /// + reader thread join + child reap. Matches `MuxClient::cleanup_closed_pane`.
    pub(super) fn cleanup_closed_pane_impl(&mut self, pane_id: PaneId) {
        // bell_panes is populated by App-level focus-gate decisions (set_bell from MuxNotification::PaneBell) for any pane id, independent of whether `self.panes` still holds the Pane object; drain unconditionally to match MuxClient::cleanup_closed_pane and prevent the teardown-race leak.
        self.bell_panes.remove(&pane_id);
        if let Some(pane) = self.panes.remove(&pane_id) {
            self.snapshot_cache.remove(&pane_id);
            self.snapshot_dirty.remove(&pane_id);
            self.renderable_cache.remove(&pane_id);
            // Drop on a background thread to avoid blocking the event loop.
            // Pane destruction involves PTY kill, reader thread join, and child reap.
            std::thread::spawn(move || drop(pane));
        }
    }

    /// Refresh the cached snapshot from the IO thread's latest snapshot.
    ///
    /// Swaps the IO thread's published snapshot into the local render
    /// buffer (no copy), then materializes it into a `PaneSnapshot` via
    /// `fill_snapshot_from_renderable`. Clears the dirty flag.
    pub(super) fn refresh_pane_snapshot_impl(&mut self, pane_id: PaneId) -> Option<&PaneSnapshot> {
        let pane = self.panes.get(&pane_id)?;
        let snapshot = self.snapshot_cache.entry(pane_id).or_default();
        let render_buf = self.renderable_cache.entry(pane_id).or_default();

        // Swap the IO thread's latest snapshot into our render buffer.
        // The IO thread is the sole producer — no lock-based fallback needed.
        if pane.swap_io_snapshot(render_buf) {
            fill_snapshot_from_renderable(pane, render_buf, snapshot);
        }

        self.snapshot_dirty.remove(&pane_id);
        self.snapshot_cache.get(&pane_id)
    }

    /// Force a fresh snapshot via IO-thread barrier, then clone the result.
    ///
    /// Contract per `MuxBackend::sync_pane_snapshot`: "guaranteed-fresh or
    /// None". The IO thread is sent a `SnapshotNow` command; if it doesn't
    /// drain within `SNAPSHOT_BARRIER_TIMEOUT` the caller knows the
    /// result would have been stale and gets `None` instead of a
    /// silently-degraded snapshot.
    pub(super) fn sync_pane_snapshot_impl(&mut self, pane_id: PaneId) -> Option<PaneSnapshot> {
        // Step 1: send the IO thread barrier and wait for it to drain
        // any earlier commands and publish a fresh snapshot.
        let pane = self.panes.get(&pane_id)?;
        let (tx, rx) = crossbeam_channel::bounded(1);
        pane.send_io_command(PaneIoCommand::SnapshotNow { reply: tx });
        if rx.recv_timeout(SNAPSHOT_BARRIER_TIMEOUT).is_err() {
            log::warn!("sync_pane_snapshot({pane_id}) timed out waiting for IO thread barrier");
            return None;
        }

        // Step 2: refresh and clone — refresh_pane_snapshot mutably
        // borrows self, so we can't return its &PaneSnapshot directly.
        let snapshot = self.refresh_pane_snapshot_impl(pane_id)?.clone();
        Some(snapshot)
    }
}
