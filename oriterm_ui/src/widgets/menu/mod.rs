//! Context menu widget — a vertical list of clickable items and separators.
//!
//! Emits `WidgetAction::Selected { id, index }` when an item is clicked or
//! activated via keyboard (Enter/Space). Separators are not clickable.
//! Supports check-mark items for toggleable options.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::input::{HoverEvent, Key, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use crate::layout::LayoutBox;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{DrawCtx, EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};

/// A single entry in a context menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEntry {
    /// A clickable text item.
    Item { label: String },
    /// A checkable item with a check mark indicator.
    Check { label: String, checked: bool },
    /// A horizontal separator line.
    Separator,
}

impl MenuEntry {
    /// Returns the label text, if any.
    fn label(&self) -> Option<&str> {
        match self {
            Self::Item { label } | Self::Check { label, .. } => Some(label),
            Self::Separator => None,
        }
    }

    /// Whether this entry is clickable (not a separator).
    fn is_clickable(&self) -> bool {
        !matches!(self, Self::Separator)
    }
}

/// Visual style for a [`MenuWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct MenuStyle {
    /// Height of each item row.
    pub item_height: f32,
    /// Vertical padding above and below items.
    pub padding_y: f32,
    /// Horizontal padding for item text.
    pub padding_x: f32,
    /// Minimum menu width.
    pub min_width: f32,
    /// Extra width beyond the widest label.
    pub extra_width: f32,
    /// Height of a separator entry.
    pub separator_height: f32,
    /// Background corner radius.
    pub corner_radius: f32,
    /// Hover highlight inset from menu edges.
    pub hover_inset: f32,
    /// Hover highlight corner radius.
    pub hover_radius: f32,
    /// Check mark size (width/height of the check area).
    pub checkmark_size: f32,
    /// Gap between check mark and label text.
    pub checkmark_gap: f32,
    /// Menu background color.
    pub bg: Color,
    /// Item text color.
    pub fg: Color,
    /// Hover highlight background color.
    pub hover_bg: Color,
    /// Separator line color.
    pub separator_color: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Check mark color.
    pub check_color: Color,
    /// Shadow color.
    pub shadow_color: Color,
    /// Font size for item labels.
    pub font_size: f32,
}

impl MenuStyle {
    /// Derives a menu style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            item_height: 32.0,
            padding_y: 4.0,
            padding_x: 12.0,
            min_width: 180.0,
            extra_width: 48.0,
            separator_height: 9.0,
            corner_radius: 8.0,
            hover_inset: 4.0,
            hover_radius: 4.0,
            checkmark_size: 10.0,
            checkmark_gap: 4.0,
            bg: theme.bg_secondary,
            fg: theme.fg_primary,
            hover_bg: theme.bg_hover,
            separator_color: theme.border,
            border_color: theme.border,
            border_width: 1.0,
            check_color: theme.accent,
            shadow_color: theme.shadow,
            font_size: theme.font_size,
        }
    }
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A context menu widget.
///
/// Displays a vertical list of items and separators. Items can be hovered
/// via mouse or navigated via keyboard arrows. Emits
/// `WidgetAction::Selected { id, index }` when activated.
#[derive(Debug)]
pub struct MenuWidget {
    id: WidgetId,
    entries: Vec<MenuEntry>,
    hovered: Option<usize>,
    style: MenuStyle,
}

impl MenuWidget {
    /// Creates a menu widget from the given entries.
    pub fn new(entries: Vec<MenuEntry>) -> Self {
        Self {
            id: WidgetId::next(),
            entries,
            hovered: None,
            style: MenuStyle::default(),
        }
    }

    /// Sets the menu style.
    #[must_use]
    pub fn with_style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[MenuEntry] {
        &self.entries
    }

    /// Returns the currently hovered index.
    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    /// Total height of all entries plus vertical padding.
    fn total_height(&self) -> f32 {
        let content: f32 = self
            .entries
            .iter()
            .map(|e| match e {
                MenuEntry::Separator => self.style.separator_height,
                _ => self.style.item_height,
            })
            .sum();
        content + self.style.padding_y * 2.0
    }

