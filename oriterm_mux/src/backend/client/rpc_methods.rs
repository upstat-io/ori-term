//! IPC roundtrip implementations for [`MuxBackend`] methods.
//!
//! Each method sends a PDU to the daemon via [`MuxClient::rpc`],
//! extracts the response, and marks snapshots dirty as needed.

use std::io;
use std::sync::mpsc;

use oriterm_core::Theme;
use oriterm_core::selection::Selection;

use crate::PaneSnapshot;
use crate::backend::{ImageConfig, MuxBackend};
use crate::domain::SpawnConfig;
use crate::in_process::ClosePaneResult;
use crate::mux_event::{MuxEvent, MuxNotification};
use crate::registry::PaneEntry;

use crate::backend::HostReply;
use crate::protocol::theme_to_wire;
use crate::protocol::{MuxPdu, WireSelection};
use crate::{DomainId, PaneId};

use super::MuxClient;
use super::transport::ClientTransport;

impl MuxBackend for MuxClient {
    fn has_pending_wakeup(&self) -> bool {
        self.transport
            .as_ref()
            .is_some_and(ClientTransport::has_pending_wakeup)
    }

    fn poll_events(&mut self) {
        self.poll_events_impl();
    }

    fn drain_notifications(&mut self, out: &mut Vec<MuxNotification>) {
        out.clear();
        std::mem::swap(&mut self.notifications, out);
    }

    fn discard_notifications(&mut self) {
        self.notifications.clear();
    }

    fn get_pane_entry(&self, _pane_id: PaneId) -> Option<PaneEntry> {
        // Daemon mode: no local pane registry.
        None
    }

    fn spawn_pane(&mut self, config: &SpawnConfig, theme: Theme) -> io::Result<PaneId> {
        self.spawn_pane_impl(config, theme)
    }

    fn close_pane(&mut self, pane_id: PaneId) -> ClosePaneResult {
        self.close_pane_impl(pane_id)
    }

