//! Sidebar navigation widget for settings panel pages.
//!
//! Displays section titles, nav items with icons, an active-page indicator,
//! and a version label. Emits `WidgetAction::Selected` on nav item click.

use crate::action::WidgetAction;
use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::icons::{IconId, SIDEBAR_NAV_ICON_SIZE};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::{FontWeight, TextStyle, TextTransform};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// Fixed width of the sidebar (logical pixels).
pub(crate) const SIDEBAR_WIDTH: f32 = 200.0;

/// Vertical padding inside the sidebar.
const SIDEBAR_PADDING_Y: f32 = 16.0;

/// Horizontal padding inside the sidebar.
const SIDEBAR_PADDING_X: f32 = 10.0;

/// Height of a nav item row.
const ITEM_HEIGHT: f32 = 32.0;

/// Height of a section title row.
const SECTION_TITLE_HEIGHT: f32 = 28.0;

/// Width of the active indicator left border.
const INDICATOR_WIDTH: f32 = 3.0;

/// A navigation section title (e.g. "General", "Advanced").
#[derive(Debug, Clone)]
pub struct NavSection {
    /// Section title label.
    pub title: String,
    /// Items under this section.
    pub items: Vec<NavItem>,
}

/// A single navigation item.
#[derive(Debug, Clone)]
pub struct NavItem {
    /// Display label.
    pub label: String,
    /// Optional icon.
    pub icon: Option<IconId>,
    /// Page index to switch to when clicked.
    pub page_index: usize,
}

/// Index of the nav item currently hovered by the pointer (`None` if no item).
///
/// Used for instant hover background highlight. Does not use animation
/// because the dialog paint path does not schedule animation frames.
type HoveredItem = Option<usize>;

/// Sidebar navigation widget.
///
/// Renders section titles, nav items with hover/active states, icons,
/// a search field placeholder, modified-page dots, version label, and
/// config path. Click on a nav item emits `Selected { id, index }`.
pub struct SidebarNavWidget {
    id: WidgetId,
    sections: Vec<NavSection>,
    active_page: usize,
    version: String,
    config_path: String,
    /// Bitset of page indices that have unsaved modifications.
    modified_pages: u64,
    hovered_item: HoveredItem,
    style: SidebarNavStyle,
}

/// Visual style for the sidebar nav.
#[derive(Debug, Clone)]
pub struct SidebarNavStyle {
    /// Sidebar background.
    pub bg: Color,
    /// Section title text color.
    pub section_title_fg: Color,
    /// Normal item text color.
    pub item_fg: Color,
    /// Active item text color.
    pub active_fg: Color,
    /// Active item background.
    pub active_bg: Color,
    /// Hover item background.
    pub hover_bg: Color,
    /// Hover item text color.
    pub hover_fg: Color,
    /// Version label text color.
    pub version_fg: Color,
    /// Border color on right edge.
    pub border: Color,
}

impl SidebarNavStyle {
    /// Derives style from theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            bg: theme.bg_secondary,
            section_title_fg: theme.fg_faint,
            item_fg: theme.fg_secondary,
            active_fg: theme.accent,
            active_bg: theme.accent_bg_strong,
            hover_bg: theme.bg_hover,
            hover_fg: theme.fg_primary,
            version_fg: theme.fg_faint,
            border: theme.border,
        }
    }
}

impl SidebarNavWidget {
    /// Creates a new sidebar nav with the given sections.
    pub fn new(sections: Vec<NavSection>, theme: &UiTheme) -> Self {
        let style = SidebarNavStyle::from_theme(theme);
        Self {
            id: WidgetId::next(),
            sections,
            active_page: 0,
            hovered_item: None,
            version: String::new(),
            config_path: String::new(),
            modified_pages: 0,
            style,
        }
    }

    /// Sets the active page index.
    pub fn set_active_page(&mut self, index: usize) {
        self.active_page = index;
    }

    /// Returns the active page index.
    pub fn active_page(&self) -> usize {
        self.active_page
    }

