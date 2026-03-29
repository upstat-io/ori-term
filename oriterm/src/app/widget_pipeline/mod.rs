//! Per-frame widget orchestration pipeline.
//!
//! Re-exports from `oriterm_ui::pipeline`. The canonical implementations
//! live in `oriterm_ui` so both the app layer and the test harness can
//! share the same code.

pub(crate) use oriterm_ui::pipeline::{
    DispatchResult, apply_dispatch_requests, collect_all_widget_ids, collect_focusable_ids,
    deregister_widget_tree, prepaint_widget_tree, prepare_widget_tree, register_widget_tree,
};
#[cfg(test)]
pub(crate) use oriterm_ui::pipeline::{dispatch_step, prepare_widget_frame};

/// Applies `ControllerRequests` side effects from a `DispatchResult`.
///
/// Convenience wrapper that unpacks `result.requests` and `result.source`
/// into `apply_dispatch_requests`.
///
/// Returns all widget IDs whose interaction state changed.
pub(crate) fn apply_requests(
    result: &DispatchResult,
    interaction: &mut oriterm_ui::interaction::InteractionManager,
    focus_manager: &mut oriterm_ui::focus::FocusManager,
) -> Vec<oriterm_ui::widget_id::WidgetId> {
    apply_dispatch_requests(result.requests, result.source, interaction, focus_manager)
}

#[cfg(test)]
mod tests;
