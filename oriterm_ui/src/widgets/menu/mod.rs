//! Menu widget — a vertical list of clickable items and separators.
//!
//! Used for both context menus and dropdown popup lists. Emits
//! `WidgetAction::Selected { id, index }` when an item is activated.
//! Supports scrolling via `max_height` for long lists.

use crate::color::Color;
use crate::controllers::{ClickController, EventController};
use crate::geometry::Point;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::DrawCtx;

mod widget_impl;

/// A single entry in a menu.
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
    pub(super) fn label(&self) -> Option<&str> {
        match self {
            Self::Item { label } | Self::Check { label, .. } => Some(label),
            Self::Separator => None,
        }
    }

    /// Whether this entry is clickable (not a separator).
    pub(super) fn is_clickable(&self) -> bool {
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
    /// Background tint for the selected item (before hover).
    pub selected_bg: Color,
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
    /// Maximum visible height before scrolling. `None` shows all items.
    pub max_height: Option<f32>,
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
            bg: theme.bg_card,
            fg: theme.fg_primary,
            hover_bg: theme.bg_hover,
            selected_bg: Color::TRANSPARENT,
            separator_color: theme.border,
            border_color: theme.border,
            border_width: 1.0,
            check_color: theme.accent,
            shadow_color: theme.shadow,
            font_size: theme.font_size,
            max_height: None,
        }
    }
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// Scrollbar width inside the menu (logical pixels).
const SCROLLBAR_WIDTH: f32 = 5.0;

/// Scrollbar thumb minimum height.
const SCROLLBAR_MIN_THUMB: f32 = 16.0;

/// Pixels per scroll wheel line.
const SCROLL_LINE_HEIGHT: f32 = 32.0;

/// A menu widget with optional scrolling.
///
/// Displays a vertical list of items and separators. Items can be hovered
/// via mouse or navigated via keyboard arrows. Emits
/// `WidgetAction::Selected { id, index }` when activated. When `max_height`
/// is set in the style, long lists scroll with a scrollbar.
pub struct MenuWidget {
    pub(super) id: WidgetId,
    pub(super) entries: Vec<MenuEntry>,
    /// Currently hovered (highlighted) entry index.
    pub(super) hovered: Option<usize>,
    /// Pre-selected entry index (shown with accent tint).
    pub(super) selected_index: Option<usize>,
    pub(super) style: MenuStyle,
    /// Scroll offset in pixels from top of content.
    pub(super) scroll_offset: f32,
    controllers: Vec<Box<dyn EventController>>,
}

impl std::fmt::Debug for MenuWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MenuWidget")
            .field("id", &self.id)
            .field("entries", &self.entries.len())
            .field("hovered", &self.hovered)
            .field("selected_index", &self.selected_index)
            .field("style", &self.style)
            .field("scroll_offset", &self.scroll_offset)
            .field("controller_count", &self.controllers.len())
            .finish()
    }
}

impl MenuWidget {
    /// Creates a menu widget from the given entries.
    pub fn new(entries: Vec<MenuEntry>) -> Self {
        Self {
            id: WidgetId::next(),
            entries,
            hovered: None,
            selected_index: None,
            style: MenuStyle::default(),
            controllers: vec![Box::new(ClickController::new())],
            scroll_offset: 0.0,
        }
    }

    /// Sets the menu style.
    #[must_use]
    pub fn with_style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the pre-selected entry index (highlighted with accent tint).
    #[must_use]
    pub fn with_selected_index(mut self, index: usize) -> Self {
        self.selected_index = Some(index);
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
    pub(super) fn total_height(&self) -> f32 {
        self.content_height() + self.style.padding_y * 2.0
    }

    /// Height of all entries (excluding padding).
    fn content_height(&self) -> f32 {
        self.entries
            .iter()
            .map(|e| match e {
                MenuEntry::Separator => self.style.separator_height,
                _ => self.style.item_height,
            })
            .sum()
    }

    /// Visible height — clamped by `max_height` if set.
    pub(super) fn visible_height(&self) -> f32 {
        let total = self.total_height();
        self.style.max_height.map_or(total, |max| total.min(max))
    }

    /// Maximum scroll offset.
    fn max_scroll(&self) -> f32 {
        (self.total_height() - self.visible_height()).max(0.0)
    }

    /// Whether the content overflows and scrolling is active.
    fn is_scrollable(&self) -> bool {
        self.max_scroll() > f32::EPSILON
    }

    /// Scroll by a pixel delta. Positive = scroll down (increase offset).
    fn scroll_by(&mut self, delta: f32) -> bool {
        let max = self.max_scroll();
        let old = self.scroll_offset;
        self.scroll_offset = (self.scroll_offset + delta).clamp(0.0, max);
        (self.scroll_offset - old).abs() > f32::EPSILON
    }

    /// Y offset of an entry relative to content top (after top padding).
    fn entry_top_y(&self, target: usize) -> f32 {
        let mut y = 0.0;
        for (i, entry) in self.entries.iter().enumerate() {
            if i == target {
                return y;
            }
            y += match entry {
                MenuEntry::Separator => self.style.separator_height,
                _ => self.style.item_height,
            };
        }
        y
    }

    /// Scrolls so the given entry is fully visible.
    pub fn ensure_visible(&mut self, index: usize) {
        if !self.is_scrollable() {
            return;
        }
        let item_y = self.entry_top_y(index);
        let item_h = match &self.entries[index] {
            MenuEntry::Separator => self.style.separator_height,
            _ => self.style.item_height,
        };
        let visible_content = self.visible_height() - self.style.padding_y * 2.0;

        if item_y < self.scroll_offset {
            self.scroll_offset = item_y;
        } else if item_y + item_h > self.scroll_offset + visible_content {
            self.scroll_offset = item_y + item_h - visible_content;
        } else {
            return;
        }
        self.scroll_offset = self.scroll_offset.clamp(0.0, self.max_scroll());
    }

    /// Hit-test: which entry index is at Y position relative to menu top.
    ///
    /// Accounts for scroll offset when scrolling is active.
    pub(super) fn entry_at_y(&self, y: f32) -> Option<usize> {
        let y = y - self.style.padding_y + self.scroll_offset;
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
    pub(super) fn navigate(&mut self, forward: bool) -> bool {
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
    pub(super) fn has_checks(&self) -> bool {
        self.entries
            .iter()
            .any(|e| matches!(e, MenuEntry::Check { .. }))
    }

    /// Left margin for label text — reserves space for checkmarks if needed.
    pub(super) fn label_left_margin(&self) -> f32 {
        if self.has_checks() {
            self.style.padding_x + self.style.checkmark_size + self.style.checkmark_gap
        } else {
            self.style.padding_x
        }
    }

    /// Builds the `TextStyle` for item labels.
    pub(super) fn text_style(&self) -> TextStyle {
        TextStyle::new(self.style.font_size, self.style.fg)
    }

    /// Draws a checkmark at the given position.
    pub(super) fn draw_checkmark(&self, ctx: &mut DrawCtx<'_>, x: f32, y: f32) {
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

#[cfg(test)]
mod tests;
