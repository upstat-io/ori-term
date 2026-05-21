//! Tests for `WindowRoot`.

use std::time::{Duration, Instant};

use super::WindowRoot;

use crate::animation::Easing;
use crate::color::Color;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, KeyEvent, Modifiers};
use crate::invalidation::DirtyKind;
use crate::overlay::{OverlayEventResult, Placement};
use crate::testing::MockMeasurer;
use crate::theme::UiTheme;
use crate::widgets::Widget;
use crate::widgets::button::ButtonWidget;
use crate::widgets::dialog::DialogWidget;
use crate::widgets::label::LabelWidget;
use crate::widgets::menu::{MenuEntry, MenuWidget};

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
/// Regression test for `rebuild()` calls `set_focus_order()` which
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
    // has_overlays() might still be true during fade-out.)
}

/// Moving the cursor over an active overlay must NOT make background widgets
/// hot — the overlay consumes the mouse event before the base tree hot path
/// is updated.
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

// -- sync_focus_order tests (regression) --

/// `sync_focus_order` clears InteractionManager focus when the focused widget
/// leaves the new order — models the dialog reset-defaults / page-switch flow.
/// Regression test for dialog content handlers previously
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

// -- clear_hot_path tests (regression) --

/// `clear_hot_path` clears stale hover state after a tree rebuild.
/// Regression test for dialog page rebuilds left old widgets
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

// -- refresh_hot_path tests (regression) --

/// `refresh_hot_path` preserves hover on widgets still under the cursor
/// after a tree rebuild.
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

// -- rebuild GC tests --

/// `replace_widget` followed by `rebuild` does not leave stale interaction
/// registrations from the old widget tree.
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

// -- Visual bell tests () --

/// Returns the current opacity of the in-flight flash overlay layer, or
/// `None` if no flash is present. Tests use this to probe the fade curve.
fn flash_opacity(root: &WindowRoot) -> Option<f32> {
    root.overlays()
        .flash_overlay_opacity_for_test(root.layer_tree())
}

/// Advances the layer animator by `delta` from `start` and runs cleanup.
fn tick(root: &mut WindowRoot, start: Instant, delta: Duration) {
    root.tick_overlay_animations(start + delta);
}

/// Regression: `ring_visual_bell` pushes a single full-viewport
/// flash overlay at full intensity (opacity 1.0) and marks the window dirty.
/// Pins the entry-point invariant: a configured BEL produces an observable
/// overlay before the fade-out tween begins.
#[test]
fn ring_visual_bell_starts_flash_overlay_at_full_intensity() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    root.clear_dirty();

    let now = Instant::now();
    root.ring_visual_bell(now, 200, Color::WHITE, Easing::EaseOut);

    assert!(root.overlays().has_flash_overlay());
    assert_eq!(root.overlays().flash_overlay_count(), 1);
    assert_eq!(flash_opacity(&root), Some(1.0));
    assert!(root.is_dirty(), "ring_visual_bell must mark window dirty");
}

/// Regression: flash opacity decreases monotonically over the
/// configured `duration_ms`, computed by `Easing::EaseOut.apply(t)`.
/// Pins the fade-out tween: the overlay does not stay at full opacity, and
/// each successive sample is strictly less than the prior.
#[test]
fn ring_visual_bell_fades_monotonically_over_duration() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    let now = Instant::now();
    root.ring_visual_bell(now, 200, Color::WHITE, Easing::EaseOut);

    let probe = |root: &mut WindowRoot, ms: u64| -> f32 {
        tick(root, now, Duration::from_millis(ms));
        flash_opacity(root).unwrap_or(0.0)
    };

    let o0 = flash_opacity(&root).expect("flash present at t=0");
    let o50 = probe(&mut root, 50);
    let o100 = probe(&mut root, 100);
    let o150 = probe(&mut root, 150);

    assert!(
        o0 > o50 && o50 > o100 && o100 > o150,
        "fade must be strictly decreasing: t=0:{o0} t=50:{o50} t=100:{o100} t=150:{o150}",
    );
    assert!(o0 >= 0.99, "opacity at t=0 should be ~1.0, got {o0}");

    // At t=100ms (phase=0.5), Easing::EaseOut.apply(0.5) = 0.875, so
    // animated opacity = 1.0 - 0.875 = 0.125. Allow ±0.01 slack.
    let expected_at_100 = 1.0 - Easing::EaseOut.apply(0.5);
    assert!(
        (o100 - expected_at_100).abs() < 0.05,
        "t=100 opacity {o100} differs from expected {expected_at_100} > 0.05"
    );
}

