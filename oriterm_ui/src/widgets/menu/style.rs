//! Visual style + private interaction-state types for [`MenuWidget`].
//!
//! `MenuStyle` is the public theming surface; `MenuScrollbarState` and
//! `DragMode` are module-private state helpers used by the scrollbar +
//! click pipeline. `SCROLL_LINE_HEIGHT` is the wheel-line scroll constant.

use crate::color::Color;
use crate::theme::UiTheme;

use super::super::scrollbar::{ScrollbarStyle, ScrollbarVisualState};

/// Visual style for a [`super::MenuWidget`].
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
    /// Scrollbar appearance for long menus.
    pub scrollbar: ScrollbarStyle,
    /// Extra vertical breathing room added to the searchable-mode query row
    /// on top of `font_size + padding_y * 2.0`. Keeps the search input
    /// visually distinct from the entries below it.
    pub query_row_extra_height: f32,
    /// Text color used for low-emphasis search affordances (the placeholder
    /// "Search…" string when the query is empty, and the "No matches"
    /// indicator when the filter returns nothing). Theme-derived so the
    /// faint hue tracks light/dark variants.
    pub no_match_text_color: Color,
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
            corner_radius: theme.corner_radius,
            hover_inset: 4.0,
            hover_radius: theme.corner_radius,
            checkmark_size: 10.0,
            checkmark_gap: 4.0,
            bg: theme.bg_input,
            fg: theme.fg_primary,
            hover_bg: theme.bg_hover,
            selected_bg: Color::TRANSPARENT,
            separator_color: theme.border,
            border_color: theme.border,
            border_width: 2.0,
            check_color: theme.accent,
            shadow_color: theme.shadow,
            font_size: 12.0,
            max_height: None,
            scrollbar: ScrollbarStyle::from_theme(theme),
            query_row_extra_height: 8.0,
            no_match_text_color: theme.fg_faint,
        }
    }
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// Vertical scrollbar interaction state for scrollable menus.
#[derive(Debug, Default)]
pub(crate) struct MenuScrollbarState {
    pub(crate) dragging: bool,
    /// Scroll offset at drag start.
    pub(crate) drag_start_offset: f32,
    /// Cursor over the track/thumb hit area.
    pub(crate) track_hovered: bool,
    /// Cursor specifically over the thumb hit area.
    pub(crate) thumb_hovered: bool,
}

impl MenuScrollbarState {
    pub(crate) fn visual_state(&self) -> ScrollbarVisualState {
        if self.dragging {
            ScrollbarVisualState::Dragging
        } else if self.track_hovered || self.thumb_hovered {
            ScrollbarVisualState::Hovered
        } else {
            ScrollbarVisualState::Rest
        }
    }
}

/// Pixels per scroll wheel line.
pub(crate) const SCROLL_LINE_HEIGHT: f32 = 32.0;

/// What was pressed during a scrub/drag interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DragMode {
    /// Scrollbar thumb — update scroll offset during drag.
    ScrollbarThumb,
    /// Scrollbar track — offset was jumped on press, no ongoing drag.
    ScrollbarTrack,
    /// Menu item — select the hovered item on release.
    ItemPress,
}
