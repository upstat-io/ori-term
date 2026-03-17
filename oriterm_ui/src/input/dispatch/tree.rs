//! Tree dispatch — walks a widget tree to deliver events to controllers.
//!
//! Provides `dispatch_to_widget_tree` (low-level: takes pre-planned delivery
//! actions) and `deliver_event_to_tree` (high-level: combines hit testing,
//! propagation planning, and controller dispatch in one call).

use std::time::Instant;

use crate::action::WidgetAction;
use crate::controllers::{
    ControllerCtxArgs, ControllerRequests, DispatchOutput, dispatch_to_controllers,
};
use crate::geometry::{Point, Rect};
use crate::input::{HitEntry, InputEvent, WidgetHitTestResult, layout_hit_test_path};
use crate::interaction::InteractionState;
use crate::layout::LayoutNode;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;

use super::{DeliveryAction, plan_propagation};

/// Accumulated result of dispatching delivery actions through a widget tree.
#[derive(Debug)]
pub struct TreeDispatchResult {
    /// Whether any controller marked the event as handled.
    pub handled: bool,
    /// Semantic actions emitted by controllers during dispatch.
    pub actions: Vec<WidgetAction>,
    /// Accumulated side-effect requests from all controllers.
    pub requests: ControllerRequests,
    /// Widget that first handled the event, if any.
    pub source: Option<WidgetId>,
}

impl TreeDispatchResult {
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
    pub fn merge(&mut self, output: DispatchOutput, widget_id: WidgetId) {
        self.actions.extend(output.actions);
        self.requests = self.requests.union(output.requests);
        if output.handled && !self.handled {
            self.handled = true;
            self.source = Some(widget_id);
        }
    }
}

impl Default for TreeDispatchResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Walks a widget tree, dispatching delivery actions to controllers of
/// widgets whose ID matches a delivery action target.
///
/// Recurses depth-first via `Widget::for_each_child_mut`. Stops early
/// if any controller marks the event as handled.
pub fn dispatch_to_widget_tree(
    widget: &mut dyn Widget,
    event: &InputEvent,
    actions: &[DeliveryAction],
    now: Instant,
    result: &mut TreeDispatchResult,
) {
    if result.handled {
        return;
    }

    let id = widget.id();

    // Dispatch any delivery actions targeting this widget.
    if actions.iter().any(|a| a.widget_id == id) {
        // Collect bounds from the first matching action.
        let widget_bounds = actions
            .iter()
            .find(|a| a.widget_id == id)
            .map_or_else(Rect::default, |a| a.bounds);

        let controllers = widget.controllers_mut();
        if !controllers.is_empty() {
            let interaction = InteractionState::default();
            for action in actions.iter().filter(|a| a.widget_id == id) {
                let args = ControllerCtxArgs {
                    widget_id: id,
                    bounds: action.bounds,
                    interaction: &interaction,
                    now,
                };
                let output = dispatch_to_controllers(controllers, event, action.phase, &args);
                result.merge(output, id);
                if result.handled {
                    break;
                }
            }
        }

        // Let the widget transform controller actions into semantic actions.
        // Done after controller dispatch to avoid borrow conflict with
        // controllers_mut().
        if !result.actions.is_empty() {
            let raw_actions = std::mem::take(&mut result.actions);
            result.actions = raw_actions
                .into_iter()
                .filter_map(|a| widget.on_action(a, widget_bounds))
                .collect();
        }

        // If controllers didn't handle the event, let the widget handle it
        // directly for widget-internal interaction logic (e.g., menu item
        // hover tracking, text input keyboard editing).
        if !result.handled {
            let input_result = widget.on_input(event, widget_bounds);
            if input_result.handled {
                result.handled = true;
                result.source = Some(id);
            }
            if let Some(action) = input_result.action {
                result.actions.push(action);
            }
        }

        if result.handled {
            return;
        }
    }

    // Recurse into children.
    widget.for_each_child_mut(&mut |child| {
        dispatch_to_widget_tree(child, event, actions, now, result);
    });
}

