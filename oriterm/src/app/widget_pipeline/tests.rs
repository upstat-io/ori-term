use std::time::Instant;

use oriterm_ui::action::WidgetAction;
use oriterm_ui::controllers::{ControllerCtx, ControllerRequests, EventController};
use oriterm_ui::geometry::Rect;
use oriterm_ui::input::dispatch::DeliveryAction;
use oriterm_ui::input::{EventPhase, InputEvent, Modifiers, MouseButton};
use oriterm_ui::interaction::{InteractionManager, InteractionState, LifecycleEvent};
use oriterm_ui::layout::LayoutBox;
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::contexts::{DrawCtx, LayoutCtx};

use super::{DispatchResult, dispatch_step, prepare_widget_frame};

// -- Test helpers --

/// Minimal widget for testing the pipeline.
struct StubWidget {
    id: WidgetId,
    controllers: Vec<Box<dyn EventController>>,
    lifecycle_calls: Vec<LifecycleEvent>,
}

impl StubWidget {
    fn new(id: WidgetId) -> Self {
        Self {
            id,
            controllers: Vec::new(),
            lifecycle_calls: Vec::new(),
        }
    }

    fn with_controller(mut self, c: impl EventController + 'static) -> Self {
        self.controllers.push(Box::new(c));
        self
    }
}

impl Widget for StubWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(0.0, 0.0)
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {}

    fn controllers(&self) -> &[Box<dyn EventController>] {
        &self.controllers
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>] {
        &mut self.controllers
    }

    fn lifecycle(
        &mut self,
        event: &LifecycleEvent,
        _ctx: &mut oriterm_ui::widgets::contexts::LifecycleCtx<'_>,
    ) {
        self.lifecycle_calls.push(event.clone());
    }
}

/// Controller that handles all events and emits a `Clicked` action.
struct HandleAllController;

impl EventController for HandleAllController {
    fn handle_event(&mut self, _event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        ctx.emit_action(WidgetAction::Clicked(ctx.widget_id));
        true
    }
}

/// Controller that ignores events (returns false).
struct IgnoreController;

impl EventController for IgnoreController {
    fn handle_event(&mut self, _event: &InputEvent, _ctx: &mut ControllerCtx<'_>) -> bool {
        false
    }
}

/// Controller that requests `SET_ACTIVE`.
struct CaptureController;

impl EventController for CaptureController {
    fn handle_event(&mut self, _event: &InputEvent, ctx: &mut ControllerCtx<'_>) -> bool {
        ctx.requests = ControllerRequests::SET_ACTIVE;
        true
    }
}

fn make_mouse_down() -> InputEvent {
    InputEvent::MouseDown {
        pos: oriterm_ui::geometry::Point::new(10.0, 10.0),
        button: MouseButton::Left,
        modifiers: Modifiers::NONE,
    }
}

fn make_delivery(id: WidgetId, phase: EventPhase) -> DeliveryAction {
    DeliveryAction {
        widget_id: id,
        phase,
        bounds: Rect::default(),
    }
}

// -- Tests --

#[test]
fn dispatch_result_new_is_empty() {
    let r = DispatchResult::new();
    assert!(!r.handled);
    assert!(r.actions.is_empty());
    assert!(r.requests.is_empty());
    assert!(r.source.is_none());
}

#[test]
fn dispatch_step_routes_to_controller() {
    let id = WidgetId::next();
    let mut widget = StubWidget::new(id).with_controller(HandleAllController);
    let mut interaction = InteractionManager::new();
    interaction.register_widget(id);
    let state = interaction.get_state(id);

    let event = make_mouse_down();
    let action = make_delivery(id, EventPhase::Target);
    let mut result = DispatchResult::new();

    let handled = dispatch_step(
        &mut result,
        &event,
        &action,
        &mut widget,
        state,
        Instant::now(),
    );

    assert!(handled);
    assert!(result.handled);
    assert_eq!(result.source, Some(id));
    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.actions[0], WidgetAction::Clicked(id));
}

#[test]
fn dispatch_step_unhandled_does_not_stop() {
    let id = WidgetId::next();
    let mut widget = StubWidget::new(id).with_controller(IgnoreController);
    let interaction = InteractionState::new();

    let event = make_mouse_down();
    let action = make_delivery(id, EventPhase::Target);
    let mut result = DispatchResult::new();

    let handled = dispatch_step(
        &mut result,
        &event,
        &action,
        &mut widget,
        &interaction,
        Instant::now(),
    );

    assert!(!handled);
    assert!(!result.handled);
    assert!(result.source.is_none());
}

