use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::SliderKeyController;

fn make_ctx<'a>(
    id: WidgetId,
    interaction: &'a InteractionState,
    actions: &'a mut Vec<WidgetAction>,
    propagation: &'a mut PropagationState,
) -> ControllerCtx<'a> {
    ControllerCtx {
        widget_id: id,
        bounds: Rect::new(0.0, 0.0, 200.0, 20.0),
        interaction,
        actions,
        requests: ControllerRequests::NONE,
        now: Instant::now(),
        propagation,
    }
}

fn key_down(key: Key) -> InputEvent {
    InputEvent::KeyDown {
        key,
        modifiers: Modifiers::NONE,
    }
}

#[test]
fn arrow_right_increments() {
    let mut ctrl = SliderKeyController::new(0.0, 1.0, 0.1);
    ctrl.set_value(0.5);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&key_down(Key::ArrowRight), &mut ctx);

    assert!(handled);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        WidgetAction::ValueChanged { value, .. } => {
            assert!((value - 0.6).abs() < 0.01);
        }
        other => panic!("expected ValueChanged, got {other:?}"),
    }
}

#[test]
fn arrow_left_decrements() {
    let mut ctrl = SliderKeyController::new(0.0, 1.0, 0.1);
    ctrl.set_value(0.5);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&key_down(Key::ArrowLeft), &mut ctx);

    assert!(handled);
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        WidgetAction::ValueChanged { value, .. } => {
            assert!((value - 0.4).abs() < 0.01);
        }
        other => panic!("expected ValueChanged, got {other:?}"),
    }
}

#[test]
fn arrow_up_increments() {
    let mut ctrl = SliderKeyController::new(0.0, 100.0, 5.0);
    ctrl.set_value(50.0);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    ctrl.handle_event(&key_down(Key::ArrowUp), &mut ctx);

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        WidgetAction::ValueChanged { value, .. } => {
            assert!((value - 55.0).abs() < f32::EPSILON);
        }
        other => panic!("expected ValueChanged, got {other:?}"),
    }
}

#[test]
fn home_jumps_to_min() {
    let mut ctrl = SliderKeyController::new(10.0, 100.0, 5.0);
    ctrl.set_value(50.0);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    ctrl.handle_event(&key_down(Key::Home), &mut ctx);

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        WidgetAction::ValueChanged { value, .. } => {
            assert!((value - 10.0).abs() < f32::EPSILON);
        }
        other => panic!("expected ValueChanged, got {other:?}"),
    }
}

#[test]
fn end_jumps_to_max() {
    let mut ctrl = SliderKeyController::new(0.0, 100.0, 5.0);
    ctrl.set_value(50.0);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    ctrl.handle_event(&key_down(Key::End), &mut ctx);

    assert_eq!(actions.len(), 1);
    match &actions[0] {
        WidgetAction::ValueChanged { value, .. } => {
            assert!((value - 100.0).abs() < f32::EPSILON);
        }
        other => panic!("expected ValueChanged, got {other:?}"),
    }
}

#[test]
fn clamps_at_max() {
    let mut ctrl = SliderKeyController::new(0.0, 1.0, 0.1);
    ctrl.set_value(1.0);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&key_down(Key::ArrowRight), &mut ctx);

    assert!(handled);
    // Value is already at max — no change, no action.
    assert!(actions.is_empty());
}

#[test]
fn clamps_at_min() {
    let mut ctrl = SliderKeyController::new(0.0, 1.0, 0.1);
    ctrl.set_value(0.0);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&key_down(Key::ArrowLeft), &mut ctx);

    assert!(handled);
    assert!(actions.is_empty());
}

#[test]
fn keyup_consumed() {
    let mut ctrl = SliderKeyController::new(0.0, 1.0, 0.1);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyUp {
        key: Key::ArrowRight,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);

    assert!(handled);
    assert!(actions.is_empty());
}

#[test]
fn unrelated_keys_ignored() {
    let mut ctrl = SliderKeyController::new(0.0, 1.0, 0.1);
    ctrl.set_value(0.5);
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&key_down(Key::Enter), &mut ctx);

    assert!(!handled);
    assert!(actions.is_empty());
}

#[test]
fn set_range_clamps_value() {
    let mut ctrl = SliderKeyController::new(0.0, 100.0, 5.0);
    ctrl.set_value(80.0);
    ctrl.set_range(0.0, 50.0, 10.0);
    assert!((ctrl.value() - 50.0).abs() < f32::EPSILON);
}
