//! IPC client backend for daemon mode.
//!
//! [`MuxClient`] implements [`MuxBackend`] by sending requests to a
//! [`MuxServer`](crate::server::MuxServer) over an IPC socket. Pane data
//! is not available locally — rendering uses `PaneSnapshot`s fetched from
//! the daemon.

mod notification;
mod rpc_helpers;
mod rpc_methods;
mod transport;

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use oriterm_core::{ImageId, RenderableImageData};

use crate::PaneId;
use crate::PaneSnapshot;
use crate::image_cache::ImageCache;
use crate::mux_event::MuxNotification;
use crate::protocol::MuxPdu;

use self::transport::ClientTransport;

/// IPC client backend for daemon mode.
/// Sends mux operations to the daemon over an IPC socket and blocks on
/// responses. Pane data is not stored locally — the daemon owns all
/// terminal state. A background reader thread receives push notifications
/// from the daemon and buffers them for [`drain_notifications`].
/// Cached [`PaneSnapshot`]s are stored locally for rendering. The dirty
/// set tracks which panes have received `PaneOutput` notifications since
/// the last render. The render path checks dirty, fetches a fresh
/// snapshot via RPC, and clears the flag.
pub struct MuxClient {
    /// IPC transport (reader thread + socket). `None` when test-only stub.
    transport: Option<ClientTransport>,

    /// Daemon socket path (stored for reconnection).
    socket_path: Option<PathBuf>,

    /// Event loop wakeup callback (stored for reconnection).
    wakeup: Option<Arc<dyn Fn() + Send + Sync>>,

    /// Buffered notifications from the background reader thread.
    notifications: Vec<MuxNotification>,

    /// Cached pane snapshots for daemon-mode rendering.
    pane_snapshots: HashMap<PaneId, PaneSnapshot>,

    /// Panes with pending content updates (from `PaneOutput` notifications).
    dirty_panes: HashSet<PaneId>,

    /// Panes awaiting an async pushed snapshot after a non-blocking
    /// `MarkAllDirty` request. Prevents `clear_pane_snapshot_dirty` from
    /// clearing the dirty flag prematurely (before the snapshot arrives).
    pending_refresh: HashSet<PaneId>,

    /// Client-local bell-indicator state. The SSOT for `has_bell()` queries
    /// in daemon mode — decoupled from `pane_snapshots` so a daemon-pushed
    /// snapshot replace cannot relight a locally-cleared bell. Populated
    /// by the App's `MuxNotification::PaneBell` arm via `set_bell`; cleared
    /// by `clear_bell` (focus-clear path) and `cleanup_closed_pane`. Bells
    /// are transient client UI state, not server-replicated.
    bell_panes: HashSet<PaneId>,

    /// Per-`(PaneId, ImageId)` decoded image pixel data resolved from wire
    /// snapshots. Bounded LRU with reachability-bounded eviction (entries
    /// referenced by the latest `pane_snapshots[pane_id].images` are never
    /// evicted under memory pressure — soft cap, correctness wins).
    /// Populated in `cache_snapshot` by draining `snapshot.image_data` BEFORE
    /// storing the stripped snapshot. Consulted by `extract_frame_from_snapshot`
    /// when a `WirePlacement` arrives without its `WireImageData` (steady-state
    /// path: server filtered out the bytes because they were already sent).
    /// `Mutex` provides interior mutability so `MuxBackend::pane_image_data`
    /// can return `Option<Arc<RenderableImageData>>` with `&self` API surface.
    /// Lock scope is short — one map lookup + Arc clone per access.
    image_cache: Arc<Mutex<ImageCache>>,
}

impl MuxClient {
    /// Connect to a running daemon at `socket_path`.
    /// Performs the Hello handshake and spawns the background reader thread.
    /// `wakeup` is called when push notifications arrive (wakes the event loop).
    pub fn connect(
        socket_path: &std::path::Path,
        wakeup: Arc<dyn Fn() + Send + Sync>,
    ) -> io::Result<Self> {
        let transport = ClientTransport::connect(socket_path, Arc::clone(&wakeup))?;
        log::info!("MuxClient connected, client_id={}", transport.client_id());
        Ok(Self {
            transport: Some(transport),
            socket_path: Some(socket_path.to_path_buf()),
            wakeup: Some(wakeup),
            notifications: Vec::new(),
            pane_snapshots: HashMap::new(),
            dirty_panes: HashSet::new(),
            pending_refresh: HashSet::new(),
            bell_panes: HashSet::new(),
            image_cache: Arc::new(Mutex::new(ImageCache::new())),
        })
    }

