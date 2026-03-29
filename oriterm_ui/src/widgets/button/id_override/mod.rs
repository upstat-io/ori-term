//! Button wrapper that overrides the widget ID.
//!
//! Used by the settings footer and settings panel to intercept `Clicked`
//! actions from buttons whose IDs are allocated externally (Save, Cancel,
//! Reset, Close).

use crate::action::KeymapAction;
use crate::controllers::EventController;
use crate::geometry::Rect;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::visual_state::transition::VisualStateAnimator;
use crate::widget_id::WidgetId;

use super::ButtonWidget;
use crate::widgets::{DrawCtx, LayoutCtx, PrepaintCtx, Widget, WidgetAction};

/// Wrapper around `ButtonWidget` that overrides its `WidgetId`.
///
/// Needed because `ButtonWidget::new()` generates its own ID internally,
/// but we need a known ID to intercept the `Clicked` action.
pub(crate) struct IdOverrideButton {
    inner: ButtonWidget,
    id_override: WidgetId,
}

impl IdOverrideButton {
    /// Create a new button with an externally-assigned ID.
    pub(crate) fn new(inner: ButtonWidget, id_override: WidgetId) -> Self {
        Self { inner, id_override }
    }

    /// Sets the disabled state on the inner button.
    #[allow(
        dead_code,
        reason = "pub(crate) API tested in id_override::tests::set_disabled_delegates"
    )]
    pub(crate) fn set_disabled(&mut self, disabled: bool) {
        self.inner.set_disabled(disabled);
    }
}

impl Widget for IdOverrideButton {
    fn id(&self) -> WidgetId {
        self.id_override
    }

    fn is_focusable(&self) -> bool {
        self.inner.is_focusable()
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        // Rewrite the widget id on the layout box.
        let mut lb = self.inner.layout(ctx);
        lb = lb.with_widget_id(self.id_override);
        lb
    }

    fn controllers(&self) -> &[Box<dyn EventController>] {
        self.inner.controllers()
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>] {
        self.inner.controllers_mut()
    }

    fn visual_states(&self) -> Option<&VisualStateAnimator> {
        self.inner.visual_states()
    }

    fn visual_states_mut(&mut self) -> Option<&mut VisualStateAnimator> {
        self.inner.visual_states_mut()
    }

    fn prepaint(&mut self, ctx: &mut PrepaintCtx<'_>) {
        self.inner.prepaint(ctx);
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        self.inner.paint(ctx);
    }

    fn on_action(&mut self, action: WidgetAction, _bounds: Rect) -> Option<WidgetAction> {
        // Rewrite the clicked id to our override.
        match action {
            WidgetAction::Clicked(_) => Some(WidgetAction::Clicked(self.id_override)),
            _ => Some(action),
        }
    }

    // No for_each_child_mut: this widget delegates controllers() and
    // visual_states() to its inner ButtonWidget, so the inner button must
    // NOT be visited separately by prepare_widget_tree — that would
    // double-update the animator with the inner button's (non-hot) ID,
    // overriding the correct update from this wrapper's ID.

    fn key_context(&self) -> Option<&'static str> {
        self.inner.key_context()
    }

    fn handle_keymap_action(
        &mut self,
        action: &dyn KeymapAction,
        bounds: Rect,
    ) -> Option<WidgetAction> {
        // Delegate to inner button, then rewrite the Clicked ID.
        self.inner
            .handle_keymap_action(action, bounds)
            .map(|a| match a {
                WidgetAction::Clicked(_) => WidgetAction::Clicked(self.id_override),
                other => other,
            })
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.inner.accept_action(action)
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        if self.is_focusable() {
            vec![self.id_override]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests;
