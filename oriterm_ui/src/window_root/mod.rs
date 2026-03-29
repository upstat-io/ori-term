//! Per-window UI composition unit.
//!
//! `WindowRoot` consolidates all pure UI framework state needed for a single
//! window: widget tree, interaction, focus, overlays, compositor, animation
//! scheduling, invalidation, and damage tracking. Both `WidgetTestHarness`
//! (testing) and `WindowContext`/`DialogWindowContext` (production) wrap this
//! type.
//!
//! `WindowRoot` owns no GPU, platform, or terminal-specific state. It can be
//! constructed in a `#[test]` without a display server, GPU device, or PTY.
//!
//! Submodules:
//! - `pipeline`: Layout computation, event dispatch, prepare/rebuild phases.
//! - `overlay_ops`: Overlay convenience methods with borrow splitting.
//! - `borrow_split`: Multi-field borrow-splitting accessor methods.

mod borrow_split;
mod overlay_ops;
mod pipeline;

use std::collections::HashMap;

use crate::action::{Keymap, WidgetAction};
use crate::animation::{FrameRequestFlags, RenderScheduler};
use crate::compositor::{LayerAnimator, LayerTree};
use crate::draw::DamageTracker;
use crate::focus::FocusManager;
use crate::geometry::Rect;
use crate::input::Key;
use crate::interaction::InteractionManager;
use crate::invalidation::{DirtyKind, InvalidationTracker};
use crate::layout::LayoutNode;
use crate::overlay::OverlayManager;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;

/// Per-window UI composition unit.
///
/// Owns the widget tree and all framework state needed to process events,
/// compute layout, manage focus, and track interaction — without requiring
/// a GPU, platform window, or terminal. Both `WidgetTestHarness` (testing)
/// and `WindowContext` (production) wrap this type.
pub struct WindowRoot {
    // Widget tree
    widget: Box<dyn Widget>,
    layout: LayoutNode,
    viewport: Rect,

    // Framework state
    interaction: InteractionManager,
    focus: FocusManager,
    overlays: OverlayManager,

    // Keymap dispatch
    keymap: Keymap,
    key_contexts: HashMap<WidgetId, &'static str>,
    last_keymap_handled: Option<Key>,

    // Compositor
    layer_tree: LayerTree,
    layer_animator: LayerAnimator,

    // Animation & scheduling
    frame_requests: FrameRequestFlags,
    scheduler: RenderScheduler,

    // Invalidation & damage
    invalidation: InvalidationTracker,
    damage: DamageTracker,

    // Redraw tracking
    dirty: bool,
    urgent_redraw: bool,

    // Action queue
    pending_actions: Vec<WidgetAction>,
}

impl WindowRoot {
    // -- Constructors --