    /// Sets the version label text.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Sets the config file path shown at the sidebar bottom.
    #[must_use]
    pub fn with_config_path(mut self, path: impl Into<String>) -> Self {
        self.config_path = path.into();
        self
    }

    /// Marks a page index as having unsaved modifications (shows warning dot).
    pub fn set_page_modified(&mut self, page_index: usize, modified: bool) {
        if modified {
            self.modified_pages |= 1 << page_index;
        } else {
            self.modified_pages &= !(1 << page_index);
        }
    }

    /// Returns whether a page has unsaved modifications.
    fn is_page_modified(&self, page_index: usize) -> bool {
        self.modified_pages & (1 << page_index) != 0
    }

    /// Returns the total number of nav items across all sections.
    fn total_item_count(&self) -> usize {
        self.sections.iter().map(|s| s.items.len()).sum()
    }

    /// Returns the `page_index` for a flat item index.
    fn page_for_flat_index(&self, flat_idx: usize) -> Option<usize> {
        self.sections
            .iter()
            .flat_map(|s| s.items.iter())
            .nth(flat_idx)
            .map(|item| item.page_index)
    }

    /// Paints a single nav item row at the given bounds.
    fn paint_nav_item(
        &self,
        ctx: &mut DrawCtx<'_>,
        item: &NavItem,
        item_rect: Rect,
        flat_idx: usize,
    ) {
        let is_active = item.page_index == self.active_page;
        let x = item_rect.x();
        let y = item_rect.y();
        let item_w = item_rect.width();

        // Active indicator (3px left border).
        if is_active {
            let indicator = Rect::new(x, y, INDICATOR_WIDTH, ITEM_HEIGHT);
            ctx.scene
                .push_quad(indicator, RectStyle::filled(self.style.active_fg));
        }

        // Background (inset past indicator for all items).
        let bg_x = x + INDICATOR_WIDTH;
        let bg_w = item_w - INDICATOR_WIDTH;
        let bg = if is_active {
            self.style.active_bg
        } else if self.hovered_item == Some(flat_idx) {
            self.style.hover_bg
        } else {
            Color::TRANSPARENT
        };
        if bg.a > 0.001 {
            let bg_rect = Rect::new(bg_x, y, bg_w, ITEM_HEIGHT);
            ctx.scene.push_quad(bg_rect, RectStyle::filled(bg));
        }

        // Icon (offset by indicator width).
        let text_x = if let Some(icon_id) = item.icon {
            let icon_size = SIDEBAR_NAV_ICON_SIZE;
            let icon_y = y + (ITEM_HEIGHT - icon_size as f32) / 2.0;
            if let Some(icons) = ctx.icons {
                if let Some(resolved) = icons.get(icon_id, icon_size) {
                    let c = if is_active {
                        self.style.active_fg
                    } else if self.hovered_item == Some(flat_idx) {
                        self.style.hover_fg.with_alpha(0.7)
                    } else {
                        self.style.item_fg.with_alpha(0.7)
                    };
                    ctx.scene.push_icon(
                        Rect::new(
                            x + INDICATOR_WIDTH + 8.0,
                            icon_y,
                            icon_size as f32,
                            icon_size as f32,
                        ),
                        resolved.atlas_page,
                        resolved.uv,
                        c,
                    );
                }
            }
            x + INDICATOR_WIDTH + 32.0
        } else {
            x + INDICATOR_WIDTH + 8.0
        };

        // Label.
        let fg = if is_active {
            self.style.active_fg
        } else if self.hovered_item == Some(flat_idx) {
            self.style.hover_fg
        } else {
            self.style.item_fg
        };
        let style = TextStyle {
            size: 13.0,
            ..TextStyle::default()
        };
        let label_y = y + (ITEM_HEIGHT - 13.0) / 2.0;
        let shaped = ctx.measurer.shape(&item.label, &style, item_w);
        ctx.scene.push_text(Point::new(text_x, label_y), shaped, fg);

        // Modified dot (6px square, warning color, right-aligned).
        if self.is_page_modified(item.page_index) {
            let dot_size = 6.0;
            let dot_x = item_rect.right() - 16.0;
            let dot_y = y + (ITEM_HEIGHT - dot_size) / 2.0;
            let dot_rect = Rect::new(dot_x, dot_y, dot_size, dot_size);
            ctx.scene
                .push_quad(dot_rect, RectStyle::filled(ctx.theme.warning));
        }
    }

