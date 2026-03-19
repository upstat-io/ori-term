//! Stack widget — Z-axis overlay container.
//!
//! Layers children on top of each other. All children share the parent's
//! bounds. The last child in the list is frontmost (drawn last, receives
//! events first). Used for absolute positioning within a relative container.

use crate::geometry::Rect;
use crate::layout::{LayoutBox, compute_layout};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// A Z-axis container that overlays children on top of each other.
///
/// All children share the same bounds (the stack's bounds). Children
/// are drawn in order — the last child is frontmost. Events are routed
/// back-to-front through the propagation pipeline (last child = highest
/// z-order = wins hit tests).
pub struct StackWidget {
    id: WidgetId,
    children: Vec<Box<dyn Widget>>,
}

impl StackWidget {
    /// Creates a stack with the given children (last = frontmost).
    pub fn new(children: Vec<Box<dyn Widget>>) -> Self {
        Self {
            id: WidgetId::next(),
            children,
        }
    }

    /// Returns the number of children.
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Finds the largest resolved size among children to size the stack.
    ///
    /// Resolves each child through the layout solver with unconstrained bounds
    /// so both `Leaf` and `Flex` children contribute their natural size.
    fn max_child_size(&self, ctx: &LayoutCtx<'_>) -> (f32, f32) {
        let mut max_w: f32 = 0.0;
        let mut max_h: f32 = 0.0;
        let unconstrained = Rect::new(0.0, 0.0, f32::INFINITY, f32::INFINITY);
        for child in &self.children {
            let child_box = child.layout(ctx);
            let node = compute_layout(&child_box, unconstrained);
            max_w = max_w.max(node.rect.width());
            max_h = max_h.max(node.rect.height());
        }
        (max_w, max_h)
    }
}

impl Widget for StackWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        // Stack sizes to the largest child. All children share the
        // stack's full bounds (positioned manually in draw/events).
        let (max_w, max_h) = self.max_child_size(ctx);
        LayoutBox::leaf(max_w, max_h).with_widget_id(self.id)
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        // Draw children in order: first = backmost, last = frontmost.
        for child in &self.children {
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                scene: ctx.scene,
                bounds: ctx.bounds,
                now: ctx.now,
                theme: ctx.theme,
                icons: ctx.icons,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            child.paint(&mut child_ctx);
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        for child in &mut self.children {
            visitor(child.as_mut());
        }
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.children
            .iter()
            .flat_map(|c| c.focusable_children())
            .collect()
    }
}

#[cfg(test)]
mod tests;
