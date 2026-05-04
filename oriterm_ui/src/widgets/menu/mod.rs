//! Menu widget — a vertical list of clickable items and separators.
//!
//! Used for both context menus and dropdown popup lists. Emits
//! `WidgetAction::Selected { id, index }` when an item is activated.
//! Supports scrolling via `max_height` for long lists.
//!
//! Setting `searchable = true` (via [`MenuWidget::with_searchable`]) adds a
//! type-to-filter input row at the top. Filtering hides separators and
//! non-matching items; `Selected` continues to emit the canonical index
//! (the index into the original `entries`), not the filter-relative position.

use crate::controllers::{EventController, HoverController, ScrubController};
use crate::geometry::Point;
use crate::text::TextStyle;
use crate::widget_id::WidgetId;

mod paint;
mod style;
mod widget_impl;

pub use style::MenuStyle;
use style::{DragMode, MenuScrollbarState};

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

/// A menu widget with optional scrolling.
///
/// Displays a vertical list of items and separators. Items can be hovered
/// via mouse or navigated via keyboard arrows. Emits
/// `WidgetAction::Selected { id, index }` when activated. When `max_height`
/// is set in the style, long lists scroll with a scrollbar.
///
/// In **searchable** mode (set via [`Self::with_searchable`]), the menu
/// renders a type-to-filter input row at the top, hides separators, and
/// only displays entries whose label contains the current `query`
/// (case-insensitive substring). Hover/selection indices remain canonical
/// (indices into `entries`, not into the filter result), so `Selected`
/// emissions match the original list ordering used by the trigger.
pub struct MenuWidget {
    pub(super) id: WidgetId,
    pub(super) entries: Vec<MenuEntry>,
    /// Currently hovered (highlighted) entry — canonical index into `entries`.
    pub(super) hovered: Option<usize>,
    /// Pre-selected entry index (shown with accent tint).
    pub(super) selected_index: Option<usize>,
    pub(super) style: MenuStyle,
    /// Scroll offset in pixels from top of content.
    pub(super) scroll_offset: f32,
    /// Scrollbar hover/drag interaction state.
    pub(super) scrollbar_state: MenuScrollbarState,
    /// Event controllers (`HoverController` + `ScrubController`).
    pub(super) controllers: Vec<Box<dyn EventController>>,
    /// Press origin for computing absolute position from `total_delta`.
    pub(super) drag_origin: Option<Point>,
    /// What was pressed during the current scrub interaction.
    pub(super) drag_mode: Option<DragMode>,
    /// When `true`, render a type-to-filter input row at the top and
    /// hide entries whose label does not contain `query`.
    pub(super) searchable: bool,
    /// Lowercased filter substring. Empty string matches everything.
    pub(super) query: String,
    /// Canonical entry indices to display, in order. In searchable mode
    /// this is the filter result and excludes separators; in non-searchable
    /// mode this mirrors `0..entries.len()` and includes separators.
    pub(super) display_indices: Vec<usize>,
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
            .field("scrollbar_state", &self.scrollbar_state)
            .field("drag_mode", &self.drag_mode)
            .field("controllers", &self.controllers.len())
            .field("drag_origin", &self.drag_origin)
            .field("searchable", &self.searchable)
            .field("query", &self.query)
            .field("display_count", &self.display_indices.len())
            .finish()
    }
}

