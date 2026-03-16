use std::time::Instant;

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key, Modifiers};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::DropdownKeyController;

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
fn arrow_down_cycles_forward() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let (consumed, requests) = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let c = ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::ArrowDown,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
        (c, ctx.requests)
    };

    assert!(consumed);
    assert_eq!(ctrl.selected(), 1);
    assert_eq!(actions, vec![WidgetAction::Selected { id, index: 1 }]);
    assert!(requests.contains(ControllerRequests::PAINT));
}

#[test]
fn arrow_down_wraps_around() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    ctrl.set_selected(2);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::ArrowDown,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }

    assert_eq!(ctrl.selected(), 0);
}

#[test]
fn arrow_up_cycles_backward() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    ctrl.set_selected(1);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::ArrowUp,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }

    assert_eq!(ctrl.selected(), 0);
    assert_eq!(actions, vec![WidgetAction::Selected { id, index: 0 }]);
}

#[test]
fn arrow_up_wraps_around() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::ArrowUp,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }

    assert_eq!(ctrl.selected(), 2);
}

#[test]
fn enter_confirms_selection() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    ctrl.set_selected(1);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Enter,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };

    assert!(consumed);
    assert_eq!(actions, vec![WidgetAction::Selected { id, index: 1 }]);
}

#[test]
fn escape_dismisses() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Escape,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };

    assert!(consumed);
    assert_eq!(actions, vec![WidgetAction::DismissOverlay(id)]);
}

#[test]
fn unhandled_key_not_consumed() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('x'),
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };

    assert!(!consumed);
    assert!(actions.is_empty());
}

#[test]
fn key_up_for_handled_keys_consumed() {
    let id = WidgetId::next();
    let mut ctrl = DropdownKeyController::new(3);
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let down_consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::ArrowDown,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(down_consumed);

    let enter_consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::Enter,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(enter_consumed);

    let x_consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::Character('x'),
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(!x_consumed);
}

#[test]
fn set_items_count_clamps_selection() {
    let mut ctrl = DropdownKeyController::new(5);
    ctrl.set_selected(4);
    assert_eq!(ctrl.selected(), 4);

    ctrl.set_items_count(3);
    assert_eq!(ctrl.selected(), 2);
}

#[test]
fn reset_resets_selection() {
    let mut ctrl = DropdownKeyController::new(5);
    ctrl.set_selected(3);

    ctrl.reset();
    assert_eq!(ctrl.selected(), 0);
}
