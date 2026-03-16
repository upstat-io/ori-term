//! Animated state transitions.
//!
//! [`VisualStateAnimator`] manages state groups, resolves active states each
//! frame, and drives [`AnimProperty`]-based transitions between states.
//! [`StateTransition`] declares per-state-pair animation overrides.

use std::collections::HashMap;
use std::time::Instant;

use crate::animation::behavior::AnimBehavior;
use crate::animation::property::AnimProperty;
use crate::color::Color;
use crate::interaction::InteractionState;

use super::{StateProperty, VisualStateGroup};

/// Declares the animation behavior for a specific state transition.
///
/// When the animator transitions between two states, it searches for a matching
/// `StateTransition`. Matching priority: exact `(from, to)` > `("*", to)` >
/// `(from, "*")` > `("*", "*")` > default behavior.
#[derive(Debug, Clone, Copy)]
pub struct StateTransition {
    /// State transitioning from (`"*"` = any).
    pub from: &'static str,
    /// State transitioning to (`"*"` = any).
    pub to: &'static str,
    /// Animation behavior for this transition.
    pub behavior: AnimBehavior,
}

/// Manages visual state groups and animates property transitions.
///
/// Owns one or more [`VisualStateGroup`]s and interpolates property values
/// between states using [`AnimProperty`]. Call `update()` each frame before
/// rendering, then read interpolated values via `get_bg_color()` etc.
///
/// If multiple groups set the same property, the group listed later in the
/// `groups` `Vec` takes precedence.
pub struct VisualStateAnimator {
    groups: Vec<VisualStateGroup>,
    transitions: Vec<StateTransition>,
    /// Default transition for state pairs without a specific override.
    default_transition: AnimBehavior,
    /// Currently interpolating color properties.
    ///
    /// Keys are [`StateProperty::key()`] discriminant strings.
    color_animations: HashMap<&'static str, AnimProperty<Color>>,
    /// Currently interpolating scalar properties.
    ///
    /// Keys are [`StateProperty::key()`] discriminant strings.
    float_animations: HashMap<&'static str, AnimProperty<f32>>,
}

impl VisualStateAnimator {
    /// Creates an animator from the given groups with default 100ms `EaseOut`.
    ///
    /// Populates `color_animations` and `float_animations` from each group's
    /// initial active state (index 0) using `AnimProperty::new()` (instant, no
    /// behavior). Behavior is attached lazily on the first state transition in
    /// `update()` via `set_behavior()`.
    pub fn new(groups: Vec<VisualStateGroup>) -> Self {
        let mut color_animations = HashMap::new();
        let mut float_animations = HashMap::new();

        for group in &groups {
            for prop in group.active_properties() {
                let key = prop.key();
                if prop.is_color() {
                    let c = prop.color_value().expect("is_color was true");
                    color_animations.insert(key, AnimProperty::new(c));
                } else {
                    let v = prop.float_value().expect("not color implies float");
                    float_animations.insert(key, AnimProperty::new(v));
                }
            }
        }

        Self {
            groups,
            transitions: Vec::new(),
            default_transition: AnimBehavior::ease_out(100),
            color_animations,
            float_animations,
        }
    }

    /// Adds a custom transition override for a specific state pair.
    #[must_use]
    pub fn with_transition(mut self, transition: StateTransition) -> Self {
        self.transitions.push(transition);
        self
    }

    /// Overrides the default transition behavior (replaces 100ms `EaseOut`).
    #[must_use]
    pub fn with_default_transition(mut self, behavior: AnimBehavior) -> Self {
        self.default_transition = behavior;
        self
    }

    /// Resolve states and start transitions if state changed.
    ///
    /// For each group, calls the group's resolver with the current interaction
    /// state. If the resolved state differs from the active state, finds the
    /// appropriate transition behavior and starts `AnimProperty` transitions
    /// for all properties in the new state.
    pub fn update(&mut self, interaction: &InteractionState, now: Instant) {
        for group_idx in 0..self.groups.len() {
            let resolve = self.groups[group_idx].resolve_fn();
            let resolved_name = resolve(interaction);

            if resolved_name == self.groups[group_idx].active_state_name() {
                continue;
            }

            let old_name = self.groups[group_idx].active_state_name();

            let new_idx = self.groups[group_idx]
                .states
                .iter()
                .position(|s| s.name == resolved_name);

            let Some(new_idx) = new_idx else {
                // Resolved name doesn't match any state — log and skip.
                log::warn!(
                    "VisualStateGroup '{}': resolved name '{}' not found in states",
                    self.groups[group_idx].name,
                    resolved_name
                );
                continue;
            };

            let behavior = find_transition(
                &self.transitions,
                &self.default_transition,
                old_name,
                resolved_name,
            );

            self.groups[group_idx].set_active(new_idx);

            // Clone the properties to avoid borrowing `self.groups` while mutating hashmaps.
            let properties: Vec<StateProperty> =
                self.groups[group_idx].states[new_idx].properties.clone();

            for prop in &properties {
                let key = prop.key();
                if prop.is_color() {
                    let c = prop.color_value().expect("is_color was true");
                    let anim = self
                        .color_animations
                        .entry(key)
                        .or_insert_with(|| AnimProperty::new(c));
                    anim.set_behavior(Some(behavior));
                    anim.set(c, now);
                } else {
                    let v = prop.float_value().expect("not color implies float");
                    let anim = self
                        .float_animations
                        .entry(key)
                        .or_insert_with(|| AnimProperty::new(v));
                    anim.set_behavior(Some(behavior));
                    anim.set(v, now);
                }
            }
        }
    }

