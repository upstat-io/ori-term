//! Tests for the two-phase event propagation pipeline.

use winit::window::CursorIcon;

use crate::geometry::{Point, Rect};
use crate::input::event::{EventPhase, InputEvent, Key, Modifiers, MouseButton, ScrollDelta};
use crate::input::hit_test::{HitEntry, WidgetHitTestResult};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::{DeliveryAction, plan_propagation};

// ── Helpers ──────────────────────────────────────────────────────────

fn empty_hit() -> WidgetHitTestResult {
    WidgetHitTestResult { path: Vec::new() }
}

fn hit_path(entries: &[(WidgetId, Rect)]) -> WidgetHitTestResult {
    WidgetHitTestResult {
        path: entries
            .iter()
            .map(|&(widget_id, bounds)| HitEntry {
                widget_id,
                bounds,
                sense: Sense::all(),
                cursor_icon: CursorIcon::Default,
            })
            .collect(),
    }
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}

fn mouse_down_event(x: f32, y: f32) -> InputEvent {
    InputEvent::MouseDown {
        pos: Point::new(x, y),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    }
}

fn mouse_move_event(x: f32, y: f32) -> InputEvent {
    InputEvent::MouseMove {
        pos: Point::new(x, y),
        modifiers: Modifiers::NONE,
    }
}

fn mouse_up_event(x: f32, y: f32) -> InputEvent {
    InputEvent::MouseUp {
        pos: Point::new(x, y),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    }
}

fn scroll_event(x: f32, y: f32) -> InputEvent {
    InputEvent::Scroll {
        pos: Point::new(x, y),
        delta: ScrollDelta::Lines { x: 0.0, y: -3.0 },
        modifiers: Modifiers::NONE,
    }
}

fn key_down_event() -> InputEvent {
    InputEvent::KeyDown {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    }
}

fn phases(actions: &[DeliveryAction]) -> Vec<EventPhase> {
    actions.iter().map(|a| a.phase).collect()
}

fn widget_ids(actions: &[DeliveryAction]) -> Vec<WidgetId> {
    actions.iter().map(|a| a.widget_id).collect()
}

// ── Mouse: normal two-phase propagation ──────────────────────────────

#[test]
fn mouse_empty_hit_path_produces_no_actions() {
    let event = mouse_down_event(50.0, 50.0);
    let hit = empty_hit();
    let mut out = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut out);
    assert!(out.is_empty());
}

#[test]
fn mouse_single_widget_capture_target_bubble() {
    let id = WidgetId::next();
    let bounds = rect(0.0, 0.0, 100.0, 100.0);
    let event = mouse_down_event(50.0, 50.0);
    let hit = hit_path(&[(id, bounds)]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut out);

    // Single widget: Capture + Target (no Bubble since there's no parent).
    assert_eq!(out.len(), 2);
    assert_eq!(phases(&out), vec![EventPhase::Capture, EventPhase::Target]);
    assert_eq!(out[0].widget_id, id);
    assert_eq!(out[1].widget_id, id);
    assert_eq!(out[0].bounds, bounds);
}

#[test]
fn mouse_three_widget_path_full_propagation() {
    let root = WidgetId::next();
    let mid = WidgetId::next();
    let leaf = WidgetId::next();

    let r_bounds = rect(0.0, 0.0, 200.0, 200.0);
    let m_bounds = rect(10.0, 10.0, 100.0, 100.0);
    let l_bounds = rect(20.0, 20.0, 50.0, 50.0);

    let event = mouse_down_event(30.0, 30.0);
    let hit = hit_path(&[(root, r_bounds), (mid, m_bounds), (leaf, l_bounds)]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut out);

    // Capture: root, mid, leaf; Target: leaf; Bubble: mid, root.
    assert_eq!(out.len(), 6);
    assert_eq!(
        phases(&out),
        vec![
            EventPhase::Capture,
            EventPhase::Capture,
            EventPhase::Capture,
            EventPhase::Target,
            EventPhase::Bubble,
            EventPhase::Bubble,
        ]
    );
    assert_eq!(widget_ids(&out), vec![root, mid, leaf, leaf, mid, root]);
}

#[test]
fn mouse_bounds_from_hit_entries() {
    let parent = WidgetId::next();
    let child = WidgetId::next();
    let p_bounds = rect(0.0, 0.0, 200.0, 200.0);
    let c_bounds = rect(50.0, 50.0, 80.0, 80.0);

    let event = mouse_down_event(60.0, 60.0);
    let hit = hit_path(&[(parent, p_bounds), (child, c_bounds)]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut out);

    // Capture phase entries should have bounds from HitEntry.
    assert_eq!(out[0].bounds, p_bounds);
    assert_eq!(out[1].bounds, c_bounds);
    // Target entry.
    assert_eq!(out[2].bounds, c_bounds);
    // Bubble entry.
    assert_eq!(out[3].bounds, p_bounds);
}

// ── Mouse: active capture ────────────────────────────────────────────

