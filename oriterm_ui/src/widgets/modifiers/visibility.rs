//! Visibility modifier — controls whether a subtree participates in layout,
//! paint, traversal, focus, and action propagation.

use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// Controls how a subtree participates in the widget pipeline.
///
/// Modeled after CSS visibility semantics:
///
/// - `Visible`: normal behavior — layout, paint, hit test, focus, traversal.
/// - `Hidden`: participates in layout (reserves space) but does not paint,
///   does not register descendants for interaction, and does not contribute
///   focusable descendants.
/// - `DisplayNone`: contributes zero layout size and is skipped for paint,
///   interaction, focus, and active traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VisibilityMode {
    /// Normal — layout, paint, hit test, focus, traversal all active.
    #[default]
    Visible,
    /// Reserves layout space but does not paint or interact.
    Hidden,
    /// Removed from layout and all pipeline stages.
    DisplayNone,
}

impl VisibilityMode {
    /// Whether the child should be laid out (reserves space).
    fn participates_in_layout(self) -> bool {
        matches!(self, Self::Visible | Self::Hidden)
    }

    /// Whether the child should be painted and interact.
    fn is_active(self) -> bool {
        matches!(self, Self::Visible)
    }
}

/// A wrapper widget that controls child visibility and display.
///
/// Delegates all widget methods to the child, gated by [`VisibilityMode`]:
///
/// - `Visible`: full delegation, identical to having no wrapper.
/// - `Hidden`: child is laid out (so it reserves space) but the returned
///   layout tree has all widget IDs and sense flags scrubbed via
///   [`LayoutBox::for_layout_only()`]. Paint, focus, and action propagation
///   are suppressed. Active traversal (`for_each_child_mut`) is skipped.
/// - `DisplayNone`: child is not laid out (zero-size leaf). Paint, focus,
///   action propagation, and active traversal are all suppressed.
///
/// In all modes, `for_each_child_mut_all()` always visits the child so that
/// full-tree maintenance tools (key context collection, deregistration)
/// retain access.
pub struct VisibilityWidget {
    id: WidgetId,
    child: Box<dyn Widget>,
    mode: VisibilityMode,
}

impl VisibilityWidget {
    /// Creates a visibility wrapper around `child` with the given mode.
    pub fn new(child: Box<dyn Widget>, mode: VisibilityMode) -> Self {
        Self {
            id: WidgetId::next(),
            child,
            mode,
        }
    }

    /// Returns the current visibility mode.
    pub fn mode(&self) -> VisibilityMode {
        self.mode
    }

    /// Sets the visibility mode.
    pub fn set_mode(&mut self, mode: VisibilityMode) {
        self.mode = mode;
    }
}

impl Widget for VisibilityWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        if !self.mode.participates_in_layout() {
            return LayoutBox::leaf(0.0, 0.0).with_widget_id(self.id);
        }

        let child_layout = self.child.layout(ctx);

        // Preserve the child's outer sizing contract so the wrapper is
        // layout-transparent. Without this, Fill/FillPortion children
        // would collapse to Hug (the flex default).
        let child_width = child_layout.width;
        let child_height = child_layout.height;

        let inner = if self.mode.is_active() {
            child_layout
        } else {
            // Hidden: child layout is preserved for sizing but scrubbed of
            // all interaction metadata.
            child_layout.for_layout_only()
        };

        LayoutBox::flex(crate::layout::Direction::Column, vec![inner])
            .with_width(child_width)
            .with_height(child_height)
            .with_widget_id(self.id)
            .with_sense(Sense::none())
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        if !self.mode.is_active() {
            return;
        }
        self.child.paint(ctx);
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        if self.mode.is_active() {
            visitor(self.child.as_mut());
        }
    }

    fn for_each_child_mut_all(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(self.child.as_mut());
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        if !self.mode.is_active() {
            return false;
        }
        self.child.accept_action(action)
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        if !self.mode.is_active() {
            return Vec::new();
        }
        self.child.focusable_children()
    }
}
