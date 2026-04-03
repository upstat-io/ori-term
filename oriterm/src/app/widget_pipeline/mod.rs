//! Per-frame widget orchestration pipeline.
//!
//! Re-exports from `oriterm_ui::pipeline`. The canonical implementations
//! live in `oriterm_ui` so both the app layer and the test harness can
//! share the same code.

#[cfg(test)]
pub(crate) use oriterm_ui::pipeline::{DispatchResult, dispatch_step, prepare_widget_frame};
pub(crate) use oriterm_ui::pipeline::{
    apply_dispatch_requests, collect_all_widget_ids, collect_focusable_ids, deregister_widget_tree,
    prepaint_widget_tree, prepare_widget_tree, register_widget_tree,
};

#[cfg(test)]
mod tests;
