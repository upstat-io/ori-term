use std::time::Instant;

use crate::action::WidgetAction;
use crate::input::{InputEvent, Key, Modifiers};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::TextEditController;

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
fn insert_character() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("ab");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyDown {
        key: Key::Character('c'),
        modifiers: Modifiers::NONE,
    };
    let (consumed, requests) = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let c = ctrl.handle_event(&event, &mut ctx);
        (c, ctx.requests)
    };

    assert!(consumed);
    assert_eq!(ctrl.text(), "abc");
    assert_eq!(ctrl.cursor(), 3);
    assert_eq!(
        actions,
        vec![WidgetAction::TextChanged {
            id,
            text: "abc".into()
        }]
    );
    assert!(requests.contains(ControllerRequests::PAINT));
}

#[test]
fn backspace_deletes_previous_char() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("abc");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyDown {
        key: Key::Backspace,
        modifiers: Modifiers::NONE,
    };
    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&event, &mut ctx)
    };

    assert!(consumed);
    assert_eq!(ctrl.text(), "ab");
    assert_eq!(ctrl.cursor(), 2);
    assert_eq!(
        actions,
        vec![WidgetAction::TextChanged {
            id,
            text: "ab".into()
        }]
    );
}

#[test]
fn backspace_at_start_no_change() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let event = InputEvent::KeyDown {
        key: Key::Backspace,
        modifiers: Modifiers::NONE,
    };
    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&event, &mut ctx)
    };

    assert!(consumed);
    assert!(actions.is_empty());
}

#[test]
fn delete_removes_next_char() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("abc");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    // Move cursor to start via Home.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let home = InputEvent::KeyDown {
            key: Key::Home,
            modifiers: Modifiers::NONE,
        };
        ctrl.handle_event(&home, &mut ctx);
    }
    assert_eq!(ctrl.cursor(), 0);
    actions.clear();

    // Delete.
    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let event = InputEvent::KeyDown {
            key: Key::Delete,
            modifiers: Modifiers::NONE,
        };
        ctrl.handle_event(&event, &mut ctx)
    };

    assert!(consumed);
    assert_eq!(ctrl.text(), "bc");
    assert_eq!(ctrl.cursor(), 0);
    assert_eq!(
        actions,
        vec![WidgetAction::TextChanged {
            id,
            text: "bc".into()
        }]
    );
}

#[test]
fn arrow_left_moves_cursor() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("abc");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let event = InputEvent::KeyDown {
            key: Key::ArrowLeft,
            modifiers: Modifiers::NONE,
        };
        ctrl.handle_event(&event, &mut ctx)
    };

    assert!(consumed);
    assert_eq!(ctrl.cursor(), 2);
    assert!(actions.is_empty());
}

#[test]
fn arrow_right_at_end_stays() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("abc");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let event = InputEvent::KeyDown {
            key: Key::ArrowRight,
            modifiers: Modifiers::NONE,
        };
        ctrl.handle_event(&event, &mut ctx)
    };

    assert!(consumed);
    assert_eq!(ctrl.cursor(), 3); // Still at end.
}

#[test]
fn home_moves_to_start() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let event = InputEvent::KeyDown {
            key: Key::Home,
            modifiers: Modifiers::NONE,
        };
        ctrl.handle_event(&event, &mut ctx);
    }

    assert_eq!(ctrl.cursor(), 0);
}

#[test]
fn end_after_home_moves_to_end() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    // Home first.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Home,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }
    assert_eq!(ctrl.cursor(), 0);

    // Then End.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::End,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }
    assert_eq!(ctrl.cursor(), 5);
}

#[test]
fn shift_arrow_creates_selection() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::ArrowLeft,
                modifiers: Modifiers::SHIFT_ONLY,
            },
            &mut ctx,
        );
    }

    assert_eq!(ctrl.cursor(), 4);
    assert_eq!(ctrl.selection_range(), Some((4, 5)));
}

#[test]
fn ctrl_a_selects_all() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('a'),
                modifiers: Modifiers::CTRL_ONLY,
            },
            &mut ctx,
        )
    };

    assert!(consumed);
    assert_eq!(ctrl.selection_range(), Some((0, 5)));
}

#[test]
fn backspace_deletes_selection() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    // Select all.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('a'),
                modifiers: Modifiers::CTRL_ONLY,
            },
            &mut ctx,
        );
    }
    actions.clear();

    // Backspace.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Backspace,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }

    assert_eq!(ctrl.text(), "");
    assert_eq!(ctrl.cursor(), 0);
    assert_eq!(
        actions,
        vec![WidgetAction::TextChanged {
            id,
            text: String::new()
        }]
    );
}

#[test]
fn unhandled_key_not_consumed() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
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

    assert!(!consumed);
}

#[test]
fn ctrl_c_not_consumed() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('c'),
                modifiers: Modifiers::CTRL_ONLY,
            },
            &mut ctx,
        )
    };

    // Clipboard deferred to app layer.
    assert!(!consumed);
}

#[test]
fn key_up_for_handled_keys_consumed() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let bs_consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::Backspace,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(bs_consumed);

    let left_consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::ArrowLeft,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(left_consumed);

    // Unhandled key-up not consumed.
    let esc_consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyUp {
                key: Key::Escape,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(!esc_consumed);
}

#[test]
fn mouse_events_not_consumed() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    let consumed = {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::MouseDown {
                pos: crate::geometry::Point::new(10.0, 10.0),
                button: crate::input::MouseButton::Left,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        )
    };
    assert!(!consumed);
}

#[test]
fn reset_clears_selection() {
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");

    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    // Select all.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('a'),
                modifiers: Modifiers::CTRL_ONLY,
            },
            &mut ctx,
        );
    }
    assert!(ctrl.selection_range().is_some());

    ctrl.reset();
    assert!(ctrl.selection_range().is_none());
    // Text and cursor are preserved.
    assert_eq!(ctrl.text(), "hello");
}

#[test]
fn multibyte_character_handling() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    // Insert a multi-byte character (e-acute, 2 bytes in UTF-8).
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('\u{00E9}'),
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }
    assert_eq!(ctrl.text(), "\u{00E9}");
    assert_eq!(ctrl.cursor(), 2);

    // Arrow left moves past the full character.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::ArrowLeft,
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }
    assert_eq!(ctrl.cursor(), 0);
}

#[test]
fn insert_replaces_selection() {
    let id = WidgetId::next();
    let mut ctrl = TextEditController::new();
    ctrl.set_text("hello");
    let interaction = InteractionState::default();
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();

    // Select all.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('a'),
                modifiers: Modifiers::CTRL_ONLY,
            },
            &mut ctx,
        );
    }
    actions.clear();

    // Type replacement.
    {
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(
            &InputEvent::KeyDown {
                key: Key::Character('X'),
                modifiers: Modifiers::NONE,
            },
            &mut ctx,
        );
    }

    assert_eq!(ctrl.text(), "X");
    assert_eq!(ctrl.cursor(), 1);
    assert_eq!(
        actions,
        vec![WidgetAction::TextChanged {
            id,
            text: "X".into()
        }]
    );
}
