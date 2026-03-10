//! Button wrapper that overrides the widget ID.
//!
//! Used by the settings panel to intercept `Clicked` actions from buttons
//! whose IDs are allocated externally (Save, Cancel, Close).

use crate::input::{HoverEvent, KeyEvent, MouseEvent};
use crate::layout::LayoutBox;
use crate::widget_id::WidgetId;

use super::super::button::ButtonWidget;
use super::super::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};

/// Wrapper around `ButtonWidget` that overrides its `WidgetId`.
///
/// Needed because `ButtonWidget::new()` generates its own ID internally,
/// but we need a known ID to intercept the `Clicked` action.
pub(super) struct IdOverrideButton {
    pub(super) inner: ButtonWidget,
    pub(super) id_override: WidgetId,
}

impl Widget for IdOverrideButton {
    fn id(&self) -> WidgetId {
        self.id_override
    }

    fn is_focusable(&self) -> bool {
        self.inner.is_focusable()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        // Rewrite the widget id on the layout box.
        let mut lb = self.inner.layout(ctx);
        lb = lb.with_widget_id(self.id_override);
        lb
    }

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        self.inner.draw(ctx);
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let resp = self.inner.handle_mouse(event, ctx);
        // Rewrite the clicked id to our override.
        match resp.action {
            Some(WidgetAction::Clicked(_)) => WidgetResponse {
                response: resp.response,
                action: Some(WidgetAction::Clicked(self.id_override)),
                capture: resp.capture,
            },
            _ => resp,
        }
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        self.inner.handle_hover(event, ctx)
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let resp = self.inner.handle_key(event, ctx);
        match resp.action {
            Some(WidgetAction::Clicked(_)) => WidgetResponse {
                response: resp.response,
                action: Some(WidgetAction::Clicked(self.id_override)),
                capture: resp.capture,
            },
            _ => resp,
        }
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        if self.is_focusable() {
            vec![self.id_override]
        } else {
            Vec::new()
        }
    }
}
