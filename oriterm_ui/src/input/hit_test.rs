//! Widget-level hit testing on a layout tree.
//!
//! Walks a [`LayoutNode`] tree back-to-front (last child = frontmost) and
//! returns the deepest widget whose rect contains the test point. Widgets
//! with `Sense::none()` or `disabled == true` are skipped. The standard
//! approach used by Chromium's `WindowTargeter` and Druid's `WidgetPod`.

use crate::geometry::{Point, Rect};
use crate::hit_test_behavior::HitTestBehavior;
use crate::layout::LayoutNode;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

/// Result of a path-collecting hit test.
///
/// Contains the full ancestor chain from root to the deepest hit widget,
/// with bounds and sense data for each entry.
#[derive(Debug, Clone)]
pub struct WidgetHitTestResult {
    /// Widgets hit, ordered root-to-leaf (outermost ancestor first,
    /// deepest hit widget last). Matches `update_hot_path` ordering.
    pub path: Vec<HitEntry>,
}

/// A single entry in a hit test path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitEntry {
    /// The widget that was hit.
    pub widget_id: WidgetId,
    /// The widget's layout bounds (unmodified by `interact_radius`).
    pub bounds: Rect,
    /// The widget's declared sense.
    pub sense: Sense,
}

impl WidgetHitTestResult {
    /// Extracts widget IDs for passing to `InteractionManager::update_hot_path`.
    pub fn widget_ids(&self) -> Vec<WidgetId> {
        self.path.iter().map(|e| e.widget_id).collect()
    }

    /// Returns the deepest (leaf) hit entry, if any.
    pub fn deepest(&self) -> Option<&HitEntry> {
        self.path.last()
    }

    /// Whether any widget was hit.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }
}

/// Finds the deepest widget under `point` in a layout tree.
///
/// Traversal is back-to-front: the last child in the children list is
/// considered frontmost (painter's algorithm). The first hit in reverse
/// order wins because it is visually on top.
///
/// Widgets with `Sense::none()` or `disabled == true` are skipped.
/// Returns `None` if no hittable widget contains the point.
pub fn layout_hit_test(root: &LayoutNode, point: Point) -> Option<WidgetId> {
    hit_test_node(root, point, None)
}

/// Finds the deepest widget under `point`, respecting a clip rectangle.
///
/// Widgets outside the clip rect are not hittable. Pass `None` for no clip.
/// Widgets with `Sense::none()` or `disabled == true` are skipped.
pub fn layout_hit_test_clipped(
    root: &LayoutNode,
    point: Point,
    clip: Option<Rect>,
) -> Option<WidgetId> {
    hit_test_node(root, point, clip)
}

/// Finds the full ancestor path from root to the deepest hit widget.
///
/// Returns an empty result if no widget contains the point. Otherwise
/// returns a root-to-leaf ordered `WidgetHitTestResult` with bounds and
/// sense data for each entry.
///
/// This is used by `InteractionManager::update_hot_path()` to compute which
/// widgets are hot (pointer over subtree) vs hot-direct (pointer over leaf).
pub fn layout_hit_test_path(root: &LayoutNode, point: Point) -> WidgetHitTestResult {
    let mut path = Vec::new();
    hit_test_path_node(root, point, None, &mut path);
    WidgetHitTestResult { path }
}

/// Returns `true` if the node is hittable (has a widget ID, non-none sense,
/// and is not disabled).
fn is_hittable(node: &LayoutNode) -> bool {
    node.widget_id.is_some() && !node.sense.is_none() && !node.disabled
}

/// Returns `true` if `point` is within the node's effective hit area,
/// accounting for `interact_radius` expansion.
fn point_in_hit_area(node: &LayoutNode, point: Point) -> bool {
    if node.interact_radius <= 0.0 {
        return node.rect.contains(point);
    }
    let r = node.interact_radius;
    let inflated = Rect::new(
        node.rect.x() - r,
        node.rect.y() - r,
        node.rect.width() + r * 2.0,
        node.rect.height() + r * 2.0,
    );
    inflated.contains(point)
}

/// Returns the squared distance from `point` to the center of `rect`.
fn center_distance_sq(rect: &Rect, point: Point) -> f32 {
    let cx = rect.x() + rect.width() * 0.5;
    let cy = rect.y() + rect.height() * 0.5;
    let dx = point.x - cx;
    let dy = point.y - cy;
    dx * dx + dy * dy
}

/// Recursive hit test on a single node.
///
/// Returns the deepest hittable `WidgetId` whose rect contains `point`.
fn hit_test_node(node: &LayoutNode, point: Point, clip: Option<Rect>) -> Option<WidgetId> {
    // Early out: point outside this node's hit area.
    if !point_in_hit_area(node, point) {
        return None;
    }

    // Early out: point outside clip rect.
    if let Some(clip) = clip {
        if !clip.contains(point) {
            return None;
        }
    }

    // Opaque widgets absorb the event — don't test children.
    if node.hit_test_behavior == HitTestBehavior::Opaque && is_hittable(node) {
        return node.widget_id;
    }

    // Compute child clip: if this node clips, intersect with its rect.
    let child_clip = if node.clip {
        Some(match clip {
            Some(c) => intersect_rects(c, node.rect),
            None => node.rect,
        })
    } else {
        clip
    };

    // Walk children back-to-front. Handle interact_radius tie-breaking.
    let child_hit = hit_test_children(node, point, child_clip);

    // DeferToChild (default) and Translucent both try children first.
    if let Some(id) = child_hit {
        return Some(id);
    }

    // No child hit — return this node if it's hittable.
    if is_hittable(node) {
        return node.widget_id;
    }

    None
}

