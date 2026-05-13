//! Request dispatch for the mux server.
//!
//! Matches incoming [`MuxPdu`] request variants and calls the appropriate
//! [`InProcessMux`] methods, returning response PDUs.

mod helpers;
mod types;

pub(in crate::server) use helpers::parse_theme;
pub(in crate::server) use helpers::remove_client_subscriptions;
use helpers::{dispatch_extract_html, dispatch_extract_text, dispatch_hello};
pub(super) use types::{DispatchContext, DispatchResult};

use std::path::PathBuf;

use oriterm_core::{CursorShape, Palette, Rgb};

use crate::MuxPdu;
use crate::PaneId;
use crate::domain::SpawnConfig;
use crate::id::HostRequestId;
use crate::pane::io_thread::PaneIoCommand;
use crate::protocol::HostReplyPayload;
use crate::server::host_request::PendingHostReplyKind;

use super::connection::ClientConnection;

use self::helpers::drop_pane_background;

/// Dispatch a client request PDU to the mux, returning a [`DispatchResult`].
///
/// The result contains the response PDU and side-effect flags that the
/// caller uses for subscription sync and pending-push cleanup.
#[allow(
    clippy::too_many_lines,
    reason = "exhaustive match dispatch — splitting would scatter the routing table"
)]
pub fn dispatch_request(
    ctx: &mut DispatchContext<'_>,
    conn: &mut ClientConnection,
    pdu: MuxPdu,
) -> DispatchResult {
    // Extract side-effect signals before consuming the PDU in the match.
    let sub_changed = matches!(&pdu, MuxPdu::Subscribe { .. } | MuxPdu::Unsubscribe { .. });
    let unsub_pane = match &pdu {
        MuxPdu::Unsubscribe { pane_id } => Some(*pane_id),
        _ => None,
    };
    let is_new_tab = matches!(&pdu, MuxPdu::RequestNewTab);
    let mut evicted_image_keys: Vec<(PaneId, oriterm_core::ImageId)> = Vec::new();
    let mut pending_image_mutations: Option<super::push::PendingImageMutations> = None;

    let response = match pdu {
        MuxPdu::Hello {
            pid,
            protocol_version,
            features,
        } => Some(dispatch_hello(conn.id(), pid, protocol_version, features)),

        MuxPdu::SpawnPane { shell, cwd, theme } => {
            let config = SpawnConfig {
                shell,
                cwd: cwd.map(PathBuf::from),
                ..SpawnConfig::default()
            };
            let theme = parse_theme(theme.as_deref());
            match ctx.mux.spawn_standalone_pane(&config, theme, ctx.wakeup) {
                Ok((pane_id, pane)) => {
                    ctx.panes.insert(pane_id, pane);
                    log::debug!("spawned {pane_id}");
                    Some(MuxPdu::SpawnPaneResponse { pane_id })
                }
                Err(e) => Some(MuxPdu::Error {
                    message: format!("spawn_pane failed: {e}"),
                }),
            }
        }

        MuxPdu::ListPanes => {
            let pane_ids: Vec<_> = ctx.panes.keys().copied().collect();
            Some(MuxPdu::ListPanesResponse { pane_ids })
        }

        MuxPdu::ClosePane { pane_id } => {
            ctx.mux.close_pane(pane_id);
            drop_pane_background(ctx.panes.remove(&pane_id));
            ctx.snapshot_cache.remove(pane_id);
            ctx.closed_panes.push(pane_id);
            log::debug!("closed {pane_id}");
            Some(MuxPdu::PaneClosedAck)
        }

        MuxPdu::Input { pane_id, data } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.write_input(&data);
            }
            None // Fire-and-forget.
        }

        MuxPdu::SignalChild { pane_id, signal } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                if let Some(sig) = signal_from_wire(signal) {
                    pane.signal_child(sig);
                } else {
                    log::warn!("unknown signal {signal} for {pane_id}");
                }
            }
            None // Fire-and-forget.
        }

        MuxPdu::Resize {
            pane_id,
            cols,
            rows,
        } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                // IO thread does reflow + PTY resize (SIGWINCH).
                // Do NOT push an immediate snapshot — the IO thread will
                // produce one after reflow completes. This prevents
                // exposing intermediate reflow frames.
                pane.send_resize(rows, cols);
            }
            None // Fire-and-forget.
        }

        MuxPdu::ScrollDisplay { pane_id, delta } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::ScrollDisplay(delta as isize));
            }
            None
        }

        MuxPdu::ScrollToBottom { pane_id } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::ScrollToBottom);
            }
            None
        }

        MuxPdu::ScrollToPrompt { pane_id, direction } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                let cmd = if direction < 0 {
                    PaneIoCommand::ScrollToPreviousPrompt
                } else {
                    PaneIoCommand::ScrollToNextPrompt
                };
                pane.send_io_command(cmd);
            }
            None
        }

        MuxPdu::SetTheme {
            pane_id,
            theme,
            palette_rgb,
        } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                let theme = parse_theme(Some(&theme));
                let mut palette = Palette::for_theme(theme);
                for (i, rgb) in palette_rgb.iter().enumerate().take(270) {
                    palette.set_indexed(
                        i,
                        Rgb {
                            r: rgb[0],
                            g: rgb[1],
                            b: rgb[2],
                        },
                    );
                }
                pane.send_io_command(PaneIoCommand::SetTheme(theme, Box::new(palette)));
            }
            None
        }

        MuxPdu::SetCursorShape { pane_id, shape } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                let wire = crate::WireCursorShape::from_u8(shape);
                let core_shape = CursorShape::from(wire);
                pane.send_io_command(PaneIoCommand::SetCursorShape(core_shape));
            }
            None
        }

        MuxPdu::SetBoldIsBright { pane_id, enabled } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::SetBoldIsBright(enabled));
            }
            None
        }

        MuxPdu::SetAnswerback { pane_id, bytes } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::SetAnswerback(bytes));
            }
            None
        }

        MuxPdu::SetCellDimensions {
            pane_id,
            width,
            height,
        } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::SetCellDimensions { width, height });
            }
            None
        }

        MuxPdu::MarkAllDirty { pane_id } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::MarkAllDirty);
            }
            None
        }

        MuxPdu::OpenSearch { pane_id } => {
            if let Some(pane) = ctx.panes.get_mut(&pane_id) {
                pane.open_search();
                pane.send_io_command(PaneIoCommand::OpenSearch);
            }
            None
        }

        MuxPdu::CloseSearch { pane_id } => {
            if let Some(pane) = ctx.panes.get_mut(&pane_id) {
                pane.close_search();
                pane.send_io_command(PaneIoCommand::CloseSearch);
            }
            None
        }

        MuxPdu::SearchSetQuery { pane_id, query } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::SearchSetQuery(query));
            }
            None
        }

        MuxPdu::SearchNextMatch { pane_id } => {
            if let Some(pane) = ctx.panes.get_mut(&pane_id) {
                if let Some(search) = pane.search_mut() {
                    search.next_match();
                }
                pane.send_io_command(PaneIoCommand::SearchNextMatch);
            }
            None
        }

        MuxPdu::SearchPrevMatch { pane_id } => {
            if let Some(pane) = ctx.panes.get_mut(&pane_id) {
                if let Some(search) = pane.search_mut() {
                    search.prev_match();
                }
                pane.send_io_command(PaneIoCommand::SearchPrevMatch);
            }
            None
        }

        MuxPdu::SetImageConfig {
            pane_id,
            enabled,
            memory_limit,
            max_single,
            animation_enabled,
        } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.send_io_command(PaneIoCommand::SetImageConfig(crate::backend::ImageConfig {
                    enabled,
                    memory_limit: memory_limit as usize,
                    max_single: max_single as usize,
                    animation_enabled,
                }));
            }
            None
        }

        MuxPdu::SetCapabilities { flags } => {
            conn.set_capabilities(flags);
            log::info!("client {} capabilities: 0x{flags:08x}", conn.id());
            None // Fire-and-forget — no ack.
        }

        MuxPdu::SetPanePriority { pane_id, priority } => {
            conn.set_pane_priority(pane_id, priority);
            None // Fire-and-forget.
        }

        MuxPdu::RequestNewTab => {
            log::info!("new-tab request from client {}", conn.id());
            Some(MuxPdu::NewTabAck)
        }

        MuxPdu::Ping => Some(MuxPdu::PingAck),

        MuxPdu::Shutdown => {
            log::info!("shutdown requested by client {}", conn.id());
            Some(MuxPdu::ShutdownAck)
        }

        MuxPdu::Subscribe { pane_id } => {
            conn.subscribe(pane_id);
            match ctx.panes.get(&pane_id) {
                Some(pane) => {
                    let (snap, evicted) = ctx.snapshot_cache.build_and_take(pane_id, pane);
                    evicted_image_keys.extend(evicted);
                    let (snap, mutations) = super::push::project_per_client_pure(
                        pane_id,
                        &snap,
                        None,
                        conn,
                        ctx.snapshot_cache,
                    );
                    pending_image_mutations = Some(mutations);
                    Some(MuxPdu::Subscribed { snapshot: snap })
                }
                None => Some(MuxPdu::Error {
                    message: format!("pane not found: {pane_id}"),
                }),
            }
        }

        MuxPdu::Unsubscribe { pane_id } => {
            conn.unsubscribe(pane_id);
            // : drop pending host-replies the unsubscribing client
            // owned for this pane — the responder is leaving the pane's
            // notification stream and won't observe / fulfill any further
            // notifications. Without this the daemon would leak entries
            // that can only be reaped on full disconnect.
            let cid = conn.id();
            let dropped = ctx
                .pending_host_replies
                .iter()
                .filter(|(_, v)| v.pane_id == pane_id && v.responder == cid)
                .count();
            ctx.pending_host_replies
                .retain(|_, v| !(v.pane_id == pane_id && v.responder == cid));
            if dropped > 0 {
                log::warn!(
                    "Unsubscribe {pane_id} from {cid}: dropped {dropped} pending host-request token(s)"
                );
            }
            Some(MuxPdu::Unsubscribed)
        }

        MuxPdu::IsWriteStalled { pane_id } => {
            let stalled = ctx
                .panes
                .get(&pane_id)
                .is_some_and(crate::pane::Pane::is_write_stalled);
            Some(MuxPdu::WriteStalledStatus { pane_id, stalled })
        }

        MuxPdu::ReplyHostRequest {
            request_id,
            payload,
        } => {
            let request_id = HostRequestId::from_raw(request_id);
            if let Some(pending) = ctx.take_validated_pending_host_reply(request_id, conn.id()) {
                match (pending.kind, payload) {
                    (
                        PendingHostReplyKind::Clipboard(token),
                        HostReplyPayload::ClipboardLoad { text },
                    ) => {
                        if let Err(e) = token.fulfill(text) {
                            log::warn!(
                                "ReplyHostRequest {request_id}: clipboard token already fulfilled: {e}"
                            );
                        }
                    }
                    (PendingHostReplyKind::Color(token), HostReplyPayload::ColorQuery { rgb }) => {
                        let color = Rgb {
                            r: rgb[0],
                            g: rgb[1],
                            b: rgb[2],
                        };
                        if let Err(e) = token.fulfill(color) {
                            log::warn!(
                                "ReplyHostRequest {request_id}: color token already fulfilled: {e}"
                            );
                        }
                    }
                    _ => log::warn!("ReplyHostRequest {request_id}: payload-kind mismatch (drop)"),
                }
            }
            None // Fire-and-forget — no response PDU.
        }

        MuxPdu::GetPaneSnapshot { pane_id } => match ctx.panes.get(&pane_id) {
            Some(pane) => {
                use std::time::Duration;
                // IO thread barrier: wait for all earlier commands (e.g.
                // ScrollDisplay) to drain and a fresh snapshot to be
                // published. Without this, callers chaining
                // `scroll_display` + `GetPaneSnapshot` could read a stale
                // snapshot because IO command processing and snapshot
                // pushes are decoupled from the wire response path.
                //
                // If the IO thread fails to acknowledge within 500ms,
                // return an explicit error rather than silently
                // returning a stale snapshot — the wire contract is
                // "guaranteed-fresh or error" so clients can decide
                // whether to retry, propagate, or fall back to the
                // pushed-snapshot fast path.
                let (tx, rx) = crossbeam_channel::bounded(1);
                pane.send_io_command(PaneIoCommand::SnapshotNow { reply: tx });
                if rx.recv_timeout(Duration::from_millis(500)).is_err() {
                    log::warn!(
                        "GetPaneSnapshot({pane_id}) timed out waiting for IO thread barrier"
                    );
                    Some(MuxPdu::Error {
                        message: format!(
                            "pane {pane_id} IO thread did not acknowledge SnapshotNow within 500ms"
                        ),
                    })
                } else {
                    let (snap, evicted) = ctx.snapshot_cache.build_and_take(pane_id, pane);
                    evicted_image_keys.extend(evicted);
                    let (snap, mutations) = super::push::project_per_client_pure(
                        pane_id,
                        &snap,
                        None,
                        conn,
                        ctx.snapshot_cache,
                    );
                    pending_image_mutations = Some(mutations);
                    Some(MuxPdu::PaneSnapshotResp { snapshot: snap })
                }
            }
            None => Some(MuxPdu::Error {
                message: format!("pane not found: {pane_id}"),
            }),
        },

        MuxPdu::ExtractText { pane_id, selection } => {
            let text = dispatch_extract_text(ctx.panes.get(&pane_id), selection.to_selection());
            Some(MuxPdu::ExtractTextResp { text })
        }

        MuxPdu::ExtractHtml {
            pane_id,
            selection,
            font_family,
            font_size_x100,
        } => {
            let (html, text) = dispatch_extract_html(
                ctx.panes.get(&pane_id),
                selection.to_selection(),
                font_family,
                f32::from(font_size_x100) / 100.0,
            );
            Some(MuxPdu::ExtractHtmlResp { html, text })
        }

        // Response/notification variants from a client are protocol violations.
        _ => {
            log::warn!(
                "unexpected PDU from client {}: {:?}",
                conn.id(),
                pdu.msg_type()
            );
            Some(MuxPdu::Error {
                message: "unexpected PDU type from client".to_string(),
            })
        }
    };

    DispatchResult {
        sub_changed,
        unsubscribed_pane: unsub_pane,
        response,
        broadcast: if is_new_tab {
            Some(MuxPdu::NotifyNewTab)
        } else {
            None
        },
        evicted_image_keys,
        pending_image_mutations,
    }
}

/// Map a wire signal byte to the `Signal` enum.
fn signal_from_wire(wire: u8) -> Option<crate::pane::Signal> {
    match wire {
        0 => Some(crate::pane::Signal::Interrupt),
        _ => None,
    }
}