    /// Creates a new `WindowRoot` with the given root widget and a default
    /// 800x600 viewport.
    ///
    /// Runs an initial `rebuild()` to register widgets and build focus order.
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self::with_viewport(widget, Rect::new(0.0, 0.0, 800.0, 600.0))
    }

    /// Creates a new `WindowRoot` with the given root widget and viewport.
    ///
    /// Runs an initial `rebuild()` to register widgets and build focus order.
    pub fn with_viewport(widget: impl Widget + 'static, viewport: Rect) -> Self {
        let mut root = Self {
            widget: Box::new(widget),
            layout: LayoutNode::new(Rect::default(), Rect::default()),
            viewport,
            interaction: InteractionManager::new(),
            focus: FocusManager::new(),
            overlays: OverlayManager::new(viewport),
            keymap: Keymap::defaults(),
            key_contexts: HashMap::new(),
            last_keymap_handled: None,
            layer_tree: LayerTree::new(viewport),
            layer_animator: LayerAnimator::new(),
            frame_requests: FrameRequestFlags::new(),
            scheduler: RenderScheduler::new(),
            invalidation: InvalidationTracker::new(),
            damage: DamageTracker::new(),
            dirty: true,
            urgent_redraw: false,
            pending_actions: Vec::new(),
        };
        root.rebuild();
        root
    }

    // -- Widget tree accessors --

    /// Returns a reference to the root widget.
    pub fn widget(&self) -> &dyn Widget {
        &*self.widget
    }

    /// Returns a mutable reference to the root widget.
    pub fn widget_mut(&mut self) -> &mut dyn Widget {
        &mut *self.widget
    }

    /// Replaces the root widget and triggers a full rebuild.
    pub fn replace_widget(&mut self, widget: Box<dyn Widget>) {
        self.widget = widget;
        self.rebuild();
    }

    /// Swaps the root widget without running rebuild.
    ///
    /// Test-only: allows verifying that `compute_layout()` handles GC
    /// independently of `rebuild()`.
    #[cfg(test)]
    pub(crate) fn set_widget_raw(&mut self, widget: Box<dyn Widget>) {
        self.widget = widget;
    }

    // -- Layout accessors --

    /// Returns the computed layout tree.
    pub fn layout(&self) -> &LayoutNode {
        &self.layout
    }

    /// Returns the current viewport.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Updates the viewport and marks layout dirty.
    pub fn set_viewport(&mut self, viewport: Rect) {
        self.viewport = viewport;
        self.overlays.set_viewport(viewport);
        let root_id = self.layer_tree.root();
        self.layer_tree.set_bounds(root_id, viewport);
        self.dirty = true;
    }

    // -- Framework accessors --

    /// Returns a reference to the interaction manager.
    pub fn interaction(&self) -> &InteractionManager {
        &self.interaction
    }

    /// Returns a mutable reference to the interaction manager.
    pub fn interaction_mut(&mut self) -> &mut InteractionManager {
        &mut self.interaction
    }

    /// Returns a reference to the focus manager.
    pub fn focus(&self) -> &FocusManager {
        &self.focus
    }

    /// Returns a mutable reference to the focus manager.
    pub fn focus_mut(&mut self) -> &mut FocusManager {
        &mut self.focus
    }

    /// Returns a reference to the overlay manager.
    pub fn overlays(&self) -> &OverlayManager {
        &self.overlays
    }

    /// Returns a mutable reference to the overlay manager.
    pub fn overlays_mut(&mut self) -> &mut OverlayManager {
        &mut self.overlays
    }

    /// Returns a reference to the keymap.
    pub fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    /// Returns a mutable reference to the keymap (for runtime rebinding).
    pub fn keymap_mut(&mut self) -> &mut Keymap {
        &mut self.keymap
    }

    /// Returns a reference to the per-widget key context map.
    pub fn key_contexts(&self) -> &HashMap<WidgetId, &'static str> {
        &self.key_contexts
    }

    /// Returns a mutable reference to the per-widget key context map.
    pub fn key_contexts_mut(&mut self) -> &mut HashMap<WidgetId, &'static str> {
        &mut self.key_contexts
    }

    /// Sets the last key handled by keymap (for `KeyUp` suppression).
    pub fn set_last_keymap_handled(&mut self, key: Option<Key>) {
        self.last_keymap_handled = key;
    }

    /// Returns the last key handled by keymap.
    pub fn last_keymap_handled(&self) -> Option<Key> {
        self.last_keymap_handled
    }

    // -- Compositor accessors --

    /// Returns a reference to the compositor layer tree.
    pub fn layer_tree(&self) -> &LayerTree {
        &self.layer_tree
    }

    /// Returns a mutable reference to the compositor layer tree.
    pub fn layer_tree_mut(&mut self) -> &mut LayerTree {
        &mut self.layer_tree
    }

    /// Returns a reference to the layer animator.
    pub fn layer_animator(&self) -> &LayerAnimator {
        &self.layer_animator
    }

    /// Returns a mutable reference to the layer animator.
    pub fn layer_animator_mut(&mut self) -> &mut LayerAnimator {
        &mut self.layer_animator
    }

    // -- Animation accessors --

    /// Returns a reference to the frame request flags.
    pub fn frame_requests(&self) -> &FrameRequestFlags {
        &self.frame_requests
    }

    /// Returns a mutable reference to the frame request flags.
    pub fn frame_requests_mut(&mut self) -> &mut FrameRequestFlags {
        &mut self.frame_requests
    }

    /// Returns a reference to the render scheduler.
    pub fn scheduler(&self) -> &RenderScheduler {
        &self.scheduler
    }

    /// Returns a mutable reference to the render scheduler.
    pub fn scheduler_mut(&mut self) -> &mut RenderScheduler {
        &mut self.scheduler
    }

    // -- Invalidation & damage accessors --

    /// Returns a reference to the invalidation tracker.
    pub fn invalidation(&self) -> &InvalidationTracker {
        &self.invalidation
    }

    /// Returns a mutable reference to the invalidation tracker.
    pub fn invalidation_mut(&mut self) -> &mut InvalidationTracker {
        &mut self.invalidation
    }

    /// Marks the given widgets as `Prepaint`-dirty in the invalidation tracker.
    ///
    /// Uses the interaction manager's parent map for dirty-ancestor propagation.
    /// This is the bridge between `InteractionManager` (which returns changed IDs)
    /// and `InvalidationTracker` (which tracks per-widget dirty state).
    pub fn mark_widgets_prepaint_dirty(&mut self, ids: &[WidgetId]) {
        let parent_map = self.interaction.parent_map_ref();
        for &id in ids {
            self.invalidation.mark(id, DirtyKind::Prepaint, parent_map);
        }
    }

    /// Clear the hot widget path and mark affected widgets dirty.
    ///
    /// Call after rebuilding a widget tree to prevent stale hover state
    /// from the old tree. The next cursor move recomputes the hot path.
    pub fn clear_hot_path(&mut self) {
        let changed = self.interaction.update_hot_path(&[]);
        self.mark_widgets_prepaint_dirty(&changed);
    }

    /// Recompute the hot path from a cursor position against the current layout.
    ///
    /// Unlike `clear_hot_path()` which unconditionally drops all hover,
    /// this preserves hover on widgets that survive a tree rebuild and are
    /// still under the cursor. Uses `layout_hit_test_path` against
    /// `self.layout` to determine which widgets the cursor is over.
    pub fn refresh_hot_path(&mut self, pos: crate::geometry::Point) {
        use crate::input::layout_hit_test_path;
        let hit_result = layout_hit_test_path(&self.layout, pos);
        let ids = hit_result.widget_ids();
        let changed = self.interaction.update_hot_path(&ids);
        self.mark_widgets_prepaint_dirty(&changed);
    }

    /// Returns a reference to the damage tracker.
    pub fn damage(&self) -> &DamageTracker {
        &self.damage
    }

    /// Returns a mutable reference to the damage tracker.
    pub fn damage_mut(&mut self) -> &mut DamageTracker {
        &mut self.damage
    }

    // -- Redraw predicates --

    /// Returns whether the window needs a redraw.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the window as needing a redraw.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Clears the dirty flag after a successful redraw.
    pub fn clear_dirty(&mut self) {
        self.dirty = false;
    }

    /// Returns whether this window should bypass the normal frame budget once.
    pub fn is_urgent_redraw(&self) -> bool {
        self.urgent_redraw
    }

    /// Sets or clears the urgent redraw flag.
    pub fn set_urgent_redraw(&mut self, urgent: bool) {
        self.urgent_redraw = urgent;
    }

    // -- Action queue --

    /// Takes all pending actions, leaving the queue empty.
    pub fn take_actions(&mut self) -> Vec<WidgetAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns whether there are any pending actions.
    pub fn has_pending_actions(&self) -> bool {
        !self.pending_actions.is_empty()
    }

    /// Clears the pending action queue without returning its contents.
    pub fn clear_actions(&mut self) {
        self.pending_actions.clear();
    }

    /// Removes and returns the first pending action, or `None`.
    pub fn pop_action(&mut self) -> Option<WidgetAction> {
        if self.pending_actions.is_empty() {
            None
        } else {
            Some(self.pending_actions.remove(0))
        }
    }
}

#[cfg(test)]
mod tests;
