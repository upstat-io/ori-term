//! Searchable-dropdown popup widget.
//!
//! Mounted by the App layer's overlay manager in response to
//! `WidgetAction::OpenSearchableDropdown`. Owns the ephemeral filter state
//! (`query`, `filtered_indices`, `highlighted`) plus a copy of `items` and
//! the trigger id for routing the selection action back. On selection emits
//! `WidgetAction::Selected { id: trigger_id, index: canonical_index }`,
//! where `canonical_index` is the index INTO `items` (NOT into the filtered
//! view).

use winit::window::CursorIcon;

use crate::color::Color;
use crate::controllers::{ControllerRequests, EventController};
use crate::draw::RectStyle;
use crate::geometry::{Insets, Point, Rect};
use crate::input::{InputEvent, Key, MouseButton};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::super::{DrawCtx, LayoutCtx, OnInputResult, Widget, WidgetAction};

/// Visual style for [`SearchableDropdownPopupWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct SearchableDropdownStyle {
    /// Popup background.
    pub bg: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Padding inside the popup frame.
    pub padding: Insets,
    /// Search-row background (slightly distinct from popup bg).
    pub query_bg: Color,
    /// Search-row text color.
    pub query_text_color: Color,
    /// Filter-row text color (un-highlighted).
    pub item_fg: Color,
    /// Highlighted-row background.
    pub highlight_bg: Color,
    /// Highlighted-row text color.
    pub highlight_fg: Color,
    /// Color used for the "no matches" indicator.
    pub no_match_text_color: Color,
    /// Font size for the query line and items.
    pub font_size: f32,
    /// Per-row vertical padding.
    pub row_padding_y: f32,
    /// Maximum visible rows before the popup scrolls. v1 does not scroll —
    /// the popup grows to fit. Track count is bounded externally by the
    /// settings dialog viewport.
    pub max_visible_rows: usize,
    /// Minimum width.
    pub min_width: f32,
}

impl SearchableDropdownStyle {
    /// Derive a style from the UI theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            bg: theme.bg_secondary,
            border_color: theme.border,
            border_width: 1.0,
            corner_radius: theme.corner_radius,
            padding: Insets::tlbr(6.0, 6.0, 6.0, 6.0),
            query_bg: theme.bg_input,
            query_text_color: theme.fg_primary,
            item_fg: theme.fg_primary,
            highlight_bg: theme.accent.with_alpha(0.18),
            highlight_fg: theme.fg_primary,
            no_match_text_color: theme.fg_faint,
            font_size: 12.0,
            row_padding_y: 4.0,
            max_visible_rows: 12,
            min_width: 180.0,
        }
    }
}

impl Default for SearchableDropdownStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// Filterable popup that pairs with [`super::SearchableDropdownWidget`].
pub struct SearchableDropdownPopupWidget {
    id: WidgetId,
    /// Routing target for emitted `Selected` actions.
    trigger_id: WidgetId,
    /// Canonical item list copied from the trigger at mount time.
    items: Vec<String>,
    /// Current filter query (lowercased substring match).
    query: String,
    /// Canonical-index references in filtered order.
    filtered_indices: Vec<usize>,
    /// Index INTO `filtered_indices` for the highlighted row (NOT canonical).
    highlighted: Option<usize>,
    /// First row drawn in the visible window. Adjusted by `move_highlight`
    /// to keep the highlighted row inside `[scroll_offset, scroll_offset +
    /// max_visible_rows)` so arrow-key navigation never targets unrendered
    /// rows (closes Round 1 codex F2 GAP per
    /// `bug-tracker/plans/BUG-02-012/section-06-tpr-findings.md`).
    scroll_offset: usize,
    /// Visual style.
    style: SearchableDropdownStyle,
    /// No event controllers — popup handles input directly via `on_input`.
    controllers: Vec<Box<dyn EventController>>,
}

impl SearchableDropdownPopupWidget {
    /// Constructs a popup with the given items + initial state.
    pub fn new(
        trigger_id: WidgetId,
        items: Vec<String>,
        selected: Option<usize>,
        initial_highlight: Option<usize>,
        style: SearchableDropdownStyle,
    ) -> Self {
        let filtered_indices: Vec<usize> = (0..items.len()).collect();
        // initial_highlight wins over selected; if neither, highlight first item.
        let highlight_canonical = initial_highlight.or(selected);
        let highlighted = highlight_canonical
            .and_then(|c| filtered_indices.iter().position(|&i| i == c))
            .or_else(|| (!filtered_indices.is_empty()).then_some(0));
        let scroll_offset = scroll_to_keep_visible(highlighted, 0, style.max_visible_rows);
        Self {
            id: WidgetId::next(),
            trigger_id,
            items,
            query: String::new(),
            filtered_indices,
            highlighted,
            scroll_offset,
            style,
            controllers: Vec::new(),
        }
    }

    /// Returns the widget id of the trigger that opened this popup.
    pub fn trigger_id(&self) -> WidgetId {
        self.trigger_id
    }

