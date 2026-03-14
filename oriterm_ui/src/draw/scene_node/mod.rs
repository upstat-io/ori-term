//! Cached draw output for a widget subtree.
//!
//! A [`SceneNode`] stores the draw command sequence produced by a widget's
//! last successful `draw()` call along with the bounds that produced it.
//! The scene composition pass replays cached commands for clean subtrees,
//! avoiding redundant `Widget::draw()` calls.
//!
//! [`SceneCache`] wraps a flat `HashMap<WidgetId, SceneNode>` and adds
//! containment tracking so that invalidation propagates upward: when a
//! deeply nested widget is dirty, ancestor containers whose cached output
//! includes that widget are also invalidated.

use std::collections::HashMap;

use crate::draw::DrawCommand;
use crate::geometry::Rect;
use crate::widget_id::WidgetId;

/// Per-widget scene cache with containment tracking for invalidation
/// propagation.
///
/// Each [`SceneNode`] records which descendant widget IDs are embedded in
/// its cached draw commands. When [`super::scene_compose::compose_scene`]
/// runs, it invalidates not only directly dirty widgets but also any
/// ancestor whose cached output includes a dirty descendant.
pub struct SceneCache {
    nodes: HashMap<WidgetId, SceneNode>,
    /// Log of widget IDs stored during the current compose pass.
    ///
    /// Containers record the log position before drawing a child, then
    /// capture the IDs stored between that position and the child's
    /// store call. This produces the set of descendant IDs contained
    /// in the child's cached output.
    store_log: Vec<WidgetId>,
}

impl SceneCache {
    /// Creates an empty scene cache.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            store_log: Vec::new(),
        }
    }

    // Accessors

    /// Returns a reference to the node for `id`, if it exists.
    pub fn get(&self, id: WidgetId) -> Option<&SceneNode> {
        self.nodes.get(&id)
    }

    /// Returns a mutable reference to the node for `id`, if it exists.
    pub fn get_mut(&mut self, id: WidgetId) -> Option<&mut SceneNode> {
        self.nodes.get_mut(&id)
    }

    /// Whether the cache contains a node for `id`.
    pub fn contains_key(&self, id: WidgetId) -> bool {
        self.nodes.contains_key(&id)
    }

    /// Returns the number of cached nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Returns an iterator over all nodes.
    pub fn values(&self) -> impl Iterator<Item = &SceneNode> {
        self.nodes.values()
    }

    /// Returns an iterator over all nodes (mutable).
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut SceneNode> {
        self.nodes.values_mut()
    }

    // Operations

    /// Inserts a node directly. Used by tests to pre-populate the cache.
    pub fn insert(&mut self, id: WidgetId, node: SceneNode) {
        self.nodes.insert(id, node);
    }

    /// Removes all cached nodes and resets containment tracking.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.store_log.clear();
    }

    /// Returns the current position in the store log.
    ///
    /// Containers call this before `child.draw()` and pass the result to
    /// [`store`] so the node records which descendants were drawn.
    pub fn log_position(&self) -> usize {
        self.store_log.len()
    }

    /// Stores a child's cached draw output with containment tracking.
    ///
    /// `log_start` is the log position captured before the child's draw.
    /// All IDs stored between `log_start` and now are recorded as contained
    /// descendants of this node.
    pub fn store(
        &mut self,
        child_id: WidgetId,
        commands: Vec<DrawCommand>,
        bounds: Rect,
        log_start: usize,
    ) {
        let contained = self.store_log[log_start..].to_vec();
        self.nodes
            .entry(child_id)
            .or_insert_with(|| SceneNode::new(child_id))
            .update_with_contained(commands, bounds, contained);
        self.store_log.push(child_id);
    }

    /// Resets the store log for a new compose pass.
    pub fn reset_log(&mut self) {
        self.store_log.clear();
    }
}

impl Default for SceneCache {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Index<&WidgetId> for SceneCache {
    type Output = SceneNode;

    fn index(&self, id: &WidgetId) -> &Self::Output {
        &self.nodes[id]
    }
}

/// Cached draw output for a single widget.
///
/// Stores the draw command sequence from the widget's last `draw()` call,
/// the layout bounds that produced it, and the set of descendant widget IDs
/// whose output is embedded in these commands (for invalidation propagation).
#[derive(Debug)]
pub struct SceneNode {
    /// Widget that owns this cache entry.
    widget_id: WidgetId,
    /// Cached draw commands from the last draw.
    commands: Vec<DrawCommand>,
    /// Bounds that produced these commands (layout output).
    bounds: Rect,
    /// Whether this cache entry is valid for replay.
    valid: bool,
    /// Descendant widget IDs whose draw output is embedded in these commands.
    ///
    /// Used by `invalidate_dirty_nodes` to propagate invalidation upward:
    /// if any contained ID is dirty, this node must also be invalidated.
    contained: Vec<WidgetId>,
}

impl SceneNode {
    /// Creates a new empty scene node for `widget_id`.
    pub fn new(widget_id: WidgetId) -> Self {
        Self {
            widget_id,
            commands: Vec::new(),
            bounds: Rect::default(),
            valid: false,
            contained: Vec::new(),
        }
    }

    // Accessors

    /// Returns the widget ID this node caches.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_id
    }

    /// Returns the cached draw commands.
    pub fn commands(&self) -> &[DrawCommand] {
        &self.commands
    }

    /// Returns the bounds that produced the cached commands.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the descendant widget IDs embedded in this node's commands.
    pub fn contained(&self) -> &[WidgetId] {
        &self.contained
    }

    // Predicates

    /// Whether the cached commands are valid for replay.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    // Operations

    /// Marks this cache entry as invalid, forcing a rebuild on next compose.
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// Replaces the cached commands and bounds, marking the node as valid.
    pub fn update(&mut self, commands: Vec<DrawCommand>, bounds: Rect) {
        self.commands = commands;
        self.bounds = bounds;
        self.valid = true;
        self.contained.clear();
    }

    /// Replaces the cached commands, bounds, and contained IDs.
    pub fn update_with_contained(
        &mut self,
        commands: Vec<DrawCommand>,
        bounds: Rect,
        contained: Vec<WidgetId>,
    ) {
        self.commands = commands;
        self.bounds = bounds;
        self.valid = true;
        self.contained = contained;
    }
}

#[cfg(test)]
mod tests;
