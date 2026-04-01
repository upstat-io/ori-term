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

use super::{ImageConfig, MuxBackend};
use crate::domain::SpawnConfig;
use crate::in_process::{ClosePaneResult, InProcessMux};
use crate::mux_event::{MuxEvent, MuxNotification};
use crate::pane::io_thread::PaneIoCommand;
use crate::pane::{MarkCursor, Pane};
use crate::registry::PaneEntry;
use crate::server::snapshot::fill_snapshot_from_renderable;
use crate::{DomainId, PaneId, PaneSnapshot};

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
        let wakeup_pending = Arc::new(AtomicBool::new(false));
        let guarded_wakeup = {
            let pending = wakeup_pending.clone();
            Arc::new(move || {
                if !pending.swap(true, Ordering::Release) {
                    (wakeup)();
                }
            }) as Arc<dyn Fn() + Send + Sync>
        };
        Self {
            mux: InProcessMux::new(),
            panes: HashMap::new(),
            guarded_wakeup,
            wakeup_pending,
            snapshot_cache: HashMap::new(),
            snapshot_dirty: HashSet::new(),
            renderable_cache: HashMap::new(),
        }
    }
}

impl MuxBackend for EmbeddedMux {
    fn has_pending_wakeup(&self) -> bool {
        self.wakeup_pending.load(Ordering::Acquire)
    }

    fn poll_events(&mut self) {
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

    fn close_pane(&mut self, pane_id: PaneId) -> ClosePaneResult {
        // Phase 1: unregister from the pane registry and push a PaneClosed
        // notification. The pane itself remains in `self.panes` so the PTY
        // process continues running until `cleanup_closed_pane` is called
        // (Phase 2), which drops the Pane on a background thread to avoid
        // blocking the event loop with PTY kill + child reap.
        self.mux.close_pane(pane_id)
    }

    fn resize_pane_grid(&mut self, pane_id: PaneId, rows: u16, cols: u16) {
        if let Some(pane) = self.panes.get(&pane_id) {
            // IO thread does reflow + PTY resize (SIGWINCH) asynchronously.
            // Do NOT mark snapshot_dirty here — the renderer should keep
            // drawing the previous cached snapshot until the IO thread
            // publishes the resized one. This prevents exposing
            // intermediate reflow frames during drag resize (TPR-05-001).
            pane.send_io_command(PaneIoCommand::Resize { rows, cols });
        }
    }

    fn pane_mode(&self, pane_id: PaneId) -> Option<u32> {
        self.panes.get(&pane_id).map(Pane::mode)
    }

    fn set_pane_theme(&mut self, pane_id: PaneId, theme: Theme, palette: oriterm_core::Palette) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetTheme(theme, Box::new(palette)));
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn set_cursor_shape(&mut self, pane_id: PaneId, shape: oriterm_core::CursorShape) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetCursorShape(shape));
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn set_bold_is_bright(&mut self, pane_id: PaneId, enabled: bool) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetBoldIsBright(enabled));
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn mark_all_dirty(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::MarkAllDirty);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn set_image_config(&mut self, pane_id: PaneId, config: ImageConfig) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::SetImageConfig(config));
        }
    }

    fn open_search(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.open_search();
            pane.send_io_command(PaneIoCommand::OpenSearch);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn close_search(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.close_search();
            pane.send_io_command(PaneIoCommand::CloseSearch);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn search_set_query(&mut self, pane_id: PaneId, query: String) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.send_io_command(PaneIoCommand::SearchSetQuery(query));
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn search_next_match(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(search) = pane.search_mut() {
                search.next_match();
            }
            pane.send_io_command(PaneIoCommand::SearchNextMatch);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn search_prev_match(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            if let Some(search) = pane.search_mut() {
                search.prev_match();
            }
            pane.send_io_command(PaneIoCommand::SearchPrevMatch);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn is_search_active(&self, pane_id: PaneId) -> bool {
        self.panes.get(&pane_id).is_some_and(Pane::is_search_active)
    }

    fn extract_text(&mut self, pane_id: PaneId, sel: &Selection) -> Option<String> {
        use std::time::Duration;
        let pane = self.panes.get(&pane_id)?;
        let (tx, rx) = crossbeam_channel::bounded(1);
        pane.send_io_command(PaneIoCommand::ExtractText {
            selection: *sel,
            reply: tx,
        });
        rx.recv_timeout(Duration::from_millis(100)).ok().flatten()
    }

    fn extract_html(
        &mut self,
        pane_id: PaneId,
        sel: &Selection,
        font_family: &str,
        font_size: f32,
    ) -> Option<(String, String)> {
        use std::time::Duration;
        let pane = self.panes.get(&pane_id)?;
        let (tx, rx) = crossbeam_channel::bounded(1);
        pane.send_io_command(PaneIoCommand::ExtractHtml {
            selection: *sel,
            font_family: font_family.to_string(),
            font_size,
            reply: tx,
        });
        rx.recv_timeout(Duration::from_millis(100)).ok().flatten()
    }

    fn scroll_display(&mut self, pane_id: PaneId, delta: isize) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollDisplay(delta));
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn scroll_to_bottom(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollToBottom);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn scroll_to_previous_prompt(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollToPreviousPrompt);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn scroll_to_next_prompt(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.send_io_command(PaneIoCommand::ScrollToNextPrompt);
        }
        self.snapshot_dirty.insert(pane_id);
    }

    fn send_input(&mut self, pane_id: PaneId, data: &[u8]) {
        if let Some(pane) = self.panes.get(&pane_id) {
            pane.write_input(data);
        }
    }

    fn set_bell(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.set_bell();
        }
    }

    fn clear_bell(&mut self, pane_id: PaneId) {
        if let Some(pane) = self.panes.get_mut(&pane_id) {
            pane.clear_bell();
        }
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
        if let Some(pane) = self.panes.remove(&pane_id) {
            self.snapshot_cache.remove(&pane_id);
            self.snapshot_dirty.remove(&pane_id);
            self.renderable_cache.remove(&pane_id);
            // Drop on a background thread to avoid blocking the event loop.
            // Pane destruction involves PTY kill, reader thread join, and child reap.
            std::thread::spawn(move || drop(pane));
        }
    }

    fn select_command_output(&self, pane_id: PaneId) -> Option<Selection> {
        use std::time::Duration;
        let pane = self.panes.get(&pane_id)?;
        let (tx, rx) = crossbeam_channel::bounded(1);
        pane.send_io_command(PaneIoCommand::SelectCommandOutput { reply: tx });
        rx.recv_timeout(Duration::from_millis(100)).ok().flatten()
    }

    fn select_command_input(&self, pane_id: PaneId) -> Option<Selection> {
        use std::time::Duration;
        let pane = self.panes.get(&pane_id)?;
        let (tx, rx) = crossbeam_channel::bounded(1);
        pane.send_io_command(PaneIoCommand::SelectCommandInput { reply: tx });
        rx.recv_timeout(Duration::from_millis(100)).ok().flatten()
    }

    fn enter_mark_mode(&mut self, pane_id: PaneId) -> Option<MarkCursor> {
        use std::time::Duration;
        let pane = self.panes.get(&pane_id)?;
        let (tx, rx) = crossbeam_channel::bounded(1);
        pane.send_io_command(PaneIoCommand::EnterMarkMode { reply: tx });
        rx.recv_timeout(Duration::from_millis(100)).ok()
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
