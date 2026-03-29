use crate::color::Color;
use crate::interaction::InteractionState;

use super::{StateProperty, common_states, focus_states};

// -- StateProperty tests --

#[test]
fn state_property_key_returns_correct_discriminant() {
    assert_eq!(StateProperty::BgColor(Color::BLACK).key(), "BgColor");
    assert_eq!(StateProperty::FgColor(Color::BLACK).key(), "FgColor");
    assert_eq!(
        StateProperty::BorderColor(Color::BLACK).key(),
        "BorderColor"
    );
    assert_eq!(StateProperty::BorderWidth(1.0).key(), "BorderWidth");
    assert_eq!(StateProperty::CornerRadius(4.0).key(), "CornerRadius");
    assert_eq!(StateProperty::Opacity(0.5).key(), "Opacity");
}

#[test]
fn state_property_color_value_returns_some_for_colors() {
    let c = Color::hex(0xFF0000);
    assert_eq!(StateProperty::BgColor(c).color_value(), Some(c));
    assert_eq!(StateProperty::FgColor(c).color_value(), Some(c));
    assert_eq!(StateProperty::BorderColor(c).color_value(), Some(c));
}

#[test]
fn state_property_color_value_returns_none_for_floats() {
    assert_eq!(StateProperty::BorderWidth(1.0).color_value(), None);
    assert_eq!(StateProperty::CornerRadius(4.0).color_value(), None);
    assert_eq!(StateProperty::Opacity(0.5).color_value(), None);
}

#[test]
fn state_property_float_value_returns_some_for_floats() {
    assert_eq!(StateProperty::BorderWidth(1.0).float_value(), Some(1.0));
    assert_eq!(StateProperty::CornerRadius(4.0).float_value(), Some(4.0));
    assert_eq!(StateProperty::Opacity(0.5).float_value(), Some(0.5));
}

#[test]
fn state_property_float_value_returns_none_for_colors() {
    let c = Color::hex(0xFF0000);
    assert_eq!(StateProperty::BgColor(c).float_value(), None);
    assert_eq!(StateProperty::FgColor(c).float_value(), None);
    assert_eq!(StateProperty::BorderColor(c).float_value(), None);
}

// -- Preset tests --

#[test]
fn common_states_creates_group_with_4_states() {
    let group = common_states(Color::BLACK, Color::WHITE, Color::BLACK, Color::WHITE);
    assert_eq!(group.name, "CommonStates");
    assert_eq!(group.states.len(), 4);
    assert_eq!(group.states[0].name, "Normal");
    assert_eq!(group.states[1].name, "Hovered");
    assert_eq!(group.states[2].name, "Pressed");
    assert_eq!(group.states[3].name, "Disabled");
}

#[test]
fn common_states_resolver_returns_correct_states() {
    let group = common_states(Color::BLACK, Color::WHITE, Color::BLACK, Color::WHITE);
    let resolve = group.resolve_fn();

    assert_eq!(resolve(&InteractionState::new()), "Normal");

    let mut hovered = InteractionState::new();
    hovered.set_hot(true);
    assert_eq!(resolve(&hovered), "Hovered");

    let mut pressed = InteractionState::new();
    pressed.set_active(true);
    assert_eq!(resolve(&pressed), "Pressed");

    assert_eq!(resolve(&InteractionState::disabled()), "Disabled");
}

#[test]
fn focus_states_creates_group_with_2_states() {
    let group = focus_states(Color::BLACK, Color::WHITE);
    assert_eq!(group.name, "FocusStates");
    assert_eq!(group.states.len(), 2);
    assert_eq!(group.states[0].name, "Unfocused");
    assert_eq!(group.states[1].name, "Focused");
}

#[test]
fn newly_created_group_has_active_index_zero() {
    let group = common_states(Color::BLACK, Color::WHITE, Color::BLACK, Color::WHITE);
    assert_eq!(group.active_state_name(), "Normal");
}
