//! Mux event pump — drains PTY events and handles mux notifications.
//!
//! Called once per event loop iteration in `about_to_wait`, before rendering.
//! Processes `MuxEvent`s from PTY reader threads via `MuxBackend::poll_events`,
//! then handles resulting `MuxNotification`s (dirty, close, clipboard, etc.).

use std::time::{Duration, Instant};

use oriterm_mux::MuxNotification;
use oriterm_mux::PaneId;

use crate::config::NotifyOnCommandFinish;
use crate::platform::audio;

use super::App;

impl App {
    /// Drain the notification buffer and invoke `handler` on each notification.
    ///
    /// Takes the buffer from `self` to avoid borrow conflicts (the handler
    /// gets `&mut Self` without conflicting with the buffer), then restores
    /// it afterward to preserve `Vec` capacity across frames.
    pub(super) fn with_drained_notifications(
        &mut self,
        mut handler: impl FnMut(&mut Self, MuxNotification),
    ) {
        let mut buf = std::mem::take(&mut self.notification_buf);
        let count = buf.len();
        #[allow(
            clippy::iter_with_drain,
            reason = "drain preserves Vec capacity; into_iter drops it"
        )]
        for n in buf.drain(..) {
            handler(self, n);
        }
        // Shrink if capacity vastly exceeds typical usage.
        let cap = buf.capacity();
        if cap > 4 * count && cap > 4096 {
            buf.shrink_to(count * 2);
        }
        self.notification_buf = buf;
    }

    /// Pump mux events and process resulting notifications.
    ///
    /// Drains PTY reader thread messages via the mux, then handles each
    /// notification (dirty, close, clipboard, etc.).
    pub(super) fn pump_mux_events(&mut self) {
        let Some(mux) = &mut self.mux else { return };

        // Check daemon connectivity.
        if mux.is_daemon_mode() && !mux.is_connected() {
            log::warn!("daemon connection lost");
            self.handle_daemon_disconnect();
            return;
        }

        // Skip polling when no PTY wakeup has arrived since the last poll.
        // The try_recv() inside poll_events() is cheap but acquires the
        // channel lock; skipping it entirely avoids even that overhead.
        if !mux.has_pending_wakeup() {
            return;
        }

        // 1. Process incoming MuxEvents from PTY reader threads.
        mux.poll_events();

        // 2. Drain notifications into our reusable buffer.
        mux.drain_notifications(&mut self.notification_buf);
        if self.notification_buf.is_empty() {
            return;
        }

        // 2a. Honor cross-batch `ClearPendingDesktopNotifications`
        //     markers BEFORE dispatching: each clear marker discards
        //     all preceding `DesktopNotification` entries for the same
        //     pane that landed in earlier IO-thread batches and
        //     accumulated in this drain. The IO-thread router already
        //     handles intra-batch collapse; this purge handles the
        //     case where a notification reached `notification_buf` in
        //     an earlier drain cycle and the clear catches it before
        //     the next dispatch tick. Per effect-cutover §01.1
        //     success criterion 24.
        purge_pending_desktop_notifications(&mut self.notification_buf);

        // 3. Handle each notification.
        self.with_drained_notifications(Self::handle_mux_notification);
    }

    /// Process a single mux notification.
    #[allow(
        clippy::too_many_lines,
        reason = "single-match notification dispatch — belongs in one place"
    )]
    fn handle_mux_notification(&mut self, notification: MuxNotification) {
        match notification {
            MuxNotification::PaneOutput(id) => {
                // Invalidate client-side selection only when terminal content
                // that affects selection coordinates has changed (scrolling,
                // character printing, erasing, etc.). Non-content operations
                // like cursor movement and SGR attribute changes do not
                // invalidate. Without this precision, selections would be
                // cleared on every prompt repaint or cursor blink output,
                // making selection highlighting impossible.
                if let Some(mux) = self.mux.as_mut() {
                    if mux.is_selection_dirty(id) {
                        mux.clear_selection_dirty(id);
                        self.clear_pane_selection(id);
                    }
                }

                // Only invalidate URL hover when the dirty pane is focused.
                // Background shell output in other panes shouldn't kill the
                // URL highlight under the cursor.
                let is_focused = self.active_pane_id() == Some(id);
                if is_focused {
                    if let Some(ctx) = self.focused_ctx_mut() {
                        ctx.url_cache.invalidate();
                        ctx.hovered_url = None;
                    }
                }

                // Background pane received output — mark as unseen so
                // the tab bar shows the "modified" indicator dot.
                if !is_focused {
                    if let Some(mux) = self.mux.as_mut() {
                        mux.set_unseen_output(id);
                    }
                    self.sync_tab_bar_from_mux();
                }
                // Mark only the window containing this pane as dirty.
                self.mark_pane_window_dirty(id);
            }
            MuxNotification::PaneClosed { pane_id, .. } => {
                self.handle_pane_closed(pane_id);
            }
            MuxNotification::PaneMetadataChanged(id) => {
                self.sync_tab_bar_from_mux();
                self.mark_pane_window_dirty(id);
            }
            MuxNotification::CommandComplete { pane_id, duration } => {
                self.handle_command_complete(pane_id, duration);
            }
            MuxNotification::PaneBell(id) => {
                // Bells on the focused pane are silent: the user is already
                // looking at it, so the sound + bell icon would just be noise.
                // The IO thread's `event_pump` already set `pane.has_bell`
                // before this arm fires; clear it here so the icon never
                // appears on the focused tab.
                let is_focused = self.active_pane_id() == Some(id);
                if let Some(mux) = self.mux.as_mut() {
                    if is_focused {
                        mux.clear_bell(id);
                    } else {
                        mux.set_bell(id);
                    }
                }
                let now = Instant::now();
                if !is_focused {
                    if let Some(idx) = self.tab_index_for_pane(id) {
                        if let Some(ctx) = self.focused_ctx_mut() {
                            ctx.tab_bar.ring_bell(idx, now);
                        }
                    }
                }
                // Refresh tab-bar entries from mux state so the persistent
                // bell icon (sourced from `mux.has_bell` in build_tab_entries)
                // appears / clears immediately.
                self.sync_tab_bar_from_mux();

                // Audible bell — closes BUG-08-001 by absorbing the BEL `\a`
                // audio path into BUG-11-016. Native OS APIs respect the
                // user's system mute / sound-effects settings. Skip the
                // audible bell entirely when the bell-ringing pane is
                // already the user's focus.
                if !is_focused && self.config.behavior.notification.is_audible() {
                    audio::play_bell();
                }

                // Visual-bell flash on the pane's OWNING window — not the
                // focused window. A bell from a background pane flashes its
                // own window. Mirrors `mark_pane_window_dirty`'s
                // owning-window walk (`oriterm/src/app/redraw/mod.rs`).
                if self.config.bell.is_enabled() {
                    let bell = &self.config.bell;
                    let color = crate::config::parse_bell_color_as_ui(bell.color.as_deref());
                    let easing = crate::config::bell_animation_to_easing(bell.animation);
                    let duration_ms = bell.duration_ms;
                    if let Some(session_wid) = self.session.window_for_pane(id) {
                        for ctx in self.windows.values_mut() {
                            if ctx.window.session_window_id() == session_wid {
                                ctx.root.ring_visual_bell(now, duration_ms, color, easing);
                                break;
                            }
                        }
                    }
                }

                self.mark_pane_window_dirty(id);
            }
            MuxNotification::PaneUrgencyHint(id) => self.handle_pane_urgency_hint(id),
            MuxNotification::ClipboardStore {
                clipboard_type,
                text,
                ..
            } => {
                self.clipboard.store(clipboard_type, &text);
            }
            MuxNotification::DesktopNotification {
                pane_id,
                title: _title,
                body: _body,
                ..
            } => {
                // Bell-focused dispatch (BUG-11-016 scope reset 2026-04-28):
                // OSC 9 / OSC 99 / OSC 777 fire the bell sound + tab bell
                // icon on the owning pane's tab. Skip both when the
                // bell-ringing pane is already focused — user is looking
                // at it.
                let is_focused = self.active_pane_id() == Some(pane_id);
                let mode = self.config.behavior.notification;
                if !is_focused && mode.is_audible() {
                    audio::play_bell();
                }
                if mode.is_visual() {
                    if let Some(mux) = self.mux.as_mut() {
                        if is_focused {
                            mux.clear_bell(pane_id);
                        } else {
                            mux.set_bell(pane_id);
                        }
                    }
                    if !is_focused {
                        if let Some(idx) = self.tab_index_for_pane(pane_id) {
                            if let Some(ctx) = self.focused_ctx_mut() {
                                ctx.tab_bar.ring_bell(idx, Instant::now());
                            }
                        }
                    }
                    self.sync_tab_bar_from_mux();
                    self.mark_pane_window_dirty(pane_id);
                }
            }
            MuxNotification::ClearPendingDesktopNotifications(_pane_id) => {
                // Desktop notifications are dispatched to the platform
                // notifier immediately on arrival (no local staging), so
                // there is nothing to purge on the main thread. Daemon-side
                // staging buffers handle their own purge in the 01.3 follow-up.
            }
            MuxNotification::HostClipboardLoad {
                pane_id,
                selection,
                reply,
                ..
            } => {
                let clipboard_type = match selection {
                    oriterm_core::effect::ClipboardSelection::Clipboard => {
                        oriterm_core::ClipboardType::Clipboard
                    }
                    oriterm_core::effect::ClipboardSelection::Primary
                    | oriterm_core::effect::ClipboardSelection::Select => {
                        oriterm_core::ClipboardType::Selection
                    }
                };
                let text = self.clipboard.load(clipboard_type);
                if let Some(mux) = self.mux.as_mut() {
                    if let Err(err) = mux.fulfill_host_request(
                        pane_id,
                        oriterm_mux::HostReply::ClipboardLoad { token: reply, text },
                    ) {
                        log::warn!("fulfill_host_request (clipboard) for {pane_id} failed: {err}");
                    }
                }
            }
            MuxNotification::HostColorQuery {
                pane_id,
                index,
                reply,
                ..
            } => {
                if let Some(mux) = self.mux.as_mut() {
                    // sync_pane_snapshot drains in-flight IO commands and
                    // forces a fresh snapshot — required for protocol
                    // replies because the IO thread emits
                    // MuxEvent::HostColorQuery BEFORE publishing the
                    // post-mutation snapshot via maybe_produce_snapshot
                    // (effect drain at oriterm_mux/src/pane/io_thread/mod.rs
                    // happens inside handle_bytes; snapshot publish
                    // happens after handle_bytes returns). refresh_pane_snapshot
                    // would race and return the pre-SET palette for OSC 4
                    // SET-then-QUERY in the same byte batch.
                    let snapshot = mux.sync_pane_snapshot(pane_id);
                    let color = resolve_host_color_query(
                        snapshot.as_ref().map(|s| s.palette.as_slice()),
                        index,
                    );
                    if let Err(err) = mux.fulfill_host_request(
                        pane_id,
                        oriterm_mux::HostReply::ColorQuery {
                            token: reply,
                            color,
                        },
                    ) {
                        log::warn!("fulfill_host_request (color) for {pane_id} failed: {err}");
                    }
                }
            }
            MuxNotification::NewTab => {
                log::info!("received new-tab request from another instance");
                if let Some(win_id) = self.active_window {
                    self.new_tab_in_window(win_id);
                }
            }
            MuxNotification::AnimationDeadlineChanged { pane_id, deadline } => {
                // Push the deadline onto the owning window's RenderScheduler so
                // the event loop's ControlFlow::WaitUntil picks it up via
                // `scheduler().next_wake_time()` at the `ControlFlowInput`
                // construction site. Also mark the window dirty so the snapshot
                // read on the next frame advances the visible image.
                self.request_pane_animation_frame_at(pane_id, deadline);
                if deadline.is_some() {
                    self.mark_pane_window_dirty(pane_id);
                }
            }
        }
    }

    /// Mode 1042 urgency-hint request — winit dispatches the per-platform
    /// attention API on the OWNING window when the pane isn't focused.
    fn handle_pane_urgency_hint(&self, id: PaneId) {
        if self.active_pane_id() == Some(id) {
            return;
        }
        let Some(session_wid) = self.session.window_for_pane(id) else { return };
        if let Some(ctx) = self
            .windows
            .values()
            .find(|c| c.window.session_window_id() == session_wid)
        {
            ctx.window
                .window()
                .request_user_attention(Some(winit::window::UserAttentionType::Critical));
        }
    }

    /// Handle a command completing in a pane.
    ///
    /// Checks config threshold and focus state to decide whether to flash
    /// the tab bar (bell pulse) and/or log the completion.
    fn handle_command_complete(&mut self, pane_id: PaneId, duration: Duration) {
        let behavior = &self.config.behavior;
        let threshold = Duration::from_secs(behavior.notify_command_threshold_secs);
        if duration < threshold {
            return;
        }

        let mode = behavior.notify_on_command_finish;
        if mode == NotifyOnCommandFinish::Never {
            return;
        }

        let is_focused = self.active_pane_id() == Some(pane_id);
        if mode == NotifyOnCommandFinish::Unfocused && is_focused {
            return;
        }

        log::info!(
            "command completed in {pane_id} after {:.1}s",
            duration.as_secs_f64()
        );

        // Flash the tab bar (reuse bell pulse) if configured.
        let is_focused = self.active_pane_id() == Some(pane_id);
        if behavior.notify_command_bell && !is_focused {
            if let Some(mux) = self.mux.as_mut() {
                mux.set_bell(pane_id);
            }
            if let Some(idx) = self.tab_index_for_pane(pane_id) {
                if let Some(ctx) = self.focused_ctx_mut() {
                    ctx.tab_bar.ring_bell(idx, Instant::now());
                    ctx.root.mark_dirty();
                }
            }
            self.sync_tab_bar_from_mux();
        }

        // Bell-focused dispatch (BUG-11-016 scope reset 2026-04-28):
        // command-complete (OSC 133;D) fires the native bell sound; the
        // tab-bell pulse above already handles the visual feedback. Skip
        // entirely when the command's pane is already focused — user
        // is looking at it.
        if !is_focused && self.config.behavior.notification.is_audible() {
            audio::play_bell();
        }
    }
}

