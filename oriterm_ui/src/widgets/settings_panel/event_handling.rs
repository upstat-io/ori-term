//! Event handling logic for `SettingsPanel`.
//!
//! Contains `handle_mouse()`, `handle_hover()`, and `handle_key()` bodies
//! as inherent methods, delegated from the `Widget` trait impl in `mod.rs`.
//! These methods are removed entirely during the Widget trait migration
//! (Section 08.3), at which point this file is deleted.

use crate::input::{HoverEvent, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::widgets::{EventCtx, Widget, WidgetAction, WidgetResponse};

use super::SettingsPanel;

impl SettingsPanel {
    /// Handles mouse events: header drag (overlay mode), child delegation,
    /// layout cache invalidation, and action translation.
    pub(super) fn handle_mouse_impl(
        &mut self,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        // Overlay mode: header drag support.
        if self.show_chrome {
            // Active drag: track movement and emit MoveOverlay deltas.
            if let Some(origin) = self.drag_origin {
                return match event.kind {
                    MouseEventKind::Move => {
                        let dx = event.pos.x - origin.x;
                        let dy = event.pos.y - origin.y;
                        self.drag_origin = Some(event.pos);
                        WidgetResponse::paint().with_action(WidgetAction::MoveOverlay {
                            delta_x: dx,
                            delta_y: dy,
                        })
                    }
                    MouseEventKind::Up(MouseButton::Left) => {
                        self.drag_origin = None;
                        WidgetResponse::paint().with_release_capture()
                    }
                    _ => WidgetResponse::handled(),
                };
            }

            // Start drag on mouse-down in the header drag zone.
            if matches!(event.kind, MouseEventKind::Down(MouseButton::Left))
                && Self::is_header_drag_zone(event.pos, ctx.bounds)
            {
                self.drag_origin = Some(event.pos);
                return WidgetResponse::handled().with_capture();
            }
        }

        // Delegate non-drag events to children.
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_node.content_rect,
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            let resp = self.container.handle_mouse(event, &child_ctx);
            if resp.response.needs_layout() {
                *self.cached_layout.borrow_mut() = None;
            }
            return self.translate_action(resp);
        }
        WidgetResponse::handled()
    }

    /// Handles hover enter/leave: delegates to inner container.
    pub(super) fn handle_hover_impl(
        &mut self,
        event: HoverEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_node.content_rect,
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            return self.container.handle_hover(event, &child_ctx);
        }
        WidgetResponse::handled()
    }

    /// Handles keyboard events: delegates to inner container with layout
    /// cache invalidation and action translation.
    pub(super) fn handle_key_impl(
        &mut self,
        event: KeyEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: child_node.content_rect,
                is_focused: false,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            let resp = self.container.handle_key(event, &child_ctx);
            if resp.response.needs_layout() {
                *self.cached_layout.borrow_mut() = None;
            }
            return self.translate_action(resp);
        }
        WidgetResponse::handled()
    }
}
