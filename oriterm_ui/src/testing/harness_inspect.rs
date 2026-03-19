//! State inspection methods for the test harness.
//!
//! After simulating input, tests need to inspect widget state: interaction
//! state, layout bounds, visual state animator progress, controller state.

use crate::draw::DrawList;
use crate::geometry::Rect;
use crate::interaction::InteractionState;
use crate::layout::LayoutNode;
use crate::sense::Sense;
use crate::widget_id::WidgetId;
use crate::widgets::Widget;
use crate::widgets::contexts::DrawCtx;

use super::harness::WidgetTestHarness;

// -- Interaction state queries --

impl WidgetTestHarness {
    /// Returns the interaction state for a widget.
    pub fn interaction_state(&self, widget_id: WidgetId) -> &InteractionState {
        self.interaction.get_state(widget_id)
    }

    /// Whether the pointer is over this widget.
    pub fn is_hot(&self, widget_id: WidgetId) -> bool {
        self.interaction.get_state(widget_id).is_hot()
    }

    /// Whether this widget has captured the mouse.
    pub fn is_active(&self, widget_id: WidgetId) -> bool {
        self.interaction.get_state(widget_id).is_active()
    }

    /// Whether this widget has keyboard focus.
    pub fn is_focused(&self, widget_id: WidgetId) -> bool {
        self.interaction.get_state(widget_id).is_focused()
    }

    /// The currently focused widget, if any.
    pub fn focused_widget(&self) -> Option<WidgetId> {
        self.interaction.focused_widget()
    }

    /// The currently active (capturing) widget, if any.
    pub fn active_widget(&self) -> Option<WidgetId> {
        self.interaction.active_widget()
    }

    // -- Layout queries --

    /// Returns the layout bounds of a widget by ID.
    ///
    /// Panics if the widget ID is not found in the layout tree.
    pub fn widget_bounds(&self, widget_id: WidgetId) -> Rect {
        self.find_widget_bounds(widget_id)
            .unwrap_or_else(|| panic!("widget {widget_id:?} not found in layout"))
    }

    /// Returns the layout bounds of a widget by ID, or `None`.
    pub fn try_widget_bounds(&self, widget_id: WidgetId) -> Option<Rect> {
        self.find_widget_bounds(widget_id)
    }

    // -- Widget tree inspection --

    /// Returns a list of all widget IDs in the layout tree.
    pub fn all_widget_ids(&self) -> Vec<WidgetId> {
        let mut ids = Vec::new();
        collect_ids_from_layout(&self.layout, &mut ids);
        ids
    }

    /// Returns a list of all focusable widget IDs in tab order.
    pub fn focusable_widgets(&self) -> Vec<WidgetId> {
        self.focus.focus_order().to_vec()
    }

    // -- Paint capture --

    /// Paints the widget tree and returns a copy of the draw commands.
    ///
    /// Uses `MockMeasurer` and test theme. No GPU required — returns
    /// the raw `DrawList` that would be sent to the GPU renderer.
    pub fn render(&mut self) -> DrawList {
        let mut draw_list = DrawList::new();
        let bounds = self.layout.rect;
        let mut ctx = DrawCtx {
            measurer: &self.measurer,
            draw_list: &mut draw_list,
            bounds,
            now: self.clock,
            theme: &self.theme,
            icons: None,
            scene_cache: None,
            interaction: Some(&self.interaction),
            widget_id: Some(self.widget.id()),
            frame_requests: Some(&self.frame_requests),
        };
        self.widget.paint(&mut ctx);
        draw_list
    }

    // -- WidgetRef --

    /// Returns a [`WidgetRef`] for the root widget.
    ///
    /// Provides typed access to widget state, interaction state, and bounds.
    pub fn get_widget(&self, widget_id: WidgetId) -> WidgetRef<'_> {
        // Widget tree traversal requires &mut (for_each_child_mut).
        // For now, only the root widget is directly accessible.
        // All other widgets can be inspected via is_hot/is_active/is_focused/widget_bounds.
        assert_eq!(
            self.widget.id(),
            widget_id,
            "get_widget currently only supports the root widget; \
             use is_hot/is_active/is_focused/widget_bounds for child widgets"
        );
        let interaction = self.interaction.get_state(widget_id);
        let bounds = self.find_widget_bounds(widget_id).unwrap_or_default();
        WidgetRef {
            widget: &*self.widget,
            interaction,
            bounds,
        }
    }
}

/// Read-only reference to a widget in the harness.
pub struct WidgetRef<'a> {
    widget: &'a dyn Widget,
    interaction: &'a InteractionState,
    bounds: Rect,
}

impl WidgetRef<'_> {
    /// Whether the pointer is over this widget.
    pub fn is_hot(&self) -> bool {
        self.interaction.is_hot()
    }

    /// Whether this widget has captured the mouse.
    pub fn is_active(&self) -> bool {
        self.interaction.is_active()
    }

    /// Whether this widget has keyboard focus.
    pub fn is_focused(&self) -> bool {
        self.interaction.is_focused()
    }

    /// The layout bounds of this widget.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }

    /// The sense flags of this widget.
    pub fn sense(&self) -> Sense {
        self.widget.sense()
    }

    /// The widget ID.
    pub fn id(&self) -> WidgetId {
        self.widget.id()
    }
}

/// Recursively collects all widget IDs from the layout tree.
fn collect_ids_from_layout(node: &LayoutNode, out: &mut Vec<WidgetId>) {
    if let Some(id) = node.widget_id {
        out.push(id);
    }
    for child in &node.children {
        collect_ids_from_layout(child, out);
    }
}