    /// Advance spring-based transitions by one frame.
    ///
    /// Must be called each frame when `is_animating()` is true and any
    /// property uses spring-based `AnimBehavior`. Safe to call unconditionally
    /// — easing-based properties treat `tick()` as a no-op.
    pub fn tick(&mut self, now: Instant) {
        for anim in self.color_animations.values_mut() {
            anim.tick(now);
        }
        for anim in self.float_animations.values_mut() {
            anim.tick(now);
        }
    }

    /// Get the current interpolated background color.
    ///
    /// Returns `Color::TRANSPARENT` if no group sets `BgColor`.
    pub fn get_bg_color(&self, now: Instant) -> Color {
        self.get_color("BgColor", now).unwrap_or(Color::TRANSPARENT)
    }

    /// Get the current interpolated foreground color.
    ///
    /// Returns `Color::TRANSPARENT` if no group sets `FgColor`.
    pub fn get_fg_color(&self, now: Instant) -> Color {
        self.get_color("FgColor", now).unwrap_or(Color::TRANSPARENT)
    }

    /// Get the current interpolated border color.
    ///
    /// Returns `Color::TRANSPARENT` if no group sets `BorderColor`.
    pub fn get_border_color(&self, now: Instant) -> Color {
        self.get_color("BorderColor", now)
            .unwrap_or(Color::TRANSPARENT)
    }

    /// Get the current interpolated opacity.
    ///
    /// Returns `0.0` if no group sets `Opacity`.
    pub fn get_opacity(&self, now: Instant) -> f32 {
        self.get_float("Opacity", now).unwrap_or(0.0)
    }

    /// Get the current interpolated border width.
    ///
    /// Returns `0.0` if no group sets `BorderWidth`.
    pub fn get_border_width(&self, now: Instant) -> f32 {
        self.get_float("BorderWidth", now).unwrap_or(0.0)
    }

    /// Get the current interpolated corner radius.
    ///
    /// Returns `0.0` if no group sets `CornerRadius`.
    pub fn get_corner_radius(&self, now: Instant) -> f32 {
        self.get_float("CornerRadius", now).unwrap_or(0.0)
    }

    /// Generic getter for any color property by discriminant key.
    pub fn get_color(&self, key: &str, now: Instant) -> Option<Color> {
        self.color_animations.get(key).map(|a| a.get(now))
    }

    /// Generic getter for any scalar property by discriminant key.
    pub fn get_float(&self, key: &str, now: Instant) -> Option<f32> {
        self.float_animations.get(key).map(|a| a.get(now))
    }

    /// Are any transitions still animating?
    pub fn is_animating(&self, now: Instant) -> bool {
        self.color_animations.values().any(|a| a.is_animating(now))
            || self.float_animations.values().any(|a| a.is_animating(now))
    }
}

impl std::fmt::Debug for VisualStateAnimator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisualStateAnimator")
            .field("groups", &self.groups)
            .field("transitions", &self.transitions)
            .field("default_transition", &self.default_transition)
            .field("color_animation_count", &self.color_animations.len())
            .field("float_animation_count", &self.float_animations.len())
            .finish()
    }
}

/// Find the best matching transition for a state change.
///
/// Search priority: exact `(from, to)` > `("*", to)` > `(from, "*")` >
/// `("*", "*")` > default.
fn find_transition(
    transitions: &[StateTransition],
    default: &AnimBehavior,
    from: &str,
    to: &str,
) -> AnimBehavior {
    let mut wildcard_to = None;
    let mut from_wildcard = None;
    let mut both_wildcard = None;

    for t in transitions {
        if t.from == from && t.to == to {
            return t.behavior;
        }
        if t.from == "*" && t.to == to {
            wildcard_to = Some(t.behavior);
        }
        if t.from == from && t.to == "*" {
            from_wildcard = Some(t.behavior);
        }
        if t.from == "*" && t.to == "*" {
            both_wildcard = Some(t.behavior);
        }
    }

    wildcard_to
        .or(from_wildcard)
        .or(both_wildcard)
        .unwrap_or(*default)
}

#[cfg(test)]
mod tests;
