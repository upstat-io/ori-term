//! Pointer-events modifier — suppresses pointer hit testing for a subtree.

use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// A wrapper widget that suppresses pointer hit testing for its child subtree.
///
/// When `enabled` is `false`, the child subtree is invisible to hover, click,
/// and drag events. Layout, paint, keyboard focus, and traversal are unaffected.
/// Analogous to CSS `pointer-events: none`.
pub struct PointerEventsWidget {
    id: WidgetId,
    child: Box<dyn Widget>,
    enabled: bool,
}

impl PointerEventsWidget {
    /// Creates a pointer-events wrapper. When `enabled` is `false`, the
    /// subtree ignores pointer events.
    pub fn new(child: Box<dyn Widget>, enabled: bool) -> Self {
        Self {
            id: WidgetId::next(),
            child,
            enabled,
        }
    }

    /// Returns whether pointer events are enabled.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Sets whether pointer events are enabled.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}

impl Widget for PointerEventsWidget {
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
        let child_layout = self.child.layout(ctx);

        // Preserve the child's outer sizing contract so the wrapper is
        // layout-transparent. Without this, Fill/FillPortion children
        // would collapse to Hug (the flex default).
        let child_width = child_layout.width;
        let child_height = child_layout.height;

        LayoutBox::flex(crate::layout::Direction::Column, vec![child_layout])
            .with_width(child_width)
            .with_height(child_height)
            .with_widget_id(self.id)
            .with_sense(Sense::none())
            .with_pointer_events(self.enabled)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        self.child.paint(ctx);
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(self.child.as_mut());
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.child.accept_action(action)
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.child.focusable_children()
    }
}