    /// Hit-test: which entry index is at Y position relative to menu top.
    fn entry_at_y(&self, y: f32) -> Option<usize> {
        let y = y - self.style.padding_y;
        if y < 0.0 {
            return None;
        }
        let mut offset = 0.0;
        for (i, entry) in self.entries.iter().enumerate() {
            let h = match entry {
                MenuEntry::Separator => self.style.separator_height,
                _ => self.style.item_height,
            };
            if y < offset + h {
                return if entry.is_clickable() { Some(i) } else { None };
            }
            offset += h;
        }
        None
    }

    /// Moves hover to the next clickable item in the given direction.
    fn navigate(&mut self, forward: bool) -> bool {
        let count = self.entries.len();
        if count == 0 {
            return false;
        }
        let start = match self.hovered {
            Some(i) => {
                if forward {
                    i + 1
                } else {
                    i + count - 1
                }
            }
            None => {
                if forward {
                    0
                } else {
                    count - 1
                }
            }
        };
        // Scan up to `count` positions, wrapping around.
        for offset in 0..count {
            let idx = if forward {
                (start + offset) % count
            } else {
                (start + count - offset) % count
            };
            if self.entries[idx].is_clickable() {
                self.hovered = Some(idx);
                return true;
            }
        }
        false
    }

    /// Whether any entry has a check mark (affects left padding).
    fn has_checks(&self) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e, MenuEntry::Check { .. }))
    }

    /// Left margin for label text — reserves space for checkmarks if needed.
    fn label_left_margin(&self) -> f32 {
        if self.has_checks() {
            self.style.padding_x + self.style.checkmark_size + self.style.checkmark_gap
        } else {
            self.style.padding_x
        }
    }

    /// Builds the `TextStyle` for item labels.
    fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.style.fg)
    }

    /// Draws a checkmark at the given position.
    fn draw_checkmark(&self, ctx: &mut DrawCtx<'_>, x: f32, y: f32) {
        let s = self.style.checkmark_size;
        let inset = s * 0.2;
        let x0 = x + inset;
        let y0 = y + s * 0.5;
        let x1 = x + s * 0.4;
        let y1 = y + s - inset;
        let x2 = x + s - inset;
        let y2 = y + inset;

        ctx.draw_list.push_line(
            Point::new(x0, y0),
            Point::new(x1, y1),
            2.0,
            self.style.check_color,
        );
        ctx.draw_list.push_line(
            Point::new(x1, y1),
            Point::new(x2, y2),
            2.0,
            self.style.check_color,
        );
    }
}

