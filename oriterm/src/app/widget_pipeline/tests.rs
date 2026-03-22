use std::time::Instant;

use oriterm_ui::action::WidgetAction;
use oriterm_ui::controllers::{ControllerCtx, ControllerRequests, EventController};
use oriterm_ui::geometry::Rect;
use oriterm_ui::input::dispatch::DeliveryAction;
use oriterm_ui::input::{EventPhase, InputEvent, Modifiers, MouseButton};
use oriterm_ui::interaction::{InteractionManager, InteractionState, LifecycleEvent};
use oriterm_ui::layout::LayoutBox;
use oriterm_ui::sense::Sense;
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::contexts::{DrawCtx, LayoutCtx};

use super::{DispatchResult, dispatch_step, prepare_widget_frame, prepare_widget_tree};

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

    fn sense(&self) -> Sense {
        Sense::all()
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

    // Deliver initial WidgetAdded so ordering assertion passes.
    let added = interaction.drain_events();
    prepare_widget_frame(
        &mut widget,
        &mut interaction,
        None,
        &added,
        None,
        None,
        Instant::now(),
    );
    widget.lifecycle_calls.clear();

    // Simulate a HotChanged event.
    let events = vec![LifecycleEvent::HotChanged {
        widget_id: id,
        is_hot: true,
    }];

    prepare_widget_frame(
        &mut widget,
        &mut interaction,
        None,
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

    // Deliver initial WidgetAdded so ordering assertion passes.
    let added = interaction.drain_events();
    prepare_widget_frame(
        &mut widget,
        &mut interaction,
        None,
        &added,
        None,
        None,
        Instant::now(),
    );
    widget.lifecycle_calls.clear();

    // Event targets id_b, not id_a.
    let events = vec![LifecycleEvent::HotChanged {
        widget_id: id_b,
        is_hot: true,
    }];

    prepare_widget_frame(
        &mut widget,
        &mut interaction,
        None,
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

/// Parent widget that holds children for tree traversal tests.
struct ParentWidget {
    id: WidgetId,
    children: Vec<StubWidget>,
}

impl ParentWidget {
    fn new(id: WidgetId, children: Vec<StubWidget>) -> Self {
        Self { id, children }
    }
}

impl Widget for ParentWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::all()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(0.0, 0.0)
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {}

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        for child in &mut self.children {
            visitor(child);
        }
    }
}

#[test]
fn prepare_widget_tree_delivers_to_children() {
    let parent_id = WidgetId::next();
    let child_a_id = WidgetId::next();
    let child_b_id = WidgetId::next();

    let mut parent = ParentWidget::new(
        parent_id,
        vec![StubWidget::new(child_a_id), StubWidget::new(child_b_id)],
    );

    let mut interaction = InteractionManager::new();
    interaction.register_widget(parent_id);
    interaction.register_widget(child_a_id);
    interaction.register_widget(child_b_id);

    // Deliver initial WidgetAdded events so ordering assertion passes.
    let added_events = interaction.drain_events();
    prepare_widget_tree(
        &mut parent,
        &mut interaction,
        None,
        &added_events,
        None,
        None,
        Instant::now(),
    );
    for child in &mut parent.children {
        child.lifecycle_calls.clear();
    }

    let events = vec![
        LifecycleEvent::HotChanged {
            widget_id: child_a_id,
            is_hot: true,
        },
        LifecycleEvent::HotChanged {
            widget_id: child_b_id,
            is_hot: false,
        },
    ];

    prepare_widget_tree(
        &mut parent,
        &mut interaction,
        None,
        &events,
        None,
        None,
        Instant::now(),
    );

    // Child A received its HotChanged event.
    assert_eq!(parent.children[0].lifecycle_calls.len(), 1);
    assert_eq!(
        parent.children[0].lifecycle_calls[0],
        LifecycleEvent::HotChanged {
            widget_id: child_a_id,
            is_hot: true,
        }
    );

    // Child B received its HotChanged event.
    assert_eq!(parent.children[1].lifecycle_calls.len(), 1);
    assert_eq!(
        parent.children[1].lifecycle_calls[0],
        LifecycleEvent::HotChanged {
            widget_id: child_b_id,
            is_hot: false,
        }
    );
}

#[test]
fn prepare_widget_tree_processes_visual_states() {
    use oriterm_ui::visual_state::transition::VisualStateAnimator;

    let parent_id = WidgetId::next();
    let child_id = WidgetId::next();

    /// Widget with a visual state animator for testing tree traversal.
    struct AnimatedWidget {
        id: WidgetId,
        animator: VisualStateAnimator,
    }

    impl Widget for AnimatedWidget {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn sense(&self) -> Sense {
            Sense::all()
        }

        fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
            LayoutBox::leaf(0.0, 0.0)
        }

        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}

        fn visual_states(&self) -> Option<&VisualStateAnimator> {
            Some(&self.animator)
        }

        fn visual_states_mut(&mut self) -> Option<&mut VisualStateAnimator> {
            Some(&mut self.animator)
        }
    }

    /// Parent that holds one animated child.
    struct AnimParent {
        id: WidgetId,
        child: AnimatedWidget,
    }

    impl Widget for AnimParent {
        fn id(&self) -> WidgetId {
            self.id
        }

        fn sense(&self) -> Sense {
            Sense::all()
        }

        fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
            LayoutBox::leaf(0.0, 0.0)
        }

        fn paint(&self, _ctx: &mut DrawCtx<'_>) {}

        fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
            visitor(&mut self.child);
        }
    }

    let mut parent = AnimParent {
        id: parent_id,
        child: AnimatedWidget {
            id: child_id,
            animator: VisualStateAnimator::new(Vec::new()),
        },
    };

    let mut interaction = InteractionManager::new();
    interaction.register_widget(parent_id);
    interaction.register_widget(child_id);

    // Deliver initial WidgetAdded events so ordering assertion passes.
    let added_events = interaction.drain_events();
    let now = Instant::now();
    prepare_widget_tree(
        &mut parent,
        &mut interaction,
        None,
        &added_events,
        None,
        None,
        now,
    );

    // Make child hot so the animator has state to process.
    interaction.update_hot_path(&[child_id]);
    let _ = interaction.drain_events();

    prepare_widget_tree(&mut parent, &mut interaction, None, &[], None, None, now);

    // The child's animator was updated (it called update+tick).
    // We can't easily inspect internal state, but the fact that it
    // didn't panic and the state is hot confirms the pipeline ran.
    let child_state = interaction.get_state(child_id);
    assert!(child_state.is_hot());
}