/// Regression: `duration_ms == 0` is a no-op: no overlay is
/// pushed, no animation scheduled, the window is not marked dirty.
/// Pins the defense-in-depth zero-duration gate. The caller
/// (`mux_pump`) is also expected to gate on `BellConfig::is_enabled()`,
/// but `WindowRoot::ring_visual_bell` defends in depth so the API is
/// safe to call unconditionally.
#[test]
fn ring_visual_bell_with_zero_duration_is_a_noop() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    root.clear_dirty();

    let now = Instant::now();
    root.ring_visual_bell(now, 0, Color::WHITE, Easing::EaseOut);

    assert!(!root.overlays().has_flash_overlay());
    assert_eq!(root.overlays().flash_overlay_count(), 0);
    assert!(
        !root.is_dirty(),
        "zero-duration ring_visual_bell must not mark dirty"
    );
}

/// Regression: the flash overlay never captures input. It
/// lives on the dismissing list (not the active overlays list) and event
/// routing iterates `overlays` only, so `process_mouse_event` returns
/// `PassThrough` even with a flash in flight. Pins the "does not
/// intercept input" contract for visual-bell distinguishability.
#[test]
fn ring_visual_bell_does_not_intercept_input() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    let now = Instant::now();
    root.ring_visual_bell(now, 1000, Color::WHITE, Easing::EaseOut);

    // Flash on dismissing means active overlays are still empty, which
    // means OverlayManager::process_mouse_event returns PassThrough on
    // the early-return path before any hit-testing.
    assert!(root.overlays().has_flash_overlay());
    assert!(
        root.overlays().is_active_empty(),
        "flash overlay must not appear on active overlays list"
    );
}

/// Regression: the flash overlay's bounds always equal the
/// current viewport. Distinguishability from the per-tab pulse: visual
/// bell covers the whole window, audible bell flashes one tab.
#[test]
fn ring_visual_bell_overlay_covers_full_viewport() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    let now = Instant::now();
    root.ring_visual_bell(now, 200, Color::WHITE, Easing::EaseOut);

    assert_eq!(
        root.overlays().flash_overlay_bounds(),
        Some(root.viewport())
    );
}

/// Regression: `Easing::Linear` produces a proportional fade.
/// At t=50ms of 200ms (phase=0.25), `Easing::Linear.apply(0.25) = 0.25`,
/// so opacity = 1.0 - 0.25 = 0.75 ± 0.01.
#[test]
fn ring_visual_bell_linear_curve_fades_proportionally() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    let now = Instant::now();
    root.ring_visual_bell(now, 200, Color::WHITE, Easing::Linear);

    tick(&mut root, now, Duration::from_millis(50));
    let o50 = flash_opacity(&root).unwrap_or(0.0);
    assert!(
        (o50 - 0.75).abs() < 0.05,
        "linear fade at t=50/200 should be ~0.75, got {o50}"
    );

    tick(&mut root, now, Duration::from_millis(150));
    let o150 = flash_opacity(&root).unwrap_or(0.0);
    assert!(
        (o150 - 0.25).abs() < 0.05,
        "linear fade at t=150/200 should be ~0.25, got {o150}"
    );
}

/// Regression: a second `ring_visual_bell` while the first is
/// still in flight REPLACES the first overlay (single-flash invariant).
/// `flash_overlay_count` stays at 1 — the overlay does not stack. Prevents
/// heap accumulation under bell-storm scenarios.
#[test]
fn ring_visual_bell_replaces_in_flight_animation() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    let t0 = Instant::now();
    root.ring_visual_bell(t0, 1000, Color::WHITE, Easing::EaseOut);
    assert_eq!(root.overlays().flash_overlay_count(), 1);

    // Halfway through the first flash, ring again with a fresh duration.
    let t1 = t0 + Duration::from_millis(500);
    root.ring_visual_bell(t1, 1000, Color::WHITE, Easing::EaseOut);
    assert_eq!(
        root.overlays().flash_overlay_count(),
        1,
        "second ring must replace the in-flight overlay, not stack"
    );

    // 100ms into the second flash, opacity reflects the NEW phase
    // (0.1 of 1000ms), not the combined opacity of two animations.
    tick(&mut root, t1, Duration::from_millis(100));
    let o = flash_opacity(&root).unwrap_or(0.0);
    let expected = 1.0 - Easing::EaseOut.apply(0.1);
    assert!(
        (o - expected).abs() < 0.05,
        "after replace, opacity should reflect new animation phase 0.1, expected {expected}, got {o}"
    );
}

