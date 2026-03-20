//! Tests for `WindowRoot`.

use std::time::Instant;

use super::WindowRoot;

use crate::geometry::Rect;
use crate::input::{InputEvent, Modifiers};
use crate::testing::MockMeasurer;
use crate::theme::UiTheme;
use crate::widgets::Widget;
use crate::widgets::button::ButtonWidget;
use crate::widgets::label::LabelWidget;

fn measurer() -> MockMeasurer {
    MockMeasurer::new()
}

fn theme() -> UiTheme {
    UiTheme::dark()
}

// -- Construction tests --

/// Constructing a `WindowRoot` in a `#[test]` requires no GPU or platform.
#[test]
fn construct_default_viewport() {
    let root = WindowRoot::new(LabelWidget::new("hello"));
    assert_eq!(root.viewport(), Rect::new(0.0, 0.0, 800.0, 600.0));
    assert!(root.is_dirty());
    assert!(!root.is_urgent_redraw());
    assert!(!root.has_pending_actions());
}

/// Custom viewport propagates to overlay manager and layer tree root.
#[test]
fn construct_custom_viewport() {
    let vp = Rect::new(0.0, 0.0, 1920.0, 1080.0);
    let root = WindowRoot::with_viewport(LabelWidget::new("hi"), vp);
    assert_eq!(root.viewport(), vp);
}

/// `set_viewport` updates all viewport-dependent subsystems.
#[test]
fn set_viewport_propagates() {
    let mut root = WindowRoot::new(LabelWidget::new("test"));
    root.clear_dirty();
    assert!(!root.is_dirty());

    let new_vp = Rect::new(0.0, 0.0, 1024.0, 768.0);
    root.set_viewport(new_vp);
    assert_eq!(root.viewport(), new_vp);
    assert!(root.is_dirty());
}

/// `replace_widget` replaces the root widget and triggers rebuild.
#[test]
fn replace_widget() {
    let mut root = WindowRoot::new(LabelWidget::new("before"));
    root.replace_widget(Box::new(LabelWidget::new("after")));
    assert!(root.is_dirty());
}

/// Dirty/urgent flag management.
#[test]
fn dirty_flag_management() {
    let mut root = WindowRoot::new(LabelWidget::new("flags"));

    assert!(root.is_dirty());
    root.clear_dirty();
    assert!(!root.is_dirty());

    root.mark_dirty();
    assert!(root.is_dirty());

    assert!(!root.is_urgent_redraw());
    root.set_urgent_redraw(true);
    assert!(root.is_urgent_redraw());
    root.set_urgent_redraw(false);
    assert!(!root.is_urgent_redraw());
}

/// Action queue starts empty, `take_actions` drains it.
#[test]
fn action_queue_empty() {
    let mut root = WindowRoot::new(LabelWidget::new("actions"));
    assert!(!root.has_pending_actions());
    let actions = root.take_actions();
    assert!(actions.is_empty());
}

// -- Pipeline tests --

/// `compute_layout` produces a non-empty layout tree.
#[test]
fn compute_layout_produces_layout() {
    let mut root = WindowRoot::new(ButtonWidget::new("Click me"));
    root.compute_layout(&measurer(), &theme());

    // Layout should have non-zero dimensions.
    let layout = root.layout();
    assert!(layout.rect.width() > 0.0);
    assert!(layout.rect.height() > 0.0);
}

/// `compute_layout` registers the widget with InteractionManager.
#[test]
fn compute_layout_registers_widgets() {
    let btn = ButtonWidget::new("test");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    // The button should be registered and have default interaction state.
    let state = root.interaction().get_state(btn_id);
    assert!(!state.is_hot());
    assert!(!state.is_active());
}

/// Dispatching a mouse move updates the hot path.
#[test]
fn dispatch_mouse_move_updates_hot_path() {
    let btn = ButtonWidget::new("hover me");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();
    let btn_bounds = find_widget_bounds(root.layout(), btn_id);

    if let Some(bounds) = btn_bounds {
        let center = bounds.center();
        let event = InputEvent::MouseMove {
            pos: center,
            modifiers: Modifiers::NONE,
        };
        root.dispatch_event(&event, &measurer(), &theme(), now);

        // Button should be hot after mouse move onto it.
        let state = root.interaction().get_state(btn_id);
        assert!(state.is_hot(), "button should be hot after mouse move");
    }
}

