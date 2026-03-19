//! Rich text label widget — multi-span styled text on a single line.
//!
//! Each span has its own [`TextStyle`] (color, size, weight). Used for the
//! font preview in settings where different colors appear on the same line.

use crate::geometry::Point;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// A single styled text run within a [`RichLabel`].
#[derive(Debug, Clone, PartialEq)]
pub struct TextSpan {
    /// The text content of this span.
    pub text: String,
    /// Visual style for this span (color, size, weight).
    pub style: TextStyle,
}

/// A non-interactive label composed of multiple styled text spans.
///
/// Renders each span sequentially at advancing x-offsets, allowing
/// multi-color text on a single line. Not focusable.
#[derive(Debug, Clone)]
pub struct RichLabel {
    id: WidgetId,
    spans: Vec<TextSpan>,
}

impl RichLabel {
    /// Creates a rich label from a list of styled spans.
    pub fn new(spans: Vec<TextSpan>) -> Self {
        Self {
            id: WidgetId::next(),
            spans,
        }
    }
}

impl Widget for RichLabel {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let mut total_w = 0.0_f32;
        let mut max_h = 0.0_f32;
        for span in &self.spans {
            let m = ctx.measurer.measure(&span.text, &span.style, f32::INFINITY);
            total_w += m.width;
            max_h = max_h.max(m.height);
        }
        LayoutBox::leaf(total_w, max_h).with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let mut x = ctx.bounds.x();
        let y = ctx.bounds.y();
        let max_w = ctx.bounds.width();

        for span in &self.spans {
            if x - ctx.bounds.x() >= max_w {
                break;
            }
            let remaining = max_w - (x - ctx.bounds.x());
            let shaped = ctx.measurer.shape(&span.text, &span.style, remaining);
            let advance = shaped.width;
            ctx.scene
                .push_text(Point::new(x, y), shaped, span.style.color);
            x += advance;
        }
    }
}

#[cfg(test)]
mod tests;
