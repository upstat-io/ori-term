//! Cursor blink state machine with smooth fade animation.
//!
//! Computes continuous opacity from elapsed time using configurable easing
//! functions. The blink cycle has four phases: visible plateau (full opacity),
//! fade-out transition, hidden plateau (zero opacity), and fade-in transition.
//!
//! Inspired by `WezTerm`'s `ColorEase` but uses a plateau-plus-fade model
//! instead of continuous oscillation, producing a crisper on/off rhythm
//! with smooth transitions at phase boundaries.

use std::time::{Duration, Instant};

use super::Easing;

/// Fraction of each phase duration occupied by the fade transition.
///
/// At 0.38, each 530ms phase has ~329ms of plateau (full on/off) and ~201ms
/// of smooth fade. This produces a snappy "settle, then transition" rhythm
/// that feels responsive while avoiding the abruptness of binary blink.
const FADE_FRACTION: f32 = 0.38;

/// Animation frame interval for fade transitions (~60fps).
const ANIMATION_FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// Default xterm cursor blink interval (530ms on, 530ms off).
#[cfg(test)]
const DEFAULT_BLINK_INTERVAL: Duration = Duration::from_millis(530);

/// Cursor blink state with smooth fade animation.
///
/// Computes continuous opacity in `[0.0, 1.0]` from elapsed time since
/// the last [`reset`](Self::reset). The blink cycle alternates between
/// visible (opacity 1.0) and hidden (opacity 0.0) phases, with smooth
/// eased transitions at phase boundaries.
///
/// The cycle structure for a single period (`in_duration + out_duration`):
///
/// ```text
/// |← visible plateau →|← fade out →|← hidden plateau →|← fade in →|
/// |    1.0 constant    | 1.0 → 0.0  |   0.0 constant   | 0.0 → 1.0 |
/// ```
///
/// Each phase's plateau occupies `1 - FADE_FRACTION` of the phase duration,
/// and the fade transition occupies `FADE_FRACTION`.
pub struct CursorBlink {
    /// Duration of the visible phase (plateau + subsequent fade-out).
    in_duration: Duration,
    /// Duration of the hidden phase (plateau + subsequent fade-in).
    out_duration: Duration,
    /// Easing function for the fade-in transition (0.0 → 1.0).
    in_ease: Easing,
    /// Easing function for the fade-out transition (1.0 → 0.0).
    out_ease: Easing,
    /// Cycle start time (reset on keypress/focus).
    epoch: Instant,
    /// Cached opacity from the last [`update`](Self::update) call.
    last_intensity: f32,
}

impl CursorBlink {
    /// Creates a new blink state with symmetric on/off phases, starting visible.
    ///
    /// Both visible and hidden phases use `interval` as their duration, with
    /// `EaseInOut` easing for fade transitions.
    pub fn new(interval: Duration) -> Self {
        Self {
            in_duration: interval,
            out_duration: interval,
            in_ease: Easing::EaseInOut,
            out_ease: Easing::EaseInOut,
            epoch: Instant::now(),
            last_intensity: 1.0,
        }
    }

    /// Returns the current cursor opacity in `[0.0, 1.0]`.
    ///
    /// Pure function of elapsed time — no accumulated drift.
    pub fn intensity(&self) -> f32 {
        self.intensity_at(self.epoch.elapsed())
    }

    /// Returns the next [`Instant`] at which opacity will change visually.
    ///
    /// During plateaus (full on or full off), returns the instant when the
    /// next fade transition begins. During fade transitions, returns ~16ms
    /// from now (animation frame rate at ~60fps).
    ///
    /// Use with `ControlFlow::WaitUntil` to schedule event loop wakeups.
    pub fn next_change(&self) -> Instant {
        let elapsed = self.epoch.elapsed();
        let total = self.in_duration + self.out_duration;
        if total.is_zero() {
            return self.epoch + Duration::from_secs(1);
        }

        let total_secs = total.as_secs_f64();
        let cycle_pos = elapsed.as_secs_f64() % total_secs;
        let cycle_start_secs = elapsed.as_secs_f64() - cycle_pos;

        let in_secs = self.in_duration.as_secs_f64();
        let out_secs = self.out_duration.as_secs_f64();
        let ff = f64::from(FADE_FRACTION);

        let fade_out_start = in_secs * (1.0 - ff);
        let hidden_plateau_end = in_secs + out_secs * (1.0 - ff);

        if cycle_pos < fade_out_start {
            // Visible plateau → wake at start of fade-out.
            self.epoch + Duration::from_secs_f64(cycle_start_secs + fade_out_start)
        } else if cycle_pos < in_secs {
            // Fade out → next animation frame.
            Instant::now() + ANIMATION_FRAME_INTERVAL
        } else if cycle_pos < hidden_plateau_end {
            // Hidden plateau → wake at start of fade-in.
            self.epoch + Duration::from_secs_f64(cycle_start_secs + hidden_plateau_end)
        } else {
            // Fade in → next animation frame.
            Instant::now() + ANIMATION_FRAME_INTERVAL
        }
    }

    /// Whether the cursor is currently visible.
    ///
    /// Returns `true` when [`intensity`](Self::intensity) exceeds 0.5.
    /// Consumers that need continuous opacity should call `intensity()` directly.
    pub fn is_visible(&self) -> bool {
        self.intensity() > 0.5
    }

    /// Alias for [`next_change`](Self::next_change).
    ///
    /// Retained for call-site compatibility; prefer `next_change()` in new code.
    pub fn next_toggle(&self) -> Instant {
        self.next_change()
    }

    /// Updates both phase durations (e.g. on config reload).
    pub fn set_interval(&mut self, interval: Duration) {
        self.in_duration = interval;
        self.out_duration = interval;
    }

    /// Resets the blink cycle to the start (full opacity).
    ///
    /// Called on keypress so the cursor stays visible while the user types.
    pub fn reset(&mut self) {
        self.epoch = Instant::now();
        self.last_intensity = 1.0;
    }

    /// Returns `true` if opacity changed enough to warrant a redraw.
    ///
    /// Threshold: > 0.01 change since the last call.
    pub fn update(&mut self) -> bool {
        let intensity = self.intensity();
        let changed = (intensity - self.last_intensity).abs() > 0.01;
        self.last_intensity = intensity;
        changed
    }

    /// Computes opacity at a given elapsed duration from epoch.
    fn intensity_at(&self, elapsed: Duration) -> f32 {
        let total = self.in_duration + self.out_duration;
        if total.is_zero() {
            return 1.0;
        }

        let total_secs = total.as_secs_f64();
        let cycle_pos = elapsed.as_secs_f64() % total_secs;

        let in_secs = self.in_duration.as_secs_f64();
        let out_secs = self.out_duration.as_secs_f64();
        let ff = f64::from(FADE_FRACTION);

        let fade_out_start = in_secs * (1.0 - ff);
        let fade_out_dur = in_secs * ff;
        let hidden_plateau_end = in_secs + out_secs * (1.0 - ff);
        let fade_in_dur = out_secs * ff;

        if cycle_pos < fade_out_start {
            1.0
        } else if cycle_pos < in_secs {
            let t = ((cycle_pos - fade_out_start) / fade_out_dur) as f32;
            1.0 - self.out_ease.apply(t)
        } else if cycle_pos < hidden_plateau_end {
            0.0
        } else {
            let t = ((cycle_pos - hidden_plateau_end) / fade_in_dur) as f32;
            self.in_ease.apply(t)
        }
    }
}

#[cfg(test)]
mod tests;
