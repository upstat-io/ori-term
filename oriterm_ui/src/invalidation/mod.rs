//! Subtree invalidation tracking for the retained UI pipeline.
//!
//! Provides typed, scoped dirty signals that propagate through the widget
//! tree. [`DirtyKind`] distinguishes paint-only changes (hover color, focus
//! ring) from structural changes (text content, child add/remove) so the
//! render path can skip unchanged subtrees.
//!
//! [`InvalidationTracker`] records which widgets are dirty and at what level,
//! replacing the coarse-grained `dirty: bool` flags on window contexts.

use std::collections::HashSet;

use crate::controllers::ControllerRequests;
use crate::widget_id::WidgetId;

/// What kind of invalidation a widget event produced.
///
/// Ordered by severity: `Clean` < `Paint` < `Layout`. The `merge` method
/// returns the higher-severity kind, used when a container receives dirty
/// signals from multiple children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyKind {
    /// No change — skip redraw entirely.
    Clean,
    /// Visual change only (hover color, focus ring, cursor blink).
    /// Repaint the widget but don't recompute layout.
    Paint,
    /// Structural change (text content, child add/remove, visibility).
    /// Recompute layout from this widget upward, then repaint.
    Layout,
}

impl DirtyKind {
    /// Merges two dirty kinds, returning the higher-severity one.
    ///
    /// Used when a container accumulates dirty signals from multiple
    /// children: `Clean.merge(Paint)` → `Paint`, `Paint.merge(Layout)` → `Layout`.
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Layout, _) | (_, Self::Layout) => Self::Layout,
            (Self::Paint, _) | (_, Self::Paint) => Self::Paint,
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
    /// - No paint/layout flags → `Clean`
    /// - `PAINT` flag → `Paint` (visual change, no relayout)
    /// - No explicit layout flag exists yet — layout invalidation is
    ///   triggered by structural widget changes, not controller requests.
    fn from(requests: ControllerRequests) -> Self {
        if requests.contains(ControllerRequests::PAINT) {
            Self::Paint
        } else {
            Self::Clean
        }
    }
}

/// Tracks which widgets are dirty and at what level.
///
/// A lightweight structure that records per-widget invalidation state.
/// The render path queries this to decide what to rebuild. A full
/// invalidation (resize, theme change) overrides per-widget tracking.
pub struct InvalidationTracker {
    /// Paint-dirty widgets (need redraw but not relayout).
    paint_dirty: HashSet<WidgetId>,
    /// Layout-dirty widgets (need relayout + redraw).
    layout_dirty: HashSet<WidgetId>,
    /// Whether the entire scene needs rebuild (e.g. theme change, resize).
    full_invalidation: bool,
}

impl InvalidationTracker {
    /// Creates a tracker with no dirty state.
    pub fn new() -> Self {
        Self {
            paint_dirty: HashSet::new(),
            layout_dirty: HashSet::new(),
            full_invalidation: false,
        }
    }

    /// Marks a widget as dirty at the given level.
    ///
    /// `Paint` adds to the paint set. `Layout` adds to the layout set
    /// (and implies paint). `Clean` is a no-op.
    pub fn mark(&mut self, id: WidgetId, kind: DirtyKind) {
        match kind {
            DirtyKind::Clean => {}
            DirtyKind::Paint => {
                self.paint_dirty.insert(id);
            }
            DirtyKind::Layout => {
                self.layout_dirty.insert(id);
            }
        }
    }

    /// Returns `true` if the widget needs repainting (paint-dirty or layout-dirty).
    pub fn is_paint_dirty(&self, id: WidgetId) -> bool {
        self.full_invalidation || self.paint_dirty.contains(&id) || self.layout_dirty.contains(&id)
    }

    /// Returns `true` if the widget needs relayout.
    pub fn is_layout_dirty(&self, id: WidgetId) -> bool {
        self.full_invalidation || self.layout_dirty.contains(&id)
    }

    /// Returns `true` if any widget is dirty or a full invalidation is pending.
    pub fn is_any_dirty(&self) -> bool {
        self.full_invalidation || !self.paint_dirty.is_empty() || !self.layout_dirty.is_empty()
    }

    /// Returns `true` if a full scene rebuild is needed.
    pub fn needs_full_rebuild(&self) -> bool {
        self.full_invalidation
    }

    /// Clears all dirty state after a render pass completes.
    pub fn clear(&mut self) {
        self.paint_dirty.clear();
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
