//! IPC client backend for daemon mode.
//!
//! [`MuxClient`] implements [`MuxBackend`] by sending requests to a
//! [`MuxServer`](crate::server::MuxServer) over an IPC socket. Pane data
//! is not available locally — `pane()`/`pane_mut()` return `None`.
//! Rendering in daemon mode uses `PaneSnapshot` (a later step).

use std::collections::HashSet;
use std::io;
use std::sync::mpsc;

use oriterm_core::Theme;

use super::MuxBackend;
use crate::domain::SpawnConfig;
use crate::in_process::ClosePaneResult;
use crate::layout::{Rect, SplitDirection};
use crate::mux_event::{MuxEvent, MuxNotification};
use crate::pane::Pane;
use crate::registry::{PaneEntry, SessionRegistry};
use crate::{DomainId, PaneId, TabId, WindowId};

/// IPC client backend for daemon mode.
///
/// Sends mux operations to the daemon over an IPC socket and blocks on
/// responses. Pane data is not stored locally — the daemon owns all
/// terminal state. A background reader thread receives push notifications
/// from the daemon and buffers them for [`drain_notifications`].
///
/// This is a stub: connect/handshake and most operations will be wired
/// in Section 44.4 (daemon launch + client connect).
pub struct MuxClient {
    /// Mirrored session state, synced from daemon responses/notifications.
    local_session: SessionRegistry,
    /// Buffered notifications from the background reader thread.
    notifications: Vec<MuxNotification>,
}

impl MuxClient {
    /// Create an unconnected client stub.
    ///
    /// Full construction (with IPC socket, reader thread, handshake) will
    /// be added in Section 44.4.
    pub fn new() -> Self {
        Self {
            local_session: SessionRegistry::new(),
            notifications: Vec::new(),
        }
    }
}

impl Default for MuxClient {
    fn default() -> Self {
        Self::new()
    }
}

impl MuxBackend for MuxClient {
    fn poll_events(&mut self) {
        // No-op: background reader thread pushes to notification_rx.
    }

    fn drain_notifications(&mut self, out: &mut Vec<MuxNotification>) {
        out.clear();
        std::mem::swap(&mut self.notifications, out);
    }

    fn discard_notifications(&mut self) {
        self.notifications.clear();
    }

    fn session(&self) -> &SessionRegistry {
        &self.local_session
    }

    fn active_tab_id(&self, window_id: WindowId) -> Option<TabId> {
        self.local_session.get_window(window_id)?.active_tab()
    }

    fn get_pane_entry(&self, _pane_id: PaneId) -> Option<PaneEntry> {
        // Stub: pane registry not yet mirrored.
        None
    }

    fn is_last_pane(&self, _pane_id: PaneId) -> bool {
        // Stub: query daemon in Section 44.4.
        false
    }

    fn create_window(&mut self) -> WindowId {
        // Stub: IPC roundtrip in Section 44.4.
        unimplemented!("MuxClient::create_window requires daemon connection")
    }

    fn close_window(&mut self, _window_id: WindowId) -> Vec<PaneId> {
        // Stub: IPC roundtrip in Section 44.4.
        Vec::new()
    }

    fn create_tab(
        &mut self,
        _window_id: WindowId,
        _config: &SpawnConfig,
        _theme: Theme,
    ) -> io::Result<(TabId, PaneId)> {
        // Stub: IPC roundtrip in Section 44.4.
        Err(io::Error::other("MuxClient::create_tab not yet connected"))
    }

    fn close_tab(&mut self, _tab_id: TabId) -> Vec<PaneId> {
        Vec::new()
    }

    fn switch_active_tab(&mut self, _window_id: WindowId, _tab_id: TabId) -> bool {
        false
    }

    fn cycle_active_tab(&mut self, _window_id: WindowId, _delta: isize) -> Option<TabId> {
        None
    }

    fn reorder_tab(&mut self, _window_id: WindowId, _from: usize, _to: usize) -> bool {
        false
    }

    fn move_tab_to_window(&mut self, _tab_id: TabId, _dest: WindowId) -> bool {
        false
    }

    fn move_tab_to_window_at(&mut self, _tab_id: TabId, _dest: WindowId, _idx: usize) -> bool {
        false
    }

    fn split_pane(
        &mut self,
        _tab_id: TabId,
        _source: PaneId,
        _dir: SplitDirection,
        _config: &SpawnConfig,
        _theme: Theme,
    ) -> io::Result<PaneId> {
        Err(io::Error::other("MuxClient::split_pane not yet connected"))
    }

    fn close_pane(&mut self, _pane_id: PaneId) -> ClosePaneResult {
        ClosePaneResult::NotFound
    }

    fn set_active_pane(&mut self, _tab_id: TabId, _pane_id: PaneId) -> bool {
        false
    }

    fn toggle_zoom(&mut self, _tab_id: TabId) {}

    fn unzoom_silent(&mut self, _tab_id: TabId) {}

    fn equalize_panes(&mut self, _tab_id: TabId) {}

    fn set_divider_ratio(&mut self, _tab_id: TabId, _before: PaneId, _after: PaneId, _ratio: f32) {}

    fn resize_pane(
        &mut self,
        _tab_id: TabId,
        _pane_id: PaneId,
        _axis: SplitDirection,
        _first: bool,
        _delta: f32,
    ) {
    }

    fn undo_split(&mut self, _tab_id: TabId, _live: &HashSet<PaneId>) -> bool {
        false
    }

    fn redo_split(&mut self, _tab_id: TabId, _live: &HashSet<PaneId>) -> bool {
        false
    }

    fn spawn_floating_pane(
        &mut self,
        _tab_id: TabId,
        _config: &SpawnConfig,
        _theme: Theme,
        _available: &Rect,
    ) -> io::Result<PaneId> {
        Err(io::Error::other(
            "MuxClient::spawn_floating_pane not yet connected",
        ))
    }

    fn move_pane_to_floating(
        &mut self,
        _tab_id: TabId,
        _pane_id: PaneId,
        _available: &Rect,
    ) -> bool {
        false
    }

    fn move_pane_to_tiled(&mut self, _tab_id: TabId, _pane_id: PaneId) -> bool {
        false
    }

    fn move_floating_pane(&mut self, _tab_id: TabId, _pane_id: PaneId, _x: f32, _y: f32) {}

    fn resize_floating_pane(&mut self, _tab_id: TabId, _pane_id: PaneId, _w: f32, _h: f32) {}

    fn set_floating_pane_rect(&mut self, _tab_id: TabId, _pane_id: PaneId, _rect: Rect) {}

    fn raise_floating_pane(&mut self, _tab_id: TabId, _pane_id: PaneId) {}

    fn pane(&self, _pane_id: PaneId) -> Option<&Pane> {
        // Daemon owns pane data — not available locally.
        None
    }

    fn pane_mut(&mut self, _pane_id: PaneId) -> Option<&mut Pane> {
        None
    }

    fn remove_pane(&mut self, _pane_id: PaneId) -> Option<Pane> {
        None
    }

    fn pane_ids(&self) -> Vec<PaneId> {
        Vec::new()
    }

    fn event_tx(&self) -> Option<&mpsc::Sender<MuxEvent>> {
        // No local event channel in daemon mode.
        None
    }

    fn default_domain(&self) -> DomainId {
        DomainId::from_raw(0)
    }
}

#[cfg(test)]
mod tests;
