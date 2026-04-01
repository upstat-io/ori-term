//! Request dispatch for the mux server.
//!
//! Matches incoming [`MuxPdu`] request variants and calls the appropriate
//! [`InProcessMux`] methods, returning response PDUs.

mod helpers;
mod types;

pub(in crate::server) use helpers::parse_theme;
pub(in crate::server) use helpers::remove_client_subscriptions;
pub(super) use types::{DispatchContext, DispatchResult};

use std::path::PathBuf;

use oriterm_core::{CursorShape, Rgb};

use crate::MuxPdu;
use crate::domain::SpawnConfig;
use crate::pane::io_thread::PaneIoCommand;

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

    let response = match pdu {
        MuxPdu::Hello { pid } => {
            log::info!("client {} handshake (pid={pid})", conn.id());
            Some(MuxPdu::HelloAck {
                client_id: conn.id(),
            })
        }

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

        MuxPdu::Resize {
            pane_id,
            cols,
            rows,
        } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                // Resize old Term for dual-Term consistency.
                pane.resize_grid(rows, cols);
                // IO thread does reflow + PTY resize (SIGWINCH).
                // Do NOT push an immediate snapshot — the IO thread will
                // produce one after reflow completes. This prevents
                // exposing intermediate reflow frames (TPR-05-001).
                pane.send_io_command(PaneIoCommand::Resize { rows, cols });
            }
            None // Fire-and-forget.
        }

        MuxPdu::ScrollDisplay { pane_id, delta } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.scroll_display(delta as isize);
                pane.send_io_command(PaneIoCommand::ScrollDisplay(delta as isize));
            }
            None
        }

        MuxPdu::ScrollToBottom { pane_id } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.scroll_to_bottom();
                pane.send_io_command(PaneIoCommand::ScrollToBottom);
            }
            None
        }

        MuxPdu::ScrollToPrompt { pane_id, direction } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                if direction < 0 {
                    pane.scroll_to_previous_prompt();
                } else {
                    pane.scroll_to_next_prompt();
                }
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
                // Update old Term for dual-Term fallback path.
                let mut term = pane.terminal().lock();
                term.set_theme(theme);
                let palette = term.palette_mut();
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
                let pal_clone = term.palette().clone();
                term.grid_mut().dirty_mut().mark_all();
                drop(term);
                pane.send_io_command(PaneIoCommand::SetTheme(theme, Box::new(pal_clone)));
            }
            None
        }

        MuxPdu::SetCursorShape { pane_id, shape } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                let wire = crate::WireCursorShape::from_u8(shape);
                let core_shape = CursorShape::from(wire);
                pane.terminal().lock().set_cursor_shape(core_shape);
                pane.send_io_command(PaneIoCommand::SetCursorShape(core_shape));
            }
            None
        }

        MuxPdu::SetBoldIsBright { pane_id, enabled } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.terminal().lock().set_bold_is_bright(enabled);
                pane.send_io_command(PaneIoCommand::SetBoldIsBright(enabled));
            }
            None
        }

        MuxPdu::MarkAllDirty { pane_id } => {
            if let Some(pane) = ctx.panes.get(&pane_id) {
                pane.terminal().lock().grid_mut().dirty_mut().mark_all();
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
            if let Some(pane) = ctx.panes.get_mut(&pane_id) {
                let grid_ref = pane.terminal().clone();
                if let Some(search) = pane.search_mut() {
                    let term = grid_ref.lock();
                    search.set_query(query.clone(), term.grid());
                }
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
                let mut term = pane.terminal().lock();
                term.set_image_protocol_enabled(enabled);
                term.set_image_limits(memory_limit as usize, max_single as usize);
                term.set_image_animation_enabled(animation_enabled);
                drop(term);
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

        MuxPdu::Ping => Some(MuxPdu::PingAck),

        MuxPdu::Shutdown => {
            log::info!("shutdown requested by client {}", conn.id());
            Some(MuxPdu::ShutdownAck)
        }

        MuxPdu::Subscribe { pane_id } => {
            conn.subscribe(pane_id);
            match ctx.panes.get(&pane_id) {
                Some(pane) => {
                    let snap = ctx.snapshot_cache.build_and_take(pane_id, pane);
                    Some(MuxPdu::Subscribed { snapshot: snap })
                }
                None => Some(MuxPdu::Error {
                    message: format!("pane not found: {pane_id}"),
                }),
            }
        }

        MuxPdu::Unsubscribe { pane_id } => {
            conn.unsubscribe(pane_id);
            Some(MuxPdu::Unsubscribed)
        }

        MuxPdu::GetPaneSnapshot { pane_id } => match ctx.panes.get(&pane_id) {
            Some(pane) => {
                let snap = ctx.snapshot_cache.build_and_take(pane_id, pane);
                Some(MuxPdu::PaneSnapshotResp { snapshot: snap })
            }
            None => Some(MuxPdu::Error {
                message: format!("pane not found: {pane_id}"),
            }),
        },

        MuxPdu::ExtractText { pane_id, selection } => {
            use std::time::Duration;
            let sel = selection.to_selection();
            let text = if let Some(pane) = ctx.panes.get(&pane_id) {
                let (tx, rx) = crossbeam_channel::bounded(1);
                pane.send_io_command(PaneIoCommand::ExtractText {
                    selection: sel,
                    reply: tx,
                });
                rx.recv_timeout(Duration::from_millis(100))
                    .ok()
                    .flatten()
                    .unwrap_or_default()
            } else {
                String::new()
            };
            Some(MuxPdu::ExtractTextResp { text })
        }

        MuxPdu::ExtractHtml {
            pane_id,
            selection,
            font_family,
            font_size_x100,
        } => {
            use std::time::Duration;
            let sel = selection.to_selection();
            let font_size = f32::from(font_size_x100) / 100.0;
            let (html, text) = if let Some(pane) = ctx.panes.get(&pane_id) {
                let (tx, rx) = crossbeam_channel::bounded(1);
                pane.send_io_command(PaneIoCommand::ExtractHtml {
                    selection: sel,
                    font_family,
                    font_size,
                    reply: tx,
                });
                rx.recv_timeout(Duration::from_millis(100))
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| (String::new(), String::new()))
            } else {
                (String::new(), String::new())
            };
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
    }
}