/// In-place collapse of `ClearPendingDesktopNotifications` against
/// preceding `DesktopNotification` entries in the same staging
/// buffer. For each clear marker at position `i` for pane `P`,
/// removes every `DesktopNotification { pane_id: P, .. }` at
/// positions `< i`. Iteration order preserves remaining markers.
///
/// Surfaced by `[high]` — the §01 fix only emitted
/// the clear marker but did not act on it in the main-thread staging
/// buffer.
fn purge_pending_desktop_notifications(buf: &mut Vec<MuxNotification>) {
    let mut i = 0;
    while i < buf.len() {
        if let MuxNotification::ClearPendingDesktopNotifications(target_pane) = buf[i] {
            let mut j = 0;
            while j < i {
                let drop_it = matches!(
                    &buf[j],
                    MuxNotification::DesktopNotification { pane_id, .. }
                        if *pane_id == target_pane
                );
                if drop_it {
                    buf.remove(j);
                    i -= 1;
                } else {
                    j += 1;
                }
            }
        }
        i += 1;
    }
}

/// Resolve an OSC 4 / OSC 10 / OSC 11 / OSC 12 color query against the
/// pane's palette snapshot.
///
/// `palette` is a slice of pre-resolved RGB triplets from
/// `PaneSnapshot::palette` — 270 entries covering 0..=255 (indexed
/// palette) and 256..=269 (named semantic slots: Foreground,
/// Background, Cursor, dim variants, etc.). `index` is the
/// pre-computed slot the VTE OSC dispatch resolved.
///
/// Returns black (`Rgb { r:0, g:0, b:0 }`) when the snapshot is
/// missing (`None`) OR the index is out of range — matches
/// `Palette::color()` contract at
/// `oriterm_core/src/color/palette/mod.rs:286-296`.
fn resolve_host_color_query(palette: Option<&[[u8; 3]]>, index: usize) -> oriterm_core::color::Rgb {
    palette.and_then(|p| p.get(index).copied()).map_or(
        oriterm_core::color::Rgb { r: 0, g: 0, b: 0 },
        |[r, g, b]| oriterm_core::color::Rgb { r, g, b },
    )
}

