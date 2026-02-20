//! Stack widget — Z-axis overlay container.
//!
//! Layers children on top of each other. All children share the parent's
//! bounds. The last child in the list is frontmost (drawn last, receives
//! events first). Used for absolute positioning within a relative container.

use crate::geometry::Point;
use crate::input::{HoverEvent, KeyEvent, MouseEvent};
use crate::layout::LayoutBox;
use crate::widget_id::WidgetId;

use super::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetResponse};

/// A Z-axis container that overlays children on top of each other.
///
/// All children share the same bounds (the stack's bounds). Children
/// are drawn in order — the last child is frontmost. Events are routed
/// back-to-front: the frontmost child that handles the event wins.
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

    /// Finds the largest intrinsic size among children to size the stack.
    fn max_child_size(&self, ctx: &LayoutCtx<'_>) -> (f32, f32) {
        let mut max_w: f32 = 0.0;
        let mut max_h: f32 = 0.0;
        for child in &self.children {
            let child_box = child.layout(ctx);
            if let crate::layout::BoxContent::Leaf {
                intrinsic_width,
                intrinsic_height,
            } = child_box.content
            {
                max_w = max_w.max(intrinsic_width);
                max_h = max_h.max(intrinsic_height);
            }
        }
        (max_w, max_h)
    }

    /// Returns the frontmost child index if the point is within bounds.
    ///
    /// All stack children share the same bounds, so if the point is inside
    /// the stack, the frontmost (last) child is the hit target.
    fn hit_test_back_to_front(&self, pos: Point, bounds: crate::geometry::Rect) -> Option<usize> {
        if !self.children.is_empty() && bounds.contains(pos) {
            Some(self.children.len() - 1)
        } else {
            None
        }
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

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        // Draw children in order: first = backmost, last = frontmost.
        for child in &self.children {
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                draw_list: ctx.draw_list,
                bounds: ctx.bounds,
                focused_widget: ctx.focused_widget,
            };
            child.draw(&mut child_ctx);
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        // Route back-to-front: frontmost child that handles it wins.
        if let Some(idx) = self.hit_test_back_to_front(event.pos, ctx.bounds) {
            // Try from frontmost (last) down to the hit child.
            for i in (idx..self.children.len()).rev() {
                let resp = self.children[i].handle_mouse(event, ctx);
                if resp.response.is_handled() {
                    return resp;
                }
            }
        }
        WidgetResponse::ignored()
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        // Hover propagates to all children (frontmost first).
        for child in self.children.iter_mut().rev() {
            let resp = child.handle_hover(event, ctx);
            if resp.response.is_handled() {
                return resp;
            }
        }
        WidgetResponse::ignored()
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        // Key events go to frontmost child that handles them.
        for child in self.children.iter_mut().rev() {
            let resp = child.handle_key(event, ctx);
            if resp.response.is_handled() {
                return resp;
            }
        }
        WidgetResponse::ignored()
    }
}

#[cfg(test)]
mod tests;
