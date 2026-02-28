//! Parallel animation groups for compositor layers.
//!
//! An [`AnimationGroup`] bundles multiple property animations on a single
//! layer so they all start simultaneously. Used by the [`AnimationBuilder`]
//! for fluent construction.
//!
//! [`AnimationBuilder`]: super::builder::AnimationBuilder

use std::time::Duration;

use crate::compositor::Transform2D;
use crate::compositor::layer::LayerId;
use crate::geometry::Rect;

use super::Easing;

/// The target value for a property animation.
#[derive(Debug, Clone, Copy)]
pub enum TransitionTarget {
    /// Target opacity value.
    Opacity(f32),
    /// Target 2D transform.
    Transform(Transform2D),
    /// Target bounds rectangle.
    Bounds(Rect),
}

/// A single property animation within a group.
#[derive(Debug, Clone, Copy)]
pub struct PropertyAnimation {
    /// Starting value (if `None`, reads current value from the layer tree).
    pub from: Option<TransitionTarget>,
    /// Target value.
    pub target: TransitionTarget,
    /// Duration for this property (overrides the group default if set).
    pub duration: Option<Duration>,
    /// Easing for this property (overrides the group default if set).
    pub easing: Option<Easing>,
}

/// A set of property animations that run in parallel on a single layer.
///
/// All animations start at the same instant. Each property can optionally
/// override the group-level duration and easing.
#[derive(Debug, Clone)]
pub struct AnimationGroup {
    /// Layer to animate.
    pub layer_id: LayerId,
    /// Property animations to run simultaneously.
    pub animations: Vec<PropertyAnimation>,
    /// Default duration for properties that don't specify their own.
    pub duration: Duration,
    /// Default easing for properties that don't specify their own.
    pub easing: Easing,
}
