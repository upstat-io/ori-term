//! Effect→MuxEvent/MuxNotification router (effect-cutover 01.1).
//!
//! The canonical dispatch home for the IO thread's
//! `drain_effects_into_mux_events()` path. Takes the effects queued by
//! `QueueingEffectSink::push()` during VTE parsing or command handling,
//! matches each variant, and emits `MuxEvent`s through the shared
//! `mux_tx` channel while firing the `wakeup` callback so the winit
//! event loop observes the state change within one cycle.
//!
//! # Wakeup contract
//!
//! Every `MuxEvent` emission flows through [`PaneIoThread::send_mux_event`],
//! which pairs `mux_tx.send(event)` with `(self.wakeup)()` in a single
//! helper. Direct `self.mux_tx.send(..)` from inside this module is
//! banned — use `send_mux_event`. For Ui effects that produce no
//! `MuxEvent` (`CursorBlinkChanged`, `MouseCursorDirty`),
//! [`PaneIoThread::fire_wakeup_only`] preserves legacy semantics.
//!
//! # `ClearPendingDesktopNotifications` collapse
//!
//! `HostEffect::ClearPendingNotifications` collapses preceding
//! `DesktopNotification` effects for the same pane in the SAME drain
//! batch (contract at `oriterm_core::effect::HostEffect::ClearPendingNotifications`
//! doc). The router walks `effects_buf` in order, holding
//! `DesktopNotification`s in a staging slot until the end of the batch
//! or until `ClearPendingNotifications` discards them.

use oriterm_core::ClipboardType;
use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{
    ClipboardSelection, Effect, HostEffect, HostRequest, PtyEffect, UiEffect,
};

use super::PaneIoThread;
use crate::mux_event::MuxEvent;

