//! Test doubles for [`MuxBackend`] consumers.
//!
//! Provides [`MockMuxBackend`] — a recording test double that implements
//! [`MuxBackend`] with call tracking for the methods the App's
//! `handle_mux_notification` and `pump_mux_events_core` call. Uncalled
//! methods use `unimplemented!()` (guarded by `#[expect(clippy::unimplemented)]`)
//! so test failures are loud.

use std::cell::RefCell;
use std::io;
use std::sync::mpsc;

use oriterm_core::selection::Selection;
use oriterm_core::{RenderableContent, Theme};

use crate::backend::{AdoptPaneRequest, HostReply, ImageConfig, MuxBackend};
use crate::domain::SpawnConfig;
use crate::in_process::ClosePaneResult;
use crate::mux_event::{MuxEvent, MuxNotification};
use crate::pane::MarkCursor;
use crate::registry::PaneEntry;
use crate::PaneId;
use crate::PaneSnapshot;
use crate::{DomainId, Signal};

/// A recorded call to a [`MuxBackend`] method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MockCall {
    IsDaemonMode,
    IsConnected,
    HasPendingWakeup,
    PollEvents,
    DrainNotifications,
    SetBell(PaneId),
    ClearBell(PaneId),
    HasBell(PaneId),
    SetUnseenOutput(PaneId),
    MarkOutputSeen(PaneId),
    IsSelectionDirty(PaneId),
    ClearSelectionDirty(PaneId),
    CleanupClosedPane(PaneId),
    FulfillHostRequest(PaneId),
    SyncPaneSnapshot(PaneId),
}

/// A recording [`MuxBackend`] for App-level unit tests.
///
/// Tracks every method call in `calls: Vec<MockCall>`. Return values are
/// configurable via builder-style setters. Methods not exercised by the
/// App's notification dispatch use `unimplemented!()`.
pub struct MockMuxBackend {
    pub calls: RefCell<Vec<MockCall>>,
    pub is_daemon_mode: bool,
    pub is_connected: bool,
    pub has_pending_wakeup: bool,
    pub drain_returns: Vec<MuxNotification>,
    pub bell_panes: RefCell<Vec<PaneId>>,
    pub unseen_output: RefCell<Vec<PaneId>>,
    pub selection_dirty: RefCell<Vec<PaneId>>,
    pub fulfilled_requests: RefCell<Vec<PaneId>>,
    pub sync_snapshot: Option<PaneSnapshot>,
}

impl MockMuxBackend {
    pub fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            is_daemon_mode: false,
            is_connected: true,
            has_pending_wakeup: false,
            drain_returns: Vec::new(),
            bell_panes: RefCell::new(Vec::new()),
            unseen_output: RefCell::new(Vec::new()),
            selection_dirty: RefCell::new(Vec::new()),
            fulfilled_requests: RefCell::new(Vec::new()),
            sync_snapshot: None,
        }
    }
}

