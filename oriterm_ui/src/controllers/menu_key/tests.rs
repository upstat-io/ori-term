use std::time::Instant;

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key, Modifiers};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::MenuKeyController;

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

/// Helper: clickable indices for a menu with items at 0, 1, 3
/// (index 2 is a separator).
fn sample_clickable() -> Vec<usize> {
    vec![0, 1, 3]
}

#[test]
fn arrow_down_navigates_forward() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
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
    assert_eq!(ctrl.hovered(), Some(0));
    assert!(requests.contains(ControllerRequests::PAINT));
}

#[test]
fn arrow_down_skips_separator() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
    ctrl.set_hovered(Some(1)); // Before separator.
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

    // Skips index 2 (separator), lands on 3.
    assert_eq!(ctrl.hovered(), Some(3));
}

#[test]
fn arrow_down_wraps_around() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
    ctrl.set_hovered(Some(3)); // Last clickable.
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

    assert_eq!(ctrl.hovered(), Some(0));
}

#[test]
fn arrow_up_navigates_backward() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
    ctrl.set_hovered(Some(1));
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

    assert_eq!(ctrl.hovered(), Some(0));
}

#[test]
fn arrow_up_wraps_around() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
    ctrl.set_hovered(Some(0));
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

    assert_eq!(ctrl.hovered(), Some(3));
}

#[test]
fn arrow_up_from_none_starts_at_last() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
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

    assert_eq!(ctrl.hovered(), Some(3));
}

#[test]
fn enter_selects_hovered() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
    ctrl.set_hovered(Some(1));
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
fn space_selects_hovered() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
    ctrl.set_hovered(Some(3));
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Space,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };

    assert!(consumed);
    assert_eq!(actions, vec![WidgetAction::Selected { id, index: 3 }]);
}

#[test]
fn enter_with_no_hover_emits_nothing() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
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
    assert!(actions.is_empty());
}

#[test]
fn escape_dismisses_overlay() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
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
    let mut ctrl = MenuKeyController::new(sample_clickable());
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
}

#[test]
fn key_up_for_handled_keys_consumed() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(sample_clickable());
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let down = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::ArrowDown,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(down);

    let space = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::Space,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(space);

    let esc = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::Escape,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(esc);

    let x = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::Character('x'),
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(!x);
}

#[test]
fn empty_clickable_list() {
    let id = WidgetId::next();
    let mut ctrl = MenuKeyController::new(vec![]);
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

    assert_eq!(ctrl.hovered(), None);
}

#[test]
fn reset_clears_hover() {
    let mut ctrl = MenuKeyController::new(sample_clickable());
    ctrl.set_hovered(Some(1));

    ctrl.reset();
    assert_eq!(ctrl.hovered(), None);
}
