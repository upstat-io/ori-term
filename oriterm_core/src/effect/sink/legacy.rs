//! Legacy adapter — bridges `Effect` to existing `Event`/`MuxEvent` consumers.
//!
//! Stub created in 03.2; full implementation in 03.4.

use std::marker::PhantomData;

use crate::event::EventListener;

use super::Effect;

/// Record of a desktop notification queued via the Effect channel.
///
/// During the legacy phase, notifications are queued here (since there is
/// no `Event` variant for desktop notifications) and drained via
/// `drain_pending_notifications()`.
#[derive(Debug, Clone)]
pub struct DesktopNotificationRecord {
    /// The notification title.
    pub title: String,
    /// The notification body.
    pub body: String,
}

/// Bridges `Effect` → existing `Event`/`EventListener` for the migration phase.
///
/// `LegacyEventSink` wraps an existing `EventListener` and translates
/// incoming `Effect` values into the corresponding `Event` variants,
/// forwarding them immediately via `EventListener::send_event()`.
///
/// This is a TEMPORARY adapter — it exists only until consumers migrate
/// to subscribe to `Effect` directly (in `plans/effect-cutover/`).
///
/// Full implementation in 03.4.
#[derive(Debug)]
pub struct LegacyEventSink<L: EventListener> {
    _marker: PhantomData<L>,
}

impl<L: EventListener> LegacyEventSink<L> {
    /// Create a new legacy adapter wrapping the given listener.
    ///
    /// Stub — full constructor in 03.4.
    pub fn new(_listener: L) -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<L: EventListener + Sync> super::EffectSink for LegacyEventSink<L> {
    fn push(&self, _effect: Effect) {
        // Stub — full implementation in 03.4.
    }

    fn drain_into(&self, _out: &mut Vec<Effect>) {
        // No-op for immediate-forward sinks — effects were already
        // delivered via push() → send_event().
    }
}
