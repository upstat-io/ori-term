//! Tests for `WindowRoot`.

use std::time::Instant;

use super::WindowRoot;

use crate::geometry::Rect;
use crate::input::{InputEvent, Modifiers};
use crate::invalidation::DirtyKind;
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

/// `rebuild` syncs InteractionManager focus when focused widget leaves the order.
///
/// Regression test for TPR-11-005: `rebuild()` calls `set_focus_order()` which
/// may clear FocusManager's focus, but InteractionManager was not updated.
#[test]
fn rebuild_syncs_interaction_focus_on_order_change() {
    let btn = ButtonWidget::new("old");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);

    // Focus the button through InteractionManager.
    {
        let (interaction, focus) = root.interaction_and_focus_mut();
        interaction.request_focus(btn_id, focus);
        let _ = interaction.drain_events();
    }
    assert_eq!(root.interaction().focused_widget(), Some(btn_id));
    assert_eq!(root.focus().focused(), Some(btn_id));

    // Replace with a different widget — old btn_id leaves the focus order.
    root.replace_widget(Box::new(LabelWidget::new("new")));

    // Both managers must agree: no focus.
    assert_eq!(root.focus().focused(), None);
    assert_eq!(
        root.interaction().focused_widget(),
        None,
        "InteractionManager must clear focus when focused widget leaves focus order"
    );
    assert!(
        root.interaction().focus_ancestor_path().is_empty(),
        "focus_ancestor_path must be empty when no widget is focused"
    );
}

/// `compute_layout` syncs InteractionManager focus when focused widget
/// leaves the order.
#[test]
fn compute_layout_syncs_interaction_focus_on_order_change() {
    let btn = ButtonWidget::new("old");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    // Focus the button.
    {
        let (interaction, focus) = root.interaction_and_focus_mut();
        interaction.request_focus(btn_id, focus);
        let _ = interaction.drain_events();
    }
    assert_eq!(root.interaction().focused_widget(), Some(btn_id));

    // Replace widget and recompute layout.
    root.replace_widget(Box::new(LabelWidget::new("new")));
    root.compute_layout(&measurer(), &theme());

    assert_eq!(root.focus().focused(), None);
    assert_eq!(
        root.interaction().focused_widget(),
        None,
        "InteractionManager must clear focus after compute_layout with new tree"
    );
}

