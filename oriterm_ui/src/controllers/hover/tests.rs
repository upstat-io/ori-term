use std::time::Instant;

use crate::action::WidgetAction;
use crate::input::InputEvent;
use crate::interaction::InteractionState;
use crate::interaction::lifecycle::LifecycleEvent;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::HoverController;

fn make_ctx<'a>(
    id: WidgetId,
    interaction: &'a InteractionState,
    actions: &'a mut Vec<WidgetAction>,
    propagation: &'a mut PropagationState,
) -> ControllerCtx<'a> {
    ControllerCtx {
        widget_id: id,
        bounds: crate::geometry::Rect::default(),
        interaction,
        actions,
        requests: ControllerRequests::NONE,
        now: Instant::now(),
        propagation,
    }
}

#[test]
fn hot_changed_true_emits_enter_action() {
    let id = WidgetId::next();
    let enter_action = WidgetAction::Clicked(id);
    let mut ctrl = HoverController::new().with_on_enter(enter_action.clone());

    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = LifecycleEvent::HotChanged {
        widget_id: id,
        is_hot: true,
    };
    ctrl.handle_lifecycle(&event, &mut ctx);

    assert_eq!(ctx.actions.len(), 1);
    assert_eq!(ctx.actions[0], enter_action);
    assert!(ctx.requests.contains(ControllerRequests::PAINT));
}

#[test]
fn hot_changed_false_emits_leave_action() {
    let id = WidgetId::next();
    let leave_action = WidgetAction::Clicked(id);
    let mut ctrl = HoverController::new().with_on_leave(leave_action.clone());

    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = LifecycleEvent::HotChanged {
        widget_id: id,
        is_hot: false,
    };
    ctrl.handle_lifecycle(&event, &mut ctx);

    assert_eq!(ctx.actions.len(), 1);
    assert_eq!(ctx.actions[0], leave_action);
    assert!(ctx.requests.contains(ControllerRequests::PAINT));
}

#[test]
fn no_actions_when_none_configured() {
    let id = WidgetId::next();
    let mut ctrl = HoverController::new();

    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = LifecycleEvent::HotChanged {
        widget_id: id,
        is_hot: true,
    };
    ctrl.handle_lifecycle(&event, &mut ctx);

    assert!(ctx.actions.is_empty());
    // Still requests paint even without actions.
    assert!(ctx.requests.contains(ControllerRequests::PAINT));
}

#[test]
fn handle_event_never_consumes() {
    let id = WidgetId::next();
    let mut ctrl = HoverController::new().with_on_move();

    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::MouseMove {
        pos: crate::geometry::Point::new(10.0, 20.0),
        modifiers: crate::input::Modifiers::NONE,
    };
    let consumed = ctrl.handle_event(&event, &mut ctx);

    assert!(!consumed);
    assert!(ctx.requests.contains(ControllerRequests::PAINT));
}

#[test]
fn on_move_false_no_paint_on_mouse_move() {
    let id = WidgetId::next();
    let mut ctrl = HoverController::new(); // on_move = false

    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::MouseMove {
        pos: crate::geometry::Point::new(10.0, 20.0),
        modifiers: crate::input::Modifiers::NONE,
    };
    ctrl.handle_event(&event, &mut ctx);

    assert!(ctx.requests.is_empty());
}

#[test]
fn widget_disabled_requests_paint() {
    let id = WidgetId::next();
    let mut ctrl = HoverController::new();

    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = LifecycleEvent::WidgetDisabled {
        widget_id: id,
        disabled: true,
    };
    ctrl.handle_lifecycle(&event, &mut ctx);

    assert!(ctx.requests.contains(ControllerRequests::PAINT));
}
