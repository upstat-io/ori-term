//! Mouse and keyboard event dispatch for `ContainerWidget`.
//!
//! Mouse events route through hit testing with capture semantics: a mouse
//! down on a child "captures" that child, so subsequent move/up events go
//! to the captured child regardless of cursor position. Hover Enter/Leave
//! is synthesized as the cursor moves between children. Keyboard events
//! route to the focused child.

use crate::geometry::Point;
use crate::input::{HoverEvent, KeyEvent, MouseEvent, MouseEventKind};

use super::{ContainerWidget, EventCtx, WidgetResponse};

impl ContainerWidget {
    /// Dispatches a mouse event to the appropriate child.
    ///
    /// Capture semantics: mouse down captures a child; subsequent events
    /// go to the captured child until mouse up releases capture. Move
    /// events without capture update hover tracking.
    pub(super) fn dispatch_mouse(
        &mut self,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);

        // If a child has capture, route events to it until release.
        if let Some(cap_idx) = self.input_state.captured_child {
            return self.dispatch_to_captured(cap_idx, event, ctx, &layout);
        }

        // Move events update hover tracking (no capture active).
        if matches!(event.kind, MouseEventKind::Move) {
            return self.update_hover(&layout, event.pos, ctx);
        }

        // Mouse down: hit test, capture the child, and deliver.
        if matches!(event.kind, MouseEventKind::Down(_)) {
            if let Some(idx) = self.hit_test_children(&layout, event.pos) {
                self.input_state.captured_child = Some(idx);
                return self.deliver_mouse_to_child(idx, event, ctx, &layout);
            }
        }

        // Scroll events: hit test and deliver without capture.
        if matches!(event.kind, MouseEventKind::Scroll(_)) {
            if let Some(idx) = self.hit_test_children(&layout, event.pos) {
                return self.deliver_mouse_to_child(idx, event, ctx, &layout);
            }
        }

        WidgetResponse::ignored()
    }

    /// Routes an event to a captured child. Releases capture on mouse up.
    fn dispatch_to_captured(
        &mut self,
        cap_idx: usize,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
        layout: &crate::layout::LayoutNode,
    ) -> WidgetResponse {
        let resp = self.deliver_mouse_to_child(cap_idx, event, ctx, layout);

        // Release capture on any mouse up.
        if matches!(event.kind, MouseEventKind::Up(_)) {
            self.input_state.captured_child = None;
        }

        resp
    }

    /// Delivers a mouse event to a specific child by index.
    fn deliver_mouse_to_child(
        &mut self,
        idx: usize,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
        layout: &crate::layout::LayoutNode,
    ) -> WidgetResponse {
        if let (Some(child), Some(child_node)) =
            (self.children.get_mut(idx), layout.children.get(idx))
        {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_node.content_rect,
                is_focused: ctx.focused_widget == Some(child.id()),
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            let resp = child.handle_mouse(event, &child_ctx);
            if resp.response.needs_layout() {
                *self.cached_layout.borrow_mut() = None;
            }
            return resp;
        }
        WidgetResponse::ignored()
    }

    /// Updates hover tracking when the cursor moves. Sends Enter/Leave to
    /// the correct child based on hit testing.
    fn update_hover(
        &mut self,
        layout: &crate::layout::LayoutNode,
        pos: Point,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let new_hover = self.hit_test_children(layout, pos);
        if new_hover == self.input_state.hovered_child {
            // Same child — still forward the move event to it.
            if let Some(idx) = new_hover {
                return self.deliver_mouse_to_child(
                    idx,
                    &MouseEvent {
                        kind: MouseEventKind::Move,
                        pos,
                        modifiers: crate::input::Modifiers::NONE,
                    },
                    ctx,
                    layout,
                );
            }
            return WidgetResponse::ignored();
        }

        // Leave old child.
        if let Some(old_idx) = self.input_state.hovered_child {
            if let (Some(child), Some(child_node)) =
                (self.children.get_mut(old_idx), layout.children.get(old_idx))
            {
                let child_ctx = EventCtx {
                    measurer: ctx.measurer,
                    bounds: child_node.content_rect,
                    is_focused: ctx.focused_widget == Some(child.id()),
                    focused_widget: ctx.focused_widget,
                    theme: ctx.theme,
                };
                child.handle_hover(HoverEvent::Leave, &child_ctx);
            }
        }

        // Enter new child.
        if let Some(new_idx) = new_hover {
            if let (Some(child), Some(child_node)) =
                (self.children.get_mut(new_idx), layout.children.get(new_idx))
            {
                let child_ctx = EventCtx {
                    measurer: ctx.measurer,
                    bounds: child_node.content_rect,
                    is_focused: ctx.focused_widget == Some(child.id()),
                    focused_widget: ctx.focused_widget,
                    theme: ctx.theme,
                };
                child.handle_hover(HoverEvent::Enter, &child_ctx);
            }
        }

        self.input_state.hovered_child = new_hover;
        WidgetResponse::paint()
    }

    /// Dispatches a keyboard event to the focused child.
    pub(super) fn dispatch_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);

        for (idx, child) in self.children.iter_mut().enumerate() {
            if let Some(child_node) = layout.children.get(idx) {
                let child_ctx = EventCtx {
                    measurer: ctx.measurer,
                    bounds: child_node.content_rect,
                    is_focused: ctx.focused_widget == Some(child.id()),
                    focused_widget: ctx.focused_widget,
                    theme: ctx.theme,
                };
                let resp = child.handle_key(event, &child_ctx);
                if resp.response.is_handled() {
                    if resp.response.needs_layout() {
                        *self.cached_layout.borrow_mut() = None;
                    }
                    return resp;
                }
            }
        }
        WidgetResponse::ignored()
    }
}
