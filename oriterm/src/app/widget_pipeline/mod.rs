//! Per-frame widget orchestration pipeline.
//!
//! Defines the delivery loop for two-phase event propagation and the
//! pre-paint mutation phase (lifecycle delivery, animation frames,
//! visual state updates). The app layer calls these functions to bridge
//! the framework primitives (`plan_propagation`, `dispatch_to_controllers`)
//! into a complete per-frame pipeline.

use std::time::Instant;

use oriterm_ui::action::WidgetAction;
use oriterm_ui::animation::FrameRequestFlags;
use oriterm_ui::controllers::{
    ControllerCtxArgs, ControllerRequests, DispatchOutput, dispatch_lifecycle_to_controllers,
    dispatch_to_controllers,
};
use oriterm_ui::input::InputEvent;
use oriterm_ui::input::dispatch::DeliveryAction;
use oriterm_ui::interaction::{InteractionManager, InteractionState, LifecycleEvent};
use oriterm_ui::widget_id::WidgetId;
use oriterm_ui::widgets::Widget;
use oriterm_ui::widgets::contexts::{AnimCtx, LifecycleCtx};

/// Result of running the event delivery loop across a propagation plan.
///
/// Accumulates controller outputs from each [`DeliveryAction`] step.
/// The caller reads `actions` to apply semantic effects (e.g., `Clicked`,
/// `DismissOverlay`) and `requests` to apply framework side effects
/// (e.g., `SET_ACTIVE` → `InteractionManager::set_active()`).
#[derive(Debug)]
pub(crate) struct DispatchResult {
    /// Whether any controller marked the event as handled.
    pub handled: bool,
    /// Semantic actions emitted by controllers during dispatch.
    pub actions: Vec<WidgetAction>,
    /// Accumulated side-effect requests from all controllers.
    pub requests: ControllerRequests,
    /// Widget that first handled the event, if any.
    pub source: Option<WidgetId>,
}

impl DispatchResult {
    /// Creates an empty result with no handling.
    pub fn new() -> Self {
        Self {
            handled: false,
            actions: Vec::new(),
            requests: ControllerRequests::NONE,
            source: None,
        }
    }

    /// Merges a single-widget `DispatchOutput` into this result.
    fn merge(&mut self, output: DispatchOutput, widget_id: WidgetId) {
        self.actions.extend(output.actions);
        self.requests = self.requests.union(output.requests);
        if output.handled && !self.handled {
            self.handled = true;
            self.source = Some(widget_id);
        }
    }
}

/// Runs one step of the delivery loop for a single widget.
///
/// Dispatches the event to the widget's controllers at the given phase,
/// merges the output into `result`, and returns `true` if the event is
/// now handled (caller should stop iterating).
///
/// # Arguments
///
/// - `result` — accumulator for the full delivery loop.
/// - `event` — the input event being delivered.
/// - `action` — the delivery action (widget ID, phase, bounds).
/// - `widget` — the target widget (provides controllers).
/// - `interaction` — per-widget interaction state from `InteractionManager`.
/// - `now` — current frame timestamp.
#[expect(
    clippy::too_many_arguments,
    reason = "delivery step: result accumulator, event, action, widget, interaction state, timestamp"
)]
pub(crate) fn dispatch_step(
    result: &mut DispatchResult,
    event: &InputEvent,
    action: &DeliveryAction,
    widget: &mut dyn Widget,
    interaction: &InteractionState,
    now: Instant,
) -> bool {
    let args = ControllerCtxArgs {
        widget_id: action.widget_id,
        bounds: action.bounds,
        interaction,
        now,
    };
    let output = dispatch_to_controllers(widget.controllers_mut(), event, action.phase, &args);
    result.merge(output, action.widget_id);
    result.handled
}

/// Runs the pre-paint mutation phase for a single widget.
///
/// Executes lifecycle event delivery, animation frame ticks, and visual
/// state animator updates. Must be called BEFORE the immutable paint
/// phase (`compose_scene` / `widget.paint()`).
///
/// During the transition period (widgets being migrated to controllers),
/// this is called on top-level widgets only (tab bar, overlay roots).
/// After migration, the framework will walk the full widget tree.
///
/// # Steps
///
/// 1. Deliver pending lifecycle events to controllers and `widget.lifecycle()`.
/// 2. Deliver animation frame event if the widget requested one.
/// 3. Update visual state animator from interaction state.
#[expect(
    clippy::too_many_arguments,
    reason = "pre-paint pipeline: widget, interaction, lifecycle events, anim event, frame flags, timestamp"
)]
pub(super) fn prepare_widget_frame(
    widget: &mut dyn Widget,
    interaction: &InteractionManager,
    lifecycle_events: &[LifecycleEvent],
    anim_event: Option<&oriterm_ui::animation::AnimFrameEvent>,
    frame_requests: Option<&FrameRequestFlags>,
    now: Instant,
) {
    let id = widget.id();
    let state = interaction.get_state(id);

    // Step 1: deliver lifecycle events targeting this widget.
    let args = ControllerCtxArgs {
        widget_id: id,
        bounds: oriterm_ui::geometry::Rect::default(),
        interaction: state,
        now,
    };
    for event in lifecycle_events.iter().filter(|e| e.widget_id() == id) {
        dispatch_lifecycle_to_controllers(widget.controllers_mut(), event, &args);

        let mut lctx = LifecycleCtx {
            widget_id: id,
            interaction: state,
            requests: ControllerRequests::NONE,
        };
        widget.lifecycle(event, &mut lctx);
    }

    // Step 2: deliver animation frame if this widget requested one.
    if let Some(anim) = anim_event {
        let mut actx = AnimCtx {
            widget_id: id,
            now,
            requests: ControllerRequests::NONE,
            frame_requests,
        };
        widget.anim_frame(anim, &mut actx);
    }

    // Step 3: update visual state animator from interaction state.
    if let Some(animator) = widget.visual_states_mut() {
        animator.update(state, now);
        animator.tick(now);
        if animator.is_animating(now) {
            if let Some(flags) = frame_requests {
                flags.request_anim_frame();
            }
        }
    }
}

/// Applies `ControllerRequests` side effects from a `DispatchResult`.
///
/// Translates request flags into `InteractionManager` mutations:
/// - `SET_ACTIVE` → `interaction.set_active(source)`.
/// - `CLEAR_ACTIVE` → `interaction.clear_active()`.
/// - `REQUEST_FOCUS` → `interaction.request_focus(source, focus_manager)`.
///
/// `PAINT` and `ANIM_FRAME` are handled by the caller (mark dirty,
/// schedule animation frame).
pub(crate) fn apply_requests(
    result: &DispatchResult,
    interaction: &mut InteractionManager,
    focus_manager: &mut oriterm_ui::focus::FocusManager,
) {
    let requests = result.requests;
    let source = result.source;

    if requests.contains(ControllerRequests::SET_ACTIVE) {
        if let Some(id) = source {
            interaction.set_active(id);
        }
    }
    if requests.contains(ControllerRequests::CLEAR_ACTIVE) {
        interaction.clear_active();
    }
    if requests.contains(ControllerRequests::REQUEST_FOCUS) {
        if let Some(id) = source {
            interaction.request_focus(id, focus_manager);
        }
    }
}

#[cfg(test)]
mod tests;
