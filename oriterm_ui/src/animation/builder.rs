//! Fluent API for constructing animation groups.
//!
//! [`AnimationBuilder`] provides a chainable interface for building
//! [`AnimationGroup`]s with default duration/easing and optional
//! per-property overrides.
//!
//! # Example
//!
//! ```ignore
//! let group = AnimationBuilder::new(layer_id)
//!     .duration(Duration::from_millis(150))
//!     .easing(Easing::EaseOut)
//!     .opacity(0.0, 1.0)
//!     .transform(from_scale, Transform2D::identity())
//!     .build();
//! ```

use std::time::Duration;

use crate::geometry::{LayerId, Rect, Transform2D};

use super::Easing;
use super::group::{AnimationGroup, PropertyAnimation, TransitionTarget};
use super::sequence::{AnimationSequence, AnimationStep};

/// Fluent builder for [`AnimationGroup`] or [`AnimationSequence`].
///
/// Set group-level defaults with [`duration`](Self::duration) and
/// [`easing`](Self::easing), then add property animations. Each property
/// method accepts `(from, to)` values. Call [`build`](Self::build) for
/// a parallel animation group, or [`build_sequence`](Self::build_sequence)
/// to include an `on_end` callback.
pub struct AnimationBuilder {
    layer_id: LayerId,
    duration: Duration,
    easing: Easing,
    animations: Vec<PropertyAnimation>,
    on_end: Option<Box<dyn FnOnce(LayerId)>>,
}

impl AnimationBuilder {
    /// Creates a new builder for the given layer.
    pub fn new(layer_id: LayerId) -> Self {
        Self {
            layer_id,
            duration: Duration::from_millis(200),
            easing: Easing::EaseOut,
            animations: Vec::new(),
            on_end: None,
        }
    }

    /// Sets the default duration for all property animations.
    #[must_use]
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Sets the default easing for all property animations.
    #[must_use]
    pub fn easing(mut self, easing: Easing) -> Self {
        self.easing = easing;
        self
    }

    /// Adds an opacity animation from `from` to `to`.
    #[must_use]
    pub fn opacity(mut self, from: f32, to: f32) -> Self {
        self.animations.push(PropertyAnimation {
            from: Some(TransitionTarget::Opacity(from)),
            target: TransitionTarget::Opacity(to),
            duration: None,
            easing: None,
        });
        self
    }

    /// Adds a transform animation from `from` to `to`.
    #[must_use]
    pub fn transform(mut self, from: Transform2D, to: Transform2D) -> Self {
        self.animations.push(PropertyAnimation {
            from: Some(TransitionTarget::Transform(from)),
            target: TransitionTarget::Transform(to),
            duration: None,
            easing: None,
        });
        self
    }

    /// Adds a bounds animation from `from` to `to`.
    #[must_use]
    pub fn bounds(mut self, from: Rect, to: Rect) -> Self {
        self.animations.push(PropertyAnimation {
            from: Some(TransitionTarget::Bounds(from)),
            target: TransitionTarget::Bounds(to),
            duration: None,
            easing: None,
        });
        self
    }

    /// Sets a callback to fire when all animations in the group finish.
    ///
    /// Only takes effect with [`build_sequence`](Self::build_sequence).
    #[must_use]
    pub fn on_end(mut self, callback: impl FnOnce(LayerId) + 'static) -> Self {
        self.on_end = Some(Box::new(callback));
        self
    }

    /// Builds an [`AnimationGroup`] for parallel execution.
    ///
    /// Any `on_end` callback is ignored — use [`build_sequence`](Self::build_sequence)
    /// to include it.
    #[must_use]
    pub fn build(self) -> AnimationGroup {
        AnimationGroup {
            layer_id: self.layer_id,
            animations: self.animations,
            duration: self.duration,
            easing: self.easing,
        }
    }

    /// Builds an [`AnimationSequence`] containing the animation group
    /// followed by the `on_end` callback (if set).
    #[must_use]
    pub fn build_sequence(self) -> AnimationSequence {
        let layer_id = self.layer_id;
        let on_end = self.on_end;
        let group = AnimationGroup {
            layer_id,
            animations: self.animations,
            duration: self.duration,
            easing: self.easing,
        };
        let mut steps = vec![AnimationStep::Animate(group)];
        if let Some(cb) = on_end {
            steps.push(AnimationStep::Callback(Some(cb)));
        }
        AnimationSequence::new(layer_id, steps)
    }
}

impl std::fmt::Debug for AnimationBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationBuilder")
            .field("layer_id", &self.layer_id)
            .field("duration", &self.duration)
            .field("easing", &self.easing)
            .field("animations", &self.animations.len())
            .field("has_on_end", &self.on_end.is_some())
            .finish()
    }
}
