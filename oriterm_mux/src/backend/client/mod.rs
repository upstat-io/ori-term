//! IPC client backend for daemon mode.
//!
//! [`MuxClient`] implements [`MuxBackend`] by sending requests to a
//! [`MuxServer`](crate::server::MuxServer) over an IPC socket. Pane data
//! is not available locally — `pane()`/`pane_mut()` return `None`.
//! Rendering in daemon mode uses `PaneSnapshot` (a later step).

mod notification;
mod rpc_methods;
mod transport;

use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;

use crate::PaneId;
use crate::PaneSnapshot;
use crate::mux_event::MuxNotification;
use crate::protocol::MuxPdu;
use crate::registry::{PaneRegistry, SessionRegistry};

use self::transport::ClientTransport;

/// IPC client backend for daemon mode.
///
/// Sends mux operations to the daemon over an IPC socket and blocks on
/// responses. Pane data is not stored locally — the daemon owns all
/// terminal state. A background reader thread receives push notifications
/// from the daemon and buffers them for [`drain_notifications`].
///
/// Cached [`PaneSnapshot`]s are stored locally for rendering. The dirty
/// set tracks which panes have received `PaneDirty` notifications since
/// the last render. The render path checks dirty, fetches a fresh
/// snapshot via RPC, and clears the flag.
pub struct MuxClient {
    /// IPC transport (reader thread + socket). `None` when test-only stub.
    transport: Option<ClientTransport>,

    /// Mirrored session state, synced from daemon responses/notifications.
    local_session: SessionRegistry,

    /// Mirrored pane registry, synced from daemon responses.
    pane_registry: PaneRegistry,

    /// Buffered notifications from the background reader thread.
    notifications: Vec<MuxNotification>,

    /// Cached pane snapshots for daemon-mode rendering.
    pane_snapshots: HashMap<PaneId, PaneSnapshot>,

    /// Panes with pending content updates (from `PaneDirty` notifications).
    dirty_panes: HashSet<PaneId>,
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
        let transport = ClientTransport::connect(socket_path, wakeup)?;
        log::info!("MuxClient connected, client_id={}", transport.client_id());
        Ok(Self {
            transport: Some(transport),
            local_session: SessionRegistry::new(),
            pane_registry: PaneRegistry::new(),
            notifications: Vec::new(),
            pane_snapshots: HashMap::new(),
            dirty_panes: HashSet::new(),
        })
    }

    /// Create an unconnected client stub for testing.
    ///
    /// All RPC methods will fail gracefully (return defaults or errors).
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            transport: None,
            local_session: SessionRegistry::new(),
            pane_registry: PaneRegistry::new(),
            notifications: Vec::new(),
            pane_snapshots: HashMap::new(),
            dirty_panes: HashSet::new(),
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

    /// Unsubscribe from a pane and remove its cached snapshot.
    pub(crate) fn unsubscribe_pane(&mut self, pane_id: PaneId) {
        match self.rpc(MuxPdu::Unsubscribe { pane_id }) {
            Ok(MuxPdu::Unsubscribed) => {
                self.remove_snapshot(pane_id);
                log::debug!("unsubscribed from pane {pane_id}");
            }
            Ok(other) => {
                log::error!("unsubscribe_pane: unexpected response: {other:?}");
            }
            Err(e) => {
                // Best-effort: remove snapshot locally even if RPC fails.
                self.remove_snapshot(pane_id);
                log::error!("unsubscribe_pane: RPC failed: {e}");
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
    pub fn ping_rpc(&mut self) -> io::Result<std::time::Duration> {
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

    /// Apply server-pushed layout updates for a set of tabs.
    ///
    /// For each `TabLayoutChanged(tab_id)` notification, takes the pushed
    /// layout data and updates the local session. Subscribes to new panes
    /// and unsubscribes from removed panes.
    fn apply_layout_update(&mut self, tab_id: crate::TabId) {
        let update = self
            .transport
            .as_ref()
            .and_then(|t| t.take_pushed_layout(tab_id));
        let Some(update) = update else {
            return;
        };

        // Collect old pane IDs before updating.
        let old_panes: HashSet<PaneId> = self
            .local_session
            .get_tab(tab_id)
            .map(|tab| tab.all_panes().into_iter().collect())
            .unwrap_or_default();

        // Update local session with server-authoritative layout.
        if let Some(tab) = self.local_session.get_tab_mut(tab_id) {
            tab.replace_layout(update.tree);
            tab.set_floating(update.floating);
            tab.set_active_pane(update.active_pane);
            tab.set_zoomed_pane(update.zoomed_pane);
        }

        // Collect new pane IDs after updating.
        let new_panes: HashSet<PaneId> = self
            .local_session
            .get_tab(tab_id)
            .map(|tab| tab.all_panes().into_iter().collect())
            .unwrap_or_default();

        // Subscribe to newly added panes.
        for &pid in &new_panes {
            if !old_panes.contains(&pid) {
                self.pane_registry.register(crate::registry::PaneEntry {
                    pane: pid,
                    tab: tab_id,
                    domain: crate::DomainId::from_raw(0),
                });
                self.subscribe_pane(pid);
            }
        }

        // Unsubscribe from removed panes.
        for &pid in &old_panes {
            if !new_panes.contains(&pid) {
                self.unsubscribe_pane(pid);
                self.pane_registry.unregister(pid);
            }
        }
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
