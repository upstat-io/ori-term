//! Per-frame widget orchestration pipeline.
//!
//! Defines the delivery loop for two-phase event propagation and the
//! pre-paint mutation phase (lifecycle delivery, animation frames,
//! visual state updates). Both the app layer and the test harness call
//! these functions to run a complete per-frame pipeline.

use std::collections::HashMap;
use std::time::Instant;

#[cfg(debug_assertions)]
use std::collections::HashSet;

use crate::action::{KeymapAction, WidgetAction};
use crate::animation::{AnimFrameEvent, FrameRequestFlags};
use crate::controllers::{
    ControllerCtxArgs, ControllerRequests, DispatchOutput, dispatch_lifecycle_to_controllers,
    dispatch_to_controllers,
};
use crate::focus::FocusManager;
use crate::geometry::Rect;
use crate::input::InputEvent;
use crate::input::dispatch::DeliveryAction;
use crate::interaction::{InteractionManager, InteractionState, LifecycleEvent};
use crate::layout::LayoutNode;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::widgets::contexts::{AnimCtx, LifecycleCtx, PrepaintCtx};

/// Result of running the event delivery loop across a propagation plan.
///
/// Accumulates controller outputs from each [`DeliveryAction`] step.
/// The caller reads `actions` to apply semantic effects (e.g., `Clicked`,
/// `DismissOverlay`) and `requests` to apply framework side effects
/// (e.g., `SET_ACTIVE` -> `InteractionManager::set_active()`).
#[derive(Debug, Default)]
pub struct DispatchResult {
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
pub fn dispatch_step(
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

/// Runs the pre-paint mutation phase for a widget and all its descendants.
///
/// Walks the widget tree depth-first via `Widget::for_each_child_mut`,
/// executing lifecycle delivery, animation ticks, and visual state updates
/// at every node. Must be called BEFORE the immutable paint phase.
#[expect(
    clippy::too_many_arguments,
    reason = "pre-paint pipeline: widget, interaction, lifecycle events, anim event, frame flags, timestamp"
)]
pub fn prepare_widget_tree(
    widget: &mut dyn Widget,
    interaction: &mut InteractionManager,
    lifecycle_events: &[LifecycleEvent],
    anim_event: Option<&AnimFrameEvent>,
    frame_requests: Option<&FrameRequestFlags>,
    now: Instant,
) {
    #[cfg(debug_assertions)]
    let id = widget.id();
    prepare_widget_frame(
        widget,
        interaction,
        lifecycle_events,
        anim_event,
        frame_requests,
        now,
    );
    #[cfg(debug_assertions)]
    let mut visited = HashSet::new();
    widget.for_each_child_mut(&mut |child| {
        #[cfg(debug_assertions)]
        {
            let child_id = child.id();
            let is_new = visited.insert(child_id);
            assert!(
                is_new,
                "Container widget {:?} visited child {:?} twice during pre-paint",
                id, child_id
            );
        }
        prepare_widget_tree(
            child,
            interaction,
            lifecycle_events,
            anim_event,
            frame_requests,
            now,
        );
    });
}

/// Runs the pre-paint mutation phase for a single widget (non-recursive).
///
/// Executes lifecycle event delivery, animation frame ticks, and visual
/// state animator updates. Called by `prepare_widget_tree` at each node.
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
pub fn prepare_widget_frame(
    widget: &mut dyn Widget,
    interaction: &mut InteractionManager,
    lifecycle_events: &[LifecycleEvent],
    anim_event: Option<&AnimFrameEvent>,
    frame_requests: Option<&FrameRequestFlags>,
    now: Instant,
) {
    let id = widget.id();

    // Pre-scan: mark WidgetAdded delivery before assertions run, so that
    // subsequent events in the same batch pass the ordering check.
    #[cfg(debug_assertions)]
    {
        for event in lifecycle_events.iter().filter(|e| e.widget_id() == id) {
            if matches!(event, LifecycleEvent::WidgetAdded { .. }) {
                interaction.mark_widget_added_delivered(id);
            }
            debug_assert!(
                interaction.is_registered(id)
                    || matches!(event, LifecycleEvent::WidgetAdded { .. }),
                "Lifecycle event {:?} delivered to unregistered widget {:?}",
                event,
                id
            );
            debug_assert!(
                matches!(event, LifecycleEvent::WidgetAdded { .. })
                    || interaction.was_widget_added_delivered(id),
                "Widget {:?} received {:?} before WidgetAdded",
                id,
                event
            );
        }
    }

    let state = interaction.get_state(id);

    // Step 1: deliver lifecycle events targeting this widget.
    let args = ControllerCtxArgs {
        widget_id: id,
        bounds: Rect::default(),
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

/// Collects per-widget bounds from a `LayoutNode` tree into a flat map.
///
/// Walks the layout tree depth-first, recording `widget_id -> rect` for
/// every node that has a `widget_id`. Used by `prepaint_widget_tree` to
/// resolve per-widget bounds without parallel tree walking.
#[expect(
    clippy::implicit_hasher,
    reason = "always default hasher, matches collect_layout_widget_ids pattern"
)]
pub fn collect_layout_bounds(node: &LayoutNode, out: &mut HashMap<WidgetId, Rect>) {
    if let Some(id) = node.widget_id {
        out.insert(id, node.rect);
    }
    for child in &node.children {
        collect_layout_bounds(child, out);
    }
}