#[test]
fn dispatch_step_accumulates_requests() {
    let id = WidgetId::next();
    let mut widget = StubWidget::new(id).with_controller(CaptureController);
    let interaction = InteractionState::new();

    let event = make_mouse_down();
    let action = make_delivery(id, EventPhase::Target);
    let mut result = DispatchResult::new();

    dispatch_step(
        &mut result,
        &event,
        &action,
        &mut widget,
        &interaction,
        Instant::now(),
    );

    assert!(result.requests.contains(ControllerRequests::SET_ACTIVE));
}

#[test]
fn multi_step_delivery_stops_on_handled() {
    let id_a = WidgetId::next();
    let id_b = WidgetId::next();
    let mut widget_a = StubWidget::new(id_a).with_controller(HandleAllController);
    let mut widget_b = StubWidget::new(id_b).with_controller(HandleAllController);
    let state = InteractionState::new();

    let event = make_mouse_down();
    let actions = [
        make_delivery(id_a, EventPhase::Target),
        make_delivery(id_b, EventPhase::Target),
    ];
    let mut result = DispatchResult::new();

    // Widget A handles it — loop stops before B.
    let stop_a = dispatch_step(
        &mut result,
        &event,
        &actions[0],
        &mut widget_a,
        &state,
        Instant::now(),
    );
    assert!(stop_a);

    // Widget B is never reached.
    assert_eq!(result.actions.len(), 1);
    assert_eq!(result.source, Some(id_a));

    // Demonstrate that if we DID call B, it would add another action.
    let stop_b = dispatch_step(
        &mut result,
        &event,
        &actions[1],
        &mut widget_b,
        &state,
        Instant::now(),
    );
    assert!(stop_b);
    assert_eq!(result.actions.len(), 2);
    // Source stays as A (first handler).
    assert_eq!(result.source, Some(id_a));
}

#[test]
fn prepare_widget_frame_delivers_lifecycle() {
    let id = WidgetId::next();
    let mut widget = StubWidget::new(id);
    let mut interaction = InteractionManager::new();
    interaction.register_widget(id);
    // Drain the WidgetAdded event from registration.
    let _ = interaction.drain_events();

    // Simulate a HotChanged event.
    let events = vec![LifecycleEvent::HotChanged {
        widget_id: id,
        is_hot: true,
    }];

    prepare_widget_frame(
        &mut widget,
        &interaction,
        &events,
        None,
        None,
        Instant::now(),
    );

    assert_eq!(widget.lifecycle_calls.len(), 1);
    assert_eq!(
        widget.lifecycle_calls[0],
        LifecycleEvent::HotChanged {
            widget_id: id,
            is_hot: true,
        }
    );
}

#[test]
fn prepare_widget_frame_skips_other_widget_events() {
    let id_a = WidgetId::next();
    let id_b = WidgetId::next();
    let mut widget = StubWidget::new(id_a);
    let mut interaction = InteractionManager::new();
    interaction.register_widget(id_a);
    let _ = interaction.drain_events();

    // Event targets id_b, not id_a.
    let events = vec![LifecycleEvent::HotChanged {
        widget_id: id_b,
        is_hot: true,
    }];

    prepare_widget_frame(
        &mut widget,
        &interaction,
        &events,
        None,
        None,
        Instant::now(),
    );

    assert!(widget.lifecycle_calls.is_empty());
}

#[test]
fn lifecycle_event_widget_id() {
    let id = WidgetId::next();

    let events = [
        LifecycleEvent::HotChanged {
            widget_id: id,
            is_hot: true,
        },
        LifecycleEvent::ActiveChanged {
            widget_id: id,
            is_active: false,
        },
        LifecycleEvent::FocusChanged {
            widget_id: id,
            is_focused: true,
        },
        LifecycleEvent::WidgetAdded { widget_id: id },
        LifecycleEvent::WidgetRemoved { widget_id: id },
        LifecycleEvent::WidgetDisabled {
            widget_id: id,
            disabled: true,
        },
    ];

    for event in &events {
        assert_eq!(event.widget_id(), id, "widget_id() mismatch for {event:?}");
    }
}
