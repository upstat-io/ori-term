//! Subtree invalidation tracking for the retained UI pipeline.
//!
//! Provides typed, scoped dirty signals that propagate through the widget
//! tree. [`DirtyKind`] distinguishes clean state from structural changes
//! (text content, child add/remove) that require relayout. Paint-only
//! invalidation is handled by full-scene rebuild via `build_scene()` +
//! `DamageTracker` diffing, so no per-widget paint tracking is needed.
//!
//! [`InvalidationTracker`] records which widgets are dirty and at what level,
//! replacing the coarse-grained `dirty: bool` flags on window contexts.

use std::collections::HashSet;

use crate::controllers::ControllerRequests;
use crate::widget_id::WidgetId;

/// What kind of invalidation a widget event produced.
///
/// Two-level: `Clean` (no change) and `Layout` (structural change).
/// Paint-only invalidation is handled by full-scene rebuild + damage
/// diffing, so there is no `Paint` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyKind {
    /// No change — skip redraw entirely.
    Clean,
    /// Structural change (text content, child add/remove, visibility).
    /// Recompute layout from this widget upward, then repaint.
    Layout,
}

impl DirtyKind {
    /// Merges two dirty kinds, returning the higher-severity one.
    ///
    /// Used when a container accumulates dirty signals from multiple
    /// children: `Clean.merge(Layout)` -> `Layout`.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Layout, _) | (_, Self::Layout) => Self::Layout,
            _ => Self::Clean,
        }
    }

    /// Returns `true` if this kind is not `Clean`.
    pub fn is_dirty(self) -> bool {
        !matches!(self, Self::Clean)
    }
}

impl From<ControllerRequests> for DirtyKind {
    /// Maps controller request flags to the corresponding dirty kind.
    ///
    /// Paint-only invalidation is handled by full-scene rebuild + damage
    /// diffing, so `PAINT` maps to `Clean`. Only structural layout flags
    /// (when added) would map to `Layout`.
    fn from(_requests: ControllerRequests) -> Self {
        Self::Clean
    }
}

/// Tracks which widgets are dirty and at what level.
///
/// A lightweight structure that records per-widget invalidation state.
/// The render path queries this to decide what to rebuild. A full
/// invalidation (resize, theme change) overrides per-widget tracking.
pub struct InvalidationTracker {
    /// Layout-dirty widgets (need relayout + redraw).
    layout_dirty: HashSet<WidgetId>,
    /// Whether the entire scene needs rebuild (e.g. theme change, resize).
    full_invalidation: bool,
}

impl InvalidationTracker {
    /// Creates a tracker with no dirty state.
    pub fn new() -> Self {
        Self {
            layout_dirty: HashSet::new(),
            full_invalidation: false,
        }
    }

    /// Marks a widget as dirty at the given level.
    ///
    /// `Layout` adds to the layout set. `Clean` is a no-op.
    pub fn mark(&mut self, id: WidgetId, kind: DirtyKind) {
        match kind {
            DirtyKind::Clean => {}
            DirtyKind::Layout => {
                self.layout_dirty.insert(id);
            }
        }
    }

    /// Returns `true` if the widget needs relayout.
    pub fn is_layout_dirty(&self, id: WidgetId) -> bool {
        self.full_invalidation || self.layout_dirty.contains(&id)
    }

    /// Returns `true` if any widget is dirty or a full invalidation is pending.
    pub fn is_any_dirty(&self) -> bool {
        self.full_invalidation || !self.layout_dirty.is_empty()
    }

    /// Returns `true` if a full scene rebuild is needed.
    pub fn needs_full_rebuild(&self) -> bool {
        self.full_invalidation
    }

    /// Clears all dirty state after a render pass completes.
    pub fn clear(&mut self) {
        self.layout_dirty.clear();
        self.full_invalidation = false;
    }

    /// Marks the entire scene for rebuild.
    ///
    /// Called on resize, theme change, font change, and scale factor change.
    pub fn invalidate_all(&mut self) {
        self.full_invalidation = true;
    }
}

impl Default for InvalidationTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
