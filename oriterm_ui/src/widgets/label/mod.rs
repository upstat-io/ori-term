//! Text label widget.
//!
//! Displays static text with configurable style, alignment, and overflow
//! behavior (clip or ellipsis truncation).

use crate::color::Color;
use crate::geometry::Point;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::{TextOverflow, TextStyle};
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget};

/// Style for a [`LabelWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct LabelStyle {
    /// Text color.
    pub color: Color,
    /// Font size in points.
    pub font_size: f32,
    /// Overflow behavior.
    pub overflow: TextOverflow,
}

impl LabelStyle {
    /// Derives a label style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            color: theme.fg_primary,
            font_size: theme.font_size,
            overflow: TextOverflow::Clip,
        }
    }
}

impl Default for LabelStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A non-interactive text label.
///
/// Displays a text string. Not focusable — use `ButtonWidget` for
/// clickable text.
#[derive(Debug, Clone)]
pub struct LabelWidget {
    id: WidgetId,
    text: String,
    style: LabelStyle,
}

impl LabelWidget {
    /// Creates a label displaying `text` with default style.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: WidgetId::next(),
            text: text.into(),
            style: LabelStyle::default(),
        }
    }

    /// Returns the displayed text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Updates the displayed text.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
    }

    /// Sets the label style.
    #[must_use]
    pub fn with_style(mut self, style: LabelStyle) -> Self {
        self.style = style;
        self
    }

    /// Builds the `TextStyle` for measurement and shaping.
    fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.style.color).with_overflow(self.style.overflow)
    }
}

impl Widget for LabelWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = self.text_style();
        let metrics = ctx.measurer.measure(&self.text, &style, f32::INFINITY);
        LayoutBox::leaf(metrics.width, metrics.height).with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        if self.text.is_empty() {
            return;
        }
        let style = self.text_style();
        let max_width = ctx.bounds.width();
        let shaped = ctx.measurer.shape(&self.text, &style, max_width);
        let pos = Point::new(ctx.bounds.x(), ctx.bounds.y());
        ctx.scene.push_text(pos, shaped, self.style.color);
    }
}

#[cfg(test)]
mod tests;