impl Widget for MenuWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = self.text_style();
        let left_margin = self.label_left_margin();

        // Measure max label width.
        let max_label_w: f32 = self
            .entries
            .iter()
            .filter_map(|e| e.label())
            .map(|label| ctx.measurer.measure(label, &style, f32::INFINITY).width)
            .fold(0.0_f32, f32::max);

        let width = (left_margin + max_label_w + self.style.extra_width).max(self.style.min_width);
        let height = self.total_height();

        LayoutBox::leaf(width, height).with_widget_id(self.id)
    }

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;
        let s = &self.style;

        // Shadow rect (offset down-right by 2px).
        let shadow_rect = Rect::new(
            bounds.x() + 2.0,
            bounds.y() + 2.0,
            bounds.width(),
            bounds.height(),
        );
        ctx.draw_list.push_rect(
            shadow_rect,
            RectStyle::filled(s.shadow_color).with_radius(s.corner_radius),
        );

        // Background layer for subpixel text compositing.
        ctx.draw_list.push_layer(s.bg);

        // Background rect with border.
        ctx.draw_list.push_rect(
            bounds,
            RectStyle::filled(s.bg)
                .with_border(s.border_width, s.border_color)
                .with_radius(s.corner_radius),
        );

        let left_margin = self.label_left_margin();
        let text_style = self.text_style();
        let mut y = bounds.y() + s.padding_y;

        for (i, entry) in self.entries.iter().enumerate() {
            match entry {
                MenuEntry::Separator => {
                    let sep_y = y + s.separator_height / 2.0;
                    let x0 = bounds.x() + s.hover_inset;
                    let x1 = bounds.right() - s.hover_inset;
                    ctx.draw_list.push_line(
                        Point::new(x0, sep_y),
                        Point::new(x1, sep_y),
                        1.0,
                        s.separator_color,
                    );
                    y += s.separator_height;
                }
                MenuEntry::Item { label } => {
                    // Hover highlight.
                    if self.hovered == Some(i) {
                        let hover_rect = Rect::new(
                            bounds.x() + s.hover_inset,
                            y,
                            bounds.width() - s.hover_inset * 2.0,
                            s.item_height,
                        );
                        ctx.draw_list.push_rect(
                            hover_rect,
                            RectStyle::filled(s.hover_bg).with_radius(s.hover_radius),
                        );
                    }

                    // Label text.
                    let text_x = bounds.x() + left_margin;
                    let text_w = bounds.width() - left_margin - s.padding_x;
                    let shaped = ctx.measurer.shape(label, &text_style, text_w);
                    let text_y = y + (s.item_height - shaped.height) / 2.0;
                    ctx.draw_list
                        .push_text(Point::new(text_x, text_y), shaped, s.fg);

                    y += s.item_height;
                }
                MenuEntry::Check { label, checked } => {
                    // Hover highlight.
                    if self.hovered == Some(i) {
                        let hover_rect = Rect::new(
                            bounds.x() + s.hover_inset,
                            y,
                            bounds.width() - s.hover_inset * 2.0,
                            s.item_height,
                        );
                        ctx.draw_list.push_rect(
                            hover_rect,
                            RectStyle::filled(s.hover_bg).with_radius(s.hover_radius),
                        );
                    }

                    // Checkmark.
                    if *checked {
                        let check_x = bounds.x() + s.padding_x;
                        let check_y = y + (s.item_height - s.checkmark_size) / 2.0;
                        self.draw_checkmark(ctx, check_x, check_y);
                    }

                    // Label text.
                    let text_x = bounds.x() + left_margin;
                    let text_w = bounds.width() - left_margin - s.padding_x;
                    let shaped = ctx.measurer.shape(label, &text_style, text_w);
                    let text_y = y + (s.item_height - shaped.height) / 2.0;
                    ctx.draw_list
                        .push_text(Point::new(text_x, text_y), shaped, s.fg);

                    y += s.item_height;
                }
            }
        }

        ctx.draw_list.pop_layer();
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        match event.kind {
            MouseEventKind::Move => {
                let rel_y = event.pos.y - ctx.bounds.y();
                let new_hover = self.entry_at_y(rel_y);
                if new_hover != self.hovered {
                    self.hovered = new_hover;
                    return WidgetResponse::redraw();
                }
                WidgetResponse::handled()
            }
            MouseEventKind::Down(MouseButton::Left) => {
                // Absorb press — action fires on release.
                WidgetResponse::handled()
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(idx) = self.hovered {
                    if self.entries[idx].is_clickable() {
                        return WidgetResponse::redraw().with_action(WidgetAction::Selected {
                            id: self.id,
                            index: idx,
                        });
                    }
                }
                WidgetResponse::handled()
            }
            _ => WidgetResponse::ignored(),
        }
    }

    fn handle_hover(&mut self, event: HoverEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        match event {
            HoverEvent::Enter => WidgetResponse::handled(),
            HoverEvent::Leave => {
                if self.hovered.is_some() {
                    self.hovered = None;
                    WidgetResponse::redraw()
                } else {
                    WidgetResponse::handled()
                }
            }
        }
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        if !ctx.is_focused {
            return WidgetResponse::ignored();
        }
        match event.key {
            Key::ArrowDown => {
                if self.navigate(true) {
                    WidgetResponse::redraw()
                } else {
                    WidgetResponse::handled()
                }
            }
            Key::ArrowUp => {
                if self.navigate(false) {
                    WidgetResponse::redraw()
                } else {
                    WidgetResponse::handled()
                }
            }
            Key::Enter | Key::Space => {
                if let Some(idx) = self.hovered {
                    if self.entries[idx].is_clickable() {
                        return WidgetResponse::redraw().with_action(WidgetAction::Selected {
                            id: self.id,
                            index: idx,
                        });
                    }
                }
                WidgetResponse::handled()
            }
            Key::Escape => {
                WidgetResponse::redraw().with_action(WidgetAction::DismissOverlay(self.id))
            }
            _ => WidgetResponse::ignored(),
        }
    }
}

#[cfg(test)]
mod tests;