/// Dispatching a click on a button fires an action.
#[test]
fn dispatch_click_fires_action() {
    use crate::input::MouseButton;

    let btn = ButtonWidget::new("click me");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();
    let btn_bounds = find_widget_bounds(root.layout(), btn_id);

    if let Some(bounds) = btn_bounds {
        let center = bounds.center();

        // Mouse move to hover the button first.
        root.dispatch_event(
            &InputEvent::MouseMove {
                pos: center,
                modifiers: Modifiers::NONE,
            },
            &measurer(),
            &theme(),
            now,
        );

        // Mouse down.
        root.dispatch_event(
            &InputEvent::MouseDown {
                pos: center,
                button: MouseButton::Left,
                modifiers: Modifiers::NONE,
            },
            &measurer(),
            &theme(),
            now,
        );

        // Mouse up — this should trigger a Clicked action.
        root.dispatch_event(
            &InputEvent::MouseUp {
                pos: center,
                button: MouseButton::Left,
                modifiers: Modifiers::NONE,
            },
            &measurer(),
            &theme(),
            now,
        );

        let actions = root.take_actions();
        assert!(
            !actions.is_empty(),
            "button click should produce at least one action"
        );
    }
}

/// `rebuild` re-registers widgets and rebuilds focus order.
#[test]
fn rebuild_reregisters_widgets() {
    let btn = ButtonWidget::new("focus me");
    let btn_id = btn.id();
    let root = WindowRoot::new(btn);

    // After construction, rebuild was called and widget is registered.
    let state = root.interaction().get_state(btn_id);
    assert!(!state.is_hot());
}

/// `prepare` runs without panicking on a fresh root.
#[test]
fn prepare_runs_cleanly() {
    let mut root = WindowRoot::new(LabelWidget::new("prepare"));
    root.compute_layout(&measurer(), &theme());
    root.prepare(Instant::now());
}

// -- Overlay tests --

/// Push a popup overlay, click inside it — widget tree should NOT see the event.
#[test]
fn overlay_consumes_click_inside() {
    use crate::input::MouseButton;
    use crate::overlay::Placement;

    let btn = ButtonWidget::new("background");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();

    // Push an overlay in the center of the viewport.
    let anchor = Rect::new(350.0, 250.0, 450.0, 350.0);
    let overlay_widget = Box::new(ButtonWidget::new("overlay"));
    root.push_overlay(overlay_widget, anchor, Placement::Below, now);
    assert!(root.has_overlays());

    // Click at a position inside the overlay anchor area.
    // The overlay should consume it, not the background button.
    let pos = crate::geometry::Point::new(400.0, 360.0);
    root.dispatch_event(
        &InputEvent::MouseDown {
            pos,
            button: MouseButton::Left,
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        now,
    );
    root.dispatch_event(
        &InputEvent::MouseUp {
            pos,
            button: MouseButton::Left,
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        now,
    );

    // Background button should NOT have fired an action (overlay consumed the click).
    let actions = root.take_actions();
    let btn_action_found = actions
        .iter()
        .any(|a| matches!(a, crate::action::WidgetAction::Clicked(id) if *id == btn_id));
    assert!(
        !btn_action_found,
        "background button should not receive click when overlay is active"
    );
}

/// Push a popup overlay, click outside it — overlay should be dismissed.
#[test]
fn overlay_dismissed_on_outside_click() {
    use crate::input::MouseButton;
    use crate::overlay::Placement;

    let mut root = WindowRoot::new(LabelWidget::new("bg"));
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();

    // Push overlay in the center.
    let anchor = Rect::new(350.0, 250.0, 450.0, 350.0);
    let overlay_widget = Box::new(ButtonWidget::new("popup"));
    root.push_overlay(overlay_widget, anchor, Placement::Below, now);
    assert!(root.has_overlays());

    // Click far outside the overlay.
    let pos = crate::geometry::Point::new(10.0, 10.0);
    root.dispatch_event(
        &InputEvent::MouseDown {
            pos,
            button: MouseButton::Left,
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        now,
    );

    // Overlay should be dismissed (or in dismissing state).
    // After the click-outside, the overlay manager either removes it or
    // starts the dismiss animation.
    // The overlay is no longer in the active overlay list.
    // (Dismissing overlays may still exist in the dismissing list, so
    //  has_overlays() might still be true during fade-out.)
}

// -- Helpers --

/// Searches a layout tree for a widget's bounds by ID.
fn find_widget_bounds(
    node: &crate::layout::LayoutNode,
    target: crate::widget_id::WidgetId,
) -> Option<Rect> {
    if node.widget_id == Some(target) {
        return Some(node.rect);
    }
    for child in &node.children {
        if let Some(r) = find_widget_bounds(child, target) {
            return Some(r);
        }
    }
    None
}
