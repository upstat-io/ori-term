use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, Modifiers, MouseButton};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::ScrubController;

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

#[test]
fn drag_start_on_mouse_down() {
    let mut ctrl = ScrubController::new();
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::MouseDown {
        pos: Point::new(50.0, 10.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);
    let reqs = ctx.requests;

    assert!(handled);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        WidgetAction::DragStart { pos, .. } if pos.x == 50.0 && pos.y == 10.0
    ));
    assert!(reqs.contains(ControllerRequests::SET_ACTIVE));
    assert!(reqs.contains(ControllerRequests::PAINT));
}

#[test]
fn drag_update_on_mouse_move() {
    let mut ctrl = ScrubController::new();
    let id = WidgetId::next();
    let interaction = InteractionState::default();

    // Press first.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let down = InputEvent::MouseDown {
            pos: Point::new(50.0, 10.0),
            button: MouseButton::Left,
            modifiers: Modifiers::NONE,
        };
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&down, &mut ctx);
    }

    // Move.
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mv = InputEvent::MouseMove {
        pos: Point::new(80.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&mv, &mut ctx);

    assert!(handled);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        WidgetAction::DragUpdate { total_delta, delta, .. }
            if (total_delta.x - 30.0).abs() < f32::EPSILON
            && (delta.x - 30.0).abs() < f32::EPSILON
    ));
}

#[test]
fn drag_end_on_mouse_up() {
    let mut ctrl = ScrubController::new();
    let id = WidgetId::next();
    let interaction = InteractionState::default();

    // Press.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let down = InputEvent::MouseDown {
            pos: Point::new(50.0, 10.0),
            button: MouseButton::Left,
            modifiers: Modifiers::NONE,
        };
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&down, &mut ctx);
    }

    // Release.
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let up = InputEvent::MouseUp {
        pos: Point::new(80.0, 10.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&up, &mut ctx);
    let reqs = ctx.requests;

    assert!(handled);
    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        WidgetAction::DragEnd { pos, .. } if pos.x == 80.0
    ));
    assert!(reqs.contains(ControllerRequests::CLEAR_ACTIVE));
}

#[test]
fn move_without_press_ignored() {
    let mut ctrl = ScrubController::new();
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let mv = InputEvent::MouseMove {
        pos: Point::new(80.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&mv, &mut ctx);

    assert!(!handled);
    assert!(actions.is_empty());
}

#[test]
fn right_click_ignored() {
    let mut ctrl = ScrubController::new();
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::MouseDown {
        pos: Point::new(50.0, 10.0),
        button: MouseButton::Right,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);

    assert!(!handled);
    assert!(actions.is_empty());
}

#[test]
fn cumulative_total_delta() {
    let mut ctrl = ScrubController::new();
    let id = WidgetId::next();
    let interaction = InteractionState::default();

    // Press at 50.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let down = InputEvent::MouseDown {
            pos: Point::new(50.0, 10.0),
            button: MouseButton::Left,
            modifiers: Modifiers::NONE,
        };
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&down, &mut ctx);
    }

    // Move to 70.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mv1 = InputEvent::MouseMove {
            pos: Point::new(70.0, 10.0),
            modifiers: Modifiers::NONE,
        };
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mv1, &mut ctx);
    }

    // Move to 90 — total_delta should be 40 (90 - 50), delta should be 20 (90 - 70).
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mv2 = InputEvent::MouseMove {
        pos: Point::new(90.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    ctrl.handle_event(&mv2, &mut ctx);

    assert_eq!(actions.len(), 1);
    assert!(matches!(
        &actions[0],
        WidgetAction::DragUpdate { total_delta, delta, .. }
            if (total_delta.x - 40.0).abs() < f32::EPSILON
            && (delta.x - 20.0).abs() < f32::EPSILON
    ));
}

#[test]
fn reset_clears_press() {
    let mut ctrl = ScrubController::new();
    let id = WidgetId::next();
    let interaction = InteractionState::default();

    // Press.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let down = InputEvent::MouseDown {
            pos: Point::new(50.0, 10.0),
            button: MouseButton::Left,
            modifiers: Modifiers::NONE,
        };
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&down, &mut ctx);
    }

    // Reset.
    ctrl.reset();

    // Move should be ignored now.
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mv = InputEvent::MouseMove {
        pos: Point::new(80.0, 10.0),
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&mv, &mut ctx);

    assert!(!handled);
    assert!(actions.is_empty());
}
