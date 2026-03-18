//! Tests for widget-level hit testing and input routing.

use crate::geometry::{Point, Rect};
use crate::hit_test_behavior::HitTestBehavior;
use crate::interaction::InteractionManager;
use crate::interaction::lifecycle::LifecycleEvent;
use crate::layout::LayoutNode;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::dispatch::{DeliveryAction, plan_propagation};
use super::event::{EventPhase, InputEvent, Modifiers, MouseButton, ScrollDelta};
use super::hit_test::{layout_hit_test, layout_hit_test_clipped, layout_hit_test_path};

// ── Helpers ──────────────────────────────────────────────────────────

fn make_node(x: f32, y: f32, w: f32, h: f32, id: Option<WidgetId>) -> LayoutNode {
    let rect = Rect::new(x, y, w, h);
    LayoutNode {
        rect,
        content_rect: rect,
        children: Vec::new(),
        widget_id: id,
        sense: Sense::all(),
        hit_test_behavior: HitTestBehavior::default(),
        clip: false,
        disabled: false,
        interact_radius: 0.0,
        content_offset: (0.0, 0.0),
    }
}

fn two_widget_tree() -> (LayoutNode, WidgetId, WidgetId) {
    let a = WidgetId::next();
    let b = WidgetId::next();
    let child_a = make_node(0.0, 0.0, 50.0, 100.0, Some(a));
    let child_b = make_node(50.0, 0.0, 50.0, 100.0, Some(b));
    let mut root = make_node(0.0, 0.0, 100.0, 100.0, None);
    root.children.push(child_a);
    root.children.push(child_b);
    (root, a, b)
}

fn mouse_move_input(x: f32, y: f32) -> InputEvent {
    InputEvent::MouseMove {
        pos: Point::new(x, y),
        modifiers: Modifiers::NONE,
    }
}

fn mouse_down_input(x: f32, y: f32, button: MouseButton) -> InputEvent {
    InputEvent::MouseDown {
        pos: Point::new(x, y),
        button,
        modifiers: Modifiers::NONE,
    }
}

fn scroll_input(x: f32, y: f32, dy: f32) -> InputEvent {
    InputEvent::Scroll {
        pos: Point::new(x, y),
        delta: ScrollDelta::Lines { x: 0.0, y: dy },
        modifiers: Modifiers::NONE,
    }
}

/// Simulates a mouse event: hit test → update hot path → plan propagation.
///
/// Returns the delivery actions and any pending lifecycle events.
fn simulate_mouse(
    event: &InputEvent,
    layout: &LayoutNode,
    mgr: &mut InteractionManager,
    actions: &mut Vec<DeliveryAction>,
) -> Vec<LifecycleEvent> {
    let pos = event.pos().expect("mouse event has pos");
    let hit = layout_hit_test_path(layout, pos);
    mgr.update_hot_path(&hit.widget_ids());
    let events = mgr.drain_events();
    plan_propagation(event, &hit, mgr.active_widget(), &[], actions);
    events
}

fn delivers_to(actions: &[DeliveryAction], target: WidgetId) -> bool {
    actions.iter().any(|a| a.widget_id == target)
}

fn hot_changed_events(events: &[LifecycleEvent]) -> Vec<(WidgetId, bool)> {
    events
        .iter()
        .filter_map(|e| match e {
            LifecycleEvent::HotChanged { widget_id, is_hot } => Some((*widget_id, *is_hot)),
            _ => None,
        })
        .collect()
}

// ── Hit Testing ──────────────────────────────────────────────────────

#[test]
fn hit_test_single_leaf() {
    let id = WidgetId::next();
    let root = make_node(0.0, 0.0, 100.0, 50.0, Some(id));

    assert_eq!(layout_hit_test(&root, Point::new(50.0, 25.0)), Some(id));
    assert_eq!(layout_hit_test(&root, Point::new(0.0, 0.0)), Some(id));
    // Half-open: right/bottom edge is outside.
    assert_eq!(layout_hit_test(&root, Point::new(100.0, 25.0)), None);
    assert_eq!(layout_hit_test(&root, Point::new(50.0, 50.0)), None);
}

#[test]
fn hit_test_miss_returns_none() {
    let id = WidgetId::next();
    let root = make_node(10.0, 10.0, 50.0, 50.0, Some(id));

    assert_eq!(layout_hit_test(&root, Point::new(0.0, 0.0)), None);
    assert_eq!(layout_hit_test(&root, Point::new(5.0, 30.0)), None);
}

