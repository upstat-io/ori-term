//! Animation behavior declarations — how properties transition on change.
//!
//! [`AnimBehavior`] declares the animation curve for a property. [`AnimCurve`]
//! unifies duration-based easing and velocity-based springs under one enum.

use std::time::Duration;

use super::Easing;
use super::spring::Spring;

/// Declares how a property transitions when its target value changes.
///
/// Attach to an [`AnimProperty`](super::property::AnimProperty) to make it
/// auto-animate on `set()`. Without a behavior, changes are instant.
#[derive(Debug, Clone, Copy)]
pub struct AnimBehavior {
    /// The animation curve to use for transitions.
    pub curve: AnimCurve,
}

impl AnimBehavior {
    /// Ease-out animation with the given duration in milliseconds.
    pub fn ease_out(ms: u64) -> Self {
        Self {
            curve: AnimCurve::Easing {
                easing: Easing::EaseOut,
                duration: Duration::from_millis(ms),
            },
        }
    }

    /// Spring-based animation with default parameters.
    pub fn spring() -> Self {
        Self {
            curve: AnimCurve::Spring(Spring::default()),
        }
    }

    /// Spring-based animation with custom parameters.
    pub fn spring_with(spring: Spring) -> Self {
        Self {
            curve: AnimCurve::Spring(spring),
        }
    }

    /// Linear animation with the given duration in milliseconds.
    pub fn linear(ms: u64) -> Self {
        Self {
            curve: AnimCurve::Easing {
                easing: Easing::Linear,
                duration: Duration::from_millis(ms),
            },
        }
    }

    /// Ease-in-out animation with the given duration in milliseconds.
    pub fn ease_in_out(ms: u64) -> Self {
        Self {
            curve: AnimCurve::Easing {
                easing: Easing::EaseInOut,
                duration: Duration::from_millis(ms),
            },
        }
    }
}

/// Unifies duration-based easing and velocity-based springs.
///
/// The two approaches are fundamentally different:
/// - **Easing** is stateless and time-fraction-based: `easing.apply(t) -> f32`.
/// - **Spring** is stateful and velocity-based: needs per-frame `step()` calls.
///
/// `AnimCurve` wraps both so [`AnimBehavior`] can use either transparently.
#[derive(Debug, Clone, Copy)]
pub enum AnimCurve {
    /// Duration-based easing (stateless, fraction-based).
    Easing {
        /// The easing function to apply.
        easing: Easing,
        /// Total animation duration.
        duration: Duration,
    },
    /// Velocity-based spring (stateful, per-frame step).
    Spring(Spring),
}

#[cfg(test)]
mod tests;