/// Delivers an input event through a widget tree using the full pipeline.
///
/// Combines hit testing, propagation planning, and controller dispatch.
/// For mouse events, hit-tests the layout tree under `bounds` to find the
/// target widget, then plans Capture → Target → Bubble delivery.
/// For keyboard events, routes through `focus_path`.
///
/// # Arguments
///
/// - `widget` — root widget of the tree.
/// - `event` — the input event to deliver.
/// - `bounds` — screen-space bounds of the root widget (for coordinate mapping).
/// - `layout_node` — layout tree for hit testing (pass `None` to skip hit test
///   and deliver to root widget directly).
/// - `active_widget` — currently captured widget (for drag/press continuation).
/// - `focus_path` — root-to-leaf ancestor chain for keyboard routing.
/// - `now` — current frame timestamp.
#[expect(
    clippy::too_many_arguments,
    reason = "pipeline dispatch: widget, event, bounds, layout, active, focus, timestamp"
)]
pub fn deliver_event_to_tree(
    widget: &mut dyn Widget,
    event: &InputEvent,
    bounds: Rect,
    layout_node: Option<&LayoutNode>,
    active_widget: Option<WidgetId>,
    focus_path: &[WidgetId],
    now: Instant,
) -> TreeDispatchResult {
    let root_id = widget.id();
    let root_sense = widget.sense();

    // Build the hit path.
    let hit_result = if event.is_keyboard() {
        WidgetHitTestResult { path: Vec::new() }
    } else if active_widget.is_some() {
        // During capture, hit test to get the active widget's actual bounds
        // (needed for on_action anchor rects). plan_propagation routes to the
        // active widget regardless of hit position.
        if let (Some(node), Some(pos)) = (layout_node, event.pos()) {
            let local = Point::new(pos.x - bounds.x(), pos.y - bounds.y());
            let mut result = layout_hit_test_path(node, local);
            for entry in &mut result.path {
                entry.bounds = Rect::new(
                    entry.bounds.x() + bounds.x(),
                    entry.bounds.y() + bounds.y(),
                    entry.bounds.width(),
                    entry.bounds.height(),
                );
            }
            // If hit test is empty (cursor outside), fall back to root bounds.
            if result.path.is_empty() {
                result.path.push(HitEntry {
                    widget_id: root_id,
                    bounds,
                    sense: root_sense,
                });
            }
            result
        } else {
            WidgetHitTestResult {
                path: vec![HitEntry {
                    widget_id: root_id,
                    bounds,
                    sense: root_sense,
                }],
            }
        }
    } else if let Some(node) = layout_node {
        if let Some(pos) = event.pos() {
            let local = Point::new(pos.x - bounds.x(), pos.y - bounds.y());
            let mut result = layout_hit_test_path(node, local);
            // Offset local-space bounds to screen-space.
            for entry in &mut result.path {
                entry.bounds = Rect::new(
                    entry.bounds.x() + bounds.x(),
                    entry.bounds.y() + bounds.y(),
                    entry.bounds.width(),
                    entry.bounds.height(),
                );
            }
            result
        } else {
            WidgetHitTestResult { path: Vec::new() }
        }
    } else {
        // No layout — deliver directly to root widget.
        WidgetHitTestResult {
            path: vec![HitEntry {
                widget_id: root_id,
                bounds,
                sense: root_sense,
            }],
        }
    };

    // Plan propagation.
    let mut delivery_actions = Vec::new();
    plan_propagation(
        event,
        &hit_result,
        active_widget,
        focus_path,
        &mut delivery_actions,
    );

    // Dispatch through widget tree.
    let mut result = TreeDispatchResult::new();
    dispatch_to_widget_tree(widget, event, &delivery_actions, now, &mut result);
    result
}
