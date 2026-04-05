//! Core test harness struct and layout management.
//!
//! [`WidgetTestHarness`] wraps a [`WindowRoot`] and provides the full framework
//! pipeline — layout, hit testing, event propagation, interaction state,
//! lifecycle events, overlay routing, and animation scheduling — without
//! requiring a GPU, window, or real font stack.

use std::time::Instant;

use crate::action::WidgetAction;
use crate::geometry::{Point, Rect};
use crate::layout::LayoutNode;
use crate::overlay::Placement;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::window_root::WindowRoot;

use super::MockMeasurer;

/// Headless test harness for widget integration testing.
///
/// Wraps a [`WindowRoot`] and provides the full framework pipeline:
/// layout solver, hit testing, event propagation, interaction state,
/// lifecycle events, overlay routing, and animation scheduling — without
/// requiring a GPU, window, or real font stack.
///
/// # Overlay Testing
///
/// Unlike previous versions, the harness now includes `OverlayManager`,
/// `LayerTree`, and `LayerAnimator` via `WindowRoot`. Use [`push_popup`],
/// [`has_overlays`], and [`dismiss_overlays`] for overlay testing.
///
/// [`push_popup`]: Self::push_popup
/// [`has_overlays`]: Self::has_overlays
/// [`dismiss_overlays`]: Self::dismiss_overlays
pub struct WidgetTestHarness {
    /// Per-window composition unit (owns widget tree + framework state).
    pub(super) root: WindowRoot,
    /// Current simulated time (advanced via `advance_time()`).
    pub(super) clock: Instant,
    /// Mock text measurer.
    pub(super) measurer: MockMeasurer,
    /// Theme for rendering.
    pub(super) theme: UiTheme,
    /// Current mouse position (for mouse_down/mouse_up without explicit pos).
    pub(super) mouse_pos: Point,
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
        let viewport = Rect::new(0.0, 0.0, width, height);
        let root = WindowRoot::with_viewport(widget, viewport);
        let clock = Instant::now();
        let measurer = MockMeasurer::new();
        let theme = UiTheme::dark();

        let mut harness = Self {
            root,
            clock,
            measurer,
            theme,
            mouse_pos: Point::new(0.0, 0.0),
        };
        harness.rebuild_layout();
        // Deliver initial WidgetAdded lifecycle events so that subsequent
        // lifecycle events (HotChanged, etc.) pass the ordering assertion.
        harness.root.prepare(harness.clock, &harness.theme);
        harness
    }

    /// Recomputes layout from the root widget's `layout()` method.
    ///
    /// Must be called after construction and after any structural change
    /// (widget add/remove, text change that affects size). Called
    /// automatically by the constructor and input simulation methods.
    pub fn rebuild_layout(&mut self) {
        self.root.compute_layout(&self.measurer, &self.theme);
    }

    /// Rebuilds the focus order from the widget tree.
    ///
    /// Collects focusable widget IDs via depth-first traversal and updates
    /// the `FocusManager` with the new tab order.
    pub fn rebuild_focus_order(&mut self) {
        self.root.rebuild();
    }

    /// Returns a reference to the root widget.
    pub fn widget(&self) -> &dyn Widget {
        self.root.widget()
    }

    /// Returns a mutable reference to the root widget.
    pub fn widget_mut(&mut self) -> &mut dyn Widget {
        self.root.widget_mut()
    }

    /// Returns the computed layout tree.
    pub fn layout(&self) -> &LayoutNode {
        self.root.layout()
    }

    /// Returns the current viewport rect.
    pub fn viewport(&self) -> Rect {
        self.root.viewport()
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
        self.root.take_actions()
    }

    /// Returns the next pending action, or `None`.
    pub fn pop_action(&mut self) -> Option<WidgetAction> {
        self.root.pop_action()
    }

    /// Returns the `WindowRoot` for direct framework state access.
    pub fn root(&self) -> &WindowRoot {
        &self.root
    }

    /// Returns a mutable reference to the `WindowRoot`.
    pub fn root_mut(&mut self) -> &mut WindowRoot {
        &mut self.root
    }

    // -- Overlay helpers --

    /// Pushes a popup overlay at the given anchor position.
    pub fn push_popup(&mut self, widget: impl Widget + 'static, anchor: Rect) {
        self.root
            .push_overlay(Box::new(widget), anchor, Placement::Below, self.clock);
    }

    /// Returns `true` if any overlays are active.
    pub fn has_overlays(&self) -> bool {
        self.root.has_overlays()
    }

    /// Dismisses all popup overlays immediately.
    pub fn dismiss_overlays(&mut self) {
        self.root.clear_popups();
    }

    /// Resizes the viewport and re-runs layout.
    ///
    /// Simulates a window resize event: updates the viewport on the
    /// `WindowRoot`, recomputes layout, and delivers lifecycle events.
    /// Widgets that respond to size changes (e.g. terminal grids, flex
    /// containers) will receive updated bounds.
    pub fn resize(&mut self, width: f32, height: f32) {
        let viewport = Rect::new(0.0, 0.0, width, height);
        self.root.set_viewport(viewport);
        self.rebuild_layout();
        self.root.prepare(self.clock, &self.theme);
    }

    /// Finds a widget's layout bounds by ID.
    ///
    /// Searches the layout tree for a node with the given widget ID.
    /// Returns `None` if the widget is not found.
    pub fn find_widget_bounds(&self, widget_id: WidgetId) -> Option<Rect> {
        find_bounds_in_layout(self.root.layout(), widget_id)
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
