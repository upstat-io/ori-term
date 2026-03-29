//! Parent map construction from the layout tree.
//!
//! Builds a `child → parent` mapping by traversing the `LayoutNode` tree.
//! Used by `InteractionManager` to walk ancestors for `focus_within` updates.

use std::collections::HashMap;

use crate::layout::LayoutNode;
use crate::widget_id::WidgetId;

/// Builds a child-to-parent widget ID map from a layout tree.
///
/// Performs a depth-first traversal of the layout tree, recording
/// `child_widget_id → parent_widget_id` for every node pair where both
/// have `widget_id: Some(_)`. Called once after each layout pass.
pub fn build_parent_map(root: &LayoutNode) -> HashMap<WidgetId, WidgetId> {
    let mut map = HashMap::new();
    traverse(root, None, &mut map);
    map
}

/// Recursive DFS traversal collecting parent relationships.
///
/// `nearest_ancestor` is the closest ancestor with a `widget_id`. When we
/// encounter a node with a `widget_id`, we record it as a child of the
/// nearest ancestor (if any), then become the new nearest ancestor for
/// our subtree.
fn traverse(
    node: &LayoutNode,
    nearest_ancestor: Option<WidgetId>,
    map: &mut HashMap<WidgetId, WidgetId>,
) {
    let current = if let Some(id) = node.widget_id {
        if let Some(parent_id) = nearest_ancestor {
            map.insert(id, parent_id);
        }
        Some(id)
    } else {
        nearest_ancestor
    };

    for child in &node.children {
        traverse(child, current, map);
    }
}