#[test]
fn hit_test_no_widget_id() {
    let root = make_node(0.0, 0.0, 100.0, 100.0, None);
    assert_eq!(layout_hit_test(&root, Point::new(50.0, 50.0)), None);
}

#[test]
fn hit_test_child_takes_priority() {
    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();

    let child = make_node(20.0, 20.0, 30.0, 30.0, Some(child_id));
    let mut parent = make_node(0.0, 0.0, 100.0, 100.0, Some(parent_id));
    parent.children.push(child);

    // Point inside child → child wins.
    assert_eq!(
        layout_hit_test(&parent, Point::new(35.0, 35.0)),
        Some(child_id)
    );
    // Point outside child but inside parent → parent wins.
    assert_eq!(
        layout_hit_test(&parent, Point::new(5.0, 5.0)),
        Some(parent_id)
    );
}

#[test]
fn hit_test_last_child_is_frontmost() {
    let parent_id = WidgetId::next();
    let back_id = WidgetId::next();
    let front_id = WidgetId::next();

    // Two overlapping children at the same position.
    let back = make_node(10.0, 10.0, 40.0, 40.0, Some(back_id));
    let front = make_node(10.0, 10.0, 40.0, 40.0, Some(front_id));

    let mut parent = make_node(0.0, 0.0, 100.0, 100.0, Some(parent_id));
    parent.children.push(back);
    parent.children.push(front);

    // Last child (front) wins.
    assert_eq!(
        layout_hit_test(&parent, Point::new(25.0, 25.0)),
        Some(front_id)
    );
}

#[test]
fn hit_test_deeply_nested() {
    let root_id = WidgetId::next();
    let mid_id = WidgetId::next();
    let leaf_id = WidgetId::next();

    let leaf = make_node(30.0, 30.0, 10.0, 10.0, Some(leaf_id));
    let mut mid = make_node(20.0, 20.0, 40.0, 40.0, Some(mid_id));
    mid.children.push(leaf);
    let mut root = make_node(0.0, 0.0, 100.0, 100.0, Some(root_id));
    root.children.push(mid);

    // Deepest node wins.
    assert_eq!(
        layout_hit_test(&root, Point::new(35.0, 35.0)),
        Some(leaf_id)
    );
    // Between mid and leaf → mid.
    assert_eq!(layout_hit_test(&root, Point::new(22.0, 22.0)), Some(mid_id));
    // Outside mid → root.
    assert_eq!(layout_hit_test(&root, Point::new(5.0, 5.0)), Some(root_id));
}

#[test]
fn hit_test_child_without_id_falls_through_to_parent() {
    let parent_id = WidgetId::next();
    // Child has no widget_id.
    let child = make_node(20.0, 20.0, 30.0, 30.0, None);
    let mut parent = make_node(0.0, 0.0, 100.0, 100.0, Some(parent_id));
    parent.children.push(child);

    // Point in child area falls through to parent.
    assert_eq!(
        layout_hit_test(&parent, Point::new(35.0, 35.0)),
        Some(parent_id)
    );
}

#[test]
fn hit_test_clipped_excludes_outside_clip() {
    let id = WidgetId::next();
    let root = make_node(0.0, 0.0, 200.0, 200.0, Some(id));
    let clip = Rect::new(50.0, 50.0, 100.0, 100.0);

    // Inside both rect and clip.
    assert_eq!(
        layout_hit_test_clipped(&root, Point::new(75.0, 75.0), Some(clip)),
        Some(id)
    );
    // Inside rect but outside clip.
    assert_eq!(
        layout_hit_test_clipped(&root, Point::new(10.0, 10.0), Some(clip)),
        None
    );
    // No clip → normal hit test.
    assert_eq!(
        layout_hit_test_clipped(&root, Point::new(10.0, 10.0), None),
        Some(id)
    );
}

// ── Integrated Routing (InteractionManager + plan_propagation) ───────

#[test]
fn routing_hover_enter_leave() {
    let (root, a, b) = two_widget_tree();
    let mut mgr = InteractionManager::new();
    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.drain_events(); // Clear registration events.

    let mut actions = Vec::new();

    // Move into widget A → HotChanged(a, true).
    let events = simulate_mouse(&mouse_move_input(25.0, 50.0), &root, &mut mgr, &mut actions);
    let hot = hot_changed_events(&events);
    assert!(hot.contains(&(a, true)), "Enter A");
    assert!(mgr.get_state(a).is_hot());

    // Move into widget B → HotChanged(a, false) + HotChanged(b, true).
    let events = simulate_mouse(&mouse_move_input(75.0, 50.0), &root, &mut mgr, &mut actions);
    let hot = hot_changed_events(&events);
    assert!(hot.contains(&(a, false)), "Leave A");
    assert!(hot.contains(&(b, true)), "Enter B");
    assert!(mgr.get_state(b).is_hot());
    assert!(!mgr.get_state(a).is_hot());
}

