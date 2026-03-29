//! Text label widget.
//!
//! Displays static text with configurable style, alignment, and overflow
//! behavior (clip or ellipsis truncation).

use crate::color::Color;
use crate::geometry::Point;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::{FontWeight, TextOverflow, TextStyle, TextTransform};
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget};

/// Style for a [`LabelWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct LabelStyle {
    /// Text color.
    pub color: Color,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Font weight.
    pub weight: FontWeight,
    /// Overflow behavior.
    pub overflow: TextOverflow,
    /// Extra spacing between characters in pixels.
    pub letter_spacing: f32,
    /// Case transformation applied before shaping.
    pub text_transform: TextTransform,
    /// Line-height multiplier override. `None` uses natural font metrics.
    pub line_height: Option<f32>,
}

impl LabelStyle {
    /// Derives a label style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            color: theme.fg_primary,
            font_size: theme.font_size,
            weight: FontWeight::NORMAL,
            overflow: TextOverflow::Clip,
            letter_spacing: 0.0,
            text_transform: TextTransform::None,
            line_height: None,
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
        let mut ts = TextStyle::new(self.style.font_size, self.style.color)
            .with_weight(self.style.weight)
            .with_overflow(self.style.overflow)
            .with_letter_spacing(self.style.letter_spacing)
            .with_text_transform(self.style.text_transform);
        ts.line_height = self.style.line_height;
        ts
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
