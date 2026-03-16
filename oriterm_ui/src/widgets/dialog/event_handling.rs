//! Event handling logic for `DialogWidget`.
//!
//! Contains `handle_mouse()`, `handle_hover()`, and `handle_key()` bodies
//! as inherent methods, delegated from the `Widget` trait impl in `mod.rs`.
//! These methods are removed entirely during the Widget trait migration
//! (Section 08.3), at which point this file is deleted.

use crate::input::{HoverEvent, Key, KeyEvent, MouseEvent, MouseEventKind};
use crate::layout::LayoutNode;
use crate::widget_id::WidgetId;
use crate::widgets::{EventCtx, Widget, WidgetAction, WidgetResponse};

use super::{DialogButton, DialogButtons, DialogWidget};

impl DialogWidget {
    /// Handles mouse events: delegates clicks to footer buttons and tracks
    /// per-button hover state on mouse move.
    pub(super) fn handle_mouse_impl(
        &mut self,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        let children = &layout.children;
        if children.len() < 2 {
            return WidgetResponse::ignored();
        }

        // Track per-button hover on mouse move.
        if event.kind == MouseEventKind::Move {
            return self.update_button_hover(event, ctx, &children[1]);
        }

        // Footer zone is children[1]; buttons are its children.
        let focused = self.focused_button;
        for (i, btn_node) in children[1].children.iter().enumerate() {
            if !btn_node.rect.contains(event.pos) {
                continue;
            }
            let (button, btn_kind) = self.button_at_index(i);
            let btn_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: btn_node.content_rect,
                is_focused: focused == btn_kind,
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            let response = button.handle_mouse(event, &btn_ctx);
            if let Some(WidgetAction::Clicked(id)) = &response.action {
                return self.map_button_click(*id);
            }
            return response;
        }
        WidgetResponse::handled()
    }

    /// Handles hover enter/leave: clears per-button hover state on leave.
    pub(super) fn handle_hover_impl(
        &mut self,
        event: HoverEvent,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        // On dialog-level Leave, clear any per-button hover state.
        if event == HoverEvent::Leave {
            self.clear_button_hover(ctx);
        }
        WidgetResponse::handled()
    }

    /// Handles keyboard events: Enter/Space activates focused button,
    /// Escape dismisses, Tab cycles focus between buttons.
    pub(super) fn handle_key_impl(
        &mut self,
        event: KeyEvent,
        _ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        match event.key {
            Key::Enter | Key::Space => match self.focused_button {
                DialogButton::Ok => {
                    WidgetResponse::layout().with_action(WidgetAction::Clicked(self.ok_button.id()))
                }
                DialogButton::Cancel => {
                    WidgetResponse::layout().with_action(WidgetAction::DismissOverlay(self.id))
                }
            },
            Key::Escape => {
                WidgetResponse::layout().with_action(WidgetAction::DismissOverlay(self.id))
            }
            Key::Tab => {
                if self.buttons == DialogButtons::OkCancel {
                    self.focus_visible = true;
                    self.focused_button = match self.focused_button {
                        DialogButton::Ok => DialogButton::Cancel,
                        DialogButton::Cancel => DialogButton::Ok,
                    };
                    WidgetResponse::layout()
                } else {
                    WidgetResponse::handled()
                }
            }
            _ => WidgetResponse::handled(),
        }
    }

    /// Map a button click to the appropriate dialog-level response.
    fn map_button_click(&self, id: WidgetId) -> WidgetResponse {
        match self.button_for_id(id) {
            Some(DialogButton::Ok) => {
                WidgetResponse::layout().with_action(WidgetAction::Clicked(id))
            }
            Some(DialogButton::Cancel) => {
                WidgetResponse::layout().with_action(WidgetAction::DismissOverlay(self.id))
            }
            None => WidgetResponse::handled(),
        }
    }

    /// Update per-button hover state based on mouse position.
    ///
    /// Sends `HoverEvent::Leave` to the previously hovered button and
    /// `HoverEvent::Enter` to the newly hovered button when the mouse
    /// moves between buttons (or enters/leaves the button area).
    fn update_button_hover(
        &mut self,
        event: &MouseEvent,
        ctx: &EventCtx<'_>,
        footer_node: &LayoutNode,
    ) -> WidgetResponse {
        // Find which button (if any) the mouse is over.
        let new_hover = footer_node
            .children
            .iter()
            .position(|btn_node| btn_node.rect.contains(event.pos));

        if new_hover == self.hovered_button {
            return WidgetResponse::handled();
        }

        let focused = self.focused_button;

        // Leave the old button.
        if let Some(old_idx) = self.hovered_button {
            if let Some(btn_node) = footer_node.children.get(old_idx) {
                let (button, btn_kind) = self.button_at_index(old_idx);
                let btn_ctx = EventCtx {
                    measurer: ctx.measurer,
                    bounds: btn_node.content_rect,
                    is_focused: focused == btn_kind,
                    focused_widget: ctx.focused_widget,
                    theme: ctx.theme,
                    interaction: None,
                    widget_id: None,
                    frame_requests: None,
                };
                button.handle_hover(HoverEvent::Leave, &btn_ctx);
            }
        }

        // Enter the new button.
        if let Some(new_idx) = new_hover {
            if let Some(btn_node) = footer_node.children.get(new_idx) {
                let (button, btn_kind) = self.button_at_index(new_idx);
                let btn_ctx = EventCtx {
                    measurer: ctx.measurer,
                    bounds: btn_node.content_rect,
                    is_focused: focused == btn_kind,
                    focused_widget: ctx.focused_widget,
                    theme: ctx.theme,
                    interaction: None,
                    widget_id: None,
                    frame_requests: None,
                };
                button.handle_hover(HoverEvent::Enter, &btn_ctx);
            }
        }

        self.hovered_button = new_hover;
        WidgetResponse::layout()
    }

    /// Clear all per-button hover state.
    fn clear_button_hover(&mut self, ctx: &EventCtx<'_>) {
        if let Some(old_idx) = self.hovered_button.take() {
            let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
            let children = &layout.children;
            if children.len() >= 2 {
                if let Some(btn_node) = children[1].children.get(old_idx) {
                    let focused = self.focused_button;
                    let (button, btn_kind) = self.button_at_index(old_idx);
                    let btn_ctx = EventCtx {
                        measurer: ctx.measurer,
                        bounds: btn_node.content_rect,
                        is_focused: focused == btn_kind,
                        focused_widget: ctx.focused_widget,
                        theme: ctx.theme,
                        interaction: None,
                        widget_id: None,
                        frame_requests: None,
                    };
                    button.handle_hover(HoverEvent::Leave, &btn_ctx);
                }
            }
        }
    }
}
