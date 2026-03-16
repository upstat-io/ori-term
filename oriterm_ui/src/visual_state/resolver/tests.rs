use crate::interaction::InteractionState;

use super::StateResolver;

#[test]
fn resolve_common_returns_disabled_when_disabled() {
    let state = InteractionState::disabled();
    assert_eq!(StateResolver::resolve_common(&state), "Disabled");
}

#[test]
fn resolve_common_returns_pressed_when_active() {
    let mut state = InteractionState::new();
    state.set_active(true);
    assert_eq!(StateResolver::resolve_common(&state), "Pressed");
}

#[test]
fn resolve_common_returns_hovered_when_hot() {
    let mut state = InteractionState::new();
    state.set_hot(true);
    assert_eq!(StateResolver::resolve_common(&state), "Hovered");
}

#[test]
fn resolve_common_returns_normal_when_none() {
    let state = InteractionState::new();
    assert_eq!(StateResolver::resolve_common(&state), "Normal");
}

#[test]
fn resolve_common_disabled_takes_priority_over_active() {
    let mut state = InteractionState::disabled();
    state.set_active(true);
    state.set_hot(true);
    assert_eq!(StateResolver::resolve_common(&state), "Disabled");
}

#[test]
fn resolve_common_active_takes_priority_over_hot() {
    let mut state = InteractionState::new();
    state.set_active(true);
    state.set_hot(true);
    assert_eq!(StateResolver::resolve_common(&state), "Pressed");
}

#[test]
fn resolve_focus_returns_focused_when_focused() {
    let mut state = InteractionState::new();
    state.set_focused(true);
    assert_eq!(StateResolver::resolve_focus(&state), "Focused");
}

#[test]
fn resolve_focus_returns_unfocused_when_not_focused() {
    let state = InteractionState::new();
    assert_eq!(StateResolver::resolve_focus(&state), "Unfocused");
}