    /// Create an unconnected client stub for testing.
    /// All RPC methods will fail gracefully (return defaults or errors).
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            transport: None,
            socket_path: None,
            wakeup: None,
            notifications: Vec::new(),
            pane_snapshots: HashMap::new(),
            dirty_panes: HashSet::new(),
            pending_refresh: HashSet::new(),
            bell_panes: HashSet::new(),
            image_cache: Arc::new(Mutex::new(ImageCache::new())),
        }
    }

    /// Test-only helper: inject a notification into the buffer so `poll_events`
    /// can scan it without going through the real transport. Used by tests
    /// that pin the no-op-mutation contract on `poll_events`.
    #[cfg(test)]
    pub fn inject_notification(&mut self, notif: MuxNotification) {
        self.notifications.push(notif);
    }

    /// Cache a snapshot for a pane (used when subscribe responses arrive).
    /// SSOT for all client snapshot ingest paths — drains `snapshot.image_data`
    /// into `self.image_cache` (per-pane keyed bounded LRU) BEFORE storing the
    /// stripped snapshot in `pane_snapshots`. Without the drain, the unbounded
    /// `pane_snapshots` map would retain megabytes of pixel data per pane.
    pub(crate) fn cache_snapshot(&mut self, pane_id: PaneId, mut snapshot: PaneSnapshot) {
        if !snapshot.image_data.is_empty() {
            // Compute the reachability set BEFORE locking the cache: this
            // pane's new placements + every other cached pane's placements.
            // The oracle is a pure closure — no nested locks, no borrow
            // conflicts.
            let mut reachable: HashSet<(PaneId, ImageId)> = self
                .pane_snapshots
                .iter()
                .filter(|(p, _)| **p != pane_id)
                .flat_map(|(p, snap)| {
                    snap.images
                        .iter()
                        .map(move |wp| (*p, ImageId::from_raw(wp.image_id)))
                })
                .collect();
            for wp in &snapshot.images {
                reachable.insert((pane_id, ImageId::from_raw(wp.image_id)));
            }
            let mut cache = self
                .image_cache
                .lock()
                .expect("client image_cache mutex poisoned");
            for wid in snapshot.image_data.drain(..) {
                let id = ImageId::from_raw(wid.id);
                let arc = Arc::new(RenderableImageData {
                    id,
                    data: Arc::new(wid.data),
                    width: wid.width,
                    height: wid.height,
                    pixel_generation: wid.pixel_generation,
                });
                cache.insert(pane_id, id, arc, |p, i| reachable.contains(&(p, i)));
            }
        }
        self.pane_snapshots.insert(pane_id, snapshot);
    }

    /// Look up image pixel data for `(pane_id, image_id)` in the client cache.
    /// Returns an owned `Arc` (cheap refcount clone) so callers don't have to
    /// hold the cache lock. Used by `MuxBackend::pane_image_data` and the
    /// extract path's borrowed-closure lookup.
    pub(crate) fn pane_image_data(
        &self,
        pane_id: PaneId,
        image_id: ImageId,
    ) -> Option<Arc<RenderableImageData>> {
        self.image_cache
            .lock()
            .expect("client image_cache mutex poisoned")
            .get(pane_id, image_id)
    }

    /// Remove all per-pane caches for `pane_id` (used when a pane is closed).
    /// Drains every backend-local index keyed by `PaneId` so explicit
    /// `close_pane`, notification-driven `cleanup_closed_pane`, and any
    /// other close path share one cleanup point. `bell_panes` is included
    /// here because the App-level focus gate populates it independently of
    /// `pane_snapshots`; without dropping it here a `close_pane` (RPC
    /// path) would leave a stale `has_bell == true` for a recycled
    /// `PaneId`.
    pub(crate) fn remove_snapshot(&mut self, pane_id: PaneId) {
        self.pane_snapshots.remove(&pane_id);
        self.dirty_panes.remove(&pane_id);
        self.pending_refresh.remove(&pane_id);
        self.bell_panes.remove(&pane_id);
        self.image_cache
            .lock()
            .expect("client image_cache mutex poisoned")
            .drop_pane(pane_id);
        if let Some(transport) = &self.transport {
            transport.invalidate_pushed_snapshot(pane_id);
        }
    }

    /// Set this client's priority for a pane (0 = focused/highest, 255 = lowest).
    /// Sends a fire-and-forget `MuxPdu::SetPanePriority` to the daemon.
    /// Returns `Err(NotConnected)` if not connected.
    pub fn set_pane_priority(&mut self, pane_id: PaneId, priority: u8) -> io::Result<()> {
        let transport = self.transport.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "not connected to daemon")
        })?;
        transport.fire_and_forget(MuxPdu::SetPanePriority { pane_id, priority });
        Ok(())
    }

    /// The client ID assigned by the daemon, if connected.
    pub fn client_id(&self) -> Option<crate::id::ClientId> {
        self.transport.as_ref().map(ClientTransport::client_id)
    }

    /// Send a Ping RPC and wait for `PingAck`. Returns the round-trip duration.
    /// Measures raw IPC overhead with zero payload (no snapshot building,
    /// no serialization of grid data). Used for latency diagnostics.
    pub fn ping_rpc(&mut self) -> io::Result<Duration> {
        let start = std::time::Instant::now();
        match self.rpc(MuxPdu::Ping)? {
            MuxPdu::PingAck => Ok(start.elapsed()),
            other => Err(io::Error::other(format!(
                "ping_rpc: unexpected response: {other:?}"
            ))),
        }
    }

    /// Whether the daemon connection is alive.
    pub fn is_connected(&self) -> bool {
        self.transport
            .as_ref()
            .is_some_and(ClientTransport::is_alive)
    }

    /// Re-subscribe to all previously cached panes after a reconnection.
    /// The transport reader thread clears its subscription state on
    /// disconnect. After `MuxClient` reconnects, it must re-establish
    /// its interest in all panes it is currently rendering so the
    /// daemon starts pushing snapshots again.
    fn resubscribe_all(&mut self, pane_ids: &[PaneId]) {
        use crate::backend::MuxBackend;
        for pane_id in pane_ids {
            if let Err(e) = self.subscribe(*pane_id) {
                log::warn!("reconnect: re-subscribe to {pane_id} failed: {e}");
            }
        }
    }

    /// Attempt to reconnect to the daemon.
    /// Drops the old transport (joining the reader thread), establishes a new
    /// connection, and re-subscribes to all panes that were in the snapshot
    /// cache. Cached snapshots survive — the UI shows last-known state during
    /// the reconnection window.
    /// Returns `Ok(())` on success, `Err` if the connection could not be
    /// re-established (daemon down, socket gone, etc.).
    pub fn reconnect(&mut self) -> io::Result<()> {
        let socket_path = self.socket_path.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotConnected,
                "no socket path for reconnection",
            )
        })?;
        let wakeup = self.wakeup.as_ref().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotConnected,
                "no wakeup callback for reconnection",
            )
        })?;

        // Drop old transport — joins reader thread, closes socket.
        self.transport = None;

        // Establish new connection.
        let transport = ClientTransport::connect(socket_path, Arc::clone(wakeup))?;
        log::info!("reconnected, new client_id={}", transport.client_id());
        self.transport = Some(transport);

        // Clear stale state.
        self.pending_refresh.clear();

        // Re-subscribe to all cached panes.
        let pane_ids: Vec<PaneId> = self.pane_snapshots.keys().copied().collect();
        self.resubscribe_all(&pane_ids);

        // Mark all panes dirty so the render loop fetches fresh snapshots.
        self.dirty_panes.extend(pane_ids);

        Ok(())
    }

    /// Attempt reconnection with exponential backoff.
    /// Tries up to `max_attempts` times with 500ms between attempts. Returns
    /// `Ok(())` on the first successful reconnection, or the last error if
    /// all attempts fail. The caller (App event loop) decides what to do on
    /// final failure (show error bar, fall back to embedded mode, etc.).
    pub fn reconnect_with_backoff(&mut self, max_attempts: u32) -> io::Result<()> {
        let delay = Duration::from_millis(500);
        let mut last_err = None;
        for attempt in 1..=max_attempts {
            match self.reconnect() {
                Ok(()) => {
                    log::info!("reconnected on attempt {attempt}/{max_attempts}");
                    return Ok(());
                }
                Err(e) => {
                    log::warn!("reconnect attempt {attempt}/{max_attempts} failed: {e}");
                    last_err = Some(e);
                    if attempt < max_attempts {
                        std::thread::sleep(delay);
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| io::Error::other("reconnect failed")))
    }

    /// Send an RPC request to the daemon and return the response.
    fn rpc(&mut self, pdu: MuxPdu) -> io::Result<MuxPdu> {
        self.transport
            .as_mut()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "not connected to daemon"))?
            .rpc(pdu)
    }
}

#[cfg(test)]
impl Default for MuxClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
