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

mod pipeline;

use std::collections::HashMap;

use std::time::Instant;

use crate::action::{Keymap, WidgetAction};
use crate::animation::{FrameRequestFlags, RenderScheduler};
use crate::compositor::{LayerAnimator, LayerTree};
use crate::draw::DamageTracker;
use crate::focus::FocusManager;
use crate::geometry::Rect;
use crate::input::Key;
use crate::interaction::InteractionManager;
use crate::invalidation::InvalidationTracker;
use crate::layout::LayoutNode;
use crate::overlay::{OverlayId, OverlayManager, Placement};
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

    /// Returns mutable references to both the interaction and focus managers.
    ///
    /// Borrow splitting: callers that need `&mut InteractionManager` and
    /// `&mut FocusManager` simultaneously (e.g. `apply_dispatch_requests`,
    /// `request_focus`) cannot call `interaction_mut()` and `focus_mut()`
    /// in the same expression because each takes `&mut self`.
    pub fn interaction_and_focus_mut(&mut self) -> (&mut InteractionManager, &mut FocusManager) {
        (&mut self.interaction, &mut self.focus)
    }

    /// Returns `&mut InteractionManager` and `&FrameRequestFlags` simultaneously.
    ///
    /// Borrow splitting: `prepare_widget_tree` needs `&mut InteractionManager`
    /// while also receiving `Option<&FrameRequestFlags>`. Since both live inside
    /// `WindowRoot`, separate accessor calls would conflict.
    pub fn interaction_mut_and_frame_requests(
        &mut self,
    ) -> (&mut InteractionManager, &FrameRequestFlags) {
        (&mut self.interaction, &self.frame_requests)
    }

    /// Returns `&InteractionManager` and `&FrameRequestFlags` simultaneously.
    ///
    /// Borrow splitting for `prepaint_widget_tree`, which reads interaction
    /// state and frame request flags in the same call.
    pub fn interaction_and_frame_requests(&self) -> (&InteractionManager, &FrameRequestFlags) {
        (&self.interaction, &self.frame_requests)
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

    // -- Overlay operations --

    /// Pushes a popup overlay at the given anchor with the specified placement.
    ///
    /// Convenience method that handles the borrow splitting needed to call
    /// `OverlayManager::push_overlay` with `LayerTree` and `LayerAnimator`.
    pub fn push_overlay(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        now: Instant,
    ) -> OverlayId {
        self.overlays.push_overlay(
            widget,
            anchor,
            placement,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Returns whether any overlays are active.
    pub fn has_overlays(&self) -> bool {
        !self.overlays.is_empty()
    }

    /// Replaces the topmost popup overlay with a new widget.
    ///
    /// Handles borrow splitting for `OverlayManager`, `LayerTree`, `LayerAnimator`.
    pub fn replace_popup(
        &mut self,
        widget: Box<dyn Widget>,
        anchor: Rect,
        placement: Placement,
        now: Instant,
    ) -> OverlayId {
        self.overlays.replace_popup(
            widget,
            anchor,
            placement,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Begins dismissing the topmost overlay with a fade-out animation.
    pub fn dismiss_topmost(&mut self, now: Instant) -> Option<OverlayId> {
        self.overlays
            .begin_dismiss_topmost(&mut self.layer_tree, &mut self.layer_animator, now)
    }

    /// Removes all popup overlays immediately.
    pub fn clear_popups(&mut self) -> usize {
        self.overlays
            .clear_popups(&mut self.layer_tree, &mut self.layer_animator)
    }

    /// Routes a mouse event through the overlay manager.
    ///
    /// Returns `PassThrough` if no overlay consumed the event.
    #[expect(
        clippy::too_many_arguments,
        reason = "forwarding overlay manager params with borrow splitting"
    )]
    pub fn process_overlay_mouse_event(
        &mut self,
        event: &crate::input::MouseEvent,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &crate::theme::UiTheme,
        focused_widget: Option<WidgetId>,
        now: Instant,
    ) -> crate::overlay::OverlayEventResult {
        self.overlays.process_mouse_event(
            event,
            measurer,
            theme,
            focused_widget,
            &mut self.layer_tree,
            &mut self.layer_animator,
            now,
        )
    }

    /// Returns the number of drawable overlays.
    pub fn overlay_draw_count(&self) -> usize {
        self.overlays.draw_count()
    }

    /// Computes layout for all overlay widgets.
    pub fn layout_overlays(
        &mut self,
        measurer: &dyn crate::widgets::TextMeasurer,
        theme: &crate::theme::UiTheme,
    ) {
        self.overlays.layout_overlays(measurer, theme);
    }

    /// Draws overlay at the given draw index, returning its opacity.
    ///
    /// Handles borrow splitting between `overlays` and `layer_tree`.
    pub fn draw_overlay_at(&self, draw_idx: usize, ctx: &mut crate::widgets::DrawCtx<'_>) -> f32 {
        self.overlays
            .draw_overlay_at(draw_idx, ctx, &self.layer_tree)
    }

    /// Ticks layer animations and cleans up dismissed overlays.
    ///
    /// Returns `true` if any animations are still in progress (the caller
    /// should schedule a repaint). Handles borrow splitting for
    /// `layer_animator`, `layer_tree`, and `overlays`.
    pub fn tick_overlay_animations(&mut self, now: Instant) -> bool {
        if !self.layer_animator.is_any_animating() {
            return false;
        }
        let animating = self.layer_animator.tick(&mut self.layer_tree, now);
        self.overlays
            .cleanup_dismissed(&mut self.layer_tree, &self.layer_animator);
        animating
    }
}

#[cfg(test)]
mod tests;
