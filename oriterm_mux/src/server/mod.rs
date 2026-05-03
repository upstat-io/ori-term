//! Mux daemon server.
//!
//! [`MuxServer`] owns an [`InProcessMux`] and runs a `mio`-based event loop
//! that accepts IPC connections from window processes, dispatches requests,
//! and pushes notifications to subscribed clients.
//!
//! The server is single-threaded: mio multiplexes the IPC listener, all
//! client streams, and a [`Waker`] that PTY reader threads use to signal
//! new [`MuxEvent`]s.

mod clients;
mod connection;
mod dispatch;
mod frame_io;
mod host_request;
mod ipc;
mod notify;
mod pid_file;
mod push;
pub(crate) mod snapshot;

use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use mio::{Events, Interest, Poll, Token, Waker};

use crate::id::{ClientId, HostRequestId, IdAllocator, PaneId};
use crate::in_process::InProcessMux;
use crate::mux_event::MuxNotification;
use crate::pane::Pane;

use self::host_request::{
    HostRequestDispatch, HostRequestDispatchCtx, PendingHostReply, host_request_to_pdu,
    select_responder,
};
use self::notify::TargetClients;
use self::snapshot::SnapshotCache;

pub(crate) use connection::ClientConnection;
pub use ipc::{IpcListener, IpcStream, socket_path};
pub use pid_file::{PidFile, pid_file_path, read_pid};

/// mio token for the IPC listener.
const LISTENER: Token = Token(0);

/// mio token for the cross-thread waker.
const WAKER: Token = Token(1);

/// First token available for client connections.
const CLIENT_BASE: usize = 2;

/// Daemon server owning all PTY sessions and managing IPC clients.
///
/// Runs a single-threaded `mio`-based event loop: accepts connections from
/// window processes, dispatches mux operations, drains PTY events, and
/// pushes notifications to subscribed clients.
pub struct MuxServer {
    // Core state.
    /// In-process multiplexer owning all panes.
    mux: InProcessMux,
    /// Platform-specific IPC listener.
    listener: IpcListener,
    /// Live pane instances, keyed by ID.
    panes: HashMap<PaneId, Pane>,

    // Connection tracking.
    /// Connected window processes keyed by client ID.
    connections: HashMap<ClientId, ClientConnection>,
    /// Pane → subscribed clients mapping.
    subscriptions: HashMap<PaneId, Vec<ClientId>>,
    /// mio token → client ID for O(1) event dispatch.
    token_to_client: HashMap<Token, ClientId>,
    /// Allocator for client IDs.
    client_alloc: IdAllocator<ClientId>,

    // Event loop infrastructure.
    /// mio poll instance.
    poll: Poll,
    /// Cross-thread waker for `MuxEvent` notifications.
    waker: Arc<Waker>,
    /// Closure that wakes the mio event loop from PTY reader threads.
    wakeup: Arc<dyn Fn() + Send + Sync>,
    /// Shutdown flag — set by signal handler or `--stop` command.
    shutdown: Arc<AtomicBool>,

    // Housekeeping.
    /// PID file handle (removed on drop).
    _pid_file: PidFile,
    /// Next mio token for client connections.
    next_token: usize,
    /// Server start time (for startup grace period).
    start_time: Instant,
    /// Set once at least one client has connected.
    had_client: bool,
    /// Reusable buffer for draining notifications.
    notification_buf: Vec<MuxNotification>,
    /// Reusable scratch buffer for collecting client IDs during dispatch.
    scratch_clients: Vec<ClientId>,
    /// Reusable scratch buffer for collecting pane IDs during dispatch.
    scratch_panes: Vec<PaneId>,
    /// Reusable scratch buffer for panes needing immediate snapshot push.
    scratch_immediate_push: Vec<PaneId>,

    // Server-push state.
    /// Per-pane timestamp of last snapshot push.
    last_snapshot_push: HashMap<PaneId, Instant>,
    /// Panes with deferred pushes (per-client tracking).
    pending_push: HashMap<PaneId, HashSet<ClientId>>,

    // Snapshot cache (allocation reuse for GetPaneSnapshot).
    /// Cached snapshots with shared render buffer — encapsulates
    /// `RenderableContent` so the server layer never touches it directly.
    snapshot_cache: SnapshotCache,

    // Daemon-mode HostRequest plumbing (BUG-11-011).
    /// Pending host-request tokens awaiting client `ReplyHostRequest`.
    pub(super) pending_host_replies: HashMap<HostRequestId, PendingHostReply>,
    /// Allocator for `HostRequestId`s.
    request_alloc: IdAllocator<HostRequestId>,
}