#[test]
fn routing_mouse_capture_on_down() {
    let id = WidgetId::next();
    let root = make_node(0.0, 0.0, 100.0, 100.0, Some(id));
    let mut mgr = InteractionManager::new();
    mgr.register_widget(id);
    mgr.drain_events();
    let mut actions = Vec::new();

    // Mouse down → set_active.
    simulate_mouse(
        &mouse_down_input(50.0, 50.0, MouseButton::Left),
        &root,
        &mut mgr,
        &mut actions,
    );
    mgr.set_active(id);
    assert_eq!(mgr.active_widget(), Some(id));

    // Mouse up → clear_active.
    mgr.clear_active();
    assert_eq!(mgr.active_widget(), None);
}

#[test]
fn routing_captured_widget_receives_events_outside_bounds() {
    let (root, a, _b) = two_widget_tree();
    let mut mgr = InteractionManager::new();
    mgr.register_widget(a);
    mgr.drain_events();

    // Capture A.
    mgr.set_active(a);
    let mut actions = Vec::new();

    // Move cursor over B while captured → event delivered to A.
    let event = mouse_move_input(75.0, 50.0);
    let hit = layout_hit_test_path(&root, Point::new(75.0, 50.0));
    plan_propagation(&event, &hit, mgr.active_widget(), &[], &mut actions);

    assert_eq!(actions.len(), 1);
    assert_eq!(actions[0].widget_id, a);
    assert_eq!(actions[0].phase, EventPhase::Target);
}

#[test]
fn routing_cursor_left_clears_hot() {
    let id = WidgetId::next();
    let root = make_node(0.0, 0.0, 100.0, 100.0, Some(id));
    let mut mgr = InteractionManager::new();
    mgr.register_widget(id);
    mgr.drain_events();
    let mut actions = Vec::new();

    // Enter.
    simulate_mouse(&mouse_move_input(50.0, 50.0), &root, &mut mgr, &mut actions);
    assert!(mgr.get_state(id).is_hot());

    // Cursor leaves window → update_hot_path(&[]) clears all hot.
    mgr.update_hot_path(&[]);
    let events = mgr.drain_events();
    let hot = hot_changed_events(&events);
    assert!(
        hot.contains(&(id, false)),
        "HotChanged(false) on cursor leave"
    );
    assert!(!mgr.get_state(id).is_hot());
}

#[test]
fn routing_no_actions_on_empty_tree() {
    let root = make_node(0.0, 0.0, 100.0, 100.0, None);
    let event = mouse_move_input(50.0, 50.0);
    let hit = layout_hit_test_path(&root, Point::new(50.0, 50.0));
    let mut actions = Vec::new();
    plan_propagation(&event, &hit, None, &[], &mut actions);
    assert!(actions.is_empty());
}

#[test]
fn routing_move_within_same_widget_no_hover_events() {
    let id = WidgetId::next();
    let root = make_node(0.0, 0.0, 100.0, 100.0, Some(id));
    let mut mgr = InteractionManager::new();
    mgr.register_widget(id);
    mgr.drain_events();
    let mut actions = Vec::new();

    // First move → HotChanged(true).
    let events = simulate_mouse(&mouse_move_input(25.0, 25.0), &root, &mut mgr, &mut actions);
    assert_eq!(hot_changed_events(&events).len(), 1);

    // Second move within same widget → no HotChanged events.
    let events = simulate_mouse(&mouse_move_input(75.0, 75.0), &root, &mut mgr, &mut actions);
    assert!(
        hot_changed_events(&events).is_empty(),
        "no hot change within same widget"
    );
}

#[test]
fn routing_active_capture_overrides_hit() {
    let (root, a, b) = two_widget_tree();
    let mut mgr = InteractionManager::new();
    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.drain_events();

    mgr.set_active(a);
    let mut actions = Vec::new();

    // Move over B with A captured → event delivered to A.
    let event = mouse_move_input(75.0, 50.0);
    let hit = layout_hit_test_path(&root, Point::new(75.0, 50.0));
    plan_propagation(&event, &hit, mgr.active_widget(), &[], &mut actions);

    assert!(delivers_to(&actions, a), "captured widget receives event");
    assert!(!delivers_to(&actions, b));

    mgr.clear_active();
    assert_eq!(mgr.active_widget(), None);
}

