//! State resolution: maps [`InteractionState`] to active state names.
//!
//! Each resolver function has signature `fn(&InteractionState) -> &'static str`,
//! matching the `resolve` field on [`VisualStateGroup`]. The returned string
//! must match one of the group's [`VisualState::name`] values exactly.

use crate::interaction::InteractionState;

/// Stateless resolver functions for built-in state groups.
///
/// Each function maps an [`InteractionState`] snapshot to the name of the
/// active state within a group. The returned `&'static str` must exactly
/// match a [`VisualState::name`] in the corresponding group.
#[derive(Debug)]
pub struct StateResolver;

impl StateResolver {
    /// Resolve the active state in a `CommonStates` group.
    ///
    /// Priority: Disabled > Pressed > Hovered > Normal.
    pub fn resolve_common(interaction: &InteractionState) -> &'static str {
        if interaction.is_disabled() {
            return "Disabled";
        }
        if interaction.is_active() {
            return "Pressed";
        }
        if interaction.is_hot() {
            return "Hovered";
        }
        "Normal"
    }

    /// Resolve the active state in a `FocusStates` group.
    pub fn resolve_focus(interaction: &InteractionState) -> &'static str {
        if interaction.is_focused() {
            "Focused"
        } else {
            "Unfocused"
        }
    }
}

#[cfg(test)]
mod tests;
