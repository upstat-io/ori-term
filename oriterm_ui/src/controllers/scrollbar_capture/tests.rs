//! Tests for `ScrollbarCaptureController`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::action::WidgetAction;
use crate::controllers::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use crate::geometry::{Point, Rect};
use crate::input::{EventPhase, InputEvent, Modifiers, MouseButton};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;
use crate::widgets::scrollbar::ScrollbarHitZones;

use super::ScrollbarCaptureController;

fn make_controller() -> (ScrollbarCaptureController, Rc<RefCell<ScrollbarHitZones>>) {
    let zones = Rc::new(RefCell::new(ScrollbarHitZones::default()));
    let ctrl = ScrollbarCaptureController::new(Rc::clone(&zones));
    (ctrl, zones)
}

fn fire(
    ctrl: &mut ScrollbarCaptureController,
    event: &InputEvent,
) -> (bool, Vec<WidgetAction>, ControllerRequests) {
    let interaction = InteractionState::new();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = ControllerCtx {
        widget_id: WidgetId::next(),
        bounds: Rect::new(0.0, 0.0, 200.0, 400.0),
        interaction: &interaction,
        actions: &mut actions,
        requests: ControllerRequests::NONE,
        now: Instant::now(),
        propagation: &mut prop,
    };
    let handled = ctrl.handle_event(event, &mut ctx);
    let requests = ctx.requests;
    (handled, actions, requests)
}

#[test]
fn phase_is_capture() {
    let (ctrl, _) = make_controller();
    assert_eq!(ctrl.phase(), EventPhase::Capture);
}

#[test]
fn ignores_click_outside_scrollbar() {
    let (mut ctrl, zones) = make_controller();
    zones.borrow_mut().v_thumb_hit = Some(Rect::new(180.0, 10.0, 20.0, 40.0));
    zones.borrow_mut().v_track_hit = Some(Rect::new(180.0, 0.0, 20.0, 400.0));

    let event = InputEvent::MouseDown {
        pos: Point::new(50.0, 100.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let (handled, actions, _) = fire(&mut ctrl, &event);
    assert!(!handled, "click outside scrollbar should not be handled");
    assert!(actions.is_empty());
}

#[test]
fn thumb_click_starts_drag_with_capture() {
    let (mut ctrl, zones) = make_controller();
    zones.borrow_mut().v_thumb_hit = Some(Rect::new(180.0, 10.0, 20.0, 40.0));
    zones.borrow_mut().v_track_hit = Some(Rect::new(180.0, 0.0, 20.0, 400.0));

    let event = InputEvent::MouseDown {
        pos: Point::new(190.0, 30.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let (handled, actions, requests) = fire(&mut ctrl, &event);
    assert!(handled, "thumb click should be handled");
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], WidgetAction::DragStart { .. }));
    assert!(requests.contains(ControllerRequests::SET_ACTIVE));
}

#[test]
fn track_click_handled_without_capture() {
    let (mut ctrl, zones) = make_controller();
    zones.borrow_mut().v_thumb_hit = Some(Rect::new(180.0, 10.0, 20.0, 40.0));
    zones.borrow_mut().v_track_hit = Some(Rect::new(180.0, 0.0, 20.0, 400.0));

    let event = InputEvent::MouseDown {
        pos: Point::new(190.0, 300.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let (handled, actions, requests) = fire(&mut ctrl, &event);
    assert!(handled, "track click should be handled");
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], WidgetAction::DragStart { .. }));
    assert!(
        !requests.contains(ControllerRequests::SET_ACTIVE),
        "track click should NOT capture"
    );
}

#[test]
fn thumb_drag_emits_update_and_end() {
    let (mut ctrl, zones) = make_controller();
    zones.borrow_mut().v_thumb_hit = Some(Rect::new(180.0, 10.0, 20.0, 40.0));
    zones.borrow_mut().v_track_hit = Some(Rect::new(180.0, 0.0, 20.0, 400.0));

    // Start drag on thumb.
    let down = InputEvent::MouseDown {
        pos: Point::new(190.0, 30.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    fire(&mut ctrl, &down);

    // Move while dragging.
    let mv = InputEvent::MouseMove {
        pos: Point::new(190.0, 50.0),
        modifiers: Modifiers::NONE,
    };
    let (handled, actions, _) = fire(&mut ctrl, &mv);
    assert!(handled, "drag move should be handled");
    assert!(matches!(actions[0], WidgetAction::DragUpdate { .. }));
    if let WidgetAction::DragUpdate { total_delta, .. } = &actions[0] {
        assert!((total_delta.y - 20.0).abs() < 0.01);
    }

    // Release.
    let up = InputEvent::MouseUp {
        pos: Point::new(190.0, 50.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let (handled, actions, requests) = fire(&mut ctrl, &up);
    assert!(handled, "drag end should be handled");
    assert!(matches!(actions[0], WidgetAction::DragEnd { .. }));
    assert!(requests.contains(ControllerRequests::CLEAR_ACTIVE));
}

#[test]
fn move_without_drag_is_ignored() {
    let (mut ctrl, _) = make_controller();
    let event = InputEvent::MouseMove {
        pos: Point::new(190.0, 30.0),
        modifiers: Modifiers::NONE,
    };
    let (handled, _, _) = fire(&mut ctrl, &event);
    assert!(!handled, "mouse move without active drag should be ignored");
}

#[test]
fn reset_clears_drag_state() {
    let (mut ctrl, zones) = make_controller();
    zones.borrow_mut().v_thumb_hit = Some(Rect::new(180.0, 10.0, 20.0, 40.0));
    zones.borrow_mut().v_track_hit = Some(Rect::new(180.0, 0.0, 20.0, 400.0));

    // Start drag.
    let down = InputEvent::MouseDown {
        pos: Point::new(190.0, 30.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    fire(&mut ctrl, &down);
    ctrl.reset();

    // Move should no longer be handled.
    let mv = InputEvent::MouseMove {
        pos: Point::new(190.0, 50.0),
        modifiers: Modifiers::NONE,
    };
    let (handled, _, _) = fire(&mut ctrl, &mv);
    assert!(!handled, "after reset, move should not be handled");
}

#[test]
fn empty_hit_zones_ignores_all_clicks() {
    let (mut ctrl, _) = make_controller();
    let event = InputEvent::MouseDown {
        pos: Point::new(190.0, 30.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let (handled, _, _) = fire(&mut ctrl, &event);
    assert!(!handled, "with no hit zones, should ignore all clicks");
}