    /// Paints the sidebar footer: version label + config path.
    fn paint_footer(&self, ctx: &mut DrawCtx<'_>, x: f32, item_w: f32) {
        let mut y = ctx.bounds.bottom() - 8.0;

        // Config path (bottom-most, faint + smaller).
        if !self.config_path.is_empty() {
            let style = TextStyle {
                size: 10.0,
                ..TextStyle::default()
            };
            let shaped = ctx.measurer.shape(&self.config_path, &style, item_w);
            y -= shaped.height;
            let fg = self.style.version_fg.with_alpha(0.7);
            ctx.scene.push_text(Point::new(x + 6.0, y), shaped, fg);
            y -= 4.0;
        }

        // Version label.
        if !self.version.is_empty() {
            let style = TextStyle {
                size: 11.0,
                ..TextStyle::default()
            };
            let shaped = ctx.measurer.shape(&self.version, &style, item_w);
            y -= shaped.height;
            ctx.scene
                .push_text(Point::new(x + 6.0, y), shaped, self.style.version_fg);
        }
    }

    /// Paints the search field placeholder at the top of the sidebar.
    fn paint_search_field(&self, ctx: &mut DrawCtx<'_>, x: f32, y: f32, w: f32) {
        let _ = &self; // Search will filter items when wired.
        let field_h = 28.0;
        let field_rect = Rect::new(x, y, w, field_h);
        let bg_style = RectStyle::filled(ctx.theme.bg_primary).with_border(2.0, ctx.theme.border);
        ctx.scene.push_quad(field_rect, bg_style);

        // Placeholder text.
        let style = TextStyle {
            size: 12.0,
            ..TextStyle::default()
        };
        let shaped = ctx.measurer.shape("Search settings...", &style, w - 32.0);
        let text_y = y + (field_h - shaped.height) / 2.0;
        ctx.scene
            .push_text(Point::new(x + 26.0, text_y), shaped, ctx.theme.fg_faint);
    }

    /// Height of the search field area (field + margin).
    const SEARCH_AREA_HEIGHT: f32 = 40.0;

    /// Hit-tests a local Y coordinate to a flat item index.
    fn hit_test_item(&self, local_y: f32) -> Option<usize> {
        // Skip search field area.
        let mut y = Self::SEARCH_AREA_HEIGHT;
        let mut flat_idx = 0;
        for section in &self.sections {
            y += SECTION_TITLE_HEIGHT;
            for _ in &section.items {
                if local_y >= y && local_y < y + ITEM_HEIGHT {
                    return Some(flat_idx);
                }
                y += ITEM_HEIGHT;
                flat_idx += 1;
            }
        }
        None
    }
}

impl std::fmt::Debug for SidebarNavWidget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SidebarNavWidget")
            .field("id", &self.id)
            .field("active_page", &self.active_page)
            .field("section_count", &self.sections.len())
            .field("hovered_item", &self.hovered_item)
            .finish_non_exhaustive()
    }
}