#[test]
fn routing_mouse_down_outside_all_widgets() {
    let id = WidgetId::next();
    let root = make_node(10.0, 10.0, 50.0, 50.0, Some(id));
    let event = mouse_down_input(5.0, 5.0, MouseButton::Left);
    let hit = layout_hit_test_path(&root, Point::new(5.0, 5.0));
    let mut actions = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut actions);
    assert!(actions.is_empty());
}

// ── Hover during capture (new behavior) ──────────────────────────────
// The new system intentionally allows hot tracking during capture.
// This enables drag-and-drop visual feedback on drop targets.

#[test]
fn routing_hover_changes_during_capture() {
    // NEW BEHAVIOR: hover transitions continue during capture.
    let (root, a, b) = two_widget_tree();
    let mut mgr = InteractionManager::new();
    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.drain_events();
    let mut actions = Vec::new();

    // Hover A, capture A.
    simulate_mouse(&mouse_move_input(25.0, 50.0), &root, &mut mgr, &mut actions);
    mgr.set_active(a);
    mgr.drain_events(); // Clear registration + hot events.

    // Move over B while captured → hot path updates to B.
    let events = simulate_mouse(&mouse_move_input(75.0, 50.0), &root, &mut mgr, &mut actions);
    let hot = hot_changed_events(&events);
    assert!(
        hot.contains(&(a, false)),
        "A loses hot during capture (new behavior)"
    );
    assert!(
        hot.contains(&(b, true)),
        "B gains hot during capture (new behavior)"
    );
    // But the delivery still goes to captured widget A.
    assert!(delivers_to(&actions, a));
}

#[test]
fn routing_capture_release_hover_already_current() {
    // Since hover updates during capture, on release the hover is
    // already at the correct widget — no deferred transition needed.
    let (root, a, b) = two_widget_tree();
    let mut mgr = InteractionManager::new();
    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.drain_events();
    let mut actions = Vec::new();

    // Hover A, capture A.
    simulate_mouse(&mouse_move_input(25.0, 50.0), &root, &mut mgr, &mut actions);
    mgr.set_active(a);
    mgr.drain_events();

    // Drag to B — hot already updates.
    simulate_mouse(&mouse_move_input(75.0, 50.0), &root, &mut mgr, &mut actions);
    mgr.drain_events();
    assert!(mgr.get_state(b).is_hot());
    assert!(!mgr.get_state(a).is_hot());

    // Release over B.
    mgr.clear_active();
    let events = mgr.drain_events();
    // Only ActiveChanged events, no HotChanged (hot is already correct).
    assert!(
        hot_changed_events(&events).is_empty(),
        "no deferred hot transition on release"
    );
    assert!(mgr.get_state(b).is_hot());
}

#[test]
fn routing_captured_move_outside_all_bounds() {
    let id = WidgetId::next();
    let root = make_node(10.0, 10.0, 50.0, 50.0, Some(id));
    let mut mgr = InteractionManager::new();
    mgr.register_widget(id);
    mgr.drain_events();
    let mut actions = Vec::new();

    // Hover, capture.
    simulate_mouse(&mouse_move_input(25.0, 25.0), &root, &mut mgr, &mut actions);
    mgr.set_active(id);
    mgr.drain_events();

    // Move outside all bounds → delivery still goes to captured widget.
    let event = mouse_move_input(200.0, 200.0);
    let hit = layout_hit_test_path(&root, Point::new(200.0, 200.0));
    mgr.update_hot_path(&hit.widget_ids());
    let events = mgr.drain_events();
    plan_propagation(&event, &hit, mgr.active_widget(), &[], &mut actions);

    assert!(delivers_to(&actions, id), "captured widget receives event");
    // Hot changes: widget loses hot (pointer outside its bounds).
    let hot = hot_changed_events(&events);
    assert!(
        hot.contains(&(id, false)),
        "hot cleared when pointer leaves"
    );
}

// ── Scroll routing ───────────────────────────────────────────────────

#[test]
fn routing_scroll_routes_to_hit_target() {
    let (root, a, _b) = two_widget_tree();
    let event = scroll_input(25.0, 50.0, -3.0);
    let hit = layout_hit_test_path(&root, Point::new(25.0, 50.0));
    let mut actions = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut actions);
    assert!(delivers_to(&actions, a), "scroll delivered to hit widget");
}