impl MuxServer {
    /// Create a new server, binding the IPC listener and writing the PID file.
    pub fn new() -> io::Result<Self> {
        Self::with_paths(&socket_path(), &pid_file_path())
    }

    /// Create with explicit paths (for testing).
    pub fn with_paths(
        socket_path: &std::path::Path,
        pid_path: &std::path::Path,
    ) -> io::Result<Self> {
        let pid_file = PidFile::create_at(pid_path)?;
        let poll = Poll::new()?;
        let waker = Arc::new(Waker::new(poll.registry(), WAKER)?);

        // Build the wakeup closure that PTY reader threads will call.
        let waker_ref = Arc::clone(&waker);
        let wakeup: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            let _ = waker_ref.wake();
        });

        let mut listener = IpcListener::bind_at(socket_path)?;
        poll.registry()
            .register(&mut listener, LISTENER, Interest::READABLE)?;

        Ok(Self {
            mux: InProcessMux::new(),
            listener,
            panes: HashMap::new(),
            connections: HashMap::new(),
            subscriptions: HashMap::new(),
            token_to_client: HashMap::new(),
            client_alloc: IdAllocator::new(),
            poll,
            waker,
            wakeup,
            shutdown: Arc::new(AtomicBool::new(false)),
            _pid_file: pid_file,
            next_token: CLIENT_BASE,
            start_time: Instant::now(),
            had_client: false,
            notification_buf: Vec::new(),
            scratch_clients: Vec::new(),
            scratch_panes: Vec::new(),
            scratch_immediate_push: Vec::new(),
            last_snapshot_push: HashMap::new(),
            pending_push: HashMap::new(),
            snapshot_cache: SnapshotCache::new(),
            pending_host_replies: HashMap::new(),
            request_alloc: IdAllocator::new(),
        })
    }

    /// Arc reference to the waker for cross-thread use.
    ///
    /// PTY reader threads call `waker.wake()` to notify the event loop
    /// that new [`MuxEvent`]s are available.
    pub fn waker(&self) -> Arc<Waker> {
        Arc::clone(&self.waker)
    }

    /// Arc reference to the shutdown flag.
    ///
    /// Signal handlers set this to `true` to trigger graceful shutdown.
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    /// Immutable access to the inner mux.
    pub fn mux(&self) -> &InProcessMux {
        &self.mux
    }

    /// Number of currently connected clients.
    pub fn client_count(&self) -> usize {
        self.connections.len()
    }

    /// Run the server event loop until shutdown.
    pub fn run(&mut self) -> io::Result<()> {
        let mut events = Events::with_capacity(64);
        log::info!(
            "oriterm-mux daemon started (pid={}, socket={})",
            std::process::id(),
            self.listener.path().display(),
        );

        while !self.shutdown.load(Ordering::Acquire) {
            let timeout = if self.pending_push.is_empty() {
                Duration::from_millis(100)
            } else {
                push::SNAPSHOT_PUSH_INTERVAL // 4ms — retries fire promptly.
            };
            self.poll.poll(&mut events, Some(timeout))?;

            for event in &events {
                match event.token() {
                    LISTENER => self.accept_connections()?,
                    WAKER => { /* MuxEvent arrived — handled below */ }
                    token => self.handle_client_event(token),
                }
            }

            // Drain `MuxEvent`s from PTY reader threads.
            self.drain_mux_events();

            // Second pass: handle client requests that arrived during
            // drain_mux_events (snapshot building can take milliseconds).
            // Non-blocking poll with zero timeout — only picks up already-ready events.
            self.poll.poll(&mut events, Some(Duration::ZERO))?;
            for event in &events {
                match event.token() {
                    LISTENER => self.accept_connections()?,
                    WAKER => { /* Will be drained on next main iteration */ }
                    token => self.handle_client_event(token),
                }
            }

            // Check exit condition: all panes exited + no clients.
            if self.should_exit() {
                log::info!("all panes exited and no clients — shutting down");
                break;
            }
        }

        log::info!("oriterm-mux daemon shutting down");
        Ok(())
    }

    /// Drain `MuxEvent`s from PTY reader threads and push notifications.
    ///
    /// Three-phase processing:
    /// 1. Trailing-edge flush — retry deferred pushes from previous cycles.
    /// 2. Route new notifications — `PaneOutput` triggers snapshot push
    ///    (or deferral); other notifications use existing routing.
    /// 3. Update write interests for connections with pending data.
    #[allow(
        clippy::too_many_lines,
        reason = "linear three-phase event loop; splitting would scatter the dispatch sequence"
    )]
    fn drain_mux_events(&mut self) {
        self.mux.poll_events(&mut self.panes);
        self.mux.drain_notifications(&mut self.notification_buf);
        let now = Instant::now();

        // Phase 1: Trailing-edge flush — retry deferred pushes.
        {
            let mut push_ctx = push::PushContext {
                last_snapshot_push: &mut self.last_snapshot_push,
                subscriptions: &self.subscriptions,
                connections: &mut self.connections,
                panes: &self.panes,
                snapshot_cache: &mut self.snapshot_cache,
                pending_push: &mut self.pending_push,
                scratch: &mut self.scratch_clients,
                scratch_panes: &mut self.scratch_panes,
            };
            push::trailing_edge_flush(&mut push_ctx, now);
        }

        // Phase 2: Route new notifications.
        //
        // Drain `notification_buf` BY-VALUE so host-request `ResponseToken`s
        // can be moved across the staging-buffer boundary into
        // `pending_host_replies` (cloning would defeat `Arc::strong_count`
        // cancellation detection per the SSOT in
        // `oriterm_core::effect::ResponseToken`). Take/drain/restore
        // preserves the buffer's heap allocation across cycles
        // (impl-hygiene §WASTE).
        let mut notifications = std::mem::take(&mut self.notification_buf);
        let pane_closed_ids: Vec<PaneId> = notifications
            .iter()
            .filter_map(|n| match n {
                MuxNotification::PaneClosed { pane_id, .. } => Some(*pane_id),
                _ => None,
            })
            .collect();
        // `drain(..)` (rather than `into_iter()`) preserves the buffer's heap
        // allocation across cycles; `notification_buf = notifications` at end
        // restores the (now-empty) Vec for reuse — impl-hygiene §WASTE.
        #[allow(
            clippy::iter_with_drain,
            reason = "drain(..) chosen specifically to preserve buffer capacity"
        )]
        for notif in notifications.drain(..) {
            if let MuxNotification::PaneOutput(pane_id) = &notif {
                let mut push_ctx = push::PushContext {
                    last_snapshot_push: &mut self.last_snapshot_push,
                    subscriptions: &self.subscriptions,
                    connections: &mut self.connections,
                    panes: &self.panes,
                    snapshot_cache: &mut self.snapshot_cache,
                    pending_push: &mut self.pending_push,
                    scratch: &mut self.scratch_clients,
                    scratch_panes: &mut self.scratch_panes,
                };
                push::push_or_defer_pane(&mut push_ctx, now, *pane_id);
                continue;
            }

            // Host-request path (single-responder routing + token capture).
            if matches!(
                &notif,
                MuxNotification::HostClipboardLoad { .. } | MuxNotification::HostColorQuery { .. }
            ) {
                self.dispatch_host_request_notification(notif);
                continue;
            }

            // Stateless fallback: existing notify::notification_to_pdu path.
            let Some((target, pdu)) = notify::notification_to_pdu(&notif, &self.panes) else {
                continue;
            };
            match target {
                TargetClients::PaneSubscribers(pane_id) => {
                    if let Some(subs) = self.subscriptions.get(&pane_id) {
                        self.scratch_clients.clear();
                        self.scratch_clients.extend_from_slice(subs);
                        for &cid in &self.scratch_clients {
                            if let Some(conn) = self.connections.get_mut(&cid) {
                                let _ = conn.queue_frame(0, &pdu);
                            }
                        }
                    }
                }
                TargetClients::SinglePaneSubscriber(_, cid) => {
                    if let Some(conn) = self.connections.get_mut(&cid) {
                        let _ = conn.queue_frame(0, &pdu);
                    }
                }
            }
        }
        self.notification_buf = notifications;

        // Phase 2b: Push IO thread snapshots for panes that produced new state
        // from commands (scroll, theme, search, etc.) without PTY output.
        {
            self.scratch_panes.clear();
            for (&pid, pane) in &self.panes {
                if pane.has_io_snapshot() && self.subscriptions.contains_key(&pid) {
                    self.scratch_panes.push(pid);
                }
            }
            if !self.scratch_panes.is_empty() {
                let mut push_ctx = push::PushContext {
                    last_snapshot_push: &mut self.last_snapshot_push,
                    subscriptions: &self.subscriptions,
                    connections: &mut self.connections,
                    panes: &self.panes,
                    snapshot_cache: &mut self.snapshot_cache,
                    pending_push: &mut self.pending_push,
                    scratch: &mut self.scratch_clients,
                    scratch_panes: &mut Vec::new(),
                };
                for &pid in &self.scratch_panes {
                    push::push_or_defer_pane(&mut push_ctx, now, pid);
                }
            }
        }

        // Post-pass: Clean up per-pane state for closed panes. Pre-collected
        // before the by-value drain because the drain consumed the buffer.
        for pane_id in pane_closed_ids {
            self.cleanup_pane_state(pane_id);
        }

        // Phase 3: Update write interests for connections with pending data.
        // Reuse scratch_clients (free after phases 1-2) to avoid per-cycle allocation.
        self.scratch_clients.clear();
        self.scratch_clients.extend(
            self.connections
                .values()
                .filter(|c| c.has_pending_writes())
                .map(ClientConnection::id),
        );
        for i in 0..self.scratch_clients.len() {
            self.update_write_interest(self.scratch_clients[i]);
        }
    }

    /// Route a host-request notification to a single responder + register
    /// the pending entry on successful queueing (BUG-11-011).
    ///
    /// `notif` MUST be `HostClipboardLoad` or `HostColorQuery`; the caller
    /// (`drain_mux_events`) already checked the variant. Token entries are
    /// registered only after a successful `queue_frame` so a failed delivery
    /// does not leak a token the consumer will never see — codex-002 round 1
    /// finding.
    fn dispatch_host_request_notification(&mut self, notif: MuxNotification) {
        let pane_id = match &notif {
            MuxNotification::HostClipboardLoad { pane_id, .. }
            | MuxNotification::HostColorQuery { pane_id, .. } => *pane_id,
            _ => unreachable!("caller pre-filters to host-request variants"),
        };
        let Some(responder) = select_responder(&self.subscriptions, &self.connections, pane_id)
        else {
            log::warn!("HostRequest for {pane_id} dropped — no subscribed client to answer");
            return;
        };
        let mut ctx = HostRequestDispatchCtx {
            request_alloc: &mut self.request_alloc,
            responder,
        };
        let Some(HostRequestDispatch {
            target,
            pdu,
            request_id,
            pending,
        }) = host_request_to_pdu(notif, &mut ctx)
        else {
            return;
        };
        // Single-responder routing — `host_request_to_pdu` always emits
        // `SinglePaneSubscriber`, but the destructuring is lifted via if-let
        // for clippy.
        let TargetClients::SinglePaneSubscriber(_, cid) = target else {
            return;
        };
        let queued = if let Some(conn) = self.connections.get_mut(&cid) {
            match conn.queue_frame(0, &pdu) {
                Ok(()) => true,
                Err(e) => {
                    log::warn!("HostRequest: queue_frame failed for {cid}: {e}; dropping token");
                    false
                }
            }
        } else {
            log::warn!(
                "HostRequest: selected responder {cid} disconnected before delivery; dropping token"
            );
            false
        };
        if queued {
            self.pending_host_replies.insert(request_id, pending);
        }
    }

    /// Remove all per-pane tracking state for a closed pane.
    ///
    /// Clears snapshot cache, push timestamps, pending pushes, subscription
    /// entries, and per-connection subscription sets. Centralizes cleanup
    /// that previously lived in three separate locations.
    pub(super) fn cleanup_pane_state(&mut self, pane_id: PaneId) {
        self.snapshot_cache.remove(pane_id);
        self.last_snapshot_push.remove(&pane_id);
        self.pending_push.remove(&pane_id);
        self.subscriptions.remove(&pane_id);
        for conn in self.connections.values_mut() {
            conn.unsubscribe(pane_id);
        }
        // BUG-11-011: drop any pending host-replies for the closed pane
        // (consumer apps inside the pane no longer exist; the IO thread
        // returns Cancelled via Arc::strong_count).
        let dropped = self
            .pending_host_replies
            .iter()
            .filter(|(_, v)| v.pane_id == pane_id)
            .count();
        self.pending_host_replies
            .retain(|_, v| v.pane_id != pane_id);
        if dropped > 0 {
            log::warn!(
                "cleanup_pane_state {pane_id}: dropped {dropped} pending host-request token(s)"
            );
        }
    }

    /// Check if the server should auto-exit.
    ///
    /// Exits when all panes have exited AND no clients are connected,
    /// with a startup grace period so the server doesn't exit immediately
    /// before any client has connected.
    fn should_exit(&self) -> bool {
        // Grace period: don't exit within first 5 seconds of startup.
        let grace = Duration::from_secs(5);
        if self.start_time.elapsed() < grace {
            return false;
        }
        // Don't exit until at least one client has connected and left.
        if !self.had_client {
            return false;
        }
        self.connections.is_empty() && self.panes.is_empty()
    }
}

#[cfg(test)]
mod tests;