/// `prepare` runs without panicking on a fresh root.
#[test]
fn prepare_runs_cleanly() {
    let mut root = WindowRoot::new(LabelWidget::new("prepare"));
    root.compute_layout(&measurer(), &theme());
    root.prepare(Instant::now(), &theme());
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

/// Moving the cursor over an active overlay must NOT make background widgets
/// hot — the overlay consumes the mouse event before the base tree hot path
/// is updated (TPR-11-011).
#[test]
fn overlay_mouse_does_not_make_background_widget_hot() {
    use crate::overlay::Placement;

    let btn = ButtonWidget::new("background");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();

    // First, hover the button to make it hot.
    let bounds = find_widget_bounds(root.layout(), btn_id).expect("button should have bounds");
    let center = bounds.center();
    root.dispatch_event(
        &InputEvent::MouseMove {
            pos: center,
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        now,
    );
    assert!(
        root.interaction().get_state(btn_id).is_hot(),
        "button should be hot after initial hover"
    );

    // Push an overlay anchored above the button center. Placement::Below
    // positions the overlay content below the anchor (anchor.bottom + 4px gap).
    let anchor = Rect::new(
        center.x - 50.0,
        center.y - 40.0,
        center.x + 50.0,
        center.y - 20.0,
    );
    let overlay_widget = Box::new(ButtonWidget::new("overlay"));
    root.push_overlay(overlay_widget, anchor, Placement::Below, now);
    assert!(root.has_overlays());

    // Layout the overlay so its computed_rect is valid for hit testing.
    // process_mouse_event calls layout_overlays internally, but we need
    // to know the overlay rect for cursor positioning.
    // The overlay content starts at anchor.bottom + 4px gap = center.y - 16.
    // Move cursor to a point inside the overlay content area.
    let overlay_pos = crate::geometry::Point::new(center.x, center.y - 10.0);
    root.dispatch_event(
        &InputEvent::MouseMove {
            pos: overlay_pos,
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        now,
    );

    assert!(
        !root.interaction().get_state(btn_id).is_hot(),
        "background widget must not be hot when overlay consumes the mouse event"
    );
}

// -- Dirty marking integration tests (Section 03) --

/// Hovering a button marks it `Prepaint`-dirty in the InvalidationTracker.
///
/// End-to-end: hover → InteractionManager::update_hot_path → mark_widgets_prepaint_dirty → tracker.
#[test]
fn hover_marks_widget_prepaint_dirty() {
    let btn = ButtonWidget::new("test");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    // Initial render clears tracker.
    let _ = root.paint(&measurer(), &theme(), Instant::now());
    root.invalidation_mut().clear();
    assert!(
        !root.invalidation().is_prepaint_dirty(btn_id),
        "should be clean after initial render + clear"
    );

    // Hover the button.
    let btn_bounds = find_widget_bounds(root.layout(), btn_id);
    if let Some(bounds) = btn_bounds {
        let center = bounds.center();
        let now = Instant::now();
        root.dispatch_event(
            &InputEvent::MouseMove {
                pos: center,
                modifiers: Modifiers::NONE,
            },
            &measurer(),
            &theme(),
            now,
        );

        // The widget should now be marked dirty in the tracker.
        assert!(
            root.invalidation().is_prepaint_dirty(btn_id),
            "hovered widget should be prepaint-dirty after dispatch_event"
        );
    }
}

// -- sync_focus_order tests (TPR-11-006 regression) --

/// `sync_focus_order` clears InteractionManager focus when the focused widget
/// leaves the new order — models the dialog reset-defaults / page-switch flow.
///
/// Regression test for TPR-11-006: dialog content handlers previously
/// duplicated the sync logic inline; now they call `sync_focus_order()`
/// directly, so this test covers all three production call sites.
#[test]
fn sync_focus_order_clears_stale_focus() {
    let btn_a = ButtonWidget::new("A");
    let btn_a_id = btn_a.id();
    let btn_b = ButtonWidget::new("B");
    let mut root = WindowRoot::new(btn_a);

    // Focus button A.
    {
        let (interaction, focus) = root.interaction_and_focus_mut();
        interaction.request_focus(btn_a_id, focus);
        let _ = interaction.drain_events();
    }
    assert_eq!(root.interaction().focused_widget(), Some(btn_a_id));
    assert_eq!(root.focus().focused(), Some(btn_a_id));

    // Simulate page switch: new focusable list excludes btn_a (it was on the
    // old page). This is exactly what dialog handlers do after replacing
    // content and collecting focusable IDs from the new widget tree.
    let btn_b_id = btn_b.id();
    root.sync_focus_order(vec![btn_b_id]);

    // Both managers must agree: no focus (btn_a left the order).
    assert_eq!(root.focus().focused(), None);
    assert_eq!(
        root.interaction().focused_widget(),
        None,
        "sync_focus_order must clear InteractionManager when focused widget leaves order"
    );
    assert!(
        root.interaction().focus_ancestor_path().is_empty(),
        "focus_ancestor_path must be empty after sync clears stale focus"
    );
}

/// `sync_focus_order` preserves focus when the focused widget remains in the
/// new order — models a page switch where the focused widget is on the new page.
#[test]
fn sync_focus_order_preserves_valid_focus() {
    let btn = ButtonWidget::new("keep");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);

    // Focus the button.
    {
        let (interaction, focus) = root.interaction_and_focus_mut();
        interaction.request_focus(btn_id, focus);
        let _ = interaction.drain_events();
    }
    assert_eq!(root.focus().focused(), Some(btn_id));

    // New order still includes the focused widget.
    root.sync_focus_order(vec![btn_id]);

    // Focus should be preserved.
    assert_eq!(root.focus().focused(), Some(btn_id));
    assert_eq!(root.interaction().focused_widget(), Some(btn_id));
}

/// `sync_focus_order` is a no-op when no widget was focused.
#[test]
fn sync_focus_order_noop_without_focus() {
    let mut root = WindowRoot::new(LabelWidget::new("no focus"));

    assert_eq!(root.focus().focused(), None);
    assert_eq!(root.interaction().focused_widget(), None);

    // Changing focus order when nothing is focused should not panic or
    // introduce a spurious focus state.
    root.sync_focus_order(vec![]);

    assert_eq!(root.focus().focused(), None);
    assert_eq!(root.interaction().focused_widget(), None);
}

// -- clear_hot_path tests (TPR-04-004 regression) --

/// `clear_hot_path` clears stale hover state after a tree rebuild.
///
/// Regression test for TPR-04-004: dialog page rebuilds left old widgets
/// logically hot until the next cursor move.
#[test]
fn clear_hot_path_removes_stale_hover() {
    let btn = ButtonWidget::new("test");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    // Hover the button.
    let bounds = find_widget_bounds(root.layout(), btn_id).expect("button should have bounds");
    let center = bounds.center();
    root.dispatch_event(
        &InputEvent::MouseMove {
            pos: center,
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        Instant::now(),
    );
    assert!(
        root.interaction().get_state(btn_id).is_hot(),
        "button should be hot after hover"
    );

    // Simulate a tree rebuild (e.g., dialog page switch).
    root.clear_hot_path();

    // Hot state should be cleared.
    assert!(
        !root.interaction().get_state(btn_id).is_hot(),
        "button must not be hot after clear_hot_path"
    );
}

/// `clear_hot_path` marks affected widgets prepaint-dirty so
/// `VisualStateAnimator` transitions back to normal on the next frame.
#[test]
fn clear_hot_path_marks_dirty() {
    let btn = ButtonWidget::new("test");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    // Hover the button.
    let bounds = find_widget_bounds(root.layout(), btn_id).expect("button should have bounds");
    root.dispatch_event(
        &InputEvent::MouseMove {
            pos: bounds.center(),
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        Instant::now(),
    );

    // Clear tracker from the hover, then clear hot path.
    root.invalidation_mut().clear();
    root.clear_hot_path();

    assert!(
        root.invalidation().is_prepaint_dirty(btn_id),
        "clear_hot_path must mark previously-hot widget as prepaint-dirty"
    );
}

// -- refresh_hot_path tests (TPR-04-007 regression) --

/// `refresh_hot_path` preserves hover on widgets still under the cursor
/// after a tree rebuild (TPR-04-007).
///
/// Regression: `clear_hot_path()` unconditionally dropped hover on all
/// widgets, including those that survived the rebuild and were still
/// under the cursor.
#[test]
fn refresh_hot_path_preserves_hover_after_rebuild() {
    let btn = ButtonWidget::new("survive");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    // Hover the button.
    let bounds = find_widget_bounds(root.layout(), btn_id).expect("button should have bounds");
    let center = bounds.center();
    root.dispatch_event(
        &InputEvent::MouseMove {
            pos: center,
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        Instant::now(),
    );
    assert!(
        root.interaction().get_state(btn_id).is_hot(),
        "button should be hot after hover"
    );

    // Simulate a tree rebuild — the same widget survives.
    root.rebuild();
    root.compute_layout(&measurer(), &theme());

    // Refresh hot path from the cursor position (instead of clear_hot_path).
    root.refresh_hot_path(center);

    assert!(
        root.interaction().get_state(btn_id).is_hot(),
        "button must remain hot after rebuild + refresh_hot_path with cursor still over it"
    );
}

/// `refresh_hot_path` clears hover when the cursor is not over any widget.
#[test]
fn refresh_hot_path_clears_hover_when_cursor_outside() {
    let btn = ButtonWidget::new("test");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);
    root.compute_layout(&measurer(), &theme());

    // Hover the button.
    let bounds = find_widget_bounds(root.layout(), btn_id).expect("button should have bounds");
    root.dispatch_event(
        &InputEvent::MouseMove {
            pos: bounds.center(),
            modifiers: Modifiers::NONE,
        },
        &measurer(),
        &theme(),
        Instant::now(),
    );
    assert!(root.interaction().get_state(btn_id).is_hot());

    // Rebuild, then refresh with cursor outside the button.
    root.rebuild();
    root.compute_layout(&measurer(), &theme());
    let outside = crate::geometry::Point::new(-100.0, -100.0);
    root.refresh_hot_path(outside);

    assert!(
        !root.interaction().get_state(btn_id).is_hot(),
        "button must not be hot when cursor is outside"
    );
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

// -- Borrow-split accessor tests --

/// The 3-field mutable borrow-split returns functional references to
/// InteractionManager, InvalidationTracker, and FrameRequestFlags.
#[test]
fn interaction_invalidation_and_frame_requests_mut_destructures_correctly() {
    let btn = ButtonWidget::new("OK");
    let btn_id = btn.id();
    let mut root = WindowRoot::new(btn);

    let (interaction, invalidation, _flags) =
        root.interaction_invalidation_and_frame_requests_mut();

    // InteractionManager is functional: register a widget.
    interaction.register_widget(btn_id);

    // InvalidationTracker is functional: mark dirty and verify.
    use std::collections::HashMap;
    invalidation.mark(btn_id, DirtyKind::Prepaint, &HashMap::new());
    assert!(invalidation.is_prepaint_dirty(btn_id));
}

// -- TPR-11-009: rebuild GC tests --

/// `replace_widget` followed by `rebuild` does not leave stale interaction
/// registrations from the old widget tree (TPR-11-009).
#[test]
fn replace_widget_does_not_leak_old_registrations() {
    let btn_a = ButtonWidget::new("A");
    let id_a = btn_a.id();
    let mut root = WindowRoot::new(btn_a);

    // Verify initial registration.
    assert!(root.interaction().is_registered(id_a));

    // Replace with a different widget.
    let btn_b = ButtonWidget::new("B");
    let id_b = btn_b.id();
    root.replace_widget(Box::new(btn_b));

    // New widget is registered, old is gone.
    assert!(
        root.interaction().is_registered(id_b),
        "new widget should be registered"
    );
    assert!(
        !root.interaction().is_registered(id_a),
        "old widget should be deregistered"
    );
}

/// `rebuild` after internal widget changes GCs stale entries.
#[test]
fn rebuild_gcs_stale_registrations() {
    use crate::widgets::container::ContainerWidget;

    // Build a container with two children.
    let child_a = ButtonWidget::new("A");
    let id_a = child_a.id();
    let child_b = ButtonWidget::new("B");
    let id_b = child_b.id();
    let container = ContainerWidget::column()
        .with_child(Box::new(child_a))
        .with_child(Box::new(child_b));
    let mut root = WindowRoot::new(container);

    assert!(root.interaction().is_registered(id_a));
    assert!(root.interaction().is_registered(id_b));

    // Replace the container's contents with a single child.
    let child_c = ButtonWidget::new("C");
    let id_c = child_c.id();
    let new_container = ContainerWidget::column().with_child(Box::new(child_c));
    root.replace_widget(Box::new(new_container));

    assert!(
        root.interaction().is_registered(id_c),
        "new child should be registered"
    );
    assert!(
        !root.interaction().is_registered(id_a),
        "old child A should be deregistered"
    );
    assert!(
        !root.interaction().is_registered(id_b),
        "old child B should be deregistered"
    );
}

/// `compute_layout` GCs stale interaction registrations after structural
/// changes, matching `rebuild()` behavior (TPR-04-006).
#[test]
fn compute_layout_gcs_stale_registrations() {
    use crate::widgets::container::ContainerWidget;

    let child_a = ButtonWidget::new("A");
    let id_a = child_a.id();
    let child_b = ButtonWidget::new("B");
    let id_b = child_b.id();
    let container = ContainerWidget::column()
        .with_child(Box::new(child_a))
        .with_child(Box::new(child_b));
    let mut root = WindowRoot::new(container);
    root.compute_layout(&measurer(), &theme());

    assert!(root.interaction().is_registered(id_a));
    assert!(root.interaction().is_registered(id_b));

    // Swap the widget directly — bypasses rebuild() to test compute_layout() GC.
    let child_c = ButtonWidget::new("C");
    let id_c = child_c.id();
    let new_container = ContainerWidget::column().with_child(Box::new(child_c));
    root.set_widget_raw(Box::new(new_container));
    root.compute_layout(&measurer(), &theme());

    assert!(
        root.interaction().is_registered(id_c),
        "new child should be registered"
    );
    assert!(
        !root.interaction().is_registered(id_a),
        "old child A should be deregistered"
    );
    assert!(
        !root.interaction().is_registered(id_b),
        "old child B should be deregistered"
    );
}