#[test]
fn routing_scroll_uses_hit_path_during_capture() {
    // Scroll always uses normal hit testing, even during capture.
    let (root, a, b) = two_widget_tree();
    let mut mgr = InteractionManager::new();
    mgr.register_widget(a);
    mgr.register_widget(b);
    mgr.set_active(a);

    // Scroll over B while A is captured → scroll goes to B's hit path.
    let event = scroll_input(75.0, 50.0, -3.0);
    let hit = layout_hit_test_path(&root, Point::new(75.0, 50.0));
    let mut actions = Vec::new();
    plan_propagation(&event, &hit, mgr.active_widget(), &[], &mut actions);

    assert!(
        delivers_to(&actions, b),
        "scroll uses hit path, not capture"
    );
    assert!(
        !delivers_to(&actions, a),
        "scroll does not go to captured widget"
    );
}

// ── Rapid sequence ───────────────────────────────────────────────────

#[test]
fn routing_rapid_down_up_sequence() {
    let id = WidgetId::next();
    let mut mgr = InteractionManager::new();
    mgr.register_widget(id);

    // Rapid click: down, up, down, up.
    mgr.set_active(id);
    assert_eq!(mgr.active_widget(), Some(id));
    mgr.clear_active();
    assert_eq!(mgr.active_widget(), None);
    mgr.set_active(id);
    assert_eq!(mgr.active_widget(), Some(id));
    mgr.clear_active();
    assert_eq!(mgr.active_widget(), None);
}

// ── Move outside all bounds then cursor leave ────────────────────────

#[test]
fn routing_move_outside_all_then_leave() {
    let id = WidgetId::next();
    let root = make_node(10.0, 10.0, 50.0, 50.0, Some(id));
    let mut mgr = InteractionManager::new();
    mgr.register_widget(id);
    mgr.drain_events();
    let mut actions = Vec::new();

    // Enter widget.
    simulate_mouse(&mouse_move_input(30.0, 30.0), &root, &mut mgr, &mut actions);
    assert!(mgr.get_state(id).is_hot());

    // Move outside all widget bounds but still in window.
    let events = simulate_mouse(&mouse_move_input(5.0, 5.0), &root, &mut mgr, &mut actions);
    let hot = hot_changed_events(&events);
    assert!(
        hot.contains(&(id, false)),
        "Leave when moving out of widget"
    );
    assert!(!mgr.get_state(id).is_hot());

    // Cursor leaves window — no duplicate leave.
    mgr.update_hot_path(&[]);
    let events = mgr.drain_events();
    assert!(hot_changed_events(&events).is_empty(), "no widget to leave");
}

#[test]
fn modifiers_bitmask_operations() {
    let ctrl_shift = Modifiers::CTRL_ONLY.union(Modifiers::SHIFT_ONLY);
    assert!(ctrl_shift.ctrl());
    assert!(ctrl_shift.shift());
    assert!(!ctrl_shift.alt());
    assert!(!ctrl_shift.logo());

    assert_eq!(Modifiers::NONE, Modifiers::default());
    assert!(!Modifiers::NONE.shift());
}

// ── Sense filtering ─────────────────────────────────────────────────

#[test]
fn hit_test_sense_none_skipped() {
    let btn_id = WidgetId::next();
    let label_id = WidgetId::next();

    // Label (Sense::none) sits on top of button (Sense::click).
    let button = make_node(0.0, 0.0, 100.0, 100.0, Some(btn_id));
    let mut label = make_node(0.0, 0.0, 100.0, 100.0, Some(label_id));
    label.sense = Sense::none();

    let root = LayoutNode::new(
        Rect::new(0.0, 0.0, 100.0, 100.0),
        Rect::new(0.0, 0.0, 100.0, 100.0),
    )
    .with_children(vec![button, label]);

    // Label is last child (frontmost) but has Sense::none — button should win.
    let hit = layout_hit_test(&root, Point::new(50.0, 50.0));
    assert_eq!(hit, Some(btn_id));
}

#[test]
fn hit_test_disabled_widget_skipped() {
    let btn_id = WidgetId::next();
    let disabled_id = WidgetId::next();

    let button = make_node(0.0, 0.0, 100.0, 100.0, Some(btn_id));
    let mut disabled = make_node(0.0, 0.0, 100.0, 100.0, Some(disabled_id));
    disabled.disabled = true;

    let root = LayoutNode::new(
        Rect::new(0.0, 0.0, 100.0, 100.0),
        Rect::new(0.0, 0.0, 100.0, 100.0),
    )
    .with_children(vec![button, disabled]);

    // Disabled widget on top — button behind should receive the hit.
    let hit = layout_hit_test(&root, Point::new(50.0, 50.0));
    assert_eq!(hit, Some(btn_id));
}

// ── interact_radius ─────────────────────────────────────────────────

