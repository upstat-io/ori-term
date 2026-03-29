use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::Point;
use crate::input::{InputEvent, Modifiers, ScrollDelta};
use crate::interaction::InteractionState;
use crate::widget_id::WidgetId;

use super::super::{ControllerCtx, ControllerRequests, EventController, PropagationState};
use super::ScrollController;

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
fn lines_converted_to_pixels() {
    let id = WidgetId::next();
    let line_height = 20.0;
    let mut ctrl = ScrollController::new(line_height);
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::Scroll {
        pos: Point::new(50.0, 50.0),
        delta: ScrollDelta::Lines { x: 0.0, y: 3.0 },
        modifiers: Modifiers::NONE,
    };

    let consumed = ctrl.handle_event(&event, &mut ctx);
    assert!(consumed);
    assert_eq!(actions.len(), 1);

    match &actions[0] {
        WidgetAction::ScrollBy {
            delta_x, delta_y, ..
        } => {
            assert!((delta_x - 0.0).abs() < 0.01);
            assert!((delta_y - 60.0).abs() < 0.01); // 3.0 * 20.0
        }
        _ => panic!("expected ScrollBy"),
    }
}

#[test]
fn pixels_passed_through() {
    let id = WidgetId::next();
    let mut ctrl = ScrollController::new(20.0);
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::Scroll {
        pos: Point::new(50.0, 50.0),
        delta: ScrollDelta::Pixels { x: 5.0, y: 42.0 },
        modifiers: Modifiers::NONE,
    };

    ctrl.handle_event(&event, &mut ctx);

    match &actions[0] {
        WidgetAction::ScrollBy {
            delta_x, delta_y, ..
        } => {
            assert!((delta_x - 5.0).abs() < 0.01);
            assert!((delta_y - 42.0).abs() < 0.01);
        }
        _ => panic!("expected ScrollBy"),
    }
}

#[test]
fn non_scroll_events_ignored() {
    let id = WidgetId::next();
    let mut ctrl = ScrollController::new(20.0);
    let interaction = InteractionState::default();

    let mut actions = Vec::new();
    let mut prop = PropagationState::default();
    let mut ctx = make_ctx(id, &interaction, &mut actions, &mut prop);

    let event = InputEvent::MouseMove {
        pos: Point::new(10.0, 10.0),
        modifiers: Modifiers::NONE,
    };

    let consumed = ctrl.handle_event(&event, &mut ctx);
    assert!(!consumed);
    assert!(actions.is_empty());
}
