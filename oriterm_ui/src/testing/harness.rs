//! Core test harness struct and layout management.
//!
//! [`WidgetTestHarness`] wraps a root widget and provides the full framework
//! pipeline — layout, hit testing, event propagation, interaction state,
//! lifecycle events, and animation scheduling — without requiring a GPU,
//! window, or real font stack.

use std::collections::HashMap;
use std::time::Instant;

use crate::action::{Keymap, WidgetAction, collect_key_contexts};
use crate::animation::FrameRequestFlags;
use crate::animation::scheduler::RenderScheduler;
use crate::focus::FocusManager;
use crate::geometry::{Point, Rect};
use crate::input::Key;
use crate::interaction::{InteractionManager, build_parent_map};
use crate::layout::{LayoutNode, compute_layout};
use crate::pipeline::{collect_focusable_ids, register_widget_tree};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::widgets::contexts::LayoutCtx;

use super::MockMeasurer;

/// Headless test harness for widget integration testing.
///
/// Wraps a root widget and wires up the full framework pipeline:
/// layout solver, hit testing, event propagation, interaction state,
/// lifecycle events, and animation scheduling — without requiring
/// a GPU, window, or real font stack.
///
/// # Overlay Testing
///
/// Overlay widgets (dropdowns, modals) are managed by `OverlayManager`,
/// which is not included in the harness. Test overlay widgets in isolation
/// by wrapping them as the root widget of a harness. The `OverlayManager`'s
/// event routing and dismiss logic can be tested via unit tests on
/// `OverlayManager` directly.
///
// TODO: OverlayTestHarness for end-to-end overlay flow testing
pub struct WidgetTestHarness {
    /// The root widget under test.
    pub(super) widget: Box<dyn Widget>,
    /// Computed layout tree (from last `rebuild_layout()`).
    pub(super) layout: LayoutNode,
    /// Interaction state manager.
    pub(super) interaction: InteractionManager,
    /// Focus manager.
    pub(super) focus: FocusManager,
    /// Animation/paint request scheduler.
    pub(super) scheduler: RenderScheduler,
    /// Current simulated time (advanced via `advance_time()`).
    pub(super) clock: Instant,
    /// Mock text measurer.
    pub(super) measurer: MockMeasurer,
    /// Theme for rendering.
    pub(super) theme: UiTheme,
    /// Viewport size.
    pub(super) viewport: Rect,
    /// Collected actions from event dispatch.
    pub(super) pending_actions: Vec<WidgetAction>,
    /// Frame request flags (shared with widget contexts).
    pub(super) frame_requests: FrameRequestFlags,
    /// Current mouse position (for mouse_down/mouse_up without explicit pos).
    pub(super) mouse_pos: Point,
    /// Keymap for action dispatch (defaults loaded at construction).
    pub(super) keymap: Keymap,
    /// Per-widget key context tags (collected during `rebuild_layout()`).
    pub(super) key_contexts: HashMap<WidgetId, &'static str>,
    /// Last key handled by keymap (for `KeyUp` suppression).
    pub(super) last_keymap_handled: Option<Key>,
}

/// Default viewport width for the test harness (800px).
const DEFAULT_WIDTH: f32 = 800.0;
/// Default viewport height for the test harness (600px).
const DEFAULT_HEIGHT: f32 = 600.0;

impl WidgetTestHarness {
    /// Creates a harness wrapping `widget` in a default 800x600 viewport.
    pub fn new(widget: impl Widget + 'static) -> Self {
        Self::with_size(widget, DEFAULT_WIDTH, DEFAULT_HEIGHT)
    }