impl MenuWidget {
    /// Creates a menu widget from the given entries.
    pub fn new(entries: Vec<MenuEntry>) -> Self {
        let display_indices = (0..entries.len()).collect();
        Self {
            id: WidgetId::next(),
            entries,
            hovered: None,
            selected_index: None,
            style: MenuStyle::default(),
            scroll_offset: 0.0,
            scrollbar_state: MenuScrollbarState::default(),
            controllers: vec![
                Box::new(HoverController::new()),
                Box::new(ScrubController::new()),
            ],
            drag_origin: None,
            drag_mode: None,
            searchable: false,
            query: String::new(),
            display_indices,
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

    /// Promotes the menu to searchable mode — adds a type-to-filter input row
    /// at the top, hides separators, and filters entries by case-insensitive
    /// substring match on label. `Selected` continues to emit canonical
    /// entry indices.
    #[must_use]
    pub fn with_searchable(mut self, searchable: bool) -> Self {
        self.searchable = searchable;
        self.rebuild_display_indices();
        // Default highlight: pre-selected entry if it's still visible after
        // filtering, otherwise the first visible entry. Mirrors the popup
        // behaviour the searchable trigger had before this consolidation.
        self.hovered = self
            .selected_index
            .filter(|i| self.display_indices.contains(i))
            .or_else(|| self.display_indices.first().copied());
        self
    }

    /// Sets the initial highlighted entry — overrides the
    /// [`Self::with_searchable`] default. `index` is canonical; if it's not
    /// in the display set or is not clickable (e.g. a separator), falls
    /// back to the first clickable display entry.
    #[must_use]
    pub fn with_initial_highlight(mut self, index: usize) -> Self {
        let valid = self.display_indices.contains(&index)
            && self.entries.get(index).is_some_and(MenuEntry::is_clickable);
        if valid {
            self.hovered = Some(index);
        } else {
            self.hovered = self
                .display_indices
                .iter()
                .copied()
                .find(|&i| self.entries[i].is_clickable());
        }
        self
    }

    /// Returns the entries.
    pub fn entries(&self) -> &[MenuEntry] {
        &self.entries
    }

    /// Returns whether searchable mode is enabled.
    pub fn is_searchable(&self) -> bool {
        self.searchable
    }

    /// Returns the current filter query (lowercased substring).
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns the canonical entry indices currently in display order.
    /// In non-searchable mode this is `0..entries.len()`; in searchable
    /// mode it reflects the active filter.
    pub fn display_indices(&self) -> &[usize] {
        &self.display_indices
    }

    /// Returns the currently hovered index.
    pub fn hovered(&self) -> Option<usize> {
        self.hovered
    }

    /// Total height — search row (when searchable) + display entries + padding.
    /// In searchable mode with zero matches, reserves `style.item_height`
    /// for the "No matches" indicator so it renders inside the menu chrome.
    pub(super) fn total_height(&self) -> f32 {
        self.query_row_height() + self.entries_height() + self.style.padding_y * 2.0
    }

    /// Height of the entries region. In searchable mode with zero matches,
    /// returns `style.item_height` so `total_height` reserves space for the
    /// "No matches" indicator (drawn by `draw_no_matches_row`).
    fn entries_height(&self) -> f32 {
        if self.searchable && self.display_indices.is_empty() {
            return self.style.item_height;
        }
        self.display_indices
            .iter()
            .map(|&i| self.entry_height(i))
            .sum()
    }

    /// Height of a single entry by canonical index.
    fn entry_height(&self, index: usize) -> f32 {
        match &self.entries[index] {
            MenuEntry::Separator => self.style.separator_height,
            _ => self.style.item_height,
        }
    }

    /// Height of the search-input row when `searchable`, else 0.
    pub(super) fn query_row_height(&self) -> f32 {
        if self.searchable {
            self.style.font_size + self.style.padding_y * 2.0 + self.style.query_row_extra_height
        } else {
            0.0
        }
    }

    /// Visible height — clamped by `max_height` if set.
    pub(super) fn visible_height(&self) -> f32 {
        let total = self.total_height();
        self.style.max_height.map_or(total, |max| total.min(max))
    }

    /// Maximum scroll offset for the entries region.
    fn max_scroll(&self) -> f32 {
        (self.total_height() - self.visible_height()).max(0.0)
    }

    /// Whether the entries content overflows and scrolling is active.
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

    /// Y offset of an entry relative to the entries-region top (i.e.
    /// excluding the search row when searchable). Returns the running total
    /// if `target` is not currently displayed.
    fn entry_top_y(&self, target: usize) -> f32 {
        let mut y = 0.0;
        for &i in &self.display_indices {
            if i == target {
                return y;
            }
            y += self.entry_height(i);
        }
        y
    }

    /// Scrolls so the given canonical entry index is fully visible.
    /// No-op if the entry is not in the current display set.
    pub fn ensure_visible(&mut self, index: usize) {
        if !self.is_scrollable() {
            return;
        }
        if !self.display_indices.contains(&index) {
            return;
        }
        let item_y = self.entry_top_y(index);
        let item_h = self.entry_height(index);
        let visible_content =
            self.visible_height() - self.query_row_height() - self.style.padding_y * 2.0;

        if item_y < self.scroll_offset {
            self.scroll_offset = item_y;
        } else if item_y + item_h > self.scroll_offset + visible_content {
            self.scroll_offset = item_y + item_h - visible_content;
        } else {
            return;
        }
        self.scroll_offset = self.scroll_offset.clamp(0.0, self.max_scroll());
    }

    /// Hit-test: which canonical entry index is at Y position relative to
    /// menu top. Accounts for the search row and scroll offset.
    pub(super) fn entry_at_y(&self, y: f32) -> Option<usize> {
        let y = y - self.style.padding_y - self.query_row_height() + self.scroll_offset;
        if y < 0.0 {
            return None;
        }
        let mut offset = 0.0;
        for &i in &self.display_indices {
            let h = self.entry_height(i);
            if y < offset + h {
                return if self.entries[i].is_clickable() {
                    Some(i)
                } else {
                    None
                };
            }
            offset += h;
        }
        None
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

    /// Navigates keyboard highlight to the next/previous clickable display
    /// entry. Wraps around at list boundaries; skips non-clickable entries
    /// (separators) implicitly because they are not in `display_indices` for
    /// searchable mode and are filtered out here for the non-searchable path.
    /// Scrolls the target entry into view if the menu is scrollable.
    pub(super) fn navigate_keyboard(&mut self, forward: bool) {
        // Build the clickable subset of display_indices once. In searchable
        // mode the filter already excludes separators, so this is a copy; in
        // non-searchable mode this drops separators inline without adding
        // them to display_indices (which preserves separator rendering).
        let clickable: Vec<usize> = self
            .display_indices
            .iter()
            .copied()
            .filter(|&i| self.entries[i].is_clickable())
            .collect();
        if clickable.is_empty() {
            return;
        }

        let Some(start) = self.hovered else {
            // `clickable.is_empty()` was checked above, so first/last are
            // guaranteed Some — `.expect` documents the invariant.
            let idx = if forward {
                *clickable
                    .first()
                    .expect("clickable is non-empty (checked above)")
            } else {
                *clickable
                    .last()
                    .expect("clickable is non-empty (checked above)")
            };
            self.hovered = Some(idx);
            self.ensure_visible(idx);
            return;
        };

        // Locate `start` in the clickable list; if it has been filtered out
        // (e.g. user typed a query that excludes the current hover), restart
        // from the first clickable entry.
        let Some(pos) = clickable.iter().position(|&i| i == start) else {
            let fallback = *clickable
                .first()
                .expect("clickable is non-empty (checked above)");
            self.hovered = Some(fallback);
            self.ensure_visible(fallback);
            return;
        };

        let len = clickable.len();
        let next = if forward {
            (pos + 1) % len
        } else {
            (pos + len - 1) % len
        };
        let idx = clickable[next];
        self.hovered = Some(idx);
        self.ensure_visible(idx);
    }

    /// Recomputes `display_indices` from `entries` + `query` + `searchable`.
    /// Resets scroll to top and prunes `hovered` if it no longer matches.
    /// Mirrors `SearchableDropdownPopupWidget::rebuild_filter` semantically;
 /// the `MenuWidget` consolidation is the canonical home post-.
    pub(super) fn rebuild_display_indices(&mut self) {
        if self.searchable {
            let q = self.query.to_lowercase();
            self.display_indices = self
                .entries
                .iter()
                .enumerate()
                .filter(|(_, e)| {
                    if !e.is_clickable() {
                        return false;
                    }
                    if q.is_empty() {
                        return true;
                    }
                    e.label().is_some_and(|l| l.to_lowercase().contains(&q))
                })
                .map(|(i, _)| i)
                .collect();
        } else {
            self.display_indices = (0..self.entries.len()).collect();
        }
        self.scroll_offset = 0.0;
        // Prune stale hover after a filter rebuild.
        if self
            .hovered
            .is_some_and(|h| !self.display_indices.contains(&h))
        {
            self.hovered = self.display_indices.first().copied();
        }
    }

    /// Appends a printable character to the filter query and rebuilds the
    /// display set. No-op when not searchable or when `c` is a control char.
    pub(super) fn handle_filter_character(&mut self, c: char) {
        if !self.searchable || c.is_control() {
            return;
        }
        self.query.push(c);
        self.rebuild_display_indices();
        // Reset highlight to first match — typing always lands on the top
        // result, mirroring the legacy popup behaviour.
        self.hovered = self.display_indices.first().copied();
    }

    /// Pops the last character from the filter query and rebuilds the display
    /// set. No-op when not searchable or when the query is empty.
    pub(super) fn handle_filter_backspace(&mut self) {
        if !self.searchable {
            return;
        }
        self.query.pop();
        self.rebuild_display_indices();
        self.hovered = self.display_indices.first().copied();
    }
}

#[cfg(test)]
mod tests;
