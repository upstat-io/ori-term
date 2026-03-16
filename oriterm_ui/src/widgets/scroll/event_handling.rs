//! Event handling logic for `ScrollWidget`.
//!
//! Contains `handle_mouse()`, `handle_hover()`, and `handle_key()` bodies
//! as inherent methods, delegated from the `Widget` trait impl in `mod.rs`.
//! These methods are removed entirely during the Widget trait migration
//! (Section 08.3), at which point this file is deleted.

use crate::geometry::Rect;
use crate::input::{HoverEvent, Key, KeyEvent, Modifiers, MouseEvent, MouseEventKind, ScrollDelta};
use crate::widgets::{CaptureRequest, EventCtx, WidgetResponse};

use super::ScrollWidget;

impl ScrollWidget {
    /// Handles mouse events: child capture, scrollbar interaction, scroll
    /// wheel, and child delegation with scroll-adjusted coordinates.
    pub(super) fn handle_mouse_impl(
        &mut self,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let (content_w, content_h) = self.child_natural_size(ctx.measurer, ctx.theme, ctx.bounds);
        let view_h = ctx.bounds.height();

        // During child capture, bypass scrollbar and scroll handling.
        if self.child_captured {
            let child_bounds = Rect::new(
                ctx.bounds.x() - self.scroll_offset_x,
                ctx.bounds.y() - self.scroll_offset,
                content_w,
                content_h,
            );
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_bounds,
                is_focused: ctx.focused_widget == Some(self.child.id()),
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            let resp = self.child.handle_mouse(event, &child_ctx);
            if resp.capture.should_release(&event.kind) {
                self.child_captured = false;
            }
            if resp.response.needs_layout() {
                *self.cached_child_layout.borrow_mut() = None;
            }
            return resp;
        }

        // Scrollbar drag takes priority.
        if let Some(resp) = self.handle_scrollbar_mouse(event, ctx.bounds, content_h, view_h) {
            return resp;
        }

        // Handle scroll events.
        //
        // Winit delivers positive y for wheel-up. Negate so positive delta_y
        // means "scroll down" (increase offset), matching traditional
        // Windows/Linux convention: wheel-up → view scrolls up.
        if let MouseEventKind::Scroll(delta) = event.kind {
            let delta_y = match delta {
                ScrollDelta::Pixels { y, .. } => -y,
                ScrollDelta::Lines { y, .. } => -y * self.line_height,
            };
            if self.scroll_by(delta_y, content_h, view_h) {
                return WidgetResponse::paint();
            }
            return WidgetResponse::handled();
        }

        // Use the same coordinate system as draw: child bounds start at
        // ctx.bounds.origin() offset by scroll. Mouse position stays in
        // screen space to match the layout nodes in the child's tree.
        let child_bounds = Rect::new(
            ctx.bounds.x() - self.scroll_offset_x,
            ctx.bounds.y() - self.scroll_offset,
            content_w,
            content_h,
        );
        let child_ctx = EventCtx {
            measurer: ctx.measurer,
            bounds: child_bounds,
            is_focused: ctx.focused_widget == Some(self.child.id()),
            focused_widget: ctx.focused_widget,
            theme: ctx.theme,
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        let resp = self.child.handle_mouse(event, &child_ctx);
        if resp.capture == CaptureRequest::Acquire {
            self.child_captured = true;
        }
        if resp.response.needs_layout() {
            *self.cached_child_layout.borrow_mut() = None;
        }
        resp
    }

    /// Handles hover enter/leave: resets scrollbar state and delegates to child.
    pub(super) fn handle_hover_impl(
        &mut self,
        event: HoverEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        // Reset scrollbar hover state when cursor leaves the widget entirely.
        if matches!(event, HoverEvent::Leave) && self.scrollbar.track_hovered {
            self.scrollbar.track_hovered = false;
        }

        let (content_w, content_h) = self.child_natural_size(ctx.measurer, ctx.theme, ctx.bounds);
        let child_bounds = Rect::new(
            ctx.bounds.x() - self.scroll_offset_x,
            ctx.bounds.y() - self.scroll_offset,
            content_w,
            content_h,
        );
        let child_ctx = EventCtx {
            measurer: ctx.measurer,
            bounds: child_bounds,
            is_focused: ctx.focused_widget == Some(self.child.id()),
            focused_widget: ctx.focused_widget,
            theme: ctx.theme,
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        self.child.handle_hover(event, &child_ctx)
    }

    /// Handles keyboard events: scroll keys (arrows, Page, Home/End) and
    /// delegates non-scroll keys to the child.
    pub(super) fn handle_key_impl(
        &mut self,
        event: KeyEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let (_, content_h) = self.child_natural_size(ctx.measurer, ctx.theme, ctx.bounds);
        let view_h = ctx.bounds.height();

        // Handle scroll keys.
        if event.modifiers == Modifiers::NONE {
            match event.key {
                Key::ArrowUp => {
                    if self.scroll_by(-self.line_height, content_h, view_h) {
                        return WidgetResponse::paint();
                    }
                    return WidgetResponse::handled();
                }
                Key::ArrowDown => {
                    if self.scroll_by(self.line_height, content_h, view_h) {
                        return WidgetResponse::paint();
                    }
                    return WidgetResponse::handled();
                }
                Key::PageUp => {
                    if self.scroll_by(-view_h, content_h, view_h) {
                        return WidgetResponse::paint();
                    }
                    return WidgetResponse::handled();
                }
                Key::PageDown => {
                    if self.scroll_by(view_h, content_h, view_h) {
                        return WidgetResponse::paint();
                    }
                    return WidgetResponse::handled();
                }
                Key::Home => {
                    let changed = self.scroll_offset > f32::EPSILON;
                    self.scroll_offset = 0.0;
                    return if changed {
                        WidgetResponse::paint()
                    } else {
                        WidgetResponse::handled()
                    };
                }
                Key::End => {
                    let max = (content_h - view_h).max(0.0);
                    let changed = (self.scroll_offset - max).abs() > f32::EPSILON;
                    self.scroll_offset = max;
                    return if changed {
                        WidgetResponse::paint()
                    } else {
                        WidgetResponse::handled()
                    };
                }
                _ => {}
            }
        }

        // Delegate to child for non-scroll keys.
        let resp = self.child.handle_key(event, ctx);
        if resp.response.needs_layout() {
            *self.cached_child_layout.borrow_mut() = None;
        }
        resp
    }
}
