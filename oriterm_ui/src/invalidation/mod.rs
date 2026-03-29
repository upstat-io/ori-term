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

use std::collections::{HashMap, HashSet};

use crate::controllers::ControllerRequests;
use crate::widget_id::WidgetId;

/// What kind of invalidation a widget event produced.
///
/// Four levels in ascending severity. Variant declaration order matches
/// severity for `derive(PartialOrd, Ord)`: `Clean < Paint < Prepaint < Layout`.
///
/// - `Clean` — no change, skip all phases.
/// - `Paint` — visual-only change (cursor blink), skip layout + prepaint.
/// - `Prepaint` — interaction state change (hover), skip layout.
/// - `Layout` — structural change, run all three phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DirtyKind {
    /// No change — skip redraw entirely.
    Clean,
    /// Visual-only change (cursor blink) — skip layout + prepaint, run paint.
    Paint,
    /// Interaction state change (hover) — skip layout, run prepaint + paint.
    Prepaint,
    /// Structural change (text content, child add/remove, visibility).
    /// Recompute layout from this widget upward, then prepaint + repaint.
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
            (Self::Prepaint, _) | (_, Self::Prepaint) => Self::Prepaint,
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
    /// `PAINT` flag maps to `Paint` (visual-only invalidation). Other flags
    /// don't imply dirty state on their own.
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
    /// Per-widget dirty level (highest severity wins).
    dirty_map: HashMap<WidgetId, DirtyKind>,
    /// Ancestor IDs of dirty widgets, for O(1) subtree-dirty queries.
    ///
    /// When `mark()` is called with a parent map, all ancestors of the
    /// marked widget are inserted here. `has_dirty_descendant(id)` checks
    /// this set to decide whether a subtree can be skipped during
    /// selective tree walks.
    dirty_ancestors: HashSet<WidgetId>,
    /// Whether the entire scene needs rebuild (e.g. theme change, resize).
    full_invalidation: bool,
}

impl InvalidationTracker {
    /// Creates a tracker with no dirty state.
    pub fn new() -> Self {
        Self {
            dirty_map: HashMap::new(),
            dirty_ancestors: HashSet::new(),
            full_invalidation: false,
        }
    }

    /// Marks a widget as dirty at the given level.
    ///
    /// Merges with existing dirty level: marking `Paint` then `Prepaint`
    /// on the same widget promotes to `Prepaint`. `Clean` is a no-op.
    ///
    /// `parent_map` maps child → parent widget IDs. When provided, all
    /// ancestors of `id` are inserted into `dirty_ancestors` for O(1)
    /// subtree-dirty queries via [`has_dirty_descendant`].
    pub fn mark(
        &mut self,
        id: WidgetId,
        kind: DirtyKind,
        parent_map: &HashMap<WidgetId, WidgetId>,
    ) {
        if kind == DirtyKind::Clean {
            return;
        }
        self.dirty_map
            .entry(id)
            .and_modify(|existing| *existing = existing.merge(kind))
            .or_insert(kind);

        // Propagate dirty-ancestor flags upward for subtree queries.
        let mut cursor = id;
        while let Some(&parent) = parent_map.get(&cursor) {
            if !self.dirty_ancestors.insert(parent) {
                break; // already marked — ancestors above are too
            }
            cursor = parent;
        }
    }

    /// Returns `true` if any descendant of `id` is dirty.
    ///
    /// O(1) lookup in the `dirty_ancestors` set. Also returns `true` if
    /// `id` itself is dirty (it may be both an ancestor and a dirty widget).
    pub fn has_dirty_descendant(&self, id: WidgetId) -> bool {
        self.dirty_ancestors.contains(&id) || self.dirty_map.contains_key(&id)
    }

    /// Returns `true` if the widget needs relayout.
    pub fn is_layout_dirty(&self, id: WidgetId) -> bool {
        self.full_invalidation
            || self
                .dirty_map
                .get(&id)
                .is_some_and(|k| *k >= DirtyKind::Layout)
    }

    /// Returns `true` if the widget needs prepaint (or higher).
    pub fn is_prepaint_dirty(&self, id: WidgetId) -> bool {
        self.full_invalidation
            || self
                .dirty_map
                .get(&id)
                .is_some_and(|k| *k >= DirtyKind::Prepaint)
    }

    /// Returns `true` if the widget needs paint (or higher).
    pub fn is_paint_dirty(&self, id: WidgetId) -> bool {
        self.full_invalidation
            || self
                .dirty_map
                .get(&id)
                .is_some_and(|k| *k >= DirtyKind::Paint)
    }

    /// Returns the highest dirty level across all tracked widgets.
    ///
    /// Used by the app layer to decide which pipeline phases to run.
    /// Returns `Layout` when `full_invalidation` is set.
    pub fn max_dirty_kind(&self) -> DirtyKind {
        if self.full_invalidation {
            return DirtyKind::Layout;
        }
        self.dirty_map
            .values()
            .copied()
            .max()
            .unwrap_or(DirtyKind::Clean)
    }

    /// Returns `true` if any widget is dirty or a full invalidation is pending.
    pub fn is_any_dirty(&self) -> bool {
        self.full_invalidation || !self.dirty_map.is_empty()
    }

    /// Returns `true` if a full scene rebuild is needed.
    pub fn needs_full_rebuild(&self) -> bool {
        self.full_invalidation
    }

    /// Clears all dirty state after a render pass completes.
    pub fn clear(&mut self) {
        self.dirty_map.clear();
        self.dirty_ancestors.clear();
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
