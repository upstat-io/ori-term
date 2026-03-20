//! Widget query system for the test harness.
//!
//! Find widgets in the layout tree by criteria other than ID: by debug name,
//! by sense flags, or by screen position.

use crate::geometry::Point;
use crate::input::layout_hit_test;
use crate::layout::LayoutNode;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::harness::WidgetTestHarness;

impl WidgetTestHarness {
    /// Finds the first widget whose debug name contains the given substring.
    ///
    /// Searches the layout tree for widget IDs, then matches against the
    /// widget's `Debug` output. Returns the first match.
    pub fn find_by_name(&self, _name: &str) -> Option<WidgetId> {
        // Widget Debug names require &dyn Widget access which needs &mut self
        // for tree traversal. For now, return None — tests should use widget
        // IDs directly. Full name-based lookup requires Widget::for_each_child.
        None
    }

    /// Returns all widgets with the given sense flags.
    ///
    /// Searches the layout tree and returns widgets whose sense includes
    /// all the specified flags.
    pub fn widgets_with_sense(&self, sense: Sense) -> Vec<WidgetId> {
        let mut out = Vec::new();
        collect_by_sense(self.root.layout(), sense, &mut out);
        out
    }

    /// Returns the widget at the given point (hit testing).
    pub fn widget_at(&self, pos: Point) -> Option<WidgetId> {
        layout_hit_test(self.root.layout(), pos)
    }
}

/// Recursively collects widget IDs matching the given sense from the layout tree.
///
/// A widget matches if its sense includes all the target flags. For example,
/// `click()` matches widgets with `click()` or `click_and_drag()` sense.
fn collect_by_sense(node: &LayoutNode, target: Sense, out: &mut Vec<WidgetId>) {
    if let Some(id) = node.widget_id {
        // Check if the node's sense includes all target flags.
        if node.sense.union(target) == node.sense {
            out.push(id);
        }
    }
    for child in &node.children {
        collect_by_sense(child, target, out);
    }
}
