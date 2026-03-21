//! Recursive tree-walk functions for the widget pipeline.
//!
//! Contains prepare, prepaint, registration, keymap dispatch, and
//! focus collection — all functions that recursively walk a widget tree
//! via `Widget::for_each_child_mut`. Extracted from `pipeline/mod.rs`
//! to keep both files under the 500-line limit.

use std::collections::HashMap;
use std::time::Instant;

#[cfg(debug_assertions)]
use std::collections::HashSet;

use crate::action::{KeymapAction, WidgetAction};
use crate::animation::{AnimFrameEvent, FrameRequestFlags};
use crate::controllers::{
    ControllerCtxArgs, ControllerRequests, dispatch_lifecycle_to_controllers,
};
use crate::geometry::Rect;
use crate::interaction::{InteractionManager, LifecycleEvent};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::widgets::contexts::{AnimCtx, LifecycleCtx, PrepaintCtx};

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