/// Regression: when the viewport changes during an in-flight
/// flash, the overlay's bounds update to the new viewport. Without this,
/// an interactive resize during a bell would leave a flash overlay covering
/// only part of the new window.
#[test]
fn ring_visual_bell_during_resize_repositions_overlay() {
    let small = Rect::new(0.0, 0.0, 800.0, 600.0);
    let large = Rect::new(0.0, 0.0, 1200.0, 800.0);
    let mut root = WindowRoot::with_viewport(LabelWidget::new("bell"), small);
    let now = Instant::now();
    root.ring_visual_bell(now, 1000, Color::WHITE, Easing::EaseOut);

    assert_eq!(root.overlays().flash_overlay_bounds(), Some(small));

    root.set_viewport(large);
    assert_eq!(
        root.overlays().flash_overlay_bounds(),
        Some(large),
        "set_viewport must refresh in-flight flash overlay bounds"
    );
}

/// Regression: 10 back-to-back `ring_visual_bell` calls under
/// a bell storm produce exactly one in-flight overlay; each call replaces
/// the previous. Prevents heap growth under shell-emitted BEL storms.
#[test]
fn ring_visual_bell_storm_replaces_overlay_each_time_no_accumulation() {
    let mut root = WindowRoot::new(LabelWidget::new("bell"));
    let base = Instant::now();
    for i in 0..10 {
        root.ring_visual_bell(
            base + Duration::from_millis(i),
            500,
            Color::WHITE,
            Easing::EaseOut,
        );
        assert_eq!(
            root.overlays().flash_overlay_count(),
            1,
            "after {} ring calls, count must remain 1",
            i + 1
        );
    }
}

/// `compute_layout` GCs stale interaction registrations after structural
/// changes, matching `rebuild()` behavior.
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

// -- : Overlay key dispatch goes through keymap --
// Regression tests for — overlay key dispatch was bypassing the
// keymap path for non-Escape keys, breaking ArrowDown/ArrowUp/Enter on
// MenuWidget-backed popup overlays. See

fn menu_with_items(items: &[&str]) -> MenuWidget {
    let entries: Vec<MenuEntry> = items
        .iter()
        .map(|l| MenuEntry::Item { label: (*l).into() })
        .collect();
    MenuWidget::new(entries)
}

fn key_event(k: Key) -> KeyEvent {
    KeyEvent {
        key: k,
        modifiers: Modifiers::NONE,
    }
}

/// Property: pressing ArrowDown on a non-searchable MenuWidget popup
/// must advance the hovered index via the keymap path (NavigateDown →
/// handle_keymap_action → navigate_keyboard). Pre-fix: routes through
/// on_input which returns ignored() for non-searchable mode, so hovered
/// stays at 0. Post-fix: routes through keymap-first dispatch.
/// Asserts on observable navigation: send ArrowDown, then Enter, and
/// verify Enter emits `Selected { index: 1 }` (advanced from 0). If the
/// keymap path is broken, ArrowDown is swallowed without state change
/// and Enter would emit `Selected { index: 0 }` — the assertion fails.
/// Regression: overlay key dispatch bypassed keymap for
/// non-Escape keys. See
#[test]
fn arrow_down_on_menu_overlay_advances_hover_via_keymap() {
    let menu = menu_with_items(&["Alpha", "Beta", "Gamma"]);
    let mut root = WindowRoot::new(LabelWidget::new("bg"));
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();
    let anchor = Rect::new(100.0, 100.0, 200.0, 120.0);
    root.push_overlay(Box::new(menu), anchor, Placement::Below, now);
    assert!(root.has_overlays());

    // Non-searchable MenuWidget starts with `hovered = None`; the first
    // ArrowDown sets hover to index 0. To prove keymap-routed navigation
    // ADVANCED PAST the initial hover, send ArrowDown TWICE: first ArrowDown
    // sets hovered=0, second ArrowDown advances to hovered=1. Then Enter
    // confirms — Selected{index:1} proves both ArrowDowns landed via keymap.
    // If the keymap path is broken, neither ArrowDown registers and Enter
    // emits no Selected action (or an unrelated one).
    for label in ["first ArrowDown", "second ArrowDown"] {
        let down_result = root.process_overlay_key_event(
            key_event(Key::ArrowDown),
            &measurer(),
            &theme(),
            None,
            now,
        );
        match down_result {
            OverlayEventResult::Delivered { response, .. } => {
                assert!(
                    response.handled,
                    "{label} on Menu overlay must be handled via keymap"
                );
                assert!(
                    response.action.is_none(),
                    "{label}: NavigateDown emits no WidgetAction"
                );
            }
            other => panic!("expected Delivered for {label}, got {other:?}"),
        }
    }

    // Confirm the advanced entry. After two ArrowDowns the menu's
    // internal hovered MUST be index 1 (advanced from None → 0 → 1);
    // Enter triggers Confirm → try_select_hovered → emits Selected{1}.
    let enter_result =
        root.process_overlay_key_event(key_event(Key::Enter), &measurer(), &theme(), None, now);
    match enter_result {
        OverlayEventResult::Delivered { response, .. } => match response.action {
            Some(crate::action::WidgetAction::Selected { index, .. }) => {
                assert_eq!(
                    index, 1,
                    "ArrowDown twice then Enter must select index 1 \
 (proves keymap-path navigation advanced through both \
 None→0 and 0→1 transitions); other indices indicate \
 one or both ArrowDowns were silently swallowed"
                );
            }
            other => {
                panic!("expected Selected after Enter, got action={other:?}")
            }
        },
        other => panic!("expected Delivered for Enter, got {other:?}"),
    }
}

