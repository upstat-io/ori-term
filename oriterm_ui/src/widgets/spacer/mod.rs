//! Spacer widget — flexible or fixed empty space.
//!
//! Used within flex containers to push siblings apart or insert fixed gaps.
//! Not interactive, not focusable.

use crate::layout::{LayoutBox, SizeSpec};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// An empty-space widget for use in flex layouts.
///
/// Two modes: fixed size (exact pixel dimensions) or fill (expands to
/// consume remaining space along both axes).
#[derive(Debug, Clone)]
pub struct SpacerWidget {
    id: WidgetId,
    width: SizeSpec,
    height: SizeSpec,
}

impl SpacerWidget {
    /// Creates a spacer with fixed pixel dimensions.
    pub fn fixed(width: f32, height: f32) -> Self {
        Self {
            id: WidgetId::next(),
            width: SizeSpec::Fixed(width),
            height: SizeSpec::Fixed(height),
        }
    }

    /// Creates a spacer that fills available space along both axes.
    ///
    /// In a Row, this pushes siblings to opposite ends horizontally.
    /// In a Column, it pushes them apart vertically.
    pub fn fill() -> Self {
        Self {
            id: WidgetId::next(),
            width: SizeSpec::Fill,
            height: SizeSpec::Fill,
        }
    }
}

impl Widget for SpacerWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        let intrinsic_w = match self.width {
            SizeSpec::Fixed(v) => v,
            _ => 0.0,
        };
        let intrinsic_h = match self.height {
            SizeSpec::Fixed(v) => v,
            _ => 0.0,
        };
        LayoutBox::leaf(intrinsic_w, intrinsic_h)
            .with_width(self.width)
            .with_height(self.height)
            .with_widget_id(self.id)
    }

    fn paint(&self, _ctx: &mut DrawCtx<'_>) {
        // Spacers are invisible — no paint commands.
    }
}

#[cfg(test)]
mod tests;