#[test]
fn hit_test_interact_radius_expands_hit_area() {
    let id = WidgetId::next();
    let mut node = make_node(50.0, 50.0, 10.0, 10.0, Some(id));
    node.interact_radius = 5.0;

    let root = LayoutNode::new(
        Rect::new(0.0, 0.0, 200.0, 200.0),
        Rect::new(0.0, 0.0, 200.0, 200.0),
    )
    .with_children(vec![node]);

    // Point outside the 10x10 widget but within 5px interact radius.
    let hit = layout_hit_test(&root, Point::new(48.0, 55.0));
    assert_eq!(hit, Some(id));
}

#[test]
fn hit_test_interact_radius_tie_breaking() {
    let left_id = WidgetId::next();
    let right_id = WidgetId::next();

    let mut left = make_node(10.0, 10.0, 10.0, 10.0, Some(left_id));
    left.interact_radius = 10.0;
    let mut right = make_node(30.0, 10.0, 10.0, 10.0, Some(right_id));
    right.interact_radius = 10.0;

    let root = LayoutNode::new(
        Rect::new(0.0, 0.0, 200.0, 200.0),
        Rect::new(0.0, 0.0, 200.0, 200.0),
    )
    .with_children(vec![left, right]);

    // Point closer to left's center (15, 15) than right's center (35, 15).
    let hit = layout_hit_test(&root, Point::new(20.0, 15.0));
    assert_eq!(hit, Some(left_id));

    // Point closer to right's center.
    let hit = layout_hit_test(&root, Point::new(30.0, 15.0));
    assert_eq!(hit, Some(right_id));
}

// ── HitTestBehavior ─────────────────────────────────────────────────

#[test]
fn hit_test_opaque_blocks_children() {
    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();

    let child = make_node(10.0, 10.0, 80.0, 80.0, Some(child_id));
    let mut parent = make_node(0.0, 0.0, 100.0, 100.0, Some(parent_id));
    parent.hit_test_behavior = HitTestBehavior::Opaque;
    parent.children = vec![child];

    // Parent is opaque — child should NOT appear in hit test.
    let hit = layout_hit_test(&parent, Point::new(50.0, 50.0));
    assert_eq!(hit, Some(parent_id));
}

#[test]
fn hit_test_translucent_includes_parent_and_child() {
    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();

    let child = make_node(10.0, 10.0, 80.0, 80.0, Some(child_id));
    let mut parent = make_node(0.0, 0.0, 100.0, 100.0, Some(parent_id));
    parent.hit_test_behavior = HitTestBehavior::Translucent;
    parent.children = vec![child];

    // Translucent — both parent and child should be in the path.
    let result = layout_hit_test_path(&parent, Point::new(50.0, 50.0));
    assert_eq!(result.widget_ids(), vec![parent_id, child_id]);
}

// ── Chromium-Inspired Edge Cases ─────────────────────────────────────

#[test]
fn hit_test_zero_size_widget_not_hittable() {
    // Zero-width widget: half-open rect [10, 10+0) has no interior.
    let id = WidgetId::next();
    let root = make_node(10.0, 10.0, 0.0, 50.0, Some(id));
    assert_eq!(layout_hit_test(&root, Point::new(10.0, 25.0)), None);

    // Zero-height.
    let root = make_node(10.0, 10.0, 50.0, 0.0, Some(id));
    assert_eq!(layout_hit_test(&root, Point::new(25.0, 10.0)), None);

    // Zero both.
    let root = make_node(10.0, 10.0, 0.0, 0.0, Some(id));
    assert_eq!(layout_hit_test(&root, Point::new(10.0, 10.0)), None);
}

#[test]
fn hit_test_one_pixel_widget() {
    let id = WidgetId::next();
    let root = make_node(10.0, 10.0, 1.0, 1.0, Some(id));

    // Exact top-left corner is inside.
    assert_eq!(layout_hit_test(&root, Point::new(10.0, 10.0)), Some(id));
    // Right/bottom edge is outside (half-open).
    assert_eq!(layout_hit_test(&root, Point::new(11.0, 10.0)), None);
    assert_eq!(layout_hit_test(&root, Point::new(10.0, 11.0)), None);
}

#[test]
fn hit_test_negative_coordinates() {
    let id = WidgetId::next();
    let root = make_node(-50.0, -50.0, 100.0, 100.0, Some(id));

    assert_eq!(layout_hit_test(&root, Point::new(-25.0, -25.0)), Some(id));
    assert_eq!(layout_hit_test(&root, Point::new(25.0, 25.0)), Some(id));
    assert_eq!(layout_hit_test(&root, Point::new(50.0, 0.0)), None);
}