impl App {
    /// Handle a pane being closed (shell exit, PTY EOF, or explicit close).
    ///
    /// Cleans up client-side state, backend resources, and removes the pane
    /// from the local session (tree/floating). If the tab becomes empty,
    /// removes the tab; if the window becomes empty, closes the window.
    fn handle_pane_closed(&mut self, id: PaneId) {
        // Clean up client-side state.
        self.pane_selections.remove(&id);
        self.mark_cursors.remove(&id);

        // Clean up backend-side resources.
        if let Some(mux) = self.mux.as_mut() {
            mux.cleanup_closed_pane(id);
        }
        for ctx in self.windows.values_mut() {
            ctx.pane_cache.remove(id);
            // Clear any queued animation deadline for this pane so the
            // orphan entry does not linger in the scheduler's per-pane
            // map (would accumulate across pane open/close cycles and
            // keep firing spurious wakes).
            ctx.root
                .scheduler_mut()
                .set_animation_deadline(id.raw(), None);
            ctx.root.mark_dirty();
        }

        // Remove pane from local session (tree/floating/tab/window).
        let result = crate::app::pane_ops::helpers::remove_pane_from_session(&mut self.session, id);
        if let Some(wid) = result.empty_window {
            self.close_empty_session_window(wid);
            return;
        }

        self.sync_tab_bar_from_mux();
        self.resize_all_panes();
    }

    /// Handle daemon disconnect by closing the window.
    ///
    /// When the daemon connection is lost the terminal state is gone —
    /// the daemon owned all panes. Closing is the honest response; it
    /// matches how `tmux` clients exit when the server dies.
    fn handle_daemon_disconnect(&self) {
        log::warn!("daemon connection lost, closing window");
        self.exit_app();
    }
}

#[cfg(test)]
mod tests;
