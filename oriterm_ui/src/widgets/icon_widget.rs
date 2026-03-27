//! Minimal icon widget that renders a pre-resolved atlas icon.
//!
//! Participates in layout with a fixed intrinsic size and paints via
//! `Scene::push_icon()`. Used as a leaf in container rows (e.g. the
//! settings footer's unsaved-changes indicator group).

use crate::color::Color;
use crate::geometry::Rect;
use crate::icons::IconId;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// A non-interactive widget that displays a single icon.
pub struct IconWidget {
    id: WidgetId,
    icon_id: IconId,
    /// Logical pixel size (width and height are equal).
    size: u32,
    color: Color,
}

impl IconWidget {
    /// Creates an icon widget for `icon_id` at the given logical pixel size.
    pub fn new(icon_id: IconId, size: u32, color: Color) -> Self {
        Self {
            id: WidgetId::next(),
            icon_id,
            size,
            color,
        }
    }
}

impl Widget for IconWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        let s = self.size as f32;
        LayoutBox::leaf(s, s).with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        if let Some(icons) = ctx.icons {
            if let Some(resolved) = icons.get(self.icon_id, self.size) {
                let s = self.size as f32;
                let icon_rect = Rect::new(ctx.bounds.x(), ctx.bounds.y(), s, s);
                ctx.scene
                    .push_icon(icon_rect, resolved.atlas_page, resolved.uv, self.color);
            }
        }
    }
}