impl Widget for SidebarNavWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(SIDEBAR_WIDTH, 0.0)
            .with_width(crate::layout::SizeSpec::Fixed(SIDEBAR_WIDTH))
            .with_height(crate::layout::SizeSpec::Fill)
            .with_widget_id(self.id)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;

        // Background + right border.
        ctx.scene
            .push_quad(bounds, RectStyle::filled(self.style.bg));
        let border_rect = Rect::new(
            bounds.x() + bounds.width() - 2.0,
            bounds.y(),
            2.0,
            bounds.height(),
        );
        ctx.scene
            .push_quad(border_rect, RectStyle::filled(self.style.border));

        let x = bounds.x() + SIDEBAR_PADDING_X;
        let item_w = bounds.width() - SIDEBAR_PADDING_X * 2.0;
        let mut y = bounds.y() + SIDEBAR_PADDING_Y;

        // Search field at top.
        self.paint_search_field(ctx, x, y, item_w);
        y += 28.0 + 12.0; // field height + margin

        let mut flat_idx = 0;
        for section in &self.sections {
            // Section title — uppercase with wide letter spacing.
            let title_style = TextStyle {
                size: 10.0,
                weight: FontWeight::NORMAL,
                letter_spacing: 1.5,
                text_transform: TextTransform::Uppercase,
                ..TextStyle::default()
            };
            let title_text = format!("// {}", section.title);
            let shaped = ctx.measurer.shape(&title_text, &title_style, item_w);
            ctx.scene
                .push_text(Point::new(x + 6.0, y), shaped, self.style.section_title_fg);
            y += SECTION_TITLE_HEIGHT;

            for item in &section.items {
                let item_rect = Rect::new(x, y, item_w, ITEM_HEIGHT);
                self.paint_nav_item(ctx, item, item_rect, flat_idx);
                y += ITEM_HEIGHT;
                flat_idx += 1;
            }
        }

        self.paint_footer(ctx, x, item_w);
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        if let WidgetAction::Selected { id, index } = action {
            // Only react to our own nav item selections, not external
            // Selected actions from other widgets (SchemeCard, CursorPicker).
            if *id == self.id && *index != self.active_page {
                self.set_active_page(*index);
                return true;
            }
        }
        false
    }

    fn on_input(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> super::OnInputResult {
        use crate::input::{InputEvent, Key};

        match event {
            // Track which item is hovered for instant per-item highlight.
            InputEvent::MouseMove { pos, .. } => {
                let local_y = pos.y - bounds.y() - SIDEBAR_PADDING_Y;
                self.hovered_item = self.hit_test_item(local_y);
                super::OnInputResult::handled()
            }
            // Route click events to nav items by position.
            InputEvent::MouseDown { pos, .. } => {
                let local_y = pos.y - bounds.y() - SIDEBAR_PADDING_Y;
                if let Some(flat_idx) = self.hit_test_item(local_y) {
                    if let Some(page_idx) = self.page_for_flat_index(flat_idx) {
                        return super::OnInputResult::handled().with_action(
                            WidgetAction::Selected {
                                id: self.id,
                                index: page_idx,
                            },
                        );
                    }
                }
                super::OnInputResult::ignored()
            }
            // Arrow keys switch the active nav item.
            InputEvent::KeyDown { key, .. } => {
                let total = self.total_item_count();
                if total == 0 {
                    return super::OnInputResult::ignored();
                }
                let new_page = match key {
                    Key::ArrowUp if self.active_page > 0 => Some(self.active_page - 1),
                    Key::ArrowDown if self.active_page + 1 < total => Some(self.active_page + 1),
                    Key::Home => Some(0),
                    Key::End => Some(total - 1),
                    _ => None,
                };
                if let Some(page_idx) = new_page {
                    super::OnInputResult::handled().with_action(WidgetAction::Selected {
                        id: self.id,
                        index: page_idx,
                    })
                } else {
                    super::OnInputResult::ignored()
                }
            }
            _ => super::OnInputResult::ignored(),
        }
    }

    fn lifecycle(
        &mut self,
        event: &crate::interaction::LifecycleEvent,
        _ctx: &mut crate::widgets::LifecycleCtx<'_>,
    ) {
        use crate::interaction::LifecycleEvent;
        // When the cursor leaves the sidebar entirely, clear item hover.
        if matches!(event, LifecycleEvent::HotChanged { is_hot: false, .. }) {
            self.hovered_item = None;
        }
    }
}

#[cfg(test)]
mod tests;