#[test]
fn hit_test_child_extends_beyond_parent() {
    // Child extends past parent's right edge.
    // Our hit test checks parent bounds first, so the child's out-of-bounds
    // portion is not reachable (implicit clipping by parent rect).
    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();

    let child = make_node(50.0, 0.0, 100.0, 50.0, Some(child_id));
    let mut parent = make_node(0.0, 0.0, 100.0, 50.0, Some(parent_id));
    parent.children.push(child);

    // Inside child AND parent → child.
    assert_eq!(
        layout_hit_test(&parent, Point::new(75.0, 25.0)),
        Some(child_id)
    );
    // Inside child but OUTSIDE parent → miss (parent clips).
    assert_eq!(layout_hit_test(&parent, Point::new(125.0, 25.0)), None);
}

#[test]
fn hit_test_three_overlapping_siblings() {
    let parent_id = WidgetId::next();
    let a = WidgetId::next();
    let b = WidgetId::next();
    let c = WidgetId::next();

    // All overlap at (20,20)-(40,40).
    let child_a = make_node(10.0, 10.0, 40.0, 40.0, Some(a));
    let child_b = make_node(20.0, 20.0, 40.0, 40.0, Some(b));
    let child_c = make_node(15.0, 15.0, 30.0, 30.0, Some(c));

    let mut parent = make_node(0.0, 0.0, 100.0, 100.0, Some(parent_id));
    parent.children.push(child_a);
    parent.children.push(child_b);
    parent.children.push(child_c);

    // Last child (c) wins in overlap region.
    assert_eq!(layout_hit_test(&parent, Point::new(30.0, 30.0)), Some(c));
    // Only b covers (55, 55).
    assert_eq!(layout_hit_test(&parent, Point::new(55.0, 55.0)), Some(b));
    // Only a covers (12, 12).
    assert_eq!(layout_hit_test(&parent, Point::new(12.0, 12.0)), Some(a));
}

#[test]
fn hit_test_no_id_middle_layer_falls_through() {
    // Root (ID) → Middle (no ID) → Leaf (ID).
    // Hit in leaf area → leaf. Hit in middle-only area → root (skip middle).
    let root_id = WidgetId::next();
    let leaf_id = WidgetId::next();

    let leaf = make_node(30.0, 30.0, 20.0, 20.0, Some(leaf_id));
    let mut middle = make_node(10.0, 10.0, 60.0, 60.0, None);
    middle.children.push(leaf);
    let mut root = make_node(0.0, 0.0, 100.0, 100.0, Some(root_id));
    root.children.push(middle);

    assert_eq!(
        layout_hit_test(&root, Point::new(35.0, 35.0)),
        Some(leaf_id)
    );
    // Inside middle but not leaf → falls through middle (no ID) to root.
    assert_eq!(
        layout_hit_test(&root, Point::new(15.0, 15.0)),
        Some(root_id)
    );
}

// ── Clip ─────────────────────────────────────────────────────────────

#[test]
fn hit_test_clip_prevents_child_outside_bounds() {
    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();

    // Child extends beyond parent's rect.
    let child = make_node(80.0, 0.0, 50.0, 50.0, Some(child_id));
    let mut parent = make_node(0.0, 0.0, 100.0, 100.0, Some(parent_id));
    parent.clip = true;
    parent.children = vec![child];

    // Point at (110, 25) is inside child but outside parent's clip.
    let hit = layout_hit_test(&parent, Point::new(110.0, 25.0));
    assert_eq!(hit, None);

    // Point at (90, 25) is inside child AND inside parent's clip.
    let hit = layout_hit_test(&parent, Point::new(90.0, 25.0));
    assert_eq!(hit, Some(child_id));
}

// ── InputEvent from/to roundtrips ────────────────────────────────────

use super::event::{Key, KeyEvent, MouseEvent, MouseEventKind};

#[test]
fn from_mouse_event_roundtrip_down() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        pos: Point::new(10.0, 20.0),
        modifiers: Modifiers::CTRL_ONLY,
    };
    let input = InputEvent::from_mouse_event(&mouse);
    let back = input.to_mouse_event().expect("should convert back");
    assert_eq!(back, mouse);
}

#[test]
fn from_mouse_event_roundtrip_up() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Up(MouseButton::Right),
        pos: Point::new(5.0, 15.0),
        modifiers: Modifiers::NONE,
    };
    let input = InputEvent::from_mouse_event(&mouse);
    let back = input.to_mouse_event().expect("should convert back");
    assert_eq!(back, mouse);
}