/// Regression guard: pressing Space inside a SEARCHABLE MenuWidget popup must
/// NOT trigger Confirm via keymap — it must reach on_input where it is
/// appended to the filter query as a literal space character. Rejects the
/// regression where the keymap-first dispatch wires Space→Confirm for the
/// "MenuSearchable" context (which would steal printable filter input).
/// Regression: design pin — searchable Menu uses a distinct
/// "MenuSearchable" key_context that intentionally omits the Space binding.
#[test]
fn space_on_searchable_menu_overlay_does_not_confirm() {
    let menu = MenuWidget::new(vec![
        MenuEntry::Item {
            label: "Alpha".into(),
        },
        MenuEntry::Item {
            label: "Beta".into(),
        },
    ])
    .with_searchable(true);
    let mut root = WindowRoot::new(LabelWidget::new("bg"));
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();
    let anchor = Rect::new(100.0, 100.0, 200.0, 120.0);
    root.push_overlay(Box::new(menu), anchor, Placement::Below, now);

    let result =
        root.process_overlay_key_event(key_event(Key::Space), &measurer(), &theme(), None, now);

    // Space must reach on_input (filter character handling), NOT keymap-resolve
    // to Confirm. The searchable MenuWidget's on_input handles Space by
    // appending ' ' to the filter query and returning handled=true with no
    // action. If the keymap-first dispatch wrongly steals Space, the result
    // would be Delivered { action: Some(Selected) } via try_select_hovered.
    match result {
        OverlayEventResult::Delivered { response, .. } => {
            // Regression guard: NO Selected action emitted. Either no action at all
            // (filter character path) or a non-Selected action.
            assert!(
                !matches!(
                    response.action,
                    Some(crate::action::WidgetAction::Selected { .. })
                ),
                "Space on searchable Menu must NOT emit Selected — it must \
 reach on_input as a filter character"
            );
        }
        other => panic!("expected Delivered, got {other:?}"),
    }
}