    /// Returns the current filter query.
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Returns the filtered canonical-index list.
    pub fn filtered_indices(&self) -> &[usize] {
        &self.filtered_indices
    }

    /// Returns the highlighted index (into `filtered_indices`).
    pub fn highlighted(&self) -> Option<usize> {
        self.highlighted
    }

    /// Rebuilds `filtered_indices` from `query` and resets highlight + scroll.
    /// Public so tests can assert filter behavior in isolation.
    pub fn rebuild_filter(&mut self) {
        let q = self.query.to_lowercase();
        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| q.is_empty() || item.to_lowercase().contains(&q))
            .map(|(i, _)| i)
            .collect();
        self.highlighted = if self.filtered_indices.is_empty() {
            None
        } else {
            Some(0)
        };
        self.scroll_offset = 0;
    }

    /// Returns the first visible row index — used by paint and click-routing.
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    fn row_height(&self) -> f32 {
        self.style.font_size + self.style.row_padding_y * 2.0
    }

    fn query_row_height(&self) -> f32 {
        self.style.font_size + self.style.row_padding_y * 2.0 + 4.0
    }

    fn rows_drawn(&self) -> usize {
        if self.filtered_indices.is_empty() {
            1 // "no matches" row
        } else {
            self.filtered_indices.len().min(self.style.max_visible_rows)
        }
    }

    fn handle_character(&mut self, c: char) -> Option<WidgetAction> {
        if c.is_control() {
            return None;
        }
        self.query.push(c);
        self.rebuild_filter();
        None
    }

    fn handle_backspace(&mut self) {
        self.query.pop();
        self.rebuild_filter();
    }

    fn move_highlight(&mut self, delta: i32) {
        if self.filtered_indices.is_empty() {
            self.highlighted = None;
            self.scroll_offset = 0;
            return;
        }
        let len = self.filtered_indices.len() as i32;
        let cur = self.highlighted.map_or(0, |h| h as i32);
        let next = ((cur + delta) % len + len) % len;
        let next_usize = next as usize;
        self.highlighted = Some(next_usize);
        // Adjust scroll_offset so the highlighted row stays inside the
        // visible window. Required by codex F2 — without this the highlight
        // can move into rows the popup never renders.
        self.scroll_offset = scroll_to_keep_visible(
            self.highlighted,
            self.scroll_offset,
            self.style.max_visible_rows,
        );
    }

    fn select_highlighted(&self) -> Option<WidgetAction> {
        let h = self.highlighted?;
        let canonical = *self.filtered_indices.get(h)?;
        Some(WidgetAction::Selected {
            id: self.trigger_id,
            index: canonical,
        })
    }

    fn pointer_to_row(&self, pos: Point, bounds: Rect) -> Option<usize> {
        let s = &self.style;
        let inner = bounds.inset(s.padding);
        let list_top = inner.y() + self.query_row_height() + 2.0;
        if pos.y < list_top {
            return None;
        }
        let row_h = self.row_height();
        let visible_idx = ((pos.y - list_top) / row_h) as usize;
        // Translate visible-window index back to filtered-index space.
        let row_idx = self.scroll_offset + visible_idx;
        if row_idx < self.filtered_indices.len() && visible_idx < self.style.max_visible_rows {
            Some(row_idx)
        } else {
            None
        }
    }
}

/// Compute the new scroll offset that keeps `highlighted` inside the visible
/// window `[scroll_offset, scroll_offset + max_visible_rows)`. Standard
/// scroll-to-show-cursor pattern; pulled out as a free function so the
/// constructor and `move_highlight` both call the same logic.
fn scroll_to_keep_visible(
    highlighted: Option<usize>,
    current_offset: usize,
    max_visible_rows: usize,
) -> usize {
    let Some(h) = highlighted else {
        return 0;
    };
    if max_visible_rows == 0 {
        return 0;
    }
    if h < current_offset {
        h
    } else if h >= current_offset + max_visible_rows {
        h + 1 - max_visible_rows
    } else {
        current_offset
    }
}

impl std::fmt::Debug for SearchableDropdownPopupWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SearchableDropdownPopupWidget")
            .field("id", &self.id)
            .field("trigger_id", &self.trigger_id)
            .field("items_len", &self.items.len())
            .field("query", &self.query)
            .field("filtered_count", &self.filtered_indices.len())
            .field("highlighted", &self.highlighted)
            .field("scroll_offset", &self.scroll_offset)
            .field("style", &self.style)
            .field("controllers_len", &self.controllers.len())
            .finish()
    }
}

