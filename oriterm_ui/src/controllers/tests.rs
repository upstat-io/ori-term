use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::{Point, Rect};
use crate::input::{EventPhase, InputEvent, Modifiers, MouseButton};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::{
    ControllerCtx, ControllerCtxArgs, ControllerRequests, EventController, PropagationState,
    dispatch_to_controllers,
};

// A test controller that always consumes events in a configurable phase.
struct AlwaysHandleController {
    declared_phase: EventPhase,
}

impl EventController for AlwaysHandleController {
    fn phase(&self) -> EventPhase {
        self.declared_phase
    }

    fn handle_event(&mut self, _event: &InputEvent, _ctx: &mut ControllerCtx<'_>) -> bool {
        true
    }
}

fn make_args_now(id: WidgetId, interaction: &InteractionState) -> ControllerCtxArgs<'_> {
    ControllerCtxArgs {
        widget_id: id,
        bounds: Rect::default(),
        interaction,
        now: Instant::now(),
    }
}

fn mouse_down_at(pos: Point) -> InputEvent {
    InputEvent::MouseDown {
        pos,
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    }
}

struct EmitOnCallController {
    declared_phase: EventPhase,
}

impl EventController for EmitOnCallController {
    fn phase(&self) -> EventPhase {
        self.declared_phase
    }

    fn handle_event(&mut self, _event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        ctx.emit_action(WidgetAction::Clicked(ctx.widget_id));
        false
    }
}

#[test]
fn capture_phase_controller_only_invoked_during_capture() {
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let event = mouse_down_at(Point::new(10.0, 10.0));

    let mut controllers: Vec<Box<dyn EventController>> = vec![Box::new(EmitOnCallController {
        declared_phase: EventPhase::Capture,
    })];

    // Capture phase — should emit action.
    let result = dispatch_to_controllers(
        &mut controllers,
        &event,
        EventPhase::Capture,
        &make_args_now(id, &interaction),
    );
    assert_eq!(result.actions.len(), 1);

    // Bubble phase — should NOT emit action (wrong phase).
    let result = dispatch_to_controllers(
        &mut controllers,
        &event,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );
    assert!(result.actions.is_empty());
}

#[test]
fn target_phase_invokes_all_controllers() {
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let event = mouse_down_at(Point::new(10.0, 10.0));

    let mut controllers: Vec<Box<dyn EventController>> = vec![
        Box::new(EmitOnCallController {
            declared_phase: EventPhase::Capture,
        }),
        Box::new(EmitOnCallController {
            declared_phase: EventPhase::Bubble,
        }),
    ];

    // Target phase invokes ALL controllers regardless of declared phase.
    let result = dispatch_to_controllers(
        &mut controllers,
        &event,
        EventPhase::Target,
        &make_args_now(id, &interaction),
    );
    assert_eq!(result.actions.len(), 2);
}

#[test]
fn all_controllers_on_same_widget_see_event() {
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let event = mouse_down_at(Point::new(10.0, 10.0));

    // First controller handles (returns true), second should still run.
    let mut controllers: Vec<Box<dyn EventController>> = vec![
        Box::new(AlwaysHandleController {
            declared_phase: EventPhase::Bubble,
        }),
        Box::new(EmitOnCallController {
            declared_phase: EventPhase::Bubble,
        }),
    ];

    let result = dispatch_to_controllers(
        &mut controllers,
        &event,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );

    // Both ran: first handled (no action), second emitted an action.
    assert!(result.handled);
    assert_eq!(result.actions.len(), 1);
}

#[test]
fn requests_accumulated_across_controllers() {
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let event = mouse_down_at(Point::new(10.0, 10.0));

    struct PaintRequestController;
    impl EventController for PaintRequestController {
        fn handle_event(&mut self, _event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
            ctx.requests.insert(ControllerRequests::PAINT);
            false
        }
    }

    struct ActiveRequestController;
    impl EventController for ActiveRequestController {
        fn handle_event(&mut self, _event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
            ctx.requests.insert(ControllerRequests::SET_ACTIVE);
            false
        }
    }

    let mut controllers: Vec<Box<dyn EventController>> = vec![
        Box::new(PaintRequestController),
        Box::new(ActiveRequestController),
    ];

    let result = dispatch_to_controllers(
        &mut controllers,
        &event,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );

    assert!(result.requests.contains(ControllerRequests::PAINT));
    assert!(result.requests.contains(ControllerRequests::SET_ACTIVE));
}