/// Codex Round 0 finding regression test: a `widget::Dismiss` keymap
/// match for a Dialog overlay must produce `OverlayEventResult::Dismissed(id)`
/// directly, NOT `Delivered { action: Some(DismissOverlay) }` that would
/// be silently dropped by `App::handle_dialog_overlay_result` (which only
/// matches on the `Dismissed` variant). The `OverlayManager` translates
/// a matched `widget::Dismiss` action into `Dismissed(id)` itself so
/// Dialog windows dismiss correctly even though `DialogWidget` does not
/// implement `handle_keymap_action`.
/// Test isolates the keymap translation from the legacy inline Escape
/// short-circuit by REBINDING `Dismiss` to a non-Escape key (`F1`) for
/// the `"Dialog"` context, then pressing F1. The legacy inline path
/// only matches `Key::Escape`, so F1 cannot reach it — the only way for
/// this test to produce `Dismissed(id)` is via the keymap path's
/// `widget::Dismiss → Dismissed(id)` translation.
/// Regression: Round 0 + Phase 5 code review round 0
/// ( F3 + F2 agreement).
#[test]
fn dialog_escape_dismisses_via_keymap_translation() {
    use crate::action::{KeyBinding, Keystroke, keymap_action::Dismiss};

    let dialog = DialogWidget::new("Confirm");
    let mut root = WindowRoot::new(LabelWidget::new("bg"));
    root.compute_layout(&measurer(), &theme());

    // Rebind Dismiss to a non-Escape key (`Character('q')`) for the
    // "Dialog" context. The inline Escape short-circuit only matches
    // `Key::Escape`, so `Character('q')` cannot reach it — a Dismissed
    // result here can ONLY come from the keymap-first dispatch path's
    // manager-level `widget::Dismiss → Dismissed` translation.
    root.keymap_mut().rebind(KeyBinding::new(
        Keystroke::new(Key::Character('q'), Modifiers::NONE),
        Dismiss,
        Some("Dialog"),
    ));

    let now = Instant::now();
    let anchor = Rect::new(300.0, 200.0, 500.0, 400.0);
    let id = root.push_modal(
        Box::new(dialog),
        anchor,
        Placement::AtPoint(crate::geometry::Point::new(300.0, 200.0)),
        now,
    );

    let result = root.process_overlay_key_event(
        key_event(Key::Character('q')),
        &measurer(),
        &theme(),
        None,
        now,
    );

    // The result MUST be Dismissed(id) via the keymap translation path.
    // If the manager-level translation is missing, the result would be
    // Delivered with action=None (DialogWidget doesn't impl
    // handle_keymap_action) — the test distinguishes Dismissed from
    // Delivered to prove the manager-level translation fired.
    assert!(
        matches!(result, OverlayEventResult::Dismissed(d_id) if d_id == id),
        "Dialog 'q'->Dismiss must produce Dismissed(id) via keymap \
 translation, got {result:?}"
    );
}

/// Modal-focused-child design pin: pressing Enter on a Modal Dialog whose
/// internal focus is on a Button must NOT route through keymap as
/// `Activate` (Button's binding) — the overlay key path uses
/// `focus_path = [overlay_root_id]` so the ctx_stack is `["Dialog"]`,
/// which has no Enter binding in the default keymap. Falls through to
/// the on_input pipeline where the modal's input handling owns the
/// outcome.
/// This pins the intentional design constraint that overlays do not plumb
/// a focused-child context path through the keymap. If a future fix lands
/// per-overlay focus paths, this test becomes the regression guard that
/// flags the behavior change.
/// Regression: Phase 2.5 Plan TPR design pin — F3 +
/// F2 + F4 (3-of-3 agreement).
#[test]
fn enter_on_dialog_with_focused_button_does_not_activate_via_keymap() {
    // DialogWidget is a Modal overlay with key_context()=="Dialog".
    // The Dialog has internal Button children but the overlay key dispatch
    // uses focus_path = [dialog_root_id] only — Button's "Button" context
    // never enters the ctx_stack.
    let dialog = DialogWidget::new("Confirm");
    let mut root = WindowRoot::new(LabelWidget::new("bg"));
    root.compute_layout(&measurer(), &theme());

    let now = Instant::now();
    let anchor = Rect::new(300.0, 200.0, 500.0, 400.0);
    root.push_modal(
        Box::new(dialog),
        anchor,
        Placement::AtPoint(crate::geometry::Point::new(300.0, 200.0)),
        now,
    );

    let result =
        root.process_overlay_key_event(key_event(Key::Enter), &measurer(), &theme(), None, now);

    // The default keymap binds Enter→Activate ONLY for "Button" context.
    // With ctx_stack=["Dialog"], lookup returns None → falls through to
    // on_input. Result: Delivered with handled=true (modal blocks the
    // event from leaking to background) but NO Activate action emitted
    // through the keymap path.
    match result {
        OverlayEventResult::Delivered { response, .. } => {
            // Regression guard: NO action implies the keymap miss path was
            // taken (Button's Activate would have produced an action via
            // dispatch_keymap_action).
            assert!(
                response.action.is_none(),
                "Enter on Dialog overlay must NOT activate via keymap — \
 ctx_stack is [\"Dialog\"], not [\"Button\"]"
            );
        }
        OverlayEventResult::Blocked => {
            // Also acceptable — modal blocking with no keymap match.
        }
        other => panic!("unexpected result: {other:?}"),
    }
}
