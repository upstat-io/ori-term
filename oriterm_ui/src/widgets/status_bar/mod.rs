//! Status bar widget for terminal metadata display.
//!
//! Renders a 22px bar at the bottom of the window showing shell name,
//! pane count, grid dimensions, encoding, and terminal type. The left
//! group is left-aligned; the right group is right-aligned. Shell and
//! term type use accent color; other items use faint text color.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

// -- Constants (from mockup CSS) --

/// Status bar height in logical pixels.
pub const STATUS_BAR_HEIGHT: f32 = 22.0;

/// Horizontal padding inside the bar.
const STATUS_BAR_PADDING_X: f32 = 10.0;

/// Top border thickness.
const STATUS_BAR_BORDER_TOP: f32 = 2.0;

/// Gap between items.
const STATUS_BAR_GAP: f32 = 16.0;

/// Font size for all status bar text.
const STATUS_BAR_FONT_SIZE: f32 = 11.0;

// -- Data model --

/// Terminal metadata displayed in the status bar.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusBarData {
    /// Shell or process name (e.g., "zsh"). Displayed in accent color.
    pub shell_name: String,
    /// Number of visible panes (e.g., "3 panes", "1 pane").
    pub pane_count: String,
    /// Grid dimensions (e.g., "120\u{00d7}30").
    pub grid_size: String,
    /// Character encoding (e.g., "UTF-8").
    pub encoding: String,
    /// Terminal type (e.g., "xterm-256color"). Displayed in accent color.
    pub term_type: String,
}

// -- Colors --

/// Colors for status bar rendering, derived from [`UiTheme`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusBarColors {
    /// Background fill.
    pub bg: Color,
    /// Top border.
    pub border: Color,
    /// Normal item text.
    pub text: Color,
    /// Accent item text (shell name, term type).
    pub accent: Color,
}

impl StatusBarColors {
    /// Construct status bar colors from a UI theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            bg: theme.bg_primary,
            border: theme.border,
            text: theme.fg_faint,
            accent: theme.accent,
        }
    }
}

// -- Widget --

/// A non-interactive status bar displaying terminal metadata.
///
/// Shows shell name and terminal type in accent color, other items in
/// faint text. Left group (shell, panes, grid) is left-aligned; right
/// group (encoding, term type) is right-aligned.
pub struct StatusBarWidget {
    id: WidgetId,
    data: StatusBarData,
    colors: StatusBarColors,
    window_width: f32,
}

impl StatusBarWidget {
    /// Create a status bar widget for the given window width and theme.
    pub fn new(window_width: f32, theme: &UiTheme) -> Self {
        Self {
            id: WidgetId::next(),
            data: StatusBarData::default(),
            colors: StatusBarColors::from_theme(theme),
            window_width,
        }
    }

    /// Update the terminal metadata.
    pub fn set_data(&mut self, data: StatusBarData) {
        self.data = data;
    }

    /// Update the window width (on resize).
    pub fn set_window_width(&mut self, width: f32) {
        self.window_width = width;
    }

    /// Re-derive colors from a new theme.
    pub fn apply_theme(&mut self, theme: &UiTheme) {
        self.colors = StatusBarColors::from_theme(theme);
    }
}

// -- Paint helpers --

impl StatusBarWidget {
    /// Paint left-aligned items, returning the x position after the last item.
    fn paint_left_items(&self, ctx: &mut DrawCtx<'_>, y: f32) -> f32 {
        let mut x = ctx.bounds.x() + STATUS_BAR_PADDING_X;
        let items: [(&str, Color); 3] = [
            (&self.data.shell_name, self.colors.accent),
            (&self.data.pane_count, self.colors.text),
            (&self.data.grid_size, self.colors.text),
        ];
        for (text, color) in items {
            if text.is_empty() {
                continue;
            }
            let style = TextStyle::new(STATUS_BAR_FONT_SIZE, color);
            let shaped = ctx.measurer.shape(text, &style, f32::INFINITY);
            let text_y = y + (STATUS_BAR_HEIGHT - shaped.height) / 2.0;
            ctx.scene.push_text(Point::new(x, text_y), shaped, color);
            x += ctx.measurer.measure(text, &style, f32::INFINITY).width + STATUS_BAR_GAP;
        }
        x
    }

    /// Paint right-aligned items, working inward from the right edge.
    fn paint_right_items(&self, ctx: &mut DrawCtx<'_>, y: f32) {
        let mut x = ctx.bounds.x() + ctx.bounds.width() - STATUS_BAR_PADDING_X;
        let items: [(&str, Color); 2] = [
            (&self.data.term_type, self.colors.accent),
            (&self.data.encoding, self.colors.text),
        ];
        for (text, color) in items {
            if text.is_empty() {
                continue;
            }
            let style = TextStyle::new(STATUS_BAR_FONT_SIZE, color);
            let shaped = ctx.measurer.shape(text, &style, f32::INFINITY);
            let w = ctx.measurer.measure(text, &style, f32::INFINITY).width;
            x -= w;
            let text_y = y + (STATUS_BAR_HEIGHT - shaped.height) / 2.0;
            ctx.scene.push_text(Point::new(x, text_y), shaped, color);
            x -= STATUS_BAR_GAP;
        }
    }
}

// -- Widget trait --

impl Widget for StatusBarWidget {
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
        LayoutBox::leaf(self.window_width, STATUS_BAR_HEIGHT).with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let x = ctx.bounds.x();
        let y = ctx.bounds.y();
        let w = ctx.bounds.width();

        // Background.
        ctx.scene.push_quad(
            Rect::new(x, y, w, STATUS_BAR_HEIGHT),
            RectStyle::filled(self.colors.bg),
        );

        // Top border.
        ctx.scene.push_quad(
            Rect::new(x, y, w, STATUS_BAR_BORDER_TOP),
            RectStyle::filled(self.colors.border),
        );

        // Left-aligned items.
        self.paint_left_items(ctx, y);

        // Right-aligned items.
        self.paint_right_items(ctx, y);
    }
}

#[cfg(test)]
mod tests;
