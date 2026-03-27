//! Animatable property values with optional implicit animation.
//!
//! [`AnimProperty<T>`] is the core building block: a value that optionally
//! transitions smoothly when changed. Attach an [`AnimBehavior`] to make
//! changes auto-animate; without one, changes are instant.
//!
//! Animations are frame-based: `tick()` advances progress by one frame step.
//! No timestamps — the caller drives the animation by calling `tick()` once
//! per frame during the prepare phase. `get()` returns the current
//! interpolated value without needing a timestamp.

use super::Lerp;
use super::behavior::{AnimBehavior, AnimCurve};
use super::transaction::current_transaction;

/// Assumed frame duration for spring physics (1/60 second).
const SPRING_DT: f32 = 1.0 / 60.0;

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
    /// Current frame within the transition (0-based, advanced by `tick()`).
    frame: u32,
    /// Total frames for the transition (from `AnimBehavior`). 0 = instant.
    total_frames: u32,
    /// Current progress (0.0 = from, 1.0 = to). Updated by `tick()`.
    progress: f32,
    /// For spring-based transitions: velocity of the progress value.
    velocity: f32,
    /// The curve driving this transition.
    curve: AnimCurve,
}

/// A value that optionally transitions smoothly when changed.
///
/// When `behavior` is `None`, changes are instant. When `behavior` is
/// `Some`, `set()` starts a transition using the behavior's curve.
/// Transitions advance one step per `tick()` call (frame-based, no
/// timestamps). Read the current value with `get()`.
pub struct AnimProperty<T: Lerp> {
    /// The target (resting) value.
    target: T,
    /// The current interpolated value (updated by `tick()` each frame).
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

    /// Replace the stored behavior without affecting any in-flight transition.
    ///
    /// Pass `None` to make future `set()` calls instant. Pass `Some(behavior)`
    /// to make future `set()` calls animate. Does not interrupt a transition
    /// already in progress.
    pub fn set_behavior(&mut self, behavior: Option<AnimBehavior>) {
        self.behavior = behavior;
    }

    /// Set the target value.
    ///
    /// If a behavior is set (and no `Transaction` overrides it to instant),
    /// starts an animation from the current value. If no behavior
    /// (or `Transaction::instant()`), changes instantly.
    pub fn set(&mut self, value: T) {
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

        // Start transition from the current value.
        let from = self.current;
        let total_frames = behavior.total_frames();
        if total_frames == 0 {
            // Zero-frame behavior = instant.
            self.current = value;
            self.transition = None;
            return;
        }

        self.transition = Some(ActiveTransition {
            from,
            to: value,
            frame: 0,
            total_frames,
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
    /// Returns the value last computed by `tick()`. For instant properties
    /// (no behavior), returns the target directly.
    pub fn get(&self) -> T {
        self.current
    }

    /// Advance the animation by one frame.
    ///
    /// Must be called once per frame during the prepare phase. Advances
    /// both easing and spring transitions. No-op if no transition is active.
    pub fn tick(&mut self) {
        let Some(t) = &mut self.transition else {
            return;
        };

        match t.curve {
            AnimCurve::Easing { easing, .. } => {
                t.frame += 1;
                if t.frame >= t.total_frames {
                    // Animation complete.
                    self.current = self.target;
                    self.transition = None;
                    return;
                }
                let frac = t.frame as f32 / t.total_frames as f32;
                t.progress = easing.apply(frac);
                self.current = T::lerp(t.from, t.to, t.progress);
            }
            AnimCurve::Spring(spring) => {
                let (new_progress, new_velocity, done) =
                    spring.step(t.progress, 1.0, t.velocity, SPRING_DT);

                t.progress = new_progress;
                t.velocity = new_velocity;

                let clamped = t.progress.clamp(0.0, 1.0);
                self.current = T::lerp(t.from, t.to, clamped);

                if done {
                    self.current = self.target;
                    self.transition = None;
                }
            }
        }
    }

    /// Is an animation currently running?
    pub fn is_animating(&self) -> bool {
        self.transition.is_some()
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

#[cfg(test)]
mod tests;
