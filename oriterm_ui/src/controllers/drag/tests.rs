use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, Modifiers, MouseButton};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::DragController;

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

#[test]
fn small_move_no_drag_start() {
    let id = WidgetId::next();
    let mut ctrl = DragController::new();
    let interaction = InteractionState::default();

    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }

    // Small move (within 4px threshold).
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_move(Point::new(12.0, 12.0)), &mut ctx);
        assert!(ctx.actions.is_empty());
    }
}

#[test]
fn threshold_exceeded_emits_drag_start() {
    let id = WidgetId::next();
    let mut ctrl = DragController::new();
    let interaction = InteractionState::default();

    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }

    // Large move (exceeds 4px threshold).
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_move(Point::new(20.0, 20.0)), &mut ctx);
        assert_eq!(ctx.actions.len(), 1);
        assert!(matches!(ctx.actions[0], WidgetAction::DragStart { .. }));
    }
}

#[test]
fn drag_update_has_correct_deltas() {
    let id = WidgetId::next();
    let mut ctrl = DragController::new();
    let interaction = InteractionState::default();

    // Mouse down at (10, 10).
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }

    // Move to (20, 20) — exceeds threshold, DragStart.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_move(Point::new(20.0, 20.0)), &mut ctx);
        assert!(matches!(ctx.actions[0], WidgetAction::DragStart { .. }));
    }

    // Move to (25, 30) — DragUpdate.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_move(Point::new(25.0, 30.0)), &mut ctx);
        assert_eq!(ctx.actions.len(), 1);
        match &ctx.actions[0] {
            WidgetAction::DragUpdate {
                delta, total_delta, ..
            } => {
                // delta = (25-20, 30-20) = (5, 10)
                assert!((delta.x - 5.0).abs() < 0.01);
                assert!((delta.y - 10.0).abs() < 0.01);
                // total_delta = (25-20, 30-20) = (5, 10) from start_pos
                assert!((total_delta.x - 5.0).abs() < 0.01);
                assert!((total_delta.y - 10.0).abs() < 0.01);
            }
            other => panic!("expected DragUpdate, got {other:?}"),
        }
    }
}

#[test]
fn mouse_up_while_dragging_emits_drag_end() {
    let id = WidgetId::next();
    let mut ctrl = DragController::new();
    let interaction = InteractionState::default();

    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }

    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_move(Point::new(20.0, 20.0)), &mut ctx);
    }

    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_up(Point::new(25.0, 25.0)), &mut ctx);
        assert_eq!(ctx.actions.len(), 1);
        assert!(matches!(ctx.actions[0], WidgetAction::DragEnd { .. }));
        assert!(ctx.requests.contains(ControllerRequests::CLEAR_ACTIVE));
    }
}

#[test]
fn mouse_up_while_pending_no_drag_events() {
    let id = WidgetId::next();
    let mut ctrl = DragController::new();
    let interaction = InteractionState::default();

    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }

    // Small move (stays pending).
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_move(Point::new(11.0, 11.0)), &mut ctx);
        assert!(ctx.actions.is_empty());
    }

    // Mouse up — no drag events.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_up(Point::new(11.0, 11.0)), &mut ctx);
        assert!(ctx.actions.is_empty());
    }
}

#[test]
fn reset_clears_drag_state() {
    let id = WidgetId::next();
    let mut ctrl = DragController::new();
    let interaction = InteractionState::default();

    // Start drag.
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_down(Point::new(10.0, 10.0)), &mut ctx);
    }
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        ctrl.handle_event(&mouse_move(Point::new(20.0, 20.0)), &mut ctx);
    }

    // Reset mid-drag.
    ctrl.reset();

    // Subsequent mouse move should be ignored (Idle).
    {
        let mut actions = Vec::new();
        let mut prop = PropagationState::default();
        let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);
        let consumed = ctrl.handle_event(&mouse_move(Point::new(25.0, 25.0)), &mut ctx);
        assert!(!consumed);
        assert!(ctx.actions.is_empty());
    }
}
