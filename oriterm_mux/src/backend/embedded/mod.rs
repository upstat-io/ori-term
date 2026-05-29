//! In-process backend wrapping [`InProcessMux`] with local pane ownership.
//!
//! [`EmbeddedMux`] stores `Pane` structs internally alongside the mux
//! orchestrator, presenting them through the [`MuxBackend`] trait. The
//! wakeup callback is captured at construction — individual methods never
//! need it as a parameter.

use std::collections::{HashMap, HashSet};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;

use oriterm_core::Selection;
use oriterm_core::{RenderableContent, Theme};

use super::{AdoptPaneRequest, HostReply, ImageConfig, MuxBackend};
use crate::domain::SpawnConfig;
use crate::in_process::{ClosePaneResult, InProcessMux};
use crate::mux_event::{MuxEvent, MuxNotification};
use crate::pane::io_thread::PaneIoCommand;
use crate::pane::{MarkCursor, Pane};
use crate::pty::AdoptedPtyHandle;
use crate::registry::PaneEntry;
use crate::{DomainId, PaneId, PaneSnapshot};

mod io_roundtrip;
mod snapshot;

/// In-process mux backend for single-process mode.
///
/// Owns the [`InProcessMux`] orchestrator, the `Pane` map, and the wakeup
/// callback. The App interacts exclusively through [`MuxBackend`] methods.
pub struct EmbeddedMux {
    mux: InProcessMux,
    panes: HashMap<PaneId, Pane>,
    /// Coalesced wakeup closure — wraps the raw wakeup with an [`AtomicBool`]
    /// guard so that only one `PostMessage` is issued per poll cycle.
    guarded_wakeup: Arc<dyn Fn() + Send + Sync>,
    /// Coalescing flag cleared in [`poll_events`](MuxBackend::poll_events).
    wakeup_pending: Arc<AtomicBool>,
    snapshot_cache: HashMap<PaneId, PaneSnapshot>,
    snapshot_dirty: HashSet<PaneId>,
    /// Client-local bell-indicator state. SSOT for `has_bell()` queries
    /// in embedded mode — uniform with `MuxClient::bell_panes`.
    /// Decouples bell state from snapshot replication so the contract
    /// holds identically across backends.
    bell_panes: HashSet<PaneId>,
    /// Per-pane [`RenderableContent`] cache, filled by
    /// [`refresh_pane_snapshot`](MuxBackend::refresh_pane_snapshot) and
    /// consumed by [`swap_renderable_content`](MuxBackend::swap_renderable_content).
    ///
    /// Bypasses the `RenderableContent → WireCell → RenderableContent` round-trip
    /// that the snapshot path requires for daemon mode IPC. Vec allocations are
    /// reused across frames via [`std::mem::swap`].
    renderable_cache: HashMap<PaneId, RenderableContent>,
}

impl EmbeddedMux {
    /// Create a new embedded backend.
    ///
    /// `wakeup` is called by PTY reader threads to wake the event loop.
    /// The closure is wrapped with an [`AtomicBool`] guard so that only
    /// one wakeup is posted per poll cycle during flood output.
    pub fn new(wakeup: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (guarded_wakeup, wakeup_pending) = crate::backend::wakeup::guarded_wakeup(wakeup);
        Self {
            mux: InProcessMux::new(),
            panes: HashMap::new(),
            guarded_wakeup,
            wakeup_pending,
            snapshot_cache: HashMap::new(),
            snapshot_dirty: HashSet::new(),
            renderable_cache: HashMap::new(),
            bell_panes: HashSet::new(),
        }
    }
}

impl MuxBackend for EmbeddedMux {
    fn has_pending_wakeup(&self) -> bool {
        self.wakeup_pending.load(Ordering::Acquire)
    }

    fn poll_events(&mut self) {
        self.poll_events_impl();
    }

    fn drain_notifications(&mut self, out: &mut Vec<MuxNotification>) {
        self.mux.drain_notifications(out);
    }

    fn discard_notifications(&mut self) {
        self.mux.discard_notifications();
    }

    fn get_pane_entry(&self, pane_id: PaneId) -> Option<PaneEntry> {
        self.mux.get_pane_entry(pane_id).cloned()
    }

    fn spawn_pane(&mut self, config: &SpawnConfig, theme: Theme) -> io::Result<PaneId> {
        let (pane_id, pane) =
            self.mux
                .spawn_standalone_pane(config, theme, &self.guarded_wakeup)?;
        self.panes.insert(pane_id, pane);
        Ok(pane_id)
    }

