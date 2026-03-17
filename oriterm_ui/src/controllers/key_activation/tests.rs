use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::input::{InputEvent, Key, Modifiers, MouseButton};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::KeyActivationController;

fn make_ctx<'a>(
    id: WidgetId,
    interaction: &'a InteractionState,
    actions: &'a mut Vec<WidgetAction>,
    propagation: &'a mut PropagationState,
) -> ControllerCtx<'a> {
    ControllerCtx {
        widget_id: id,
        bounds: Rect::new(0.0, 0.0, 100.0, 30.0),
        interaction,
        actions,
        requests: ControllerRequests::NONE,
        now: Instant::now(),
        propagation,
    }
}

#[test]
fn enter_emits_clicked() {
    let id = WidgetId::next();
    let mut ctrl = KeyActivationController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyDown {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);
    let reqs = ctx.requests;
    drop(ctx);

    assert!(handled);
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], WidgetAction::Clicked(wid) if wid == id));
    assert!(reqs.contains(ControllerRequests::PAINT));
}

#[test]
fn space_emits_clicked() {
    let id = WidgetId::next();
    let mut ctrl = KeyActivationController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyDown {
        key: Key::Space,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);
    drop(ctx);

    assert!(handled);
    assert_eq!(actions.len(), 1);
    assert!(matches!(actions[0], WidgetAction::Clicked(wid) if wid == id));
}

#[test]
fn enter_keyup_consumed_silently() {
    let id = WidgetId::next();
    let mut ctrl = KeyActivationController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyUp {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);
    drop(ctx);

    assert!(handled);
    assert!(actions.is_empty(), "KeyUp should not emit actions");
}

#[test]
fn space_keyup_consumed() {
    let id = WidgetId::next();
    let mut ctrl = KeyActivationController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyUp {
        key: Key::Space,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);
    drop(ctx);

    assert!(handled);
    assert!(actions.is_empty());
}

#[test]
fn other_keys_ignored() {
    let id = WidgetId::next();
    let mut ctrl = KeyActivationController::new();
    let interaction = InteractionState::default();

    for key in [Key::ArrowUp, Key::Escape, Key::Tab, Key::Backspace] {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let event = InputEvent::KeyDown {
            key,
            modifiers: Modifiers::NONE,
        };
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let handled = ctrl.handle_event(&event, &mut ctx);
        drop(ctx);
        assert!(!handled, "key {key:?} should not be handled");
        assert!(actions.is_empty());
    }
}

#[test]
fn mouse_events_ignored() {
    let id = WidgetId::next();
    let mut ctrl = KeyActivationController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::MouseDown {
        pos: crate::geometry::Point::new(10.0, 10.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
    let handled = ctrl.handle_event(&event, &mut ctx);
    drop(ctx);

    assert!(!handled);
    assert!(actions.is_empty());
}
