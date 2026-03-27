//! Tests for `IdOverrideButton`.

use crate::action::keymap_action::Activate;
use crate::geometry::Rect;
use crate::widget_id::WidgetId;
use crate::widgets::{Widget, WidgetAction};

use super::IdOverrideButton;
use crate::widgets::button::ButtonWidget;

#[test]
fn key_context_delegates_to_button() {
    let inner = ButtonWidget::new("OK");
    let override_id = WidgetId::next();
    let wrapper = IdOverrideButton::new(inner, override_id);

    assert_eq!(wrapper.key_context(), Some("Button"));
}

#[test]
fn keyboard_activate_rewrites_id() {
    let inner = ButtonWidget::new("OK");
    let override_id = WidgetId::next();
    let mut wrapper = IdOverrideButton::new(inner, override_id);
    let bounds = Rect::new(0.0, 0.0, 100.0, 30.0);

    let result = wrapper.handle_keymap_action(&Activate, bounds);
    assert!(
        matches!(result, Some(WidgetAction::Clicked(id)) if id == override_id),
        "Enter/Space should produce Clicked with override ID, got: {result:?}"
    );
}

#[test]
fn set_disabled_delegates() {
    let inner = ButtonWidget::new("OK");
    let override_id = WidgetId::next();
    let mut wrapper = IdOverrideButton::new(inner, override_id);

    assert!(wrapper.is_focusable(), "enabled button should be focusable");
    wrapper.set_disabled(true);
    assert!(
        !wrapper.is_focusable(),
        "disabled button should not be focusable"
    );
    wrapper.set_disabled(false);
    assert!(
        wrapper.is_focusable(),
        "re-enabled button should be focusable"
    );
}

#[test]
fn returns_overridden_id() {
    let inner = ButtonWidget::new("OK");
    let inner_id = inner.id();
    let override_id = WidgetId::next();
    let wrapper = IdOverrideButton::new(inner, override_id);

    assert_eq!(wrapper.id(), override_id);
    assert_ne!(wrapper.id(), inner_id);
}
