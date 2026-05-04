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
use crate::protocol::messages::theme_to_wire;
use crate::protocol::{HostReplyPayload, MuxPdu, WireSelection};
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
        if let Some(transport) = &self.transport {
            transport.clear_wakeup_pending();
            transport.poll_notifications(&mut self.notifications);
        }

        // Scan buffered notifications ONLY to mark panes dirty for rendering.
        // Bell-indicator state lives in `self.bell_panes` and is mutated by
        // the App's MuxNotification::PaneBell arm (via `set_bell`) AFTER its
        // focus-gate decision — `poll_events` does NOT make focus decisions
        // and must NOT mutate bell state. Per BUG-11-028 + Plan TPR Round 1
        // gemini F2: App is the SSOT for focus-decided bell state.
        for notif in &self.notifications {
            if let MuxNotification::PaneOutput(pane_id) = notif {
                self.dirty_panes.insert(*pane_id);
            }
        }
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
        let pdu = MuxPdu::SpawnPane {
            shell: config.shell.clone(),
            cwd: config.cwd.as_ref().map(|p| p.display().to_string()),
            theme: theme_to_wire(theme).map(str::to_owned),
        };

        match self.rpc(pdu)? {
            MuxPdu::SpawnPaneResponse { pane_id } => {
                // Subscribe to the new pane and cache its initial snapshot.
                self.subscribe_pane(pane_id);
                log::info!("daemon spawned pane {pane_id}");
                Ok(pane_id)
            }
            other => Err(io::Error::other(format!(
                "spawn_pane: unexpected response: {other:?}"
            ))),
        }
    }

    fn close_pane(&mut self, pane_id: PaneId) -> ClosePaneResult {
        match self.rpc(MuxPdu::ClosePane { pane_id }) {
            Ok(MuxPdu::PaneClosedAck) => {
                self.remove_snapshot(pane_id);
                ClosePaneResult::PaneRemoved
            }
            Ok(other) => {
                log::error!("close_pane: unexpected response: {other:?}");
                ClosePaneResult::NotFound
            }
            Err(e) => {
                log::error!("close_pane: RPC failed: {e}");
                ClosePaneResult::NotFound
            }
        }
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
        // Bell-indicator state is client-local UI state per BUG-11-028.
        // Reads from `bell_panes`, NOT `pane_snapshots[*].has_bell`
        // (which has been removed from the wire as of this fix). The App
        // populates `bell_panes` via `set_bell` after its focus-gate
        // decision; `clear_bell` (focus-clear path) and
        // `cleanup_closed_pane` (pane lifecycle) remove entries.
        self.bell_panes.contains(&pane_id)
    }

    fn set_bell(&mut self, pane_id: PaneId) {
        self.bell_panes.insert(pane_id);
    }

    fn clear_bell(&mut self, pane_id: PaneId) {
        self.bell_panes.remove(&pane_id);
    }

    fn cleanup_closed_pane(&mut self, pane_id: PaneId) {
        // Per BUG-11-028 Plan TPR Round 1 critical (gemini+opencode
        // agreement): without this override the trait default at
        // `backend/mod.rs:346` is a no-op and `bell_panes` leaks for
        // every notification-driven pane closure (shell exit, PTY EOF).
        self.bell_panes.remove(&pane_id);
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

    fn fulfill_host_request(&mut self, _pane_id: PaneId, reply: HostReply) -> io::Result<()> {
        // : package the reply into a `MuxPdu::ReplyHostRequest` and
        // fire it back to the daemon. The pending entry — created by the
        // reader thread when it received the originating notification — is
        // looked up by the token's `slot_id` to recover the daemon-allocated
        // `request_id` that must be echoed back.
        let transport = self.transport.as_mut().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotConnected, "not connected to daemon")
        })?;
        let (slot_id, payload) = match reply {
            HostReply::ClipboardLoad { token, text } => {
                (token.slot_id(), HostReplyPayload::ClipboardLoad { text })
            }
            HostReply::ColorQuery { token, color } => (
                token.slot_id(),
                HostReplyPayload::ColorQuery {
                    rgb: [color.r, color.g, color.b],
                },
            ),
        };
        let Some(pending) = transport.take_pending_reply(slot_id) else {
            log::warn!(
                "fulfill_host_request: no pending reply for slot_id {slot_id:?} \
                 (token already replied or never registered)"
            );
            return Ok(());
        };
        let request_id = pending.request_id();
        transport.try_fire_and_forget(MuxPdu::ReplyHostRequest {
            request_id,
            payload,
        })
    }

    // -- Snapshot access --

    fn pane_snapshot(&self, pane_id: PaneId) -> Option<&PaneSnapshot> {
        self.pane_snapshots.get(&pane_id)
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
                // Cache the fresh snapshot so subsequent pane_snapshot()
                // calls see it. Also drop any pending refresh marker —
                // we just got a fresher value than any in-flight push.
                self.pane_snapshots.insert(pane_id, snapshot.clone());
                self.pending_refresh.remove(&pane_id);
                Some(snapshot)
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
            self.pane_snapshots.insert(pane_id, snapshot);
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