#[test]
fn active_widget_receives_mouse_move_directly() {
    let active = WidgetId::next();
    let other = WidgetId::next();
    let event = mouse_move_event(999.0, 999.0);
    let hit = hit_path(&[(other, rect(0.0, 0.0, 100.0, 100.0))]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, Some(active), &[], &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].widget_id, active);
    assert_eq!(out[0].phase, EventPhase::Target);
}

#[test]
fn active_widget_receives_mouse_up_directly() {
    let active = WidgetId::next();
    let event = mouse_up_event(50.0, 50.0);
    let hit = empty_hit();
    let mut out = Vec::new();

    plan_propagation(&event, &hit, Some(active), &[], &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].widget_id, active);
    assert_eq!(out[0].phase, EventPhase::Target);
}

#[test]
fn scroll_during_capture_uses_hit_path() {
    let active = WidgetId::next();
    let scroll_target = WidgetId::next();
    let event = scroll_event(50.0, 50.0);
    let hit = hit_path(&[(scroll_target, rect(0.0, 0.0, 100.0, 100.0))]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, Some(active), &[], &mut out);

    // Scroll should route through hit path, not to active widget.
    assert!(out.len() > 1, "scroll uses full propagation");
    assert!(
        widget_ids(&out).contains(&scroll_target),
        "scroll targets hit widget"
    );
    assert!(
        !widget_ids(&out).contains(&active),
        "scroll does not target active widget"
    );
}

#[test]
fn mouse_down_during_capture_uses_hit_path() {
    let active = WidgetId::next();
    let hit_target = WidgetId::next();
    let event = mouse_down_event(50.0, 50.0);
    let hit = hit_path(&[(hit_target, rect(0.0, 0.0, 100.0, 100.0))]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, Some(active), &[], &mut out);

    // MouseDown during capture uses normal routing (edge case).
    assert!(widget_ids(&out).contains(&hit_target));
}

// ── Keyboard propagation ─────────────────────────────────────────────

#[test]
fn keyboard_empty_focus_path_produces_no_actions() {
    let event = key_down_event();
    let mut out = Vec::new();

    plan_propagation(&event, &empty_hit(), None, &[], &mut out);
    assert!(out.is_empty());
}

#[test]
fn keyboard_single_focused_widget() {
    let focused = WidgetId::next();
    let event = key_down_event();
    let mut out = Vec::new();

    plan_propagation(&event, &empty_hit(), None, &[focused], &mut out);

    assert_eq!(out.len(), 2);
    assert_eq!(phases(&out), vec![EventPhase::Capture, EventPhase::Target]);
    assert_eq!(out[0].widget_id, focused);
    assert_eq!(out[1].widget_id, focused);
}

#[test]
fn keyboard_routes_through_focus_ancestors() {
    let root = WidgetId::next();
    let panel = WidgetId::next();
    let input = WidgetId::next();
    let event = key_down_event();
    let mut out = Vec::new();

    plan_propagation(&event, &empty_hit(), None, &[root, panel, input], &mut out);

    // Capture: root, panel, input; Target: input; Bubble: panel, root.
    assert_eq!(out.len(), 6);
    assert_eq!(
        phases(&out),
        vec![
            EventPhase::Capture,
            EventPhase::Capture,
            EventPhase::Capture,
            EventPhase::Target,
            EventPhase::Bubble,
            EventPhase::Bubble,
        ]
    );
    assert_eq!(
        widget_ids(&out),
        vec![root, panel, input, input, panel, root]
    );
}

// ── Cursor-left: not a plan_propagation concern ──────────────────────
// Cursor-left is handled by InteractionManager::update_hot_path(&[]),
// tested in interaction/tests.rs.

// ── Buffer reuse ─────────────────────────────────────────────────────

#[test]
fn buffer_capacity_retained_across_calls() {
    let id = WidgetId::next();
    let event = mouse_down_event(50.0, 50.0);
    let hit = hit_path(&[(id, rect(0.0, 0.0, 100.0, 100.0))]);
    let mut out = Vec::with_capacity(32);

    plan_propagation(&event, &hit, None, &[], &mut out);
    assert!(!out.is_empty());

    let cap = out.capacity();
    plan_propagation(&event, &empty_hit(), None, &[], &mut out);
    assert!(out.is_empty());
    assert_eq!(out.capacity(), cap, "capacity retained after clear");
}

// ── Parent intercepts in capture phase ───────────────────────────────

#[test]
fn parent_capture_precedes_child() {
    let parent = WidgetId::next();
    let child = WidgetId::next();
    let event = mouse_down_event(50.0, 50.0);
    let hit = hit_path(&[
        (parent, rect(0.0, 0.0, 200.0, 200.0)),
        (child, rect(10.0, 10.0, 50.0, 50.0)),
    ]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut out);

    // First action is parent in Capture phase — parent sees event first.
    assert_eq!(out[0].widget_id, parent);
    assert_eq!(out[0].phase, EventPhase::Capture);
    // If parent handles it, caller stops — child never sees the event.
}

