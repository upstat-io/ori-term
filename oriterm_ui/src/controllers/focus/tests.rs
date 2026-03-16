use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, Key, Modifiers, MouseButton};
use crate::interaction::InteractionState;
use crate::interaction::lifecycle::LifecycleEvent;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::FocusController;

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
fn tab_sets_focus_next() {
    let id = WidgetId::next();
    let mut ctrl = FocusController::new();
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers::NONE,
    };
    let consumed = ctrl.handle_event(&event, &mut ctx);

    assert!(consumed);
    assert!(ctx.requests.contains(ControllerRequests::FOCUS_NEXT));
    assert!(!ctx.requests.contains(ControllerRequests::FOCUS_PREV));
}

#[test]
fn shift_tab_sets_focus_prev() {
    let id = WidgetId::next();
    let mut ctrl = FocusController::new();
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers::SHIFT_ONLY,
    };
    let consumed = ctrl.handle_event(&event, &mut ctx);

    assert!(consumed);
    assert!(ctx.requests.contains(ControllerRequests::FOCUS_PREV));
    assert!(!ctx.requests.contains(ControllerRequests::FOCUS_NEXT));
}

#[test]
fn key_up_tab_consumed() {
    let id = WidgetId::next();
    let mut ctrl = FocusController::new();
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::KeyUp {
        key: Key::Tab,
        modifiers: Modifiers::NONE,
    };
    let consumed = ctrl.handle_event(&event, &mut ctx);

    assert!(consumed);
    // No requests set — consuming the up event is just to prevent leaking.
    assert!(ctx.requests.is_empty());
}

#[test]
fn mouse_down_requests_focus() {
    let id = WidgetId::next();
    let mut ctrl = FocusController::new();
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::MouseDown {
        pos: Point::new(10.0, 10.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let consumed = ctrl.handle_event(&event, &mut ctx);

    // Does NOT consume — lets ClickController also handle the press.
    assert!(!consumed);
    assert!(ctx.requests.contains(ControllerRequests::REQUEST_FOCUS));
}

#[test]
fn focus_changed_requests_paint() {
    let id = WidgetId::next();
    let mut ctrl = FocusController::new();
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = LifecycleEvent::FocusChanged {
        widget_id: id,
        is_focused: true,
    };
    ctrl.handle_lifecycle(&event, &mut ctx);

    assert!(ctx.requests.contains(ControllerRequests::PAINT));
}

#[test]
fn non_tab_keys_not_consumed() {
    let id = WidgetId::next();
    let mut ctrl = FocusController::new();
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::KeyDown {
        key: Key::Enter,
        modifiers: Modifiers::NONE,
    };
    let consumed = ctrl.handle_event(&event, &mut ctx);

    assert!(!consumed);
    assert!(ctx.requests.is_empty());
}

#[test]
fn tab_index_accessor() {
    let ctrl = FocusController::new();
    assert_eq!(ctrl.tab_index(), None);

    let ctrl = FocusController::new().with_tab_index(5);
    assert_eq!(ctrl.tab_index(), Some(5));
}
