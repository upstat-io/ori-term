use std::time::{Duration, Instant};

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, Modifiers, MouseButton};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::ClickController;

fn make_ctx_at<'a>(
    id: WidgetId,
    interaction: &'a InteractionState,
    actions: &'a mut Vec<WidgetAction>,
    propagation: &'a mut PropagationState,
    now: Instant,
) -> ControllerCtx<'a> {
    ControllerCtx {
        widget_id: id,
        bounds: crate::geometry::Rect::default(),
        interaction,
        actions,
        requests: ControllerRequests::NONE,
        now,
        propagation,
    }
}

fn mouse_down(pos: Point) -> InputEvent {
    InputEvent::MouseDown {
        pos,
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    }
}

fn mouse_up(pos: Point) -> InputEvent {
    InputEvent::MouseUp {
        pos,
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    }
}

fn mouse_move(pos: Point) -> InputEvent {
    InputEvent::MouseMove {
        pos,
        modifiers: Modifiers::NONE,
    }
}

/// Helper: sends mouse_down + mouse_up and returns the actions from the up event.
fn click(
    ctrl: &mut ClickController,
    id: WidgetId,
    interaction: &InteractionState,
    pos: Point,
    now: Instant,
) -> Vec<WidgetAction> {
    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx_at(id, interaction, &mut actions, &mut prop, now);
    ctrl.handle_event(&mouse_down(pos), &mut ctx);
    drop(ctx);

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx_at(id, interaction, &mut actions, &mut prop, now);
    ctrl.handle_event(&mouse_up(pos), &mut ctx);
    drop(ctx);

    actions
}

#[test]
fn single_click() {
    let id = WidgetId::next();
    let mut ctrl = ClickController::new();
    let interaction = InteractionState::default();
    let now = Instant::now();
    let origin = Point::new(10.0, 10.0);

    // Mouse down — check SET_ACTIVE.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx_at(id, &interaction, &mut actions, &mut prop, now);
        ctrl.handle_event(&mouse_down(origin), &mut ctx);
        assert!(ctx.requests.contains(ControllerRequests::SET_ACTIVE));
    }

    // Mouse up — check Clicked + CLEAR_ACTIVE.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx_at(id, &interaction, &mut actions, &mut prop, now);
        ctrl.handle_event(&mouse_up(origin), &mut ctx);
        assert_eq!(ctx.actions.len(), 1);
        assert_eq!(ctx.actions[0], WidgetAction::Clicked(id));
        assert!(ctx.requests.contains(ControllerRequests::CLEAR_ACTIVE));
    }
}

#[test]
fn double_click() {
    let id = WidgetId::next();
    let mut ctrl = ClickController::new();
    let interaction = InteractionState::default();
    let now = Instant::now();
    let origin = Point::new(10.0, 10.0);

    // First click.
    let first = click(&mut ctrl, id, &interaction, origin, now);
    assert_eq!(first[0], WidgetAction::Clicked(id));

    // Second click within timeout.
    let now2 = now + Duration::from_millis(200);
    let second = click(&mut ctrl, id, &interaction, origin, now2);
    assert_eq!(second[0], WidgetAction::DoubleClicked(id));
}

#[test]
fn triple_click() {
    let id = WidgetId::next();
    let mut ctrl = ClickController::new();
    let interaction = InteractionState::default();
    let now = Instant::now();
    let origin = Point::new(10.0, 10.0);

    let expected = [
        WidgetAction::Clicked(id),
        WidgetAction::DoubleClicked(id),
        WidgetAction::TripleClicked(id),
    ];

    for (i, expected_action) in expected.iter().enumerate() {
        let t = now + Duration::from_millis(100 * i as u64);
        let actions = click(&mut ctrl, id, &interaction, origin, t);
        assert_eq!(&actions[0], expected_action, "click {i}");
    }
}

#[test]
fn click_cancelled_by_drag() {
    let id = WidgetId::next();
    let mut ctrl = ClickController::new();
    let interaction = InteractionState::default();
    let now = Instant::now();

    // Mouse down.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx_at(id, &interaction, &mut actions, &mut prop, now);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }

    // Move far away (exceeds 4px threshold).
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx_at(id, &interaction, &mut actions, &mut prop, now);
        ctrl.handle_event(&mouse_move(Point::new(50.0, 50.0)), &mut ctx);
    }

    // Mouse up — no click emitted.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx_at(id, &interaction, &mut actions, &mut prop, now);
        ctrl.handle_event(&mouse_up(Point::new(50.0, 50.0)), &mut ctx);
        assert!(ctx.actions.is_empty());
    }
}

#[test]
fn timeout_resets_click_count() {
    let id = WidgetId::next();
    let mut ctrl = ClickController::new();
    let interaction = InteractionState::default();
    let now = Instant::now();
    let origin = Point::new(10.0, 10.0);

    // First click.
    let first = click(&mut ctrl, id, &interaction, origin, now);
    assert_eq!(first[0], WidgetAction::Clicked(id));

    // Second click AFTER timeout — should be Clicked (reset), not DoubleClicked.
    let late = now + Duration::from_millis(600);
    let second = click(&mut ctrl, id, &interaction, origin, late);
    assert_eq!(second[0], WidgetAction::Clicked(id));
}

#[test]
fn reset_clears_state() {
    let id = WidgetId::next();
    let mut ctrl = ClickController::new();
    let interaction = InteractionState::default();
    let now = Instant::now();

    // Start a click.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx_at(id, &interaction, &mut actions, &mut prop, now);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }

    // Reset.
    ctrl.reset();

    // Mouse up should NOT produce a click (press_pos was cleared).
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx_at(id, &interaction, &mut actions, &mut prop, now);
        ctrl.handle_event(&mouse_up(Point::new(10.0, 10.0)), &mut ctx);
        assert!(ctx.actions.is_empty());
    }
}