/// Runs the prepaint phase for a widget and all its descendants.
///
/// Walks the widget tree depth-first via `Widget::for_each_child_mut`,
/// calling `widget.prepaint()` on each node with the resolved bounds from
/// the layout map. Must be called AFTER `prepare_widget_tree` and layout,
/// BEFORE `build_scene` (paint).
#[expect(
    clippy::too_many_arguments,
    reason = "prepaint pipeline: widget, bounds map, interaction, theme, timestamp, frame flags"
)]
#[expect(
    clippy::implicit_hasher,
    reason = "always default hasher, matches collect_layout_widget_ids pattern"
)]
pub fn prepaint_widget_tree(
    widget: &mut dyn Widget,
    bounds_map: &HashMap<WidgetId, Rect>,
    interaction: Option<&InteractionManager>,
    theme: &UiTheme,
    now: Instant,
    frame_requests: Option<&FrameRequestFlags>,
) {
    let id = widget.id();
    let bounds = bounds_map.get(&id).copied().unwrap_or_default();
    let mut ctx = PrepaintCtx {
        widget_id: id,
        bounds,
        interaction,
        theme,
        now,
        frame_requests,
    };
    widget.prepaint(&mut ctx);

    widget.for_each_child_mut(&mut |child| {
        prepaint_widget_tree(child, bounds_map, interaction, theme, now, frame_requests);
    });
}

/// Registers all widgets in a tree with `InteractionManager`.
///
/// Walks the widget tree depth-first via `Widget::for_each_child_mut`,
/// calling `register_widget` on each node. Uses `for_each_child_mut`
/// (not `_all`) because registration queues `WidgetAdded` lifecycle
/// events that must be delivered by `prepare_widget_tree` — which also
/// uses `for_each_child_mut`. Registering hidden-page widgets would
/// queue events that can never be delivered, causing ordering violations.
/// Idempotent — safe to call multiple times.
pub fn register_widget_tree(widget: &mut dyn Widget, interaction: &mut InteractionManager) {
    interaction.register_widget(widget.id());
    widget.for_each_child_mut(&mut |child| {
        register_widget_tree(child, interaction);
    });
}

/// Dispatches a keymap action to the target widget by ID.
///
/// Walks the widget tree depth-first to find the widget with `target` ID,
/// then calls `handle_keymap_action()` on it. Returns the resulting
/// `WidgetAction` if the widget handled the action.
///
/// O(n) in tree size — only runs on keymap-matched keyboard events.
pub fn dispatch_keymap_action(
    widget: &mut dyn Widget,
    target: WidgetId,
    action: &dyn KeymapAction,
    bounds: Rect,
) -> Option<WidgetAction> {
    if widget.id() == target {
        return widget.handle_keymap_action(action, bounds);
    }
    let mut result = None;
    widget.for_each_child_mut(&mut |child| {
        if result.is_none() {
            result = dispatch_keymap_action(child, target, action, bounds);
        }
    });
    result
}

/// Collects focusable widget IDs in tree traversal order.
///
/// Walks the widget tree depth-first, appending IDs of widgets where
/// `is_focusable()` returns `true`. The resulting order is suitable for
/// `FocusManager::set_focus_order()`.
pub fn collect_focusable_ids(widget: &mut dyn Widget, out: &mut Vec<WidgetId>) {
    if widget.is_focusable() {
        out.push(widget.id());
    }
    widget.for_each_child_mut(&mut |child| {
        collect_focusable_ids(child, out);
    });
}

/// Applies controller request side effects from any dispatch result.
///
/// Translates request flags into `InteractionManager` mutations:
/// - `SET_ACTIVE` -> `interaction.set_active(source)`.
/// - `CLEAR_ACTIVE` -> `interaction.clear_active()`.
/// - `REQUEST_FOCUS` -> `interaction.request_focus(source, focus_manager)`.
///
/// `PAINT` and `ANIM_FRAME` are handled by the caller (mark dirty,
/// schedule animation frame).
pub fn apply_dispatch_requests(
    requests: ControllerRequests,
    source: Option<WidgetId>,
    interaction: &mut InteractionManager,
    focus_manager: &mut FocusManager,
) {
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
    if requests.contains(ControllerRequests::FOCUS_NEXT) {
        focus_manager.focus_next();
        if let Some(new_id) = focus_manager.focused() {
            interaction.request_focus(new_id, focus_manager);
        }
    }
    if requests.contains(ControllerRequests::FOCUS_PREV) {
        focus_manager.focus_prev();
        if let Some(new_id) = focus_manager.focused() {
            interaction.request_focus(new_id, focus_manager);
        }
    }
}

/// Walks a `LayoutNode` tree and collects all `Some(widget_id)` values.
#[cfg(debug_assertions)]
#[expect(
    clippy::implicit_hasher,
    reason = "debug-only function, always default hasher"
)]
pub fn collect_layout_widget_ids(node: &LayoutNode, out: &mut HashSet<WidgetId>) {
    if let Some(id) = node.widget_id {
        out.insert(id);
    }
    for child in &node.children {
        collect_layout_widget_ids(child, out);
    }
}

/// Asserts that every laid-out widget was also visited during dispatch.
#[cfg(debug_assertions)]
#[expect(
    clippy::implicit_hasher,
    reason = "debug-only function, always default hasher"
)]
pub fn check_cross_phase_consistency(
    layout_ids: &HashSet<WidgetId>,
    dispatch_ids: &HashSet<WidgetId>,
) {
    for id in layout_ids {
        debug_assert!(
            dispatch_ids.contains(id),
            "Cross-phase mismatch: widget {:?} was laid out but never \
             visited by for_each_child_mut during dispatch",
            id
        );
    }
}

#[cfg(test)]
mod tests;