#[test]
fn from_mouse_event_roundtrip_move() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Move,
        pos: Point::new(30.0, 40.0),
        modifiers: Modifiers::SHIFT_ONLY,
    };
    let input = InputEvent::from_mouse_event(&mouse);
    let back = input.to_mouse_event().expect("should convert back");
    assert_eq!(back, mouse);
}

#[test]
fn from_mouse_event_roundtrip_scroll() {
    let mouse = MouseEvent {
        kind: MouseEventKind::Scroll(ScrollDelta::Lines { x: 0.0, y: -3.0 }),
        pos: Point::new(50.0, 60.0),
        modifiers: Modifiers::ALT_ONLY,
    };
    let input = InputEvent::from_mouse_event(&mouse);
    let back = input.to_mouse_event().expect("should convert back");
    assert_eq!(back, mouse);
}

#[test]
fn from_key_event_produces_key_down() {
    let key = KeyEvent {
        key: Key::Enter,
        modifiers: Modifiers::CTRL_ONLY,
    };
    let input = InputEvent::from_key_event(key);
    assert_eq!(
        input,
        InputEvent::KeyDown {
            key: Key::Enter,
            modifiers: Modifiers::CTRL_ONLY,
        }
    );
    // Round-trips back to KeyEvent.
    let back = input.to_key_event().expect("should convert back");
    assert_eq!(back, key);
}

// ── Scroll offset hit testing ─────────────────────────────────────────

#[test]
fn hit_test_with_content_offset_translates_point() {
    // Simulate a scroll container (200px tall viewport) with a child
    // button at y=300 in content space. After scrolling down 250px,
    // the button is visually at y=50 in the viewport.
    let btn_id = WidgetId::next();
    let scroll_id = WidgetId::next();

    let button = make_node(0.0, 300.0, 200.0, 40.0, Some(btn_id));

    let rect = Rect::new(0.0, 0.0, 200.0, 200.0);
    let scroll = LayoutNode {
        rect,
        content_rect: rect,
        children: vec![button],
        widget_id: Some(scroll_id),
        sense: Sense::hover(),
        hit_test_behavior: HitTestBehavior::default(),
        clip: true,
        disabled: false,
        interact_radius: 0.0,
        // Scrolled down 250px: content_offset = (0, -250).
        content_offset: (0.0, -250.0),
    };

    // Click at y=50 in viewport → should hit button at y=300 in content
    // because the scroll offset translates the point.
    let result = layout_hit_test(&scroll, Point::new(100.0, 50.0));
    assert_eq!(
        result,
        Some(btn_id),
        "should hit button through scroll offset"
    );
}

#[test]
fn hit_test_with_content_offset_misses_outside_clip() {
    // Button at y=300, scroll offset 50px → button visually at y=250,
    // which is outside the 200px viewport.
    let btn_id = WidgetId::next();
    let scroll_id = WidgetId::next();

    let button = make_node(0.0, 300.0, 200.0, 40.0, Some(btn_id));

    let rect = Rect::new(0.0, 0.0, 200.0, 200.0);
    let scroll = LayoutNode {
        rect,
        content_rect: rect,
        children: vec![button],
        widget_id: Some(scroll_id),
        sense: Sense::hover(),
        hit_test_behavior: HitTestBehavior::default(),
        clip: true,
        disabled: false,
        interact_radius: 0.0,
        content_offset: (0.0, -50.0),
    };

    // Click at y=50 → adjusted to y=100 in content space.
    // Button is at y=300..340, so this should miss.
    let result = layout_hit_test(&scroll, Point::new(100.0, 50.0));
    assert_eq!(
        result,
        Some(scroll_id),
        "should hit scroll container, not button"
    );
}

#[test]
fn hit_test_path_with_content_offset() {
    let btn_id = WidgetId::next();
    let scroll_id = WidgetId::next();

    let button = make_node(0.0, 300.0, 200.0, 40.0, Some(btn_id));

    let rect = Rect::new(0.0, 0.0, 200.0, 200.0);
    let scroll = LayoutNode {
        rect,
        content_rect: rect,
        children: vec![button],
        widget_id: Some(scroll_id),
        sense: Sense::hover(),
        hit_test_behavior: HitTestBehavior::default(),
        clip: true,
        disabled: false,
        interact_radius: 0.0,
        content_offset: (0.0, -250.0),
    };

    // Click at y=50 → should produce path [scroll, button].
    let result = layout_hit_test_path(&scroll, Point::new(100.0, 50.0));
    assert_eq!(result.path.len(), 2);
    assert_eq!(result.path[0].widget_id, scroll_id);
    assert_eq!(result.path[1].widget_id, btn_id);
}