impl Widget for SearchableDropdownPopupWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = TextStyle::new(self.style.font_size, self.style.item_fg);
        let max_w = self
            .items
            .iter()
            .map(|item| ctx.measurer.measure(item, &style, f32::INFINITY).width)
            .fold(0.0_f32, f32::max);
        let w = (max_w + self.style.padding.width() + 16.0).max(self.style.min_width);
        let h = self.style.padding.height()
            + self.query_row_height()
            + 2.0
            + self.row_height() * self.rows_drawn() as f32;
        LayoutBox::leaf(w, h)
            .with_widget_id(self.id)
            .with_cursor_icon(CursorIcon::Pointer)
    }

    fn controllers(&self) -> &[Box<dyn EventController>] {
        &self.controllers
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn EventController>] {
        &mut self.controllers
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;
        let s = &self.style;

        // Outer frame.
        let frame_style = RectStyle::filled(s.bg)
            .with_border(s.border_width, s.border_color)
            .with_radius(s.corner_radius);
        ctx.scene.push_quad(bounds, frame_style);

        let inner = bounds.inset(s.padding);

        // Search row.
        let query_row = Rect::new(inner.x(), inner.y(), inner.width(), self.query_row_height());
        let query_row_style = RectStyle::filled(s.query_bg).with_radius(s.corner_radius * 0.5);
        ctx.scene.push_quad(query_row, query_row_style);

        let text_style = TextStyle::new(s.font_size, s.query_text_color);
        let display_query = if self.query.is_empty() {
            "Search…"
        } else {
            self.query.as_str()
        };
        let query_color = if self.query.is_empty() {
            s.no_match_text_color
        } else {
            s.query_text_color
        };
        let shaped = ctx
            .measurer
            .shape(display_query, &text_style, query_row.width() - 12.0);
        let qy = query_row.y() + (query_row.height() - shaped.height) / 2.0;
        ctx.scene
            .push_text(Point::new(query_row.x() + 6.0, qy), shaped, query_color);

        // Filtered list.
        let list_top = inner.y() + self.query_row_height() + 2.0;
        if self.filtered_indices.is_empty() {
            let nm_style = TextStyle::new(s.font_size, s.no_match_text_color);
            let shaped = ctx.measurer.shape("No matches", &nm_style, inner.width());
            let ny = list_top + (self.row_height() - shaped.height) / 2.0;
            ctx.scene.push_text(
                Point::new(inner.x() + 6.0, ny),
                shaped,
                s.no_match_text_color,
            );
            return;
        }

        let row_h = self.row_height();
        for (visible_i, &canonical) in self
            .filtered_indices
            .iter()
            .skip(self.scroll_offset)
            .take(s.max_visible_rows)
            .enumerate()
        {
            // `visible_i` is the screen-space slot (0..max_visible_rows);
            // `filter_i` is the index into `filtered_indices` (canonical-
            // index space). Highlight comparison uses filter_i so the right
            // row in the visible window draws the highlight even when
            // scrolled past the top.
            let filter_i = self.scroll_offset + visible_i;
            let row_y = list_top + (visible_i as f32) * row_h;
            let row_rect = Rect::new(inner.x(), row_y, inner.width(), row_h);
            let is_highlighted = self.highlighted == Some(filter_i);
            if is_highlighted {
                let hl_style = RectStyle::filled(s.highlight_bg).with_radius(s.corner_radius * 0.5);
                ctx.scene.push_quad(row_rect, hl_style);
            }
            let label = self.items.get(canonical).map_or("", String::as_str);
            let fg = if is_highlighted {
                s.highlight_fg
            } else {
                s.item_fg
            };
            let row_text_style = TextStyle::new(s.font_size, fg);
            let shaped = ctx
                .measurer
                .shape(label, &row_text_style, row_rect.width() - 12.0);
            let ty = row_rect.y() + (row_rect.height() - shaped.height) / 2.0;
            ctx.scene
                .push_text(Point::new(row_rect.x() + 6.0, ty), shaped, fg);
        }
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        match *event {
            InputEvent::KeyDown { key, .. } => match key {
                Key::Character(c) => match self.handle_character(c) {
                    Some(action) => OnInputResult::handled().with_action(action),
                    None => OnInputResult::handled(),
                },
                Key::Space => match self.handle_character(' ') {
                    Some(action) => OnInputResult::handled().with_action(action),
                    None => OnInputResult::handled(),
                },
                Key::Backspace => {
                    self.handle_backspace();
                    OnInputResult::handled()
                }
                Key::ArrowDown => {
                    self.move_highlight(1);
                    OnInputResult::handled()
                }
                Key::ArrowUp => {
                    self.move_highlight(-1);
                    OnInputResult::handled()
                }
                Key::Enter => match self.select_highlighted() {
                    Some(action) => OnInputResult::handled().with_action(action),
                    None => OnInputResult::handled(),
                },
                // Escape, Tab, Home/End/PageUp/PageDown — pass through to overlay manager.
                _ => OnInputResult::ignored(),
            },
            InputEvent::MouseDown {
                pos,
                button: MouseButton::Left,
                ..
            } => {
                if let Some(row_idx) = self.pointer_to_row(pos, bounds) {
                    self.highlighted = Some(row_idx);
                    if let Some(action) = self.select_highlighted() {
                        let mut result = OnInputResult::handled().with_action(action);
                        result.requests.insert(ControllerRequests::SET_ACTIVE);
                        return result;
                    }
                }
                OnInputResult::handled()
            }
            _ => OnInputResult::ignored(),
        }
    }
}

#[cfg(test)]
mod tests;
