//! Cursor blink state machine.
//!
//! Tracks the on/off phase of a blinking cursor at a configurable interval.
//! The application owns this state — the terminal only declares whether
//! blinking is enabled (`TermMode::CURSOR_BLINKING`); the actual
//! visibility toggle is driven here.
//!
//! Visibility is a pure function of elapsed time since the last reset
//! (epoch). This avoids drift that accumulates with toggle-based state
//! machines when the event loop fires late.

use std::time::{Duration, Instant};

/// Default xterm cursor blink interval (530ms on, 530ms off).
#[cfg(test)]
const DEFAULT_BLINK_INTERVAL: Duration = Duration::from_millis(530);

/// Cursor blink state.
///
/// Visibility alternates every [`interval`](Self::interval) based on
/// elapsed time since [`epoch`](Self::epoch). Reset on keypress to keep
/// the cursor visible while the user types.
pub struct CursorBlink {
    /// When the current blink cycle started (reset on keypress/focus).
    epoch: Instant,
    /// Duration of each blink phase (on/off).
    interval: Duration,
    /// Cached visibility from the last [`update`](Self::update) call,
    /// used to detect transitions and mark dirty.
    last_visible: bool,
}

impl CursorBlink {
    /// Create a new blink state with the given interval, starting visible.
    pub fn new(interval: Duration) -> Self {
        Self {
            epoch: Instant::now(),
            interval,
            last_visible: true,
        }
    }

    /// Whether the cursor is currently in the visible phase.
    ///
    /// Pure function of elapsed time: phase 0 (visible), phase 1 (hidden),
    /// phase 2 (visible), etc. No accumulated drift.
    pub fn is_visible(&self) -> bool {
        let elapsed_ms = self.epoch.elapsed().as_millis() as u64;
        let interval_ms = self.interval.as_millis().max(1) as u64;
        (elapsed_ms / interval_ms).is_multiple_of(2)
    }

    /// Update the blink interval (e.g. on config reload).
    pub fn set_interval(&mut self, interval: Duration) {
        self.interval = interval;
    }

    /// Reset blink to the visible phase.
    ///
    /// Called on keypress so the cursor stays visible while the user types.
    pub fn reset(&mut self) {
        self.epoch = Instant::now();
        self.last_visible = true;
    }

    /// Check whether visibility changed since the last call.
    ///
    /// Returns `true` if the phase transitioned (caller should mark dirty).
    pub fn update(&mut self) -> bool {
        let vis = self.is_visible();
        let changed = vis != self.last_visible;
        self.last_visible = vis;
        changed
    }

    /// The instant at which the next phase toggle should occur.
    ///
    /// Used with `ControlFlow::WaitUntil` to schedule the event loop
    /// wakeup without busy-waiting.
    pub fn next_toggle(&self) -> Instant {
        let elapsed_ms = self.epoch.elapsed().as_millis() as u64;
        let interval_ms = self.interval.as_millis().max(1) as u64;
        let current_phase = elapsed_ms / interval_ms;
        let next_phase_ms = (current_phase + 1) * interval_ms;
        self.epoch + Duration::from_millis(next_phase_ms)
    }
}

#[cfg(test)]
mod tests;
