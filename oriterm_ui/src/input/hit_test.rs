//! Widget-level hit testing on a layout tree.
//!
//! Walks a [`LayoutNode`] tree back-to-front (last child = frontmost) and
//! returns the deepest widget whose rect contains the test point. This is
//! the standard approach used by Chromium's `WindowTargeter` and Druid's
//! `WidgetPod`.

use crate::geometry::{Point, Rect};
use crate::layout::LayoutNode;
use crate::widget_id::WidgetId;

/// Finds the deepest widget under `point` in a layout tree.
///
/// Traversal is back-to-front: the last child in the children list is
/// considered frontmost (painter's algorithm). The first hit in reverse
/// order wins because it is visually on top.
///
/// Returns `None` if no widget with a `widget_id` contains the point.
pub fn layout_hit_test(root: &LayoutNode, point: Point) -> Option<WidgetId> {
    hit_test_node(root, point, None)
}

/// Finds the deepest widget under `point`, respecting a clip rectangle.
///
/// Widgets outside the clip rect are not hittable. Pass `None` for no clip.
pub fn layout_hit_test_clipped(
    root: &LayoutNode,
    point: Point,
    clip: Option<Rect>,
) -> Option<WidgetId> {
    hit_test_node(root, point, clip)
}

/// Finds the full ancestor path from root to the deepest hit widget.
///
/// Returns an empty `Vec` if no widget contains the point. Otherwise returns
/// a root-to-leaf ordered list of all `WidgetId`s along the path to the
/// deepest hit widget.
///
/// This is used by `InteractionManager::update_hot_path()` to compute which
/// widgets are hot (pointer over subtree) vs hot-direct (pointer over leaf).
pub fn layout_hit_test_path(root: &LayoutNode, point: Point) -> Vec<WidgetId> {
    let mut path = Vec::new();
    hit_test_path_node(root, point, None, &mut path);
    path
}

/// Recursive path-collecting hit test.
///
/// Returns `true` if a hit was found in this subtree. Pushes widget IDs
/// top-down: the current node's ID is pushed speculatively before recursing
/// into children, and popped if no hit is found.
fn hit_test_path_node(
    node: &LayoutNode,
    point: Point,
    clip: Option<Rect>,
    path: &mut Vec<WidgetId>,
) -> bool {
    // Early out: point outside this node's rect.
    if !node.rect.contains(point) {
        return false;
    }

    // Early out: point outside clip rect.
    if let Some(clip) = clip {
        if !clip.contains(point) {
            return false;
        }
    }

    // Push current node's ID speculatively (root-to-leaf order).
    let pushed = if let Some(id) = node.widget_id {
        path.push(id);
        true
    } else {
        false
    };

    // Walk children back-to-front (last child = frontmost).
    for child in node.children.iter().rev() {
        if hit_test_path_node(child, point, clip, path) {
            return true;
        }
    }

    // No child hit. If we pushed our ID, we're the deepest hit widget.
    if pushed {
        return true;
    }

    false
}

/// Recursive hit test on a single node.
///
/// Returns the deepest `WidgetId` whose rect contains `point`, or `None`.
fn hit_test_node(node: &LayoutNode, point: Point, clip: Option<Rect>) -> Option<WidgetId> {
    // Early out: point outside this node's rect.
    if !node.rect.contains(point) {
        return None;
    }

    // Early out: point outside clip rect.
    if let Some(clip) = clip {
        if !clip.contains(point) {
            return None;
        }
    }

    // Walk children back-to-front (last child = frontmost).
    for child in node.children.iter().rev() {
        if let Some(id) = hit_test_node(child, point, clip) {
            return Some(id);
        }
    }

    // No child claimed it — return this node's widget_id (if any).
    node.widget_id
}