impl Default for MockMuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[expect(
    clippy::unimplemented,
    reason = "MockMuxBackend implements only the methods App dispatch calls; \
              reaching any other method is a test-protocol error"
)]
impl MuxBackend for MockMuxBackend {
    fn has_pending_wakeup(&self) -> bool {
        self.calls.borrow_mut().push(MockCall::HasPendingWakeup);
        self.has_pending_wakeup
    }
    fn poll_events(&mut self) {
        self.calls.borrow_mut().push(MockCall::PollEvents);
    }
    fn drain_notifications(&mut self, out: &mut Vec<MuxNotification>) {
        self.calls.borrow_mut().push(MockCall::DrainNotifications);
        out.clear();
        out.append(&mut self.drain_returns);
    }
    fn discard_notifications(&mut self) {
        self.drain_returns.clear();
    }
    fn get_pane_entry(&self, _: PaneId) -> Option<PaneEntry> {
        unimplemented!("MockMuxBackend: App dispatch does not call get_pane_entry")
    }
    fn spawn_pane(&mut self, _: &SpawnConfig, _: Theme) -> io::Result<PaneId> {
        unimplemented!("MockMuxBackend: App dispatch does not call spawn_pane")
    }
    fn adopt_pane(
        &mut self,
        _: crate::pty::AdoptedPtyHandle,
        _: AdoptPaneRequest,
    ) -> io::Result<PaneId> {
        unimplemented!("MockMuxBackend: App dispatch does not call adopt_pane")
    }
    fn close_pane(&mut self, _: PaneId) -> ClosePaneResult {
        unimplemented!("MockMuxBackend: App dispatch does not call close_pane")
    }
    fn resize_pane_grid(&mut self, _: PaneId, _: u16, _: u16) {
        unimplemented!("MockMuxBackend: App dispatch does not call resize_pane_grid")
    }
    fn pane_mode(&self, _: PaneId) -> Option<u64> {
        unimplemented!("MockMuxBackend: App dispatch does not call pane_mode")
    }
    fn set_pane_theme(&mut self, _: PaneId, _: Theme, _: oriterm_core::Palette) {
        unimplemented!("MockMuxBackend: App dispatch does not call set_pane_theme")
    }
    fn set_cursor_shape(&mut self, _: PaneId, _: oriterm_core::CursorShape) {
        unimplemented!("MockMuxBackend: App dispatch does not call set_cursor_shape")
    }
    fn set_bold_is_bright(&mut self, _: PaneId, _: bool) {
        unimplemented!("MockMuxBackend: App dispatch does not call set_bold_is_bright")
    }
    fn mark_all_dirty(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call mark_all_dirty")
    }
    fn set_image_config(&mut self, _: PaneId, _: ImageConfig) {
        unimplemented!("MockMuxBackend: App dispatch does not call set_image_config")
    }
    fn set_cell_dimensions(&mut self, _: PaneId, _: u16, _: u16) {
        unimplemented!("MockMuxBackend: App dispatch does not call set_cell_dimensions")
    }
    fn scroll_display(&mut self, _: PaneId, _: isize) {
        unimplemented!("MockMuxBackend: App dispatch does not call scroll_display")
    }
    fn scroll_to_bottom(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call scroll_to_bottom")
    }
    fn scroll_to_previous_prompt(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call scroll_to_previous_prompt")
    }
    fn scroll_to_next_prompt(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call scroll_to_next_prompt")
    }
    fn open_search(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call open_search")
    }
    fn close_search(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call close_search")
    }
    fn search_set_query(&mut self, _: PaneId, _: String) {
        unimplemented!("MockMuxBackend: App dispatch does not call search_set_query")
    }
    fn search_next_match(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call search_next_match")
    }
    fn search_prev_match(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call search_prev_match")
    }
    fn is_search_active(&self, _: PaneId) -> bool {
        unimplemented!("MockMuxBackend: App dispatch does not call is_search_active")
    }
    fn extract_text(&mut self, _: PaneId, _: &Selection) -> Option<String> {
        unimplemented!("MockMuxBackend: App dispatch does not call extract_text")
    }
    fn extract_html(
        &mut self,
        _: PaneId,
        _: &Selection,
        _: &str,
        _: f32,
    ) -> Option<(String, String)> {
        unimplemented!("MockMuxBackend: App dispatch does not call extract_html")
    }
    fn send_input(&mut self, _: PaneId, _: &[u8]) {
        unimplemented!("MockMuxBackend: App dispatch does not call send_input")
    }
    fn is_write_stalled(&mut self, _: PaneId) -> bool {
        unimplemented!("MockMuxBackend: App dispatch does not call is_write_stalled")
    }
    fn signal_child(&mut self, _: PaneId, _: Signal) -> bool {
        unimplemented!("MockMuxBackend: App dispatch does not call signal_child")
    }
    fn pane_cwd(&self, _: PaneId) -> Option<&str> {
        unimplemented!("MockMuxBackend: App dispatch does not call pane_cwd")
    }

    // -- Methods the App's handle_mux_notification calls --

