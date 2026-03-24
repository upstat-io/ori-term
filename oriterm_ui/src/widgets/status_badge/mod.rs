//! Lightweight display-only text badge.
//!
//! Draws a rounded-rect background with styled text inside. Not a full
//! [`Widget`](super::Widget) — doesn't handle events or participate in
//! the layout tree. Used for floating status indicators: search bars,
//! toast notifications, scroll position displays.

use crate::color::Color;
use crate::draw::{RectStyle, Scene};
use crate::geometry::{Insets, Point, Rect};
use crate::text::{TextOverflow, TextStyle};
use crate::theme::UiTheme;

use super::TextMeasurer;

/// Visual style for a [`StatusBadge`].
#[derive(Debug, Clone, PartialEq)]
pub struct StatusBadgeStyle {
    /// Background fill color.
    pub bg: Color,
    /// Text color.
    pub fg: Color,
    /// Font size in logical pixels.
    pub font_size: f32,
    /// Corner radius for the background rect.
    pub corner_radius: f32,
    /// Padding between text and badge edges.
    pub padding: Insets,
}

impl StatusBadgeStyle {
    /// Derives a style from the given theme.
    ///
    /// Uses secondary background with alpha for floating overlays,
    /// primary foreground, and small font size.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            bg: theme.bg_secondary.with_alpha(0.9),
            fg: theme.fg_primary,
            font_size: theme.font_size_small,
            corner_radius: theme.corner_radius,
            padding: Insets::vh(5.0, 8.0),
        }
    }
}

impl Default for StatusBadgeStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A lightweight text badge for floating status indicators.
///
/// Draws a rounded-rect background with text inside. Not a full
/// [`Widget`](super::Widget) — doesn't handle events or participate in
/// the layout tree. Suitable for search bars, toast notifications,
/// scroll position displays, and other transient overlays.
///
/// # Usage
///
/// ```ignore
/// let badge = StatusBadge::new("Search: foo  2 of 5");
/// let (w, _h) = badge.measure(&measurer, max_width);
/// let pos = Point::new(viewport_w - w - margin, top_y);
/// badge.draw(&mut scene, &measurer, pos, max_width);
/// ```
#[derive(Debug)]
pub struct StatusBadge<'a> {
    text: &'a str,
    style: StatusBadgeStyle,
}

impl<'a> StatusBadge<'a> {
    /// Creates a badge displaying `text` with default style.
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            style: StatusBadgeStyle::default(),
        }
    }

    /// Sets the badge style.
    #[must_use]
    pub fn with_style(mut self, style: StatusBadgeStyle) -> Self {
        self.style = style;
        self
    }

    /// Computes the badge dimensions for the given max text width.
    ///
    /// Returns `(width, height)` in logical pixels. Use this to determine
    /// positioning before calling [`draw`](Self::draw).
    pub fn measure(&self, measurer: &dyn TextMeasurer, max_text_width: f32) -> (f32, f32) {
        let shaped = measurer.shape(self.text, &self.text_style(), max_text_width);
        (
            shaped.width + self.style.padding.width(),
            shaped.height + self.style.padding.height(),
        )
    }

    /// Draws the badge at `pos` (top-left corner) into the scene.
    ///
    /// `max_text_width` constrains the text; text exceeding this width is
    /// truncated with an ellipsis. Returns the bounds of the drawn badge.
    pub fn draw(
        &self,
        scene: &mut Scene,
        measurer: &dyn TextMeasurer,
        pos: Point,
        max_text_width: f32,
    ) -> Rect {
        let shaped = measurer.shape(self.text, &self.text_style(), max_text_width);
        let w = shaped.width + self.style.padding.width();
        let h = shaped.height + self.style.padding.height();
        let rect = Rect::new(pos.x, pos.y, w, h);

        scene.push_layer_bg(self.style.bg);
        scene.push_quad(
            rect,
            RectStyle::filled(self.style.bg).with_radius(self.style.corner_radius),
        );
        scene.push_text(
            Point::new(
                pos.x + self.style.padding.left,
                pos.y + self.style.padding.top,
            ),
            shaped,
            self.style.fg,
        );
        scene.pop_layer_bg();

        rect
    }

    /// Builds the internal text style from badge style.
    fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.style.fg).with_overflow(TextOverflow::Ellipsis)
    }
}

#[cfg(test)]
mod tests;
