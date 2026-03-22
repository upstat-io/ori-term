//! Per-frame widget orchestration pipeline.
//!
//! Defines the delivery loop for two-phase event propagation and the
//! pre-paint mutation phase (lifecycle delivery, animation frames,
//! visual state updates). Both the app layer and the test harness call
//! these functions to run a complete per-frame pipeline.
//!
//! Tree-walk functions (prepare, prepaint, registration, keymap dispatch,
//! focus collection) live in the [`tree_walk`] submodule.

mod tree_walk;

pub use tree_walk::*;

use std::collections::HashMap;
use std::time::Instant;

#[cfg(debug_assertions)]
use std::collections::HashSet;

use crate::action::WidgetAction;
use crate::controllers::{
    ControllerCtxArgs, ControllerRequests, DispatchOutput, dispatch_to_controllers,
};
use crate::focus::FocusManager;
use crate::geometry::Rect;
use crate::input::InputEvent;
use crate::input::dispatch::DeliveryAction;
use crate::interaction::{InteractionManager, InteractionState};
use crate::layout::LayoutNode;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;

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

/// Applies controller request side effects from any dispatch result.
///
/// Translates request flags into `InteractionManager` mutations:
/// - `SET_ACTIVE` -> `interaction.set_active(source)`.
/// - `CLEAR_ACTIVE` -> `interaction.clear_active()`.
/// - `REQUEST_FOCUS` -> `interaction.request_focus(source, focus_manager)`.
///
/// `PAINT` and `ANIM_FRAME` are handled by the caller (mark dirty,
/// schedule animation frame).
///
/// Returns all widget IDs whose interaction state changed, so the caller
/// can mark them dirty in the `InvalidationTracker`.
pub fn apply_dispatch_requests(
    requests: ControllerRequests,
    source: Option<WidgetId>,
    interaction: &mut InteractionManager,
    focus_manager: &mut FocusManager,
) -> Vec<WidgetId> {
    let mut changed = Vec::new();
    if requests.contains(ControllerRequests::SET_ACTIVE) {
        if let Some(id) = source {
            changed.extend(interaction.set_active(id));
        }
    }
    if requests.contains(ControllerRequests::CLEAR_ACTIVE) {
        changed.extend(interaction.clear_active());
    }
    if requests.contains(ControllerRequests::REQUEST_FOCUS) {
        if let Some(id) = source {
            changed.extend(interaction.request_focus(id, focus_manager));
        }
    }
    if requests.contains(ControllerRequests::FOCUS_NEXT) {
        focus_manager.focus_next();
        if let Some(new_id) = focus_manager.focused() {
            changed.extend(interaction.request_focus(new_id, focus_manager));
        }
    }
    if requests.contains(ControllerRequests::FOCUS_PREV) {
        focus_manager.focus_prev();
        if let Some(new_id) = focus_manager.focused() {
            changed.extend(interaction.request_focus(new_id, focus_manager));
        }
    }
    changed
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
