//! Separator widget — a divider line with optional label.
//!
//! Renders as a horizontal or vertical line. When a label is provided,
//! the line splits around the label text.

use crate::color::Color;
use crate::geometry::{Point, Rect};
use crate::layout::Direction;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget};

/// Style for a [`SeparatorWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct SeparatorStyle {
    /// Line color.
    pub color: Color,
    /// Line thickness in logical pixels.
    pub thickness: f32,
    /// Label text color (if label is set).
    pub label_color: Color,
    /// Label font size in logical pixels.
    pub label_font_size: f32,
    /// Padding between the label and line segments.
    pub label_gap: f32,
}

impl SeparatorStyle {
    /// Derives a separator style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            color: theme.border,
            thickness: 1.0,
            label_color: theme.fg_primary,
            label_font_size: theme.font_size_small,
            label_gap: theme.spacing,
        }
    }
}

impl Default for SeparatorStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A non-interactive divider line with optional label.
///
/// Not focusable. The separator is always rendered along the given
/// direction: `Row` draws a horizontal line, `Column` draws a vertical line.
#[derive(Debug, Clone)]
pub struct SeparatorWidget {
    id: WidgetId,
    direction: Direction,
    label: Option<String>,
    style: SeparatorStyle,
}

impl SeparatorWidget {
    /// Creates a horizontal separator with no label.
    pub fn horizontal() -> Self {
        Self {
            id: WidgetId::next(),
            direction: Direction::Row,
            label: None,
            style: SeparatorStyle::default(),
        }
    }

    /// Creates a vertical separator with no label.
    pub fn vertical() -> Self {
        Self {
            id: WidgetId::next(),
            direction: Direction::Column,
            label: None,
            style: SeparatorStyle::default(),
        }
    }

    /// Sets an optional label displayed in the middle of the line.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets the separator style.
    #[must_use]
    pub fn with_style(mut self, style: SeparatorStyle) -> Self {
        self.style = style;
        self
    }

    /// Builds the `TextStyle` for the label.
    fn label_text_style(&self) -> TextStyle {
        TextStyle::new(self.style.label_font_size, self.style.label_color)
    }
}

impl Widget for SeparatorWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        match self.direction {
            Direction::Row => {
                // Horizontal: full width (Fill), height = thickness or label height.
                let height = if let Some(ref label) = self.label {
                    let style = self.label_text_style();
                    let m = ctx.measurer.measure(label, &style, f32::INFINITY);
                    m.height
                } else {
                    self.style.thickness
                };
                LayoutBox::leaf(0.0, height)
                    .with_width(crate::layout::SizeSpec::Fill)
                    .with_widget_id(self.id)
            }
            Direction::Column => {
                // Vertical: full height (Fill), width = thickness.
                LayoutBox::leaf(self.style.thickness, 0.0)
                    .with_height(crate::layout::SizeSpec::Fill)
                    .with_widget_id(self.id)
            }
        }
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let b = ctx.bounds;
        match self.direction {
            Direction::Row => self.draw_horizontal(ctx, b),
            Direction::Column => self.draw_vertical(ctx, b),
        }
    }
}

impl SeparatorWidget {
    /// Draws a horizontal separator, optionally with a centered label.
    fn draw_horizontal(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let y = bounds.y() + bounds.height() / 2.0;

        if let Some(ref label) = self.label {
            let style = self.label_text_style();
            let shaped = ctx.measurer.shape(label, &style, bounds.width());
            let label_x = bounds.x() + (bounds.width() - shaped.width) / 2.0;
            let gap = self.style.label_gap;

            // Left line segment.
            let left_end = label_x - gap;
            if left_end > bounds.x() {
                ctx.scene.push_line(
                    Point::new(bounds.x(), y),
                    Point::new(left_end, y),
                    self.style.thickness,
                    self.style.color,
                );
            }

            // Right line segment.
            let right_start = label_x + shaped.width + gap;
            if right_start < bounds.right() {
                ctx.scene.push_line(
                    Point::new(right_start, y),
                    Point::new(bounds.right(), y),
                    self.style.thickness,
                    self.style.color,
                );
            }

            // Label text — vertically centered.
            let text_y = bounds.y() + (bounds.height() - shaped.height) / 2.0;
            ctx.scene
                .push_text(Point::new(label_x, text_y), shaped, self.style.label_color);
        } else {
            // Plain line across full width.
            ctx.scene.push_line(
                Point::new(bounds.x(), y),
                Point::new(bounds.right(), y),
                self.style.thickness,
                self.style.color,
            );
        }
    }

    /// Draws a vertical separator.
    fn draw_vertical(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let x = bounds.x() + bounds.width() / 2.0;
        ctx.scene.push_line(
            Point::new(x, bounds.y()),
            Point::new(x, bounds.bottom()),
            self.style.thickness,
            self.style.color,
        );
    }
}

#[cfg(test)]
mod tests;