    fn adopt_pane(
        &mut self,
        adopted: AdoptedPtyHandle,
        request: AdoptPaneRequest,
    ) -> io::Result<PaneId> {
        let (pane_id, pane) =
            self.mux
                .adopt_standalone_pane(adopted, request, &self.guarded_wakeup)?;
        self.panes.insert(pane_id, pane);
        Ok(pane_id)
    }

    fn close_pane(&mut self, pane_id: PaneId) -> ClosePaneResult {
        // Phase 1 (unregister + PaneClosed notify); Phase 2 is `cleanup_closed_pane` which drops Pane on a background thread (avoids blocking the event loop on PTY kill + child reap).
        self.mux.close_pane(pane_id)
    }

    fn fulfill_host_request(&mut self, pane_id: PaneId, reply: HostReply) -> io::Result<()> {
        let Some(pane) = self.panes.get(&pane_id) else {
            return Err(io::Error::other(format!(
                "fulfill_host_request: pane {pane_id} not found"
            )));
        };
        let handle = pane.io_handle();
        let result = match reply {
            HostReply::ClipboardLoad { token, text } => handle.fulfill_clipboard_load(&token, text),
            HostReply::ColorQuery { token, color } => handle.fulfill_color_query(&token, color),
        };
        if let Err(err) = result {
            log::warn!(
                "fulfill_host_request: duplicate fulfill on pane {pane_id} (routing bug): {err}"
            );
        }
        Ok(())
    }

    fn resize_pane_grid(&mut self, pane_id: PaneId, rows: u16, cols: u16) {
        if let Some(pane) = self.panes.get(&pane_id) {
            // IO thread reflows + PTY-resizes (SIGWINCH) asynchronously; do NOT mark snapshot_dirty here — renderer keeps the cached snapshot until IO publishes the resized one (prevents exposing intermediate reflow frames during drag resize).
            pane.send_resize(rows, cols);
        }
    }

    fn pane_mode(&self, pane_id: PaneId) -> Option<u64> {
        self.panes.get(&pane_id).map(Pane::mode)
    }

    fn pane_dec_locator_active(&self, pane_id: PaneId) -> bool {
        self.panes
            .get(&pane_id)
            .is_some_and(Pane::dec_locator_active)
    }