impl<S: EffectSink> PaneIoThread<S> {
    /// Drain queued effects from the terminal's sink and route them to
    /// `MuxEvent` / wakeup / pending-response registrations.
    /// Called after every `handle_bytes()` parse chunk, after
    /// `drain_commands` finishes (so a fulfilled reply in the same tick
    /// enters the same drain), and after `handle_sync_timeout`. The
    /// `effects_buf: Vec<Effect>` scratch is reused (cleared, never
    /// shrunk) so the hot path stays zero-alloc after warmup.
    #[allow(
        clippy::too_many_lines,
        reason = "single-match-statement router: dispatch table for 20+ Effect variants \
 belongs in one place; splitting would violate SSOT ()"
    )]
    pub(crate) fn drain_effects_into_mux_events(&mut self) {
        self.terminal
            .effect_sink()
            .drain_into(&mut self.effects_buf);
        if self.effects_buf.is_empty() {
            return;
        }

        // First pass: locate the LAST `ClearPendingNotifications` index
        // in the batch. `DesktopNotification` effects at strictly lower
        // indices are suppressed by the clear (intra-batch
        // collapse — preceding only); `DesktopNotification` effects at
        // higher indices pass through unchanged. Tracking the LAST
        // index handles a multi-clear batch correctly: a notification
        // emitted between two clears is still preceding the second
        // clear and so is suppressed.
        let mut last_clear_index: Option<usize> = None;
        for (i, effect) in self.effects_buf.iter().enumerate() {
            if matches!(effect, Effect::Host(HostEffect::ClearPendingNotifications)) {
                last_clear_index = Some(i);
            }
        }

        // Second pass: route each effect. `std::mem::take` moves the
        // Vec out so we can `drain(..)` it without holding `&mut self`
        // across the match arms that call `send_mux_event(&self,..)`.
        // Capacity is restored at the end so the hot path stays
        // zero-alloc after warmup.
        let mut effects = std::mem::take(&mut self.effects_buf);
        // `drain(..)` here preserves capacity on the returned Vec so the
        // scratch buffer stays allocation-stable across drain cycles.
        #[allow(
            clippy::iter_with_drain,
            reason = "drain(..) preserves Vec capacity for scratch reuse"
        )]
        for (idx, effect) in effects.drain(..).enumerate() {
            match effect {
                Effect::Pty(PtyEffect::Write { bytes, .. }) => {
                    self.send_mux_event(MuxEvent::PtyWrite {
                        pane_id: self.pane_id,
                        data: bytes,
                    });
                }
                Effect::Host(HostEffect::Bell) => {
                    self.send_mux_event(MuxEvent::PaneBell(self.pane_id));
                }
                Effect::Host(HostEffect::UrgencyHint) => {
                    self.send_mux_event(MuxEvent::PaneUrgencyHint(self.pane_id));
                }
                Effect::Host(HostEffect::TitleSet { value }) => {
                    self.send_mux_event(MuxEvent::PaneTitleChanged {
                        pane_id: self.pane_id,
                        title: value.unwrap_or_default(),
                    });
                }
                Effect::Host(HostEffect::IconNameSet { value }) => {
                    self.send_mux_event(MuxEvent::PaneIconChanged {
                        pane_id: self.pane_id,
                        icon_name: value.unwrap_or_default(),
                    });
                }
                Effect::Host(HostEffect::CwdSet { cwd }) => {
                    self.send_mux_event(MuxEvent::PaneCwdChanged {
                        pane_id: self.pane_id,
                        cwd,
                    });
                }
                Effect::Host(HostEffect::CommandComplete { duration }) => {
                    self.send_mux_event(MuxEvent::CommandComplete {
                        pane_id: self.pane_id,
                        duration,
                    });
                }
                Effect::Host(HostEffect::ChildExit { code }) => {
                    self.send_mux_event(MuxEvent::PaneExited {
                        pane_id: self.pane_id,
                        exit_code: code,
                    });
                }
                Effect::Host(HostEffect::ClipboardStore { selection, data }) => {
                    self.send_mux_event(MuxEvent::ClipboardStore {
                        pane_id: self.pane_id,
                        clipboard_type: selection_to_clipboard_type(selection),
                        text: data,
                    });
                }
                Effect::Host(HostEffect::DesktopNotification {
                    source,
                    title,
                    body,
                }) => {
                    // Suppress only when this notification PRECEDES the
                    // last clear marker in the batch. Notifications
                    // emitted AFTER every clear marker survive — the
                    // contract is "clear discards preceding only" per
                    // host.rs:42-50 and plan blind-spot §7.
                    let suppressed = matches!(
                    last_clear_index,
                    Some(clear_at) if idx < clear_at
                    );
                    if !suppressed {
                        self.send_mux_event(MuxEvent::DesktopNotification {
                            pane_id: self.pane_id,
                            source,
                            title,
                            body,
                        });
                    }
                }
                Effect::Host(HostEffect::ClearPendingNotifications) => {
                    // Forward via the dedicated MuxEvent variant so the
                    // event pump pushes
                    // `MuxNotification::ClearPendingDesktopNotifications`
                    // into the staging buffers (mux_pump,
                    // window_management, daemon broadcast). Earlier
                    // notifications in the same batch were already
                    // suppressed above; this signal handles cross-batch
                    // notifications that landed in main-thread staging
                    // before this clear arrived.
                    self.send_mux_event(MuxEvent::ClearPendingDesktopNotifications(self.pane_id));
                }
                Effect::Host(HostEffect::AudioRequest(_) | HostEffect::PrintRequest(_)) => {
                    log::info!(
                        "PaneIoThread ({}): dropping fire-and-forget host effect (no MuxEvent \
 variant yet — tracked as)",
                        self.pane_id,
                    );
                }
                Effect::HostRequest(HostRequest::ClipboardLoad {
                    selection,
                    clipboard_char,
                    terminator,
                    reply,
                }) => {
                    // Register BEFORE sending the mux event: the main
                    // thread's handler may race us — receive the event,
                    // fulfill the token, and send a wake byte all before
                    // this closure's next line. If register ran after
                    // send, the wake would fire against an empty pending
                    // list, no reply would emit, and the caller would
                    // time out. Registering first closes that window —
                    // `poll_pending_responses` always sees the token
                    // (Pending or Fulfilled) by the time any wake path
                    // reaches it.
                    self.register_host_request_response(HostRequest::ClipboardLoad {
                        selection,
                        clipboard_char,
                        terminator: terminator.clone(),
                        reply: reply.clone(),
                    });
                    self.send_mux_event(MuxEvent::HostClipboardLoad {
                        pane_id: self.pane_id,
                        selection,
                        clipboard_char,
                        terminator,
                        reply,
                    });
                }
                Effect::HostRequest(HostRequest::ColorQuery {
                    prefix,
                    index,
                    terminator,
                    reply,
                }) => {
                    // See ClipboardLoad arm above for the register-before-
                    // send rationale.
                    self.register_host_request_response(HostRequest::ColorQuery {
                        prefix: prefix.clone(),
                        index,
                        terminator: terminator.clone(),
                        reply: reply.clone(),
                    });
                    self.send_mux_event(MuxEvent::HostColorQuery {
                        pane_id: self.pane_id,
                        prefix,
                        index,
                        terminator,
                        reply,
                    });
                }
                Effect::Ui(UiEffect::CursorBlinkChanged { .. } | UiEffect::MouseCursorDirty) => {
                    self.fire_wakeup_only();
                }
                Effect::Presentation(p) => {
                    log::info!(
                        "PaneIoThread ({}): presentation effect logged without mux event: {p:?}",
                        self.pane_id,
                    );
                }
                Effect::ImageDecode(req) => {
                    let seq = req.sequence_id;
                    let image_id = req.image_id;
                    let payload_bytes = req.payload.len();
                    let reply_ctx = req.reply_ctx;
                    let placement = req.placement.clone();
                    if let Err(err) = self.image_worker.enqueue(req) {
                        // Enqueue failure routes through Term::apply_decoded_image
                        // so the reply sequencer resolves in command order.
                        // Layer boundary: oriterm_mux passes the raw
                        // EnqueueError variant via ImageDecodeError;
                        // oriterm_core's apply_decoded_image formats the
                        // kitty reply string.
                        let synthetic = super::image_worker::synthesize_enqueue_failure(
                            seq,
                            image_id,
                            payload_bytes,
                            reply_ctx,
                            placement,
                            err,
                        );
                        self.terminal.apply_decoded_image(synthetic);
                    }
                }
            }
        }

        // Restore the scratch buffer with retained capacity.
        self.effects_buf = effects;
    }

    /// Canonical `MuxEvent` emission path.
    /// Every `MuxEvent` produced by the router MUST go through this
    /// helper so the wakeup callback fires — the winit event loop
    /// otherwise never observes the queued state change and
    /// `pump_mux_events()` never runs. Direct `self.mux_tx.send(..)`
    /// inside this file is banned.
    pub(crate) fn send_mux_event(&self, event: MuxEvent) {
        let _ = self.mux_tx.send(event);
        (self.wakeup)();
    }

    /// Fire the wakeup callback WITHOUT queuing a `MuxEvent`.
    /// Used for `UiEffect::CursorBlinkChanged` and `UiEffect::MouseCursorDirty`,
    /// which have no `MuxEvent` counterpart but still need the winit loop
    /// to observe the state change (cursor blink enable/disable, mouse
    /// shape change).
    pub(crate) fn fire_wakeup_only(&self) {
        (self.wakeup)();
    }
}

/// Map the [`ClipboardSelection`] enum (X11-aware: `Clipboard` /
/// `Primary` / `Select`) to the platform-clipboard [`ClipboardType`]
/// (`Clipboard` / `Selection`) used by `MuxEvent::ClipboardStore`.
/// `Primary` and `Select` both collapse to `Selection`.
fn selection_to_clipboard_type(selection: ClipboardSelection) -> ClipboardType {
    match selection {
        ClipboardSelection::Clipboard => ClipboardType::Clipboard,
        ClipboardSelection::Primary | ClipboardSelection::Select => ClipboardType::Selection,
    }
}

#[cfg(test)]
mod tests;
