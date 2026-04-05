//! IPC client backend for daemon mode.
//!
//! [`MuxClient`] implements [`MuxBackend`] by sending requests to a
//! [`MuxServer`](crate::server::MuxServer) over an IPC socket. Pane data
//! is not available locally — rendering uses `PaneSnapshot`s fetched from
//! the daemon.

mod notification;
mod rpc_methods;
mod transport;

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::PaneId;
use crate::PaneSnapshot;
use crate::mux_event::MuxNotification;
use crate::protocol::MuxPdu;

use self::transport::ClientTransport;

/// IPC client backend for daemon mode.
///
/// Sends mux operations to the daemon over an IPC socket and blocks on
/// responses. Pane data is not stored locally — the daemon owns all
/// terminal state. A background reader thread receives push notifications
/// from the daemon and buffers them for [`drain_notifications`].
///
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
}

impl MuxClient {
    /// Connect to a running daemon at `socket_path`.
    ///
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
        })
    }

    /// Create an unconnected client stub for testing.
    ///
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
        }
    }

    /// Cache a snapshot for a pane (used when subscribe responses arrive).
    pub(crate) fn cache_snapshot(&mut self, pane_id: PaneId, snapshot: PaneSnapshot) {
        self.pane_snapshots.insert(pane_id, snapshot);
    }

    /// Remove a cached snapshot (used when a pane is closed).
    pub(crate) fn remove_snapshot(&mut self, pane_id: PaneId) {
        self.pane_snapshots.remove(&pane_id);
        self.dirty_panes.remove(&pane_id);
        self.pending_refresh.remove(&pane_id);
        if let Some(transport) = &self.transport {
            transport.invalidate_pushed_snapshot(pane_id);
        }
    }

    /// Subscribe to a pane and cache the initial snapshot from the response.
    pub(crate) fn subscribe_pane(&mut self, pane_id: PaneId) {
        match self.rpc(MuxPdu::Subscribe { pane_id }) {
            Ok(MuxPdu::Subscribed { snapshot }) => {
                self.cache_snapshot(pane_id, snapshot);
                log::debug!("subscribed to pane {pane_id}");
            }
            Ok(other) => {
                log::error!("subscribe_pane: unexpected response: {other:?}");
            }
            Err(e) => {
                log::error!("subscribe_pane: RPC failed: {e}");
            }
        }
    }

    /// The client ID assigned by the daemon, if connected.
    pub fn client_id(&self) -> Option<crate::id::ClientId> {
        self.transport.as_ref().map(ClientTransport::client_id)
    }

    /// Send a Ping RPC and wait for `PingAck`. Returns the round-trip duration.
    ///
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

    /// Attempt to reconnect to the daemon.
    ///
    /// Drops the old transport (joining the reader thread), establishes a new
    /// connection, and re-subscribes to all panes that were in the snapshot
    /// cache. Cached snapshots survive — the UI shows last-known state during
    /// the reconnection window.
    ///
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

        // Re-subscribe to all cached panes. Collect keys first (borrow checker).
        let pane_ids: Vec<PaneId> = self.pane_snapshots.keys().copied().collect();
        for pane_id in &pane_ids {
            self.subscribe_pane(*pane_id);
        }

        // Mark all panes dirty so the render loop fetches fresh snapshots.
        self.dirty_panes.extend(pane_ids);

        Ok(())
    }

    /// Attempt reconnection with exponential backoff.
    ///
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
