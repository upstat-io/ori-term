//! Key context stack for keymap scope gating.
//!
//! During widget tree registration, each widget's `key_context()` is collected
//! into a map. At dispatch time, `build_context_stack` resolves the focus path
//! into a context stack for keymap lookup.

use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::widget_id::WidgetId;

/// Builds a context stack from a focus path using a pre-collected context map.
///
/// Looks up each focus path widget ID in `context_map` and collects the
/// non-None `key_context()` values. The resulting stack is ordered root-to-leaf
/// (same order as the focus path). Deeper entries win in keymap lookup.
pub fn build_context_stack<S: BuildHasher>(
    context_map: &HashMap<WidgetId, &'static str, S>,
    focus_path: &[WidgetId],
) -> Vec<&'static str> {
    focus_path
        .iter()
        .filter_map(|id| context_map.get(id).copied())
        .collect()
}

/// Collects `key_context()` from a widget and its descendants into a map.
///
/// Walks the widget tree via `for_each_child_mut`. Only inserts entries for
/// widgets that return `Some` from `key_context()`. Called during
/// `register_widget_tree()` or as a parallel pass.
pub fn collect_key_contexts<S: BuildHasher>(
    widget: &mut dyn crate::widgets::Widget,
    out: &mut HashMap<WidgetId, &'static str, S>,
) {
    if let Some(ctx) = widget.key_context() {
        out.insert(widget.id(), ctx);
    }
    widget.for_each_child_mut(&mut |child| {
        collect_key_contexts(child, out);
    });
}