#[test]
fn child_handles_in_target_prevents_parent_bubble() {
    let parent = WidgetId::next();
    let child = WidgetId::next();
    let event = mouse_down_event(50.0, 50.0);
    let hit = hit_path(&[
        (parent, rect(0.0, 0.0, 200.0, 200.0)),
        (child, rect(10.0, 10.0, 50.0, 50.0)),
    ]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, None, &[], &mut out);

    // Target action for child exists.
    let target_idx = out
        .iter()
        .position(|a| a.widget_id == child && a.phase == EventPhase::Target)
        .expect("child has Target action");

    // Bubble for parent comes after — but caller stops at Target if handled.
    let bubble_idx = out
        .iter()
        .position(|a| a.widget_id == parent && a.phase == EventPhase::Bubble)
        .expect("parent has Bubble action");

    assert!(target_idx < bubble_idx);
}

// -- Regression: captured mouse uses leaf bounds, not root bounds --

#[test]
fn captured_mouse_up_uses_leaf_widget_bounds() {
    // Hit path: [scroll_container(wide), dropdown(narrow)] — root-to-leaf.
    // plan_captured_mouse must use the LAST entry (dropdown) for bounds,
    // not the FIRST (scroll container). Using first() caused dropdown
    // popup menus to be full-width.
    let scroll = WidgetId::next();
    let dropdown = WidgetId::next();
    let scroll_bounds = rect(0.0, 0.0, 400.0, 300.0);
    let dropdown_bounds = rect(120.0, 80.0, 150.0, 24.0);

    let event = mouse_up_event(150.0, 90.0);
    let hit = hit_path(&[(scroll, scroll_bounds), (dropdown, dropdown_bounds)]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, Some(dropdown), &[], &mut out);

    assert_eq!(
        out.len(),
        1,
        "captured mouse-up should produce one delivery"
    );
    assert_eq!(out[0].widget_id, dropdown);
    assert_eq!(
        out[0].bounds, dropdown_bounds,
        "bounds must come from the leaf (dropdown), not the root (scroll)"
    );
}

#[test]
fn captured_mouse_up_single_entry_uses_that_entry() {
    let widget = WidgetId::next();
    let bounds = rect(50.0, 100.0, 120.0, 28.0);

    let event = mouse_up_event(80.0, 110.0);
    let hit = hit_path(&[(widget, bounds)]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, Some(widget), &[], &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].bounds, bounds);
}

#[test]
fn captured_mouse_move_uses_leaf_bounds() {
    let root = WidgetId::next();
    let leaf = WidgetId::next();
    let root_bounds = rect(0.0, 0.0, 800.0, 600.0);
    let leaf_bounds = rect(100.0, 200.0, 80.0, 20.0);

    let event = mouse_move_event(120.0, 210.0);
    let hit = hit_path(&[(root, root_bounds), (leaf, leaf_bounds)]);
    let mut out = Vec::new();

    plan_propagation(&event, &hit, Some(leaf), &[], &mut out);

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].widget_id, leaf);
    assert_eq!(
        out[0].bounds, leaf_bounds,
        "move during capture must use leaf bounds"
    );
}

// -- Safety rail: double-visit in dispatch --

/// Container that yields two children with the same WidgetId.
struct DoubleVisitDispatchContainer {
    id: WidgetId,
    child_a: StubDispatchWidget,
    child_b: StubDispatchWidget,
}

/// Minimal widget for dispatch double-visit test.
struct StubDispatchWidget {
    id: WidgetId,
}

impl crate::widgets::Widget for StubDispatchWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &crate::widgets::contexts::LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(10.0, 10.0)
    }

    fn paint(&self, _ctx: &mut crate::widgets::contexts::DrawCtx<'_>) {}
}

impl crate::widgets::Widget for DoubleVisitDispatchContainer {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &crate::widgets::contexts::LayoutCtx<'_>) -> crate::layout::LayoutBox {
        crate::layout::LayoutBox::leaf(100.0, 100.0)
    }

    fn paint(&self, _ctx: &mut crate::widgets::contexts::DrawCtx<'_>) {}

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn crate::widgets::Widget)) {
        visitor(&mut self.child_a);
        visitor(&mut self.child_b); // same ID — triggers assertion
    }
}

#[test]
#[should_panic(expected = "visited child")]
fn double_visit_in_dispatch_to_widget_tree_panics() {
    use std::time::Instant;

    use super::tree::{TreeDispatchResult, dispatch_to_widget_tree};

    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();
    let mut container = DoubleVisitDispatchContainer {
        id: parent_id,
        child_a: StubDispatchWidget { id: child_id },
        child_b: StubDispatchWidget { id: child_id },
    };
    let event = InputEvent::MouseMove {
        pos: Point::new(5.0, 5.0),
        modifiers: Modifiers::NONE,
    };
    let mut result = TreeDispatchResult::new();
    dispatch_to_widget_tree(
        &mut container,
        &event,
        &[],
        Instant::now(),
        &mut result,
        None,
    );
}
