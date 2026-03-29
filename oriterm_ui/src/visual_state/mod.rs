//! Visual state management for widgets.
//!
//! Widgets declare state groups (e.g., `CommonStates { Normal, Hovered, Pressed,
//! Disabled }`) with property values per state. The framework resolves the active
//! state from [`InteractionState`] and animates transitions via [`AnimProperty`].
//!
//! Inspired by WPF's `VisualStateManager` and QML's `states` + `transitions`.

pub mod resolver;
pub mod transition;

use crate::color::Color;
use crate::interaction::InteractionState;

pub use self::resolver::StateResolver;
pub use self::transition::{StateTransition, VisualStateAnimator};

/// A group of mutually exclusive visual states.
///
/// Each group has a resolver function that maps [`InteractionState`] to the
/// active state name. Only one state per group can be active at a time.
/// Multiple groups compose independently (e.g., `CommonStates` targets
/// `BgColor` while `FocusStates` targets `BorderColor`).
pub struct VisualStateGroup {
    /// Name of this group (for debugging/identification).
    pub name: &'static str,
    /// Available states in this group (mutually exclusive).
    pub states: Vec<VisualState>,
    /// Index of the currently active state in `states`.
    active: usize,
    /// Resolver function for this group.
    ///
    /// Maps `&InteractionState` to an active state name (`&'static str`).
    resolve: fn(&InteractionState) -> &'static str,
}

impl VisualStateGroup {
    /// Creates a new state group with the given states and resolver.
    ///
    /// The first state in `states` is the initial active state.
    pub fn new(
        name: &'static str,
        states: Vec<VisualState>,
        resolve: fn(&InteractionState) -> &'static str,
    ) -> Self {
        Self {
            name,
            states,
            active: 0,
            resolve,
        }
    }

    /// Returns the name of the currently active state.
    pub fn active_state_name(&self) -> &'static str {
        self.states[self.active].name
    }

    /// Returns the properties of the currently active state.
    pub fn active_properties(&self) -> &[StateProperty] {
        &self.states[self.active].properties
    }

    /// Returns the resolver function for this group.
    pub fn resolve_fn(&self) -> fn(&InteractionState) -> &'static str {
        self.resolve
    }

    /// Returns a mutable reference to the active index (for `VisualStateAnimator`).
    pub(crate) fn set_active(&mut self, index: usize) {
        self.active = index;
    }
}

impl std::fmt::Debug for VisualStateGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VisualStateGroup")
            .field("name", &self.name)
            .field("states", &self.states)
            .field("active", &self.active)
            .field("resolve", &"fn(&InteractionState) -> &str")
            .finish()
    }
}

/// A single visual state within a group.
///
/// Contains the property values that should be applied when this state is active.
#[derive(Debug, Clone)]
pub struct VisualState {
    /// Name of this state (e.g., "Normal", "Hovered", "Pressed").
    pub name: &'static str,
    /// Property values when this state is active.
    pub properties: Vec<StateProperty>,
}

/// A typed property value that can be animated between states.
///
/// Each variant carries a value and maps to a discriminant key string used
/// for `HashMap` lookup in [`VisualStateAnimator`].
#[derive(Debug, Clone, Copy)]
pub enum StateProperty {
    /// Background color.
    BgColor(Color),
    /// Foreground/text color.
    FgColor(Color),
    /// Border color.
    BorderColor(Color),
    /// Border width in logical pixels.
    BorderWidth(f32),
    /// Corner radius in logical pixels.
    CornerRadius(f32),
    /// Opacity (0.0 = transparent, 1.0 = opaque).
    Opacity(f32),
}

impl StateProperty {
    /// Returns the discriminant string used as the `HashMap` key.
    pub fn key(&self) -> &'static str {
        match self {
            Self::BgColor(_) => "BgColor",
            Self::FgColor(_) => "FgColor",
            Self::BorderColor(_) => "BorderColor",
            Self::BorderWidth(_) => "BorderWidth",
            Self::CornerRadius(_) => "CornerRadius",
            Self::Opacity(_) => "Opacity",
        }
    }

    /// Returns `true` for color variants (`BgColor`, `FgColor`, `BorderColor`).
    pub fn is_color(&self) -> bool {
        matches!(
            self,
            Self::BgColor(_) | Self::FgColor(_) | Self::BorderColor(_)
        )
    }

    /// Extracts the inner `Color`, or `None` for float variants.
    pub fn color_value(&self) -> Option<Color> {
        match self {
            Self::BgColor(c) | Self::FgColor(c) | Self::BorderColor(c) => Some(*c),
            Self::BorderWidth(_) | Self::CornerRadius(_) | Self::Opacity(_) => None,
        }
    }

    /// Extracts the inner `f32`, or `None` for color variants.
    pub fn float_value(&self) -> Option<f32> {
        match self {
            Self::BorderWidth(v) | Self::CornerRadius(v) | Self::Opacity(v) => Some(*v),
            Self::BgColor(_) | Self::FgColor(_) | Self::BorderColor(_) => None,
        }
    }
}

impl PartialEq for StateProperty {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BgColor(a), Self::BgColor(b))
            | (Self::FgColor(a), Self::FgColor(b))
            | (Self::BorderColor(a), Self::BorderColor(b)) => a == b,
            (Self::BorderWidth(a), Self::BorderWidth(b))
            | (Self::CornerRadius(a), Self::CornerRadius(b))
            | (Self::Opacity(a), Self::Opacity(b)) => a == b,
            _ => false,
        }
    }
}

/// Creates a `CommonStates` group: Normal, Hovered, Pressed, Disabled.
///
/// Targets `BgColor`. Resolver: disabled > pressed > hovered > normal.
/// Includes an instant transition to Disabled (convention: disabled state
/// changes should not animate).
pub fn common_states(
    normal_bg: Color,
    hover_bg: Color,
    pressed_bg: Color,
    disabled_bg: Color,
) -> VisualStateGroup {
    VisualStateGroup::new(
        "CommonStates",
        vec![
            VisualState {
                name: "Normal",
                properties: vec![StateProperty::BgColor(normal_bg)],
            },
            VisualState {
                name: "Hovered",
                properties: vec![StateProperty::BgColor(hover_bg)],
            },
            VisualState {
                name: "Pressed",
                properties: vec![StateProperty::BgColor(pressed_bg)],
            },
            VisualState {
                name: "Disabled",
                properties: vec![StateProperty::BgColor(disabled_bg)],
            },
        ],
        StateResolver::resolve_common,
    )
}

/// Creates a `FocusStates` group: Unfocused, Focused.
///
/// Targets `BorderColor`. Resolver: focused > unfocused.
pub fn focus_states(unfocused_border: Color, focused_border: Color) -> VisualStateGroup {
    VisualStateGroup::new(
        "FocusStates",
        vec![
            VisualState {
                name: "Unfocused",
                properties: vec![StateProperty::BorderColor(unfocused_border)],
            },
            VisualState {
                name: "Focused",
                properties: vec![StateProperty::BorderColor(focused_border)],
            },
        ],
        StateResolver::resolve_focus,
    )
}

#[cfg(test)]
mod tests;