/// Walks children back-to-front with `interact_radius` tie-breaking.
fn hit_test_children(parent: &LayoutNode, point: Point, clip: Option<Rect>) -> Option<WidgetId> {
    let has_radius = parent.children.iter().any(|c| c.interact_radius > 0.0);

    if !has_radius {
        // Fast path: no interact_radius, first hit wins.
        for child in parent.children.iter().rev() {
            if let Some(id) = hit_test_node(child, point, clip) {
                return Some(id);
            }
        }
        return None;
    }

    // Slow path: collect candidates, pick nearest center.
    pick_nearest_candidate(parent, point, clip)
}

/// When siblings have `interact_radius > 0`, multiple inflated rects may
/// overlap. Collect all candidates and pick the one whose center is nearest
/// to `point`.
fn pick_nearest_candidate(
    parent: &LayoutNode,
    point: Point,
    clip: Option<Rect>,
) -> Option<WidgetId> {
    let mut best_id: Option<WidgetId> = None;
    let mut best_dist = f32::INFINITY;

    for child in parent.children.iter().rev() {
        if let Some(id) = hit_test_node(child, point, clip) {
            let dist = center_distance_sq(&child.rect, point);
            if dist < best_dist {
                best_dist = dist;
                best_id = Some(id);
            }
        }
    }
    best_id
}

/// Recursive path-collecting hit test.
///
/// Returns `true` if a hit was found in this subtree. Pushes `HitEntry`
/// records top-down: the current node's entry is pushed speculatively
/// before recursing into children, and popped if no hit is found.
fn hit_test_path_node(
    node: &LayoutNode,
    point: Point,
    clip: Option<Rect>,
    path: &mut Vec<HitEntry>,
) -> bool {
    // Early out: point outside this node's hit area.
    if !point_in_hit_area(node, point) {
        return false;
    }

    // Early out: point outside clip rect.
    if let Some(clip) = clip {
        if !clip.contains(point) {
            return false;
        }
    }

    // Push current node's entry speculatively (root-to-leaf order).
    let pushed = if let Some(id) = node.widget_id {
        // Skip disabled/none-sense widgets from the path.
        if node.disabled || node.sense.is_none() {
            false
        } else {
            path.push(HitEntry {
                widget_id: id,
                bounds: node.rect,
                sense: node.sense,
            });
            true
        }
    } else {
        false
    };

    // Opaque widgets absorb the event — don't recurse into children.
    if pushed && node.hit_test_behavior == HitTestBehavior::Opaque {
        return true;
    }

    // Compute child clip.
    let child_clip = if node.clip {
        Some(match clip {
            Some(c) => intersect_rects(c, node.rect),
            None => node.rect,
        })
    } else {
        clip
    };

    // Translucent: keep this node in the path AND recurse into children.
    // DeferToChild: keep this node only if a child is also hit.
    let child_hit = hit_test_path_children(node, point, child_clip, path);

    if child_hit {
        return true;
    }

    // No child hit.
    if pushed {
        if node.hit_test_behavior == HitTestBehavior::Translucent {
            // Translucent: keep self in path even without child hits.
            return true;
        }
        // DeferToChild: self is the deepest hit.
        return true;
    }

    false
}

/// Walks children for path collection with `interact_radius` tie-breaking.
fn hit_test_path_children(
    parent: &LayoutNode,
    point: Point,
    clip: Option<Rect>,
    path: &mut Vec<HitEntry>,
) -> bool {
    let has_radius = parent.children.iter().any(|c| c.interact_radius > 0.0);

    if !has_radius {
        // Fast path: first hit wins.
        for child in parent.children.iter().rev() {
            if hit_test_path_node(child, point, clip, path) {
                return true;
            }
        }
        return false;
    }

    // Slow path: try each child, keep the one with nearest center.
    let mut best_path: Option<Vec<HitEntry>> = None;
    let mut best_dist = f32::INFINITY;

    for child in parent.children.iter().rev() {
        let mut candidate_path = Vec::new();
        if hit_test_path_node(child, point, clip, &mut candidate_path) {
            let dist = center_distance_sq(&child.rect, point);
            if dist < best_dist {
                best_dist = dist;
                best_path = Some(candidate_path);
            }
        }
    }

    if let Some(best) = best_path {
        path.extend(best);
        return true;
    }
    false
}

/// Computes the intersection of two rectangles.
fn intersect_rects(a: Rect, b: Rect) -> Rect {
    let x1 = a.x().max(b.x());
    let y1 = a.y().max(b.y());
    let x2 = (a.x() + a.width()).min(b.x() + b.width());
    let y2 = (a.y() + a.height()).min(b.y() + b.height());
    Rect::new(x1, y1, (x2 - x1).max(0.0), (y2 - y1).max(0.0))
}
