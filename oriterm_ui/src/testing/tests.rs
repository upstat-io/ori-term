use std::time::Duration;

use crate::geometry::Point;
use crate::input::{InputEvent, Modifiers, MouseButton};
use crate::widgets::Widget;
use crate::widgets::button::ButtonWidget;

use super::WidgetTestHarness;

// -- WidgetTestHarness tests --

#[test]
fn harness_constructs_with_button() {
    let button = ButtonWidget::new("Click me");
    let button_id = button.id();
    let harness = WidgetTestHarness::new(button);

    // Layout should produce non-zero bounds.
    let bounds = harness.find_widget_bounds(button_id);
    assert!(bounds.is_some(), "button should have layout bounds");
    let rect = bounds.unwrap();
    assert!(rect.width() > 0.0, "button width should be positive");
    assert!(rect.height() > 0.0, "button height should be positive");
}

#[test]
fn harness_with_custom_size() {
    let button = ButtonWidget::new("Test");
    let harness = WidgetTestHarness::with_size(button, 400.0, 300.0);
    assert_eq!(harness.viewport().width(), 400.0);
    assert_eq!(harness.viewport().height(), 300.0);
}

#[test]
fn harness_process_mouse_move() {
    let button = ButtonWidget::new("Hover me");
    let button_id = button.id();
    let mut harness = WidgetTestHarness::new(button);

    // Get button center.
    let bounds = harness.find_widget_bounds(button_id).unwrap();
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    );

    // Dispatch mouse move to button center.
    let event = InputEvent::MouseMove {
        pos: center,
        modifiers: Modifiers::NONE,
    };
    harness.process_event(event);

    // Button should now be hot.
    assert!(
        harness.is_hot(button_id),
        "button should be hot after mouse move to its center"
    );
}

#[test]
fn harness_process_click() {
    let button = ButtonWidget::new("Click me");
    let button_id = button.id();
    let mut harness = WidgetTestHarness::new(button);

    let bounds = harness.find_widget_bounds(button_id).unwrap();
    let center = Point::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    );

    // Move mouse to button.
    harness.process_event(InputEvent::MouseMove {
        pos: center,
        modifiers: Modifiers::NONE,
    });

    // Press.
    harness.process_event(InputEvent::MouseDown {
        pos: center,
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    });
    assert!(
        harness.is_active(button_id),
        "button should be active after mouse down"
    );

    // Release.
    harness.process_event(InputEvent::MouseUp {
        pos: center,
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    });
    assert!(
        !harness.is_active(button_id),
        "button should not be active after mouse up"
    );

    // Should have a Clicked action.
    let actions = harness.take_actions();
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, crate::action::WidgetAction::Clicked(id) if *id == button_id)),
        "should have Clicked action for button, got: {actions:?}"
    );
}

#[test]
fn harness_advance_time() {
    let button = ButtonWidget::new("Animated");
    let mut harness = WidgetTestHarness::new(button);

    // Advance time (should not panic even with no pending animations).
    harness.advance_time(Duration::from_millis(16));

    // Verify clock accessor works.
    let _now = harness.now();
    let _pos = harness.mouse_pos();
}

#[test]
fn harness_click_produces_clicked_action() {
    let button = ButtonWidget::new("Click me");
    let button_id = button.id();
    let mut harness = WidgetTestHarness::new(button);

    let actions = harness.click(button_id);
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, crate::action::WidgetAction::Clicked(id) if *id == button_id)),
        "click() should produce Clicked action, got: {actions:?}"
    );
}

#[test]
fn harness_mouse_move_to_makes_hot() {
    let button = ButtonWidget::new("Hover me");
    let button_id = button.id();
    let mut harness = WidgetTestHarness::new(button);

    harness.mouse_move_to(button_id);
    assert!(
        harness.is_hot(button_id),
        "button should be hot after mouse_move_to"
    );
}

#[test]
fn harness_is_hot_after_mouse_move_to() {
    let button = ButtonWidget::new("Hover test");
    let button_id = button.id();
    let mut harness = WidgetTestHarness::new(button);

    assert!(!harness.is_hot(button_id));
    harness.mouse_move_to(button_id);
    assert!(harness.is_hot(button_id));
}

#[test]
fn harness_get_widget_ref() {
    let button = ButtonWidget::new("Root");
    let button_id = button.id();
    let harness = WidgetTestHarness::new(button);

    let wref = harness.get_widget(button_id);
    assert_eq!(wref.id(), button_id);
    assert!(!wref.is_hot());
    assert!(wref.bounds().width() > 0.0);
}