    fn set_pane_theme(&mut self, pane_id: PaneId, theme: Theme, palette: oriterm_core::Palette) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetTheme(theme, Box::new(palette)));
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn set_cursor_shape(&mut self, pane_id: PaneId, shape: oriterm_core::CursorShape) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetCursorShape(shape));
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn set_bold_is_bright(&mut self, pane_id: PaneId, enabled: bool) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetBoldIsBright(enabled));
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn set_answerback(&mut self, pane_id: PaneId, bytes: Vec<u8>) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetAnswerback(bytes));
            // Intentionally NO snapshot_dirty: answerback affects only ENQ response, no visible state (matches set_image_config — existing non-visual precedent).
        }
    }

    fn mark_all_dirty(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::MarkAllDirty);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn set_image_config(&mut self, pane_id: PaneId, config: ImageConfig) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetImageConfig(config));
        }
    }

    fn set_cell_dimensions(&mut self, pane_id: PaneId, width: u16, height: u16) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetCellDimensions { width, height });
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn open_search(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.open_search();
            pane.send_io_command(PaneIoCommand::OpenSearch);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn close_search(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.close_search();
            pane.send_io_command(PaneIoCommand::CloseSearch);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn search_set_query(&mut self, pane_id: PaneId, query: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.send_io_command(PaneIoCommand::SearchSetQuery(query));
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn search_next_match(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(search) = pane.search_mut() {
                search.next_match();
            }
            pane.send_io_command(PaneIoCommand::SearchNextMatch);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn search_prev_match(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(search) = pane.search_mut() {
                search.prev_match();
            }
            pane.send_io_command(PaneIoCommand::SearchPrevMatch);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn is_search_active(&self, pane_id: PaneId) -> bool {
        self.panes.get(&pane_id).is_some_and(Pane::is_search_active)
    }

    fn extract_text(&mut self, pane_id: PaneId, sel: &Selection) -> Option<String> {
        self.extract_text_impl(pane_id, sel)
    }

    fn extract_html(
        &mut self,
        pane_id: PaneId,
        sel: &Selection,
        font_family: &str,
        font_size: f32,
    ) -> Option<(String, String)> {
        self.extract_html_impl(pane_id, sel, font_family, font_size)
    }

    fn scroll_display(&mut self, pane_id: PaneId, delta: isize) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollDisplay(delta));
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn scroll_to_bottom(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollToBottom);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn scroll_to_previous_prompt(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollToPreviousPrompt);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn scroll_to_next_prompt(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollToNextPrompt);
            self.snapshot_dirty.insert(pane_id);
        }
    }

    fn send_input(&mut self, pane_id: PaneId, data: &[u8]) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.write_input(data);
        }
    }

    fn send_mouse_input(
        &mut self,
        pane_id: PaneId,
        event: &oriterm_core::encode::mouse::MouseEvent,
    ) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::HandleMouseInput(*event));
        }
    }

    fn observe_pane_locator_input(
        &mut self,
        pane_id: PaneId,
        event: &oriterm_core::encode::mouse::MouseEvent,
    ) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ObserveLocatorInput(*event));
        }
    }

    fn send_focus_event(&mut self, pane_id: PaneId, focused: bool) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::HandleFocusEvent(focused));
        }
    }

    fn is_write_stalled(&mut self, pane_id: PaneId) -> bool {
        self.panes.get(&pane_id).is_some_and(Pane::is_write_stalled)
    }

    fn signal_child(&mut self, pane_id: PaneId, signal: crate::Signal) -> bool {
        self.panes
            .get(&pane_id)
            .is_some_and(|p| p.signal_child(signal))
    }

    fn has_bell(&self, pane_id: PaneId) -> bool {
        // Bell state lives in client-local `bell_panes`, uniform with
        // MuxClient. Decoupled from snapshot replication.
        self.bell_panes.contains(&pane_id)
    }

    fn set_bell(&mut self, pane_id: PaneId) {
        self.bell_panes.insert(pane_id);
    }

    fn clear_bell(&mut self, pane_id: PaneId) {
        self.bell_panes.remove(&pane_id);
    }

    fn set_unseen_output(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.set_unseen_output();
        }
    }

    fn mark_output_seen(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.mark_output_seen();
        }
    }

    fn has_unseen_output(&self, pane_id: PaneId) -> bool {
        self.panes
            .get(&pane_id)
            .is_some_and(Pane::has_unseen_output)
    }

    fn cleanup_closed_pane(&mut self, pane_id: PaneId) {
        self.cleanup_closed_pane_impl(pane_id);
    }

    fn select_command_output(&self, pane_id: PaneId) -> Option<Selection> {
        self.select_command_output_impl(pane_id)
    }

    fn select_command_input(&self, pane_id: PaneId) -> Option<Selection> {
        self.select_command_input_impl(pane_id)
    }

    fn enter_mark_mode(&mut self, pane_id: PaneId) -> Option<MarkCursor> {
        self.enter_mark_mode_impl(pane_id)
    }

    fn pane_ids(&self) -> Vec<PaneId> {
        self.panes.keys().copied().collect()
    }

    fn event_tx(&self) -> Option<&mpsc::Sender<MuxEvent>> {
        Some(self.mux.event_tx())
    }

    fn default_domain(&self) -> DomainId {
        self.mux.default_domain()
    }

    fn is_daemon_mode(&self) -> bool {
        false
    }

    fn swap_renderable_content(&mut self, pane_id: PaneId, target: &mut RenderableContent) -> bool {
        let Some(cached) = self.renderable_cache.get_mut(&pane_id) else {
            return false;
        };
        // Whole-struct swap: both sides retain their Vec allocations.
        // Simpler than field-by-field and future-proof against new fields.
        std::mem::swap(target, cached);
        true
    }

    fn pane_snapshot(&self, pane_id: PaneId) -> Option<&PaneSnapshot> {
        self.snapshot_cache.get(&pane_id)
    }

    fn is_pane_snapshot_dirty(&self, pane_id: PaneId) -> bool {
        self.snapshot_dirty.contains(&pane_id)
    }

    fn refresh_pane_snapshot(&mut self, pane_id: PaneId) -> Option<&PaneSnapshot> {
        self.refresh_pane_snapshot_impl(pane_id)
    }

    fn sync_pane_snapshot(&mut self, pane_id: PaneId) -> Option<PaneSnapshot> {
        self.sync_pane_snapshot_impl(pane_id)
    }

    fn clear_pane_snapshot_dirty(&mut self, pane_id: PaneId) {
        self.snapshot_dirty.remove(&pane_id);
    }

    fn is_selection_dirty(&self, pane_id: PaneId) -> bool {
        self.panes
            .get(&pane_id)
            .is_some_and(Pane::is_io_selection_dirty)
    }

    fn clear_selection_dirty(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.clear_io_selection_dirty();
        }
    }

    fn maybe_shrink_renderable_caches(&mut self) {
        for content in self.renderable_cache.values_mut() {
            content.maybe_shrink();
        }
    }
}

#[cfg(test)]
mod tests;
