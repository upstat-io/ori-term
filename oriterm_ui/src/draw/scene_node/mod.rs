//! Cached draw output for a widget subtree.
//!
//! A [`SceneNode`] stores the draw command sequence produced by a widget's
//! last successful `draw()` call along with the bounds that produced it.
//! The scene composition pass replays cached commands for clean subtrees,
//! avoiding redundant `Widget::draw()` calls.

use std::collections::HashMap;

use crate::draw::DrawCommand;
use crate::geometry::Rect;
use crate::widget_id::WidgetId;

/// Flat per-widget scene cache mapping widget IDs to their cached draw output.
pub type SceneCache = HashMap<WidgetId, SceneNode>;

/// Cached draw output for a single widget.
///
/// Flat per-widget cache — does not own child nodes. The widget tree already
/// provides the hierarchy; the scene node is a cache entry, not a parallel tree.
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
}

impl SceneNode {
    /// Creates a new empty scene node for `widget_id`.
    pub fn new(widget_id: WidgetId) -> Self {
        Self {
            widget_id,
            commands: Vec::new(),
            bounds: Rect::default(),
            valid: false,
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
    }
}

#[cfg(test)]
mod tests;
