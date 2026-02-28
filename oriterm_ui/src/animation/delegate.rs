//! Lifecycle callbacks for layer property animations.
//!
//! An [`AnimationDelegate`] receives notifications when animations
//! end or are canceled, enabling cleanup (e.g., removing a layer
//! after a fade-out completes).

use crate::compositor::LayerId;

/// Identifies which layer property is being animated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimatableProperty {
    /// Layer opacity (0.0..1.0).
    Opacity,
    /// Layer 2D affine transform.
    Transform,
    /// Layer bounds (position and size).
    Bounds,
}

/// Lifecycle callbacks for layer property animations.
///
/// Use cases: overlay manager (remove layer after fade-out), expose
/// mode (remove thumbnail after exit animation), Quick Terminal
/// (hide panel after slide-out).
pub trait AnimationDelegate {
    /// Called when an animation reaches its target value.
    fn animation_ended(&mut self, layer_id: LayerId, property: AnimatableProperty);

    /// Called when an animation is interrupted before completion.
    fn animation_canceled(&mut self, layer_id: LayerId, property: AnimatableProperty);
}
