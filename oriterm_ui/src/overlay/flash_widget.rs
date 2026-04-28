//! No-op widget body used as the `Overlay.widget` placeholder for `Flash`
//! overlays.
//!
//! The visual flash is emitted by the Flash arm of
//! [`OverlayManager::draw_overlay_at`](super::manager::OverlayManager::draw_overlay_at)
//! — it pushes a colored quad with animated opacity directly. The widget tree
//! contributes nothing for `Flash`, so this placeholder satisfies the
//! `Box<dyn Widget>` field on `Overlay` without rendering, hit-testing, or
//! requesting any input.

use crate::action::WidgetAction;
use crate::geometry::Rect;
use crate::layout::LayoutBox;
use crate::widget_id::WidgetId;
use crate::widgets::{LayoutCtx, Widget};

/// A widget that contributes nothing to layout, paint, or input.
///
/// Used as the `widget` field for `OverlayKind::Flash` overlays. The flash
/// quad is emitted manually from `OverlayManager::draw_overlay_at`; this
/// type only exists so `Overlay { widget: Box<dyn Widget>, .. }` stays
/// uniform across overlay kinds without a special `Option<Box<dyn Widget>>`.
pub(in crate::overlay) struct FlashWidget {
    id: WidgetId,
}

impl FlashWidget {
    pub(in crate::overlay) fn new() -> Self {
        Self {
            id: WidgetId::next(),
        }
    }
}

impl Widget for FlashWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(0.0, 0.0)
    }

    /// Flash never participates in the action pipeline. The `Widget` default
    /// would forward `Some(action)` — explicit `None` makes the
    /// no-input-pass-through contract uniform with the rest of the file.
    fn on_action(&mut self, _action: WidgetAction, _bounds: Rect) -> Option<WidgetAction> {
        None
    }
}
