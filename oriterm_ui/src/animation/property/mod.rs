//! Animatable property values with optional implicit animation.
//!
//! [`AnimProperty<T>`] is the core building block: a value that optionally
//! transitions smoothly when changed. Attach an [`AnimBehavior`] to make
//! changes auto-animate; without one, changes are instant.

use std::time::{Duration, Instant};

use super::Lerp;
use super::behavior::{AnimBehavior, AnimCurve};
use super::transaction::current_transaction;

/// In-flight transition state for an `AnimProperty`.
///
/// Both easing and spring produce a scalar progress value (0.0 to 1.0),
/// which is fed to `Lerp` to compute the actual value.
#[derive(Debug, Clone, Copy)]
struct ActiveTransition<T: Lerp> {
    /// Value at the start of the transition.
    from: T,
    /// Value at the end of the transition (same as `AnimProperty::target`).
    to: T,
    /// When the transition started.
    start: Instant,
    /// Timestamp of the last `tick()` call. Used to compute delta time
    /// for spring physics. Equal to `start` before the first tick.
    last_tick: Instant,
    /// Current progress through the transition (0.0 = from, 1.0 = to).
    /// For easing: computed lazily from elapsed time.
    /// For springs: advanced by `tick()` each frame.
    progress: f32,
    /// For spring-based transitions: velocity of the progress value.
    /// Tracks rate of change of the normalized progress (0.0 to 1.0),
    /// NOT velocity in the value's coordinate space.
    /// Unused for easing-based transitions.
    velocity: f32,
    /// The curve driving this transition.
    curve: AnimCurve,
}

/// A value that optionally transitions smoothly when changed.
///
/// Replaces `AnimatedValue<T>`. When `behavior` is `None`, changes are instant.
/// When `behavior` is `Some`, `set()` starts a transition using the behavior's
/// curve. Transactions can override the behavior for a block of state changes.
pub struct AnimProperty<T: Lerp> {
    /// The target (resting) value.
    target: T,
    /// The current interpolated value (updated by `tick()` for springs,
    /// computed lazily for easing-based transitions).
    current: T,
    /// Optional animation behavior — `None` means instant changes.
    behavior: Option<AnimBehavior>,
    /// In-flight transition, if any.
    transition: Option<ActiveTransition<T>>,
}

impl<T: Lerp> AnimProperty<T> {
    /// Creates an instantly-changing property (no animation).
    pub fn new(value: T) -> Self {
        Self {
            target: value,
            current: value,
            behavior: None,
            transition: None,
        }
    }

    /// Creates a property with auto-animation on `set()`.
    pub fn with_behavior(value: T, behavior: AnimBehavior) -> Self {
        Self {
            target: value,
            current: value,
            behavior: Some(behavior),
            transition: None,
        }
    }

    /// Set the target value.
    ///
    /// If a behavior is set (and no `Transaction` overrides it to instant),
    /// starts an animation from the current interpolated value. If no behavior
    /// (or `Transaction::instant()`), changes instantly.
    ///
    /// Requires `now` to compute the current interpolated value for smooth
    /// interruption (starting the new animation from mid-flight).
    pub fn set(&mut self, value: T, now: Instant) {
        self.target = value;

        // Determine the effective behavior: transaction overrides property.
        let effective = match current_transaction() {
            Some(tx) => tx.animation,
            None => self.behavior,
        };

        let Some(behavior) = effective else {
            // Instant change — no animation.
            self.current = value;
            self.transition = None;
            return;
        };

        // Start transition from the current interpolated value.
        let from = self.get(now);
        self.transition = Some(ActiveTransition {
            from,
            to: value,
            start: now,
            last_tick: now,
            progress: 0.0,
            velocity: 0.0,
            curve: behavior.curve,
        });
    }

    /// Set without animation (even if behavior exists).
    pub fn set_immediate(&mut self, value: T) {
        self.target = value;
        self.current = value;
        self.transition = None;
    }

    /// Get the current interpolated value.
    ///
    /// For easing-based transitions: computes the value from elapsed time.
    /// For spring-based transitions: returns the last value computed by `tick()`.
    pub fn get(&self, now: Instant) -> T {
        let Some(t) = &self.transition else {
            return self.current;
        };

        match t.curve {
            AnimCurve::Easing { easing, duration } => {
                let progress = easing_progress(t.start, now, duration, easing);
                T::lerp(t.from, t.to, progress)
            }
            AnimCurve::Spring(_) => {
                // Spring progress is advanced by tick(); return the cached current.
                self.current
            }
        }
    }

    /// Advance spring-based transitions by one frame.
    ///
    /// Must be called each frame for spring animations (during `anim_frame()`).
    /// No-op for easing-based transitions (those are computed lazily in `get()`).
    /// No-op if no transition is active.
    pub fn tick(&mut self, now: Instant) {
        let Some(t) = &mut self.transition else {
            return;
        };

        let AnimCurve::Spring(spring) = t.curve else {
            return;
        };

        let dt = now.duration_since(t.last_tick).as_secs_f32();
        t.last_tick = now;

        if dt <= 0.0 {
            return;
        }

        let (new_progress, new_velocity, done) = spring.step(t.progress, 1.0, t.velocity, dt);

        t.progress = new_progress;
        t.velocity = new_velocity;

        // Clamp progress for lerp to avoid extrapolation beyond the target.
        let clamped = t.progress.clamp(0.0, 1.0);
        self.current = T::lerp(t.from, t.to, clamped);

        if done {
            self.current = self.target;
            self.transition = None;
        }
    }

    /// Is an animation currently running?
    pub fn is_animating(&self, now: Instant) -> bool {
        let Some(t) = &self.transition else {
            return false;
        };

        match t.curve {
            AnimCurve::Easing { duration, .. } => now.duration_since(t.start) < duration,
            AnimCurve::Spring(_) => true, // Active until tick() clears it.
        }
    }

    /// Returns the final resting value.
    pub fn target(&self) -> T {
        self.target
    }
}

impl<T: Lerp + std::fmt::Debug> std::fmt::Debug for AnimProperty<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimProperty")
            .field("target", &self.target)
            .field("current", &self.current)
            .field("has_behavior", &self.behavior.is_some())
            .field("is_transitioning", &self.transition.is_some())
            .finish()
    }
}

impl<T: Lerp> Clone for AnimProperty<T> {
    fn clone(&self) -> Self {
        Self {
            target: self.target,
            current: self.current,
            behavior: self.behavior,
            transition: self.transition,
        }
    }
}

/// Compute easing progress from elapsed time.
fn easing_progress(start: Instant, now: Instant, duration: Duration, easing: super::Easing) -> f32 {
    if duration.is_zero() {
        return 1.0;
    }
    let elapsed = now.duration_since(start);
    if elapsed >= duration {
        return 1.0;
    }
    let t = elapsed.as_secs_f32() / duration.as_secs_f32();
    easing.apply(t)
}

#[cfg(test)]
mod tests;
