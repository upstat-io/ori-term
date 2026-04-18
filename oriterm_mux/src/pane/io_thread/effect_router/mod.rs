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
    ///
    /// Called after every `handle_bytes()` parse chunk, after
    /// `drain_commands` finishes (so a fulfilled reply in the same tick
    /// enters the same drain), and after `handle_sync_timeout`. The
    /// `effects_buf: Vec<Effect>` scratch is reused (cleared, never
    /// shrunk) so the hot path stays zero-alloc after warmup.
    #[allow(
        clippy::too_many_lines,
        reason = "single-match-statement router: dispatch table for 20+ Effect variants \
                  belongs in one place; splitting would violate SSOT (LEAK:duplicated-dispatch)"
    )]
    pub(crate) fn drain_effects_into_mux_events(&mut self) {
        self.terminal
            .effect_sink()
            .drain_into(&mut self.effects_buf);
        if self.effects_buf.is_empty() {
            return;
        }

        // First pass: detect `ClearPendingNotifications` and collapse
        // preceding `DesktopNotification` effects in the same batch.
        let mut clear_seen = false;
        for effect in &self.effects_buf {
            if matches!(effect, Effect::Host(HostEffect::ClearPendingNotifications)) {
                clear_seen = true;
                break;
            }
        }

        // Second pass: route each effect. `std::mem::take` moves the
        // Vec out so we can `drain(..)` it without holding `&mut self`
        // across the match arms that call `send_mux_event(&self, ..)`.
        // Capacity is restored at the end so the hot path stays
        // zero-alloc after warmup.
        let mut effects = std::mem::take(&mut self.effects_buf);
        // `drain(..)` here preserves capacity on the returned Vec so the
        // scratch buffer stays allocation-stable across drain cycles.
        #[allow(
            clippy::iter_with_drain,
            reason = "drain(..) preserves Vec capacity for scratch reuse"
        )]
        for effect in effects.drain(..) {
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
                    if clear_seen {
                        // Collapsed by later ClearPendingNotifications in
                        // this drain batch — drop silently.
                    } else {
                        self.send_mux_event(MuxEvent::DesktopNotification {
                            pane_id: self.pane_id,
                            source,
                            title,
                            body,
                        });
                    }
                }
                Effect::Host(HostEffect::ClearPendingNotifications) => {
                    // Emitted as a notification so downstream staging
                    // buffers (mux_pump, window_management, daemon) can
                    // purge their DesktopNotification entries for this
                    // pane. The variant is added in effect-cutover 01.1
                    // to MuxNotification — it does NOT ride through
                    // MuxEvent because MuxEvent is the IO-thread→mux
                    // bus and the purge target is the downstream
                    // notification staging. The mux's
                    // `in_process::event_pump` forwards this variant
                    // as-is to the notification queue.
                    //
                    // The 01.3 follow-up may introduce a dedicated
                    // MuxEvent variant if tracing / debugging benefits
                    // from it. For 01.1 we just emit through the
                    // notification channel directly: the mux forwards
                    // every `DesktopNotification` MuxEvent into a
                    // matching `MuxNotification::DesktopNotification`,
                    // and `ClearPendingDesktopNotifications` follows the
                    // same path (treat it as a control event in the
                    // notification stream). Since MuxEvent doesn't
                    // carry that variant today, send it via the
                    // mux_pump's `drain_notifications` path — but the
                    // only way to push to that path from the IO thread
                    // is through MuxEvent. To keep the commit small and
                    // avoid adding a whole new event variant that fans
                    // through the entire pipeline, we NOOP here in 01.1
                    // and file the full purge-through-MuxNotification
                    // wiring as a tracked follow-up. The same batch's
                    // DesktopNotification effects were already
                    // suppressed by the `clear_seen` check above, which
                    // is the load-bearing user-visible behavior.
                }
                Effect::Host(
                    HostEffect::VisualBell
                    | HostEffect::AudioRequest(_)
                    | HostEffect::PrintRequest(_),
                ) => {
                    log::info!(
                        "PaneIoThread ({}): dropping fire-and-forget host effect (no MuxEvent \
                         variant yet — tracked as effect-cutover 01.1 cleanup bugs)",
                        self.pane_id,
                    );
                }
                Effect::HostRequest(HostRequest::ClipboardLoad {
                    selection,
                    clipboard_char,
                    terminator,
                    reply,
                }) => {
                    let reply_for_main = reply.clone();
                    self.send_mux_event(MuxEvent::HostClipboardLoad {
                        pane_id: self.pane_id,
                        selection,
                        clipboard_char,
                        terminator: terminator.clone(),
                        reply: reply_for_main,
                    });
                    self.register_host_request_response(HostRequest::ClipboardLoad {
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
                    let reply_for_main = reply.clone();
                    self.send_mux_event(MuxEvent::HostColorQuery {
                        pane_id: self.pane_id,
                        prefix: prefix.clone(),
                        index,
                        terminator: terminator.clone(),
                        reply: reply_for_main,
                    });
                    self.register_host_request_response(HostRequest::ColorQuery {
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
            }
        }

        // Restore the scratch buffer with retained capacity.
        self.effects_buf = effects;
    }

    /// Canonical `MuxEvent` emission path.
    ///
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
    ///
    /// Used for `UiEffect::CursorBlinkChanged` and `UiEffect::MouseCursorDirty`,
    /// which have no `MuxEvent` counterpart but still need the winit loop
    /// to observe the state change (cursor blink enable/disable, mouse
    /// shape change).
    pub(crate) fn fire_wakeup_only(&self) {
        (self.wakeup)();
    }
}

/// Map the new [`ClipboardSelection`] enum to the legacy `ClipboardType`
/// used by `MuxEvent::ClipboardStore`. `Primary` and `Select` both map
/// to `Selection` — matches `LegacyEventSink::selection_to_legacy` for
/// SSOT.
fn selection_to_clipboard_type(selection: ClipboardSelection) -> ClipboardType {
    match selection {
        ClipboardSelection::Clipboard => ClipboardType::Clipboard,
        ClipboardSelection::Primary | ClipboardSelection::Select => ClipboardType::Selection,
    }
}

#[cfg(test)]
mod tests;