    /// Creates a harness with a custom viewport size.
    pub fn with_size(widget: impl Widget + 'static, width: f32, height: f32) -> Self {
        let mut harness = Self {
            widget: Box::new(widget),
            layout: LayoutNode::new(Rect::default(), Rect::default()),
            interaction: InteractionManager::new(),
            focus: FocusManager::new(),
            scheduler: RenderScheduler::new(),
            clock: Instant::now(),
            measurer: MockMeasurer::new(),
            theme: UiTheme::dark(),
            viewport: Rect::new(0.0, 0.0, width, height),
            pending_actions: Vec::new(),
            frame_requests: FrameRequestFlags::new(),
            mouse_pos: Point::new(0.0, 0.0),
            keymap: Keymap::defaults(),
            key_contexts: HashMap::new(),
            last_keymap_handled: None,
        };
        harness.rebuild_layout();
        // Deliver initial WidgetAdded lifecycle events so that subsequent
        // lifecycle events (HotChanged, etc.) pass the ordering assertion.
        harness.deliver_lifecycle_events();
        harness
    }

    /// Recomputes layout from the root widget's `layout()` method.
    ///
    /// Must be called after construction and after any structural change
    /// (widget add/remove, text change that affects size). Called
    /// automatically by the constructor and input simulation methods.
    pub fn rebuild_layout(&mut self) {
        let ctx = LayoutCtx {
            measurer: &self.measurer,
            theme: &self.theme,
        };
        let layout_box = self.widget.layout(&ctx);
        self.layout = compute_layout(&layout_box, self.viewport);

        // Rebuild parent map for focus_within tracking.
        let parent_map = build_parent_map(&self.layout);
        self.interaction.set_parent_map(parent_map);

        // Register all widget IDs with InteractionManager (idempotent).
        register_widget_tree(&mut *self.widget, &mut self.interaction);

        // Collect key contexts for keymap scope gating.
        self.key_contexts.clear();
        collect_key_contexts(&mut *self.widget, &mut self.key_contexts);

        // Rebuild focus order from tree traversal.
        self.rebuild_focus_order();
    }

    /// Rebuilds the focus order from the widget tree.
    ///
    /// Collects focusable widget IDs via depth-first traversal and updates
    /// the `FocusManager` with the new tab order.
    pub fn rebuild_focus_order(&mut self) {
        let mut focusable = Vec::new();
        collect_focusable_ids(&mut *self.widget, &mut focusable);
        self.focus.set_focus_order(focusable);
    }

    /// Returns a reference to the root widget.
    pub fn widget(&self) -> &dyn Widget {
        &*self.widget
    }

    /// Returns a mutable reference to the root widget.
    pub fn widget_mut(&mut self) -> &mut dyn Widget {
        &mut *self.widget
    }

    /// Returns the computed layout tree.
    pub fn layout(&self) -> &LayoutNode {
        &self.layout
    }

    /// Returns the current viewport rect.
    pub fn viewport(&self) -> Rect {
        self.viewport
    }

    /// Returns the current simulated time.
    pub fn now(&self) -> Instant {
        self.clock
    }

    /// Returns the current mouse position.
    pub fn mouse_pos(&self) -> Point {
        self.mouse_pos
    }

    /// Returns and clears all pending actions from the last event dispatch.
    pub fn take_actions(&mut self) -> Vec<WidgetAction> {
        std::mem::take(&mut self.pending_actions)
    }

    /// Returns the next pending action, or `None`.
    pub fn pop_action(&mut self) -> Option<WidgetAction> {
        if self.pending_actions.is_empty() {
            None
        } else {
            Some(self.pending_actions.remove(0))
        }
    }

    /// Finds a widget's layout bounds by ID.
    ///
    /// Searches the layout tree for a node with the given widget ID.
    /// Returns `None` if the widget is not found.
    pub fn find_widget_bounds(&self, widget_id: WidgetId) -> Option<Rect> {
        find_bounds_in_layout(&self.layout, widget_id)
    }
}

/// Recursively searches a layout tree for a node with the given widget ID.
fn find_bounds_in_layout(node: &LayoutNode, target: WidgetId) -> Option<Rect> {
    if node.widget_id == Some(target) {
        return Some(node.rect);
    }
    for child in &node.children {
        if let Some(rect) = find_bounds_in_layout(child, target) {
            return Some(rect);
        }
    }
    None
}