#[test]
fn propagation_state_set_handled_via_ctx() {
    let id = WidgetId::next();
    let interaction = InteractionState::default();
    let event = mouse_down_at(Point::new(10.0, 10.0));

    struct SetHandledController;
    impl EventController for SetHandledController {
        fn handle_event(&mut self, _event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
            ctx.propagation.set_handled();
            false // returns false, but propagation says handled
        }
    }

    let mut controllers: Vec<Box<dyn EventController>> = vec![Box::new(SetHandledController)];

    let result = dispatch_to_controllers(
        &mut controllers,
        &event,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );

    assert!(result.handled);
}

#[test]
fn composition_hover_click_focus() {
    use super::{ClickController, FocusController, HoverController};
    use crate::input::Key;

    let id = WidgetId::next();
    let interaction = InteractionState::default();

    let mut controllers: Vec<Box<dyn EventController>> = vec![
        Box::new(HoverController::new()),
        Box::new(ClickController::new()),
        Box::new(FocusController::new()),
    ];

    // Mouse down — ClickController requests SET_ACTIVE, FocusController requests FOCUS.
    let result = dispatch_to_controllers(
        &mut controllers,
        &mouse_down_at(Point::new(10.0, 10.0)),
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );
    assert!(result.requests.contains(ControllerRequests::SET_ACTIVE));
    assert!(result.requests.contains(ControllerRequests::REQUEST_FOCUS));

    // Mouse up — ClickController emits Clicked and requests CLEAR_ACTIVE.
    let up = InputEvent::MouseUp {
        pos: Point::new(10.0, 10.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result = dispatch_to_controllers(
        &mut controllers,
        &up,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );
    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.actions[0], WidgetAction::Clicked(id));
    assert!(result.requests.contains(ControllerRequests::CLEAR_ACTIVE));

    // Tab key — FocusController requests FOCUS_NEXT.
    let tab = InputEvent::KeyDown {
        key: Key::Tab,
        modifiers: Modifiers::NONE,
    };
    let result = dispatch_to_controllers(
        &mut controllers,
        &tab,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );
    assert!(result.requests.contains(ControllerRequests::FOCUS_NEXT));
}

#[test]
fn click_drag_composition_large_move_produces_drag_not_click() {
    use super::{ClickController, DragController};

    let id = WidgetId::next();
    let interaction = InteractionState::default();

    let mut controllers: Vec<Box<dyn EventController>> = vec![
        Box::new(ClickController::new()),
        Box::new(DragController::new()),
    ];

    // Mouse down.
    let result = dispatch_to_controllers(
        &mut controllers,
        &mouse_down_at(Point::new(10.0, 10.0)),
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );
    assert!(result.requests.contains(ControllerRequests::SET_ACTIVE));

    // Large move — exceeds both thresholds.
    let mv = InputEvent::MouseMove {
        pos: Point::new(50.0, 50.0),
        modifiers: Modifiers::NONE,
    };
    let result = dispatch_to_controllers(
        &mut controllers,
        &mv,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );
    // DragStart emitted.
    assert!(
        result
            .actions
            .iter()
            .any(|a| matches!(a, WidgetAction::DragStart { .. }))
    );

    // Mouse up — should be DragEnd, NOT Clicked.
    let up = InputEvent::MouseUp {
        pos: Point::new(50.0, 50.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    };
    let result = dispatch_to_controllers(
        &mut controllers,
        &up,
        EventPhase::Bubble,
        &make_args_now(id, &interaction),
    );

    // DragEnd should be present, Clicked should NOT.
    assert!(
        result
            .actions
            .iter()
            .any(|a| matches!(a, WidgetAction::DragEnd { .. }))
    );
    assert!(
        !result
            .actions
            .iter()
            .any(|a| matches!(a, WidgetAction::Clicked(_)))
    );
}

#[test]
fn controller_requests_bitmask() {
    let mut r = ControllerRequests::NONE;
    assert!(r.is_empty());

    r.insert(ControllerRequests::PAINT);
    assert!(r.contains(ControllerRequests::PAINT));
    assert!(!r.contains(ControllerRequests::SET_ACTIVE));

    r.insert(ControllerRequests::SET_ACTIVE);
    assert!(r.contains(ControllerRequests::PAINT));
    assert!(r.contains(ControllerRequests::SET_ACTIVE));

    let combined = ControllerRequests::PAINT.union(ControllerRequests::ANIM_FRAME);
    assert!(combined.contains(ControllerRequests::PAINT));
    assert!(combined.contains(ControllerRequests::ANIM_FRAME));
    assert!(!combined.contains(ControllerRequests::SET_ACTIVE));
}

#[test]
fn propagation_state_default_not_handled() {
    let p = PropagationState::default();
    assert!(!p.is_handled());
}

#[test]
fn propagation_state_set_handled() {
    let mut p = PropagationState::default();
    p.set_handled();
    assert!(p.is_handled());
}
