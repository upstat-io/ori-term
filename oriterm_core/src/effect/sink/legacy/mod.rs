//! Legacy adapter — bridges `Effect` to existing `Event`/`EventListener` consumers.
//!
//! This is the migration bridge: the VTE handler emits `Effect`, the legacy
//! adapter wraps an existing `EventListener` and forwards each `Effect` as the
//! equivalent `Event`. Existing consumers (`oriterm_mux`, `oriterm`) don't need
//! to change — they keep receiving `Event`s.
//!
//! `LegacyEventSink` is a TEMPORARY adapter — it exists only until consumers
//! migrate to subscribe to `Effect` directly (in `plans/effect-cutover/`).

use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::color::Rgb;
use crate::effect::Effect;
use crate::effect::families::{
    ClipboardSelection, HostEffect, HostRequest, NotificationSource, PtyEffect, UiEffect,
};
use crate::event::{ClipboardType, Event, EventListener};

use super::EffectSink;

/// Record for a desktop notification queued inside the legacy adapter.
///
/// Notifications have no legacy `Event` variant, so they cannot be forwarded
/// via `listener.send_event()`. Instead they are queued here and drained by
/// `drain_pending_notifications()`.
#[derive(Debug, Clone)]
pub struct DesktopNotificationRecord {
    /// Which OSC sequence originated the notification.
    pub source: NotificationSource,
    /// The notification title.
    pub title: String,
    /// The notification body.
    pub body: String,
}

/// Bridges `Effect` → existing `Event`/`EventListener` for the migration phase.
///
/// # Sync bound
///
/// `L` must be `Send + Sync` (not just `Send`) because `EffectSink` requires
/// `Sync`. All existing `EventListener` impls in the workspace satisfy `Sync`.
pub struct LegacyEventSink<L: EventListener + Sync> {
    listener: L,
    /// Secondary queue for `DesktopNotification` effects.
    /// Notifications have no legacy `Event` variant, so they are queued
    /// here and drained by `drain_pending_notifications()`.
    pending_notifications: parking_lot::Mutex<Vec<DesktopNotificationRecord>>,
}

impl<L: EventListener + Sync> LegacyEventSink<L> {
    /// Create a new legacy adapter wrapping the given listener.
    pub fn new(listener: L) -> Self {
        Self {
            listener,
            pending_notifications: parking_lot::Mutex::new(Vec::new()),
        }
    }

    /// Drain all queued desktop notifications.
    ///
    /// Called by the thin `Term::drain_notifications()` shim during the
    /// legacy phase.
    pub fn drain_pending_notifications(&self) -> Vec<DesktopNotificationRecord> {
        std::mem::take(&mut *self.pending_notifications.lock())
    }
}

impl<L: EventListener + Sync> EffectSink for LegacyEventSink<L> {
    fn push(&self, effect: Effect) {
        let event = match effect {
            Effect::Pty(PtyEffect::Write { bytes, .. }) => {
                Event::PtyWrite(String::from_utf8_lossy(&bytes).into_owned())
            }
            Effect::Host(HostEffect::Bell) => Event::Bell,
            // No legacy Event variant for these — drop silently.
            Effect::Host(
                HostEffect::VisualBell | HostEffect::AudioRequest(_) | HostEffect::PrintRequest(_),
            )
            | Effect::Presentation(_) => return,
            Effect::Host(HostEffect::TitleSet { value: Some(t) }) => Event::Title(t),
            Effect::Host(HostEffect::TitleSet { value: None }) => Event::ResetTitle,
            Effect::Host(HostEffect::IconNameSet { value: Some(n) }) => Event::IconName(n),
            Effect::Host(HostEffect::IconNameSet { value: None }) => Event::ResetIconName,
            Effect::Host(HostEffect::CwdSet { cwd }) => Event::Cwd(cwd),
            Effect::Host(HostEffect::CommandComplete { duration }) => {
                Event::CommandComplete(duration)
            }
            Effect::Host(HostEffect::ChildExit { code }) => Event::ChildExit(code),
            Effect::Host(HostEffect::DesktopNotification {
                source,
                title,
                body,
            }) => {
                self.pending_notifications
                    .lock()
                    .push(DesktopNotificationRecord {
                        source,
                        title,
                        body,
                    });
                return;
            }
            Effect::Host(HostEffect::ClearPendingNotifications) => {
                self.pending_notifications.lock().clear();
                return;
            }
            Effect::Host(HostEffect::ClipboardStore { selection, data }) => {
                Event::ClipboardStore(selection_to_legacy(selection), data)
            }
            Effect::HostRequest(HostRequest::ClipboardLoad {
                selection,
                clipboard_char,
                terminator,
                reply,
            }) => {
                let cb = clipboard_char;
                let term = terminator;
                Event::ClipboardLoad(
                    selection_to_legacy(selection),
                    Arc::new(move |text: &str| {
                        reply.fulfill(text.to_string());
                        format!(
                            "\x1b]52;{};{}{}",
                            cb as char,
                            STANDARD.encode(text.as_bytes()),
                            term,
                        )
                    }),
                )
            }
            Effect::HostRequest(HostRequest::ColorQuery {
                prefix,
                index,
                terminator,
                reply,
            }) => {
                let pfx = prefix;
                let term = terminator;
                Event::ColorRequest(
                    index,
                    Arc::new(move |color: Rgb| {
                        reply.fulfill(color);
                        format!(
                            "\x1b]{};rgb:{:02x}{:02x}/{:02x}{:02x}/{:02x}{:02x}{}",
                            pfx, color.r, color.r, color.g, color.g, color.b, color.b, term,
                        )
                    }),
                )
            }
            Effect::Ui(UiEffect::CursorBlinkChanged { .. }) => Event::CursorBlinkingChange,
            Effect::Ui(UiEffect::MouseCursorDirty) => Event::MouseCursorDirty,
        };
        self.listener.send_event(event);
    }

    fn drain_into(&self, _out: &mut Vec<Effect>) {
        // Legacy adapter is push-only — effects are forwarded immediately
        // as Events. No internal queue to drain.
    }
}

/// Map `ClipboardSelection` (new) to `ClipboardType` (legacy).
fn selection_to_legacy(s: ClipboardSelection) -> ClipboardType {
    match s {
        ClipboardSelection::Clipboard => ClipboardType::Clipboard,
        // Both Primary and Select map to the legacy Selection type.
        ClipboardSelection::Primary | ClipboardSelection::Select => ClipboardType::Selection,
    }
}

impl<L: EventListener + Sync + std::fmt::Debug> std::fmt::Debug for LegacyEventSink<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LegacyEventSink")
            .field("listener", &self.listener)
            .field(
                "pending_notifications",
                &self.pending_notifications.lock().len(),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests;