    fn set_bell(&mut self, pane_id: PaneId) {
        self.calls.borrow_mut().push(MockCall::SetBell(pane_id));
        self.bell_panes.borrow_mut().push(pane_id);
    }
    fn clear_bell(&mut self, pane_id: PaneId) {
        self.calls.borrow_mut().push(MockCall::ClearBell(pane_id));
        self.bell_panes.borrow_mut().retain(|p| *p != pane_id);
    }
    fn has_bell(&self, pane_id: PaneId) -> bool {
        self.calls.borrow_mut().push(MockCall::HasBell(pane_id));
        self.bell_panes.borrow().contains(&pane_id)
    }
    fn set_unseen_output(&mut self, pane_id: PaneId) {
        self.calls.borrow_mut().push(MockCall::SetUnseenOutput(pane_id));
        self.unseen_output.borrow_mut().push(pane_id);
    }
    fn mark_output_seen(&mut self, pane_id: PaneId) {
        self.calls.borrow_mut().push(MockCall::MarkOutputSeen(pane_id));
        self.unseen_output.borrow_mut().retain(|p| *p != pane_id);
    }
    fn has_unseen_output(&self, _: PaneId) -> bool {
        false
    }
    fn is_selection_dirty(&self, pane_id: PaneId) -> bool {
        self.calls.borrow_mut().push(MockCall::IsSelectionDirty(pane_id));
        self.selection_dirty.borrow().contains(&pane_id)
    }
    fn clear_selection_dirty(&mut self, pane_id: PaneId) {
        self.calls
            .borrow_mut()
            .push(MockCall::ClearSelectionDirty(pane_id));
        self.selection_dirty.borrow_mut().retain(|p| *p != pane_id);
    }
    fn cleanup_closed_pane(&mut self, pane_id: PaneId) {
        self.calls
            .borrow_mut()
            .push(MockCall::CleanupClosedPane(pane_id));
    }
    fn fulfill_host_request(&mut self, pane_id: PaneId, _: HostReply) -> io::Result<()> {
        self.calls
            .borrow_mut()
            .push(MockCall::FulfillHostRequest(pane_id));
        self.fulfilled_requests.borrow_mut().push(pane_id);
        Ok(())
    }
    fn sync_pane_snapshot(&mut self, pane_id: PaneId) -> Option<PaneSnapshot> {
        self.calls
            .borrow_mut()
            .push(MockCall::SyncPaneSnapshot(pane_id));
        self.sync_snapshot.clone()
    }

    // -- Remaining trait methods --
    fn enter_mark_mode(&mut self, _: PaneId) -> Option<MarkCursor> {
        unimplemented!("MockMuxBackend: App dispatch does not call enter_mark_mode")
    }
    fn pane_ids(&self) -> Vec<PaneId> {
        unimplemented!("MockMuxBackend: App dispatch does not call pane_ids")
    }
    fn event_tx(&self) -> Option<&mpsc::Sender<MuxEvent>> {
        unimplemented!("MockMuxBackend: App dispatch does not call event_tx")
    }
    fn default_domain(&self) -> DomainId {
        unimplemented!("MockMuxBackend: App dispatch does not call default_domain")
    }
    fn is_connected(&self) -> bool {
        self.calls.borrow_mut().push(MockCall::IsConnected);
        self.is_connected
    }
    fn is_daemon_mode(&self) -> bool {
        self.calls.borrow_mut().push(MockCall::IsDaemonMode);
        self.is_daemon_mode
    }
    fn swap_renderable_content(
        &mut self,
        _: PaneId,
        _: &mut RenderableContent,
    ) -> bool {
        unimplemented!("MockMuxBackend: App dispatch does not call swap_renderable_content")
    }
    fn pane_snapshot(&self, _: PaneId) -> Option<&PaneSnapshot> {
        unimplemented!("MockMuxBackend: App dispatch does not call pane_snapshot")
    }
    fn is_pane_snapshot_dirty(&self, _: PaneId) -> bool {
        unimplemented!("MockMuxBackend: App dispatch does not call is_pane_snapshot_dirty")
    }
    fn refresh_pane_snapshot(&mut self, _: PaneId) -> Option<&PaneSnapshot> {
        unimplemented!("MockMuxBackend: App dispatch does not call refresh_pane_snapshot")
    }
    fn clear_pane_snapshot_dirty(&mut self, _: PaneId) {
        unimplemented!("MockMuxBackend: App dispatch does not call clear_pane_snapshot_dirty")
    }
    fn select_command_output(&self, _: PaneId) -> Option<Selection> {
        unimplemented!("MockMuxBackend: App dispatch does not call select_command_output")
    }
    fn select_command_input(&self, _: PaneId) -> Option<Selection> {
        unimplemented!("MockMuxBackend: App dispatch does not call select_command_input")
    }
    fn maybe_shrink_renderable_caches(&mut self) {}
}