    fn resize_pane_grid(&mut self, pane_id: PaneId, rows: u16, cols: u16) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::Resize {
                pane_id,
                cols,
                rows,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn pane_mode(&self, pane_id: PaneId) -> Option<u64> {
        self.pane_snapshots.get(&pane_id).map(|s| s.modes)
    }

    /// See: bug-tracker/section-11-mux.md (BUG-11-067) — daemon DEC Locator
    /// wire propagation. Until that lands, daemon mode reports the locator
    /// inactive; embedded mode (the default) reads the real lock-free cache.
    fn pane_dec_locator_active(&self, _pane_id: PaneId) -> bool {
        false
    }

    fn set_pane_theme(&mut self, pane_id: PaneId, theme: Theme, palette: oriterm_core::Palette) {
        let theme_str = theme_to_wire(theme).unwrap_or("dark").to_owned();
        let palette_rgb: Vec<[u8; 3]> = (0..270)
            .map(|i| {
                let rgb = palette.color(i);
                [rgb.r, rgb.g, rgb.b]
            })
            .collect();
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SetTheme {
                pane_id,
                theme: theme_str,
                palette_rgb,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn set_cursor_shape(&mut self, pane_id: PaneId, shape: oriterm_core::CursorShape) {
        let wire = crate::WireCursorShape::from(shape) as u8;
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SetCursorShape {
                pane_id,
                shape: wire,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn set_bold_is_bright(&mut self, pane_id: PaneId, enabled: bool) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SetBoldIsBright { pane_id, enabled });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn set_answerback(&mut self, pane_id: PaneId, bytes: Vec<u8>) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SetAnswerback { pane_id, bytes });
            // Intentionally NO transport.invalidate_pushed_snapshot:
            // answerback doesn't change rendered cells. Deliberately
            // diverges from set_bold_is_bright AND set_image_config
            // (both call invalidate at this same site) because answerback
            // has zero visual effect on either backend.
        }
        // Intentionally NO self.dirty_panes.insert: same reason.
    }

    fn mark_all_dirty(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::MarkAllDirty { pane_id });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn set_image_config(&mut self, pane_id: PaneId, config: ImageConfig) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SetImageConfig {
                pane_id,
                enabled: config.enabled,
                memory_limit: config.memory_limit as u64,
                max_single: config.max_single as u64,
                animation_enabled: config.animation_enabled,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn set_cell_dimensions(&mut self, pane_id: PaneId, width: u16, height: u16) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SetCellDimensions {
                pane_id,
                width,
                height,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn open_search(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::OpenSearch { pane_id });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn close_search(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::CloseSearch { pane_id });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn search_set_query(&mut self, pane_id: PaneId, query: String) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SearchSetQuery { pane_id, query });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn search_next_match(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SearchNextMatch { pane_id });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn search_prev_match(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SearchPrevMatch { pane_id });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn is_search_active(&self, pane_id: PaneId) -> bool {
        self.pane_snapshots
            .get(&pane_id)
            .is_some_and(|s| s.search_active)
    }

    fn extract_text(&mut self, pane_id: PaneId, selection: &Selection) -> Option<String> {
        let pdu = MuxPdu::ExtractText {
            pane_id,
            selection: WireSelection::from_selection(selection),
        };
        match self.rpc(pdu) {
            Ok(MuxPdu::ExtractTextResp { text }) => (!text.is_empty()).then_some(text),
            Ok(other) => {
                log::error!("extract_text: unexpected response: {other:?}");
                None
            }
            Err(e) => {
                log::error!("extract_text: RPC failed: {e}");
                None
            }
        }
    }

    fn extract_html(
        &mut self,
        pane_id: PaneId,
        selection: &Selection,
        font_family: &str,
        font_size: f32,
    ) -> Option<(String, String)> {
        let pdu = MuxPdu::ExtractHtml {
            pane_id,
            selection: WireSelection::from_selection(selection),
            font_family: font_family.to_string(),
            font_size_x100: (font_size * 100.0) as u16,
        };
        match self.rpc(pdu) {
            Ok(MuxPdu::ExtractHtmlResp { html, text }) => {
                (!text.is_empty()).then_some((html, text))
            }
            Ok(other) => {
                log::error!("extract_html: unexpected response: {other:?}");
                None
            }
            Err(e) => {
                log::error!("extract_html: RPC failed: {e}");
                None
            }
        }
    }

    fn scroll_display(&mut self, pane_id: PaneId, delta: isize) {
        let wire_delta = delta.clamp(i32::MIN as isize, i32::MAX as isize) as i32;
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::ScrollDisplay {
                pane_id,
                delta: wire_delta,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn scroll_to_bottom(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::ScrollToBottom { pane_id });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn scroll_to_previous_prompt(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::ScrollToPrompt {
                pane_id,
                direction: -1,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn scroll_to_next_prompt(&mut self, pane_id: PaneId) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::ScrollToPrompt {
                pane_id,
                direction: 1,
            });
            transport.invalidate_pushed_snapshot(pane_id);
        }
        self.dirty_panes.insert(pane_id);
    }

    fn send_input(&mut self, pane_id: PaneId, data: &[u8]) {
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::Input {
                pane_id,
                data: data.to_vec(),
            });
        }
    }

    fn is_write_stalled(&mut self, pane_id: PaneId) -> bool {
        match self.rpc(MuxPdu::IsWriteStalled { pane_id }) {
            Ok(MuxPdu::WriteStalledStatus { stalled, .. }) => stalled,
            Ok(other) => {
                log::error!("is_write_stalled: unexpected response: {other:?}");
                false
            }
            Err(e) => {
                log::error!("is_write_stalled: RPC failed: {e}");
                false
            }
        }
    }

    fn has_bell(&self, pane_id: PaneId) -> bool {
        // Bell-indicator state is client-local UI state. Reads from
        // `bell_panes`, NOT from any snapshot field — the snapshot wire
        // does not carry bell state. The App populates `bell_panes` via
        // `set_bell` after its focus-gate decision; `clear_bell`
        // (focus-clear path) and `cleanup_closed_pane` (pane lifecycle)
        // remove entries.
        self.bell_panes.contains(&pane_id)
    }

    fn set_bell(&mut self, pane_id: PaneId) {
        self.bell_panes.insert(pane_id);
    }

    fn clear_bell(&mut self, pane_id: PaneId) {
        self.bell_panes.remove(&pane_id);
    }

    fn cleanup_closed_pane(&mut self, pane_id: PaneId) {
        // The trait default body is a no-op. Without this override the
        // notification-driven close path (shell exit, PTY EOF) would
        // leak every per-pane backend cache. `remove_snapshot` is the
        // SSOT for client-side close cleanup — it drains
        // `pane_snapshots`, `dirty_panes`, `pending_refresh`,
        // `bell_panes`, and the transport's pushed-snapshot cache in
        // one place.
        self.remove_snapshot(pane_id);
    }

    fn signal_child(&mut self, pane_id: PaneId, signal: crate::Signal) -> bool {
        let sig = match signal {
            crate::Signal::Interrupt => 0,
        };
        if let Some(transport) = &mut self.transport {
            transport.fire_and_forget(MuxPdu::SignalChild {
                pane_id,
                signal: sig,
            });
            return true;
        }
        false
    }

    fn pane_ids(&self) -> Vec<PaneId> {
        self.pane_snapshots.keys().copied().collect()
    }

    fn event_tx(&self) -> Option<&mpsc::Sender<MuxEvent>> {
        // No local event channel in daemon mode.
        None
    }

    fn default_domain(&self) -> DomainId {
        DomainId::LOCAL
    }

    fn is_connected(&self) -> bool {
        // Delegates to the inherent `MuxClient::is_connected` which checks
        // transport liveness, overriding the trait's default `true`.
        Self::is_connected(self)
    }

    fn is_daemon_mode(&self) -> bool {
        true
    }

    fn subscribe(&mut self, pane_id: PaneId) -> io::Result<()> {
        match self.rpc(MuxPdu::Subscribe { pane_id })? {
            MuxPdu::Subscribed { snapshot } => {
                self.cache_snapshot(pane_id, snapshot);
                log::debug!("subscribed to pane {pane_id}");
                Ok(())
            }
            other => Err(io::Error::other(format!(
                "subscribe: unexpected response: {other:?}"
            ))),
        }
    }

    fn unsubscribe(&mut self, pane_id: PaneId) -> io::Result<()> {
        match self.rpc(MuxPdu::Unsubscribe { pane_id })? {
            MuxPdu::Unsubscribed => {
                self.remove_snapshot(pane_id);
                log::debug!("unsubscribed from pane {pane_id}");
                Ok(())
            }
            other => Err(io::Error::other(format!(
                "unsubscribe: unexpected response: {other:?}"
            ))),
        }
    }

    fn fulfill_host_request(&mut self, pane_id: PaneId, reply: HostReply) -> io::Result<()> {
        self.fulfill_host_request_impl(pane_id, reply)
    }

    // -- Snapshot access --

    fn pane_snapshot(&self, pane_id: PaneId) -> Option<&PaneSnapshot> {
        self.pane_snapshots.get(&pane_id)
    }

    fn pane_image_data(
        &self,
        pane_id: PaneId,
        image_id: oriterm_core::ImageId,
    ) -> Option<std::sync::Arc<oriterm_core::RenderableImageData>> {
        Self::pane_image_data(self, pane_id, image_id)
    }

    fn is_pane_snapshot_dirty(&self, pane_id: PaneId) -> bool {
        self.dirty_panes.contains(&pane_id)
    }

    fn sync_pane_snapshot(&mut self, pane_id: PaneId) -> Option<PaneSnapshot> {
        // The daemon's GetPaneSnapshot dispatcher sends a SnapshotNow IO
        // barrier and waits for the IO thread to drain in-flight commands
        // before building the response. So a single round-trip RPC is
        // enough to get a guaranteed-fresh snapshot.
        let pdu = MuxPdu::GetPaneSnapshot { pane_id };
        match self.rpc(pdu) {
            Ok(MuxPdu::PaneSnapshotResp { snapshot }) => {
                // Route through `cache_snapshot` SSOT so `image_data` drains
                // into the bounded image_cache (preventing unbounded
                // pane_snapshots growth) and the pending_refresh marker
                // clears via the same code path.
                let result = snapshot.clone();
                self.cache_snapshot(pane_id, snapshot);
                self.pending_refresh.remove(&pane_id);
                Some(result)
            }
            Ok(other) => {
                log::error!("sync_pane_snapshot: unexpected response: {other:?}");
                None
            }
            Err(e) => {
                log::error!("sync_pane_snapshot: RPC failed: {e}");
                None
            }
        }
    }

    fn refresh_pane_snapshot(&mut self, pane_id: PaneId) -> Option<&PaneSnapshot> {
        // Fast path: server-pushed snapshot available.
        let pushed = self
            .transport
            .as_ref()
            .and_then(|t| t.take_pushed_snapshot(pane_id));
        if let Some(snapshot) = pushed {
            // Route through cache_snapshot SSOT (drains image_data into the
            // bounded image_cache before storing the stripped snapshot).
            self.cache_snapshot(pane_id, snapshot);
            self.pending_refresh.remove(&pane_id);
            return self.pane_snapshots.get(&pane_id);
        }

        // Non-blocking fallback: trigger a pushed snapshot via MarkAllDirty.
        // Returns stale data (or None for brand-new panes). The daemon will
        // push a NotifyPaneSnapshot via the subscription channel, picked up
        // on the next event loop tick.
        //
        // Note: call fire_and_forget directly — do NOT use the mark_all_dirty()
        // wrapper which calls invalidate_pushed_snapshot(), dropping the very
        // snapshot we are waiting for.
        if self.pending_refresh.insert(pane_id) {
            if let Some(transport) = &mut self.transport {
                transport.fire_and_forget(MuxPdu::MarkAllDirty { pane_id });
            }
        }
        self.dirty_panes.insert(pane_id);
        self.pane_snapshots.get(&pane_id)
    }

    fn clear_pane_snapshot_dirty(&mut self, pane_id: PaneId) {
        // Don't clear dirty if an async refresh is still outstanding —
        // the pane must stay dirty so the next render tick retries.
        if self.pending_refresh.contains(&pane_id) {
            return;
        }
        self.dirty_panes.remove(&pane_id);
    }
}