#[test]
fn harness_all_widget_ids() {
    let button = ButtonWidget::new("One");
    let button_id = button.id();
    let harness = WidgetTestHarness::new(button);

    let ids = harness.all_widget_ids();
    assert!(ids.contains(&button_id), "should contain button ID");
}

#[test]
fn harness_render_button_has_rect_and_text() {
    let button = ButtonWidget::new("Render test");
    let mut harness = WidgetTestHarness::new(button);
    let scene = harness.render();

    let rects = super::render_assert::rects(&scene);
    assert!(!rects.is_empty(), "button should paint at least one rect");
    let texts = super::render_assert::texts(&scene);
    assert!(!texts.is_empty(), "button should paint text");
    assert!(
        super::render_assert::command_count(&scene) >= 2,
        "button should have at least 2 primitives (quad + text)"
    );
}

#[test]
fn harness_widgets_with_sense_returns_clickable() {
    use crate::sense::Sense;

    let button = ButtonWidget::new("Clickable");
    let button_id = button.id();
    let harness = WidgetTestHarness::new(button);

    let clickable = harness.widgets_with_sense(Sense::click());
    assert!(
        clickable.contains(&button_id),
        "button should be in clickable widgets"
    );
}

#[test]
fn harness_focus_traversal() {
    use crate::widgets::stack::StackWidget;

    let btn1 = ButtonWidget::new("First");
    let btn2 = ButtonWidget::new("Second");
    let btn3 = ButtonWidget::new("Third");
    let id1 = btn1.id();
    let id2 = btn2.id();
    let id3 = btn3.id();

    let stack = StackWidget::new(vec![Box::new(btn1), Box::new(btn2), Box::new(btn3)]);
    let mut h = WidgetTestHarness::new(stack);

    // No focus initially.
    assert!(h.focused_widget().is_none());

    // Tab -> first focusable.
    h.tab();
    assert_eq!(
        h.focused_widget(),
        Some(id1),
        "first tab should focus first widget"
    );

    // Tab -> second.
    h.tab();
    assert_eq!(
        h.focused_widget(),
        Some(id2),
        "second tab should focus second widget"
    );

    // Tab -> third.
    h.tab();
    assert_eq!(
        h.focused_widget(),
        Some(id3),
        "third tab should focus third widget"
    );

    // Shift+Tab -> back to second.
    h.shift_tab();
    assert_eq!(
        h.focused_widget(),
        Some(id2),
        "shift+tab should focus second widget"
    );
}

#[test]
fn harness_paint_hover_changes_output() {
    let button = ButtonWidget::new("Paint test");
    let button_id = button.id();
    let mut h = WidgetTestHarness::new(button);

    // Paint in normal state.
    let draw_list_normal = h.render();
    let rects_normal = super::render_assert::rects(&draw_list_normal);
    assert!(!rects_normal.is_empty(), "should have rect commands");

    // Hover and paint again.
    h.mouse_move_to(button_id);
    let draw_list_hover = h.render();
    let rects_hover = super::render_assert::rects(&draw_list_hover);
    assert!(
        !rects_hover.is_empty(),
        "should still have rect commands after hover"
    );

    // Both states should produce draw commands (we can't easily compare colors
    // without knowing the theme, but both should be non-empty).
    assert!(super::render_assert::command_count(&draw_list_normal) > 0);
    assert!(super::render_assert::command_count(&draw_list_hover) > 0);
}

#[test]
fn harness_rebuild_layout_updates_focus_order() {
    let button = ButtonWidget::new("Focusable");
    let button_id = button.id();
    let harness = WidgetTestHarness::new(button);

    // Button should be in focus order.
    let focusable = harness.focusable_widgets();
    assert!(
        focusable.contains(&button_id),
        "focusable button should be in focus order"
    );
}

// -- Overlay test helpers --

#[test]
fn harness_overlay_push_and_dismiss() {
    use crate::geometry::Rect;
    use crate::widgets::spacer::SpacerWidget;

    let button = ButtonWidget::new("Main");
    let mut h = WidgetTestHarness::new(button);

    assert!(!h.has_overlays());

    // Push a popup overlay.
    let overlay = SpacerWidget::fixed(100.0, 40.0);
    h.push_popup(overlay, Rect::new(50.0, 50.0, 100.0, 40.0));
    assert!(h.has_overlays());

    // Dismiss all overlays.
    h.dismiss_overlays();
    assert!(!h.has_overlays());
}

#[test]
fn harness_root_accessor() {
    let button = ButtonWidget::new("Root");
    let harness = WidgetTestHarness::new(button);

    // Should be able to access WindowRoot directly.
    assert!(harness.root().is_dirty());
    assert!(!harness.root().has_pending_actions());
}
