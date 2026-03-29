//! Animation behavior declarations — how properties transition on change.
//!
//! [`AnimBehavior`] declares the animation curve for a property. [`AnimCurve`]
//! unifies frame-count easing and velocity-based springs under one enum.
//! Durations are specified in milliseconds at the API layer and converted
//! to frame counts at 60 fps internally.

use super::Easing;
use super::spring::Spring;

/// Assumed frame rate for ms → frame conversion.
const FRAMES_PER_SECOND: f32 = 60.0;

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
    ///
    /// Converted to frame count at 60 fps (e.g. 100ms = 6 frames).
    pub fn ease_out(ms: u64) -> Self {
        Self {
            curve: AnimCurve::Easing {
                easing: Easing::EaseOut,
                total_frames: ms_to_frames(ms),
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
    ///
    /// Converted to frame count at 60 fps.
    pub fn linear(ms: u64) -> Self {
        Self {
            curve: AnimCurve::Easing {
                easing: Easing::Linear,
                total_frames: ms_to_frames(ms),
            },
        }
    }

    /// Ease-in-out animation with the given duration in milliseconds.
    ///
    /// Converted to frame count at 60 fps.
    pub fn ease_in_out(ms: u64) -> Self {
        Self {
            curve: AnimCurve::Easing {
                easing: Easing::EaseInOut,
                total_frames: ms_to_frames(ms),
            },
        }
    }

    /// Total frames for easing curves, 0 for springs (driven by physics).
    pub fn total_frames(&self) -> u32 {
        match self.curve {
            AnimCurve::Easing { total_frames, .. } => total_frames,
            // Springs run until physics settles — no fixed frame count.
            // AnimProperty handles this via the `done` flag from `step()`.
            AnimCurve::Spring(_) => u32::MAX,
        }
    }
}

/// Unifies frame-count easing and velocity-based springs.
///
/// The two approaches are fundamentally different:
/// - **Easing** is frame-count-based: `progress = frame / total_frames`.
/// - **Spring** is velocity-based: needs per-frame `step()` calls.
///
/// `AnimCurve` wraps both so `AnimBehavior` can use either transparently.
#[derive(Debug, Clone, Copy)]
pub enum AnimCurve {
    /// Frame-count easing (`progress = current_frame / total_frames`).
    Easing {
        /// The easing function to apply.
        easing: Easing,
        /// Total frames for the animation. 0 = instant.
        total_frames: u32,
    },
    /// Velocity-based spring (stateful, per-frame step).
    Spring(Spring),
}

/// Convert milliseconds to frame count at 60 fps (minimum 1 frame if ms > 0).
fn ms_to_frames(ms: u64) -> u32 {
    if ms == 0 {
        return 0;
    }
    ((ms as f32 / 1000.0) * FRAMES_PER_SECOND).ceil() as u32
}

#[cfg(test)]
mod tests;
