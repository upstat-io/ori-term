//! Sidebar navigation widget for settings panel pages.
//!
//! Displays section titles, nav items with icons, an active-page indicator,
//! and a version label. Emits `WidgetAction::Selected` on nav item click.

use crate::action::WidgetAction;
use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::icons::IconId;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::{FontWeight, TextStyle};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// Fixed width of the sidebar (logical pixels).
const SIDEBAR_WIDTH: f32 = 200.0;

/// Vertical padding inside the sidebar.
const SIDEBAR_PADDING_Y: f32 = 16.0;

/// Horizontal padding inside the sidebar.
const SIDEBAR_PADDING_X: f32 = 10.0;

/// Height of a nav item row.
const ITEM_HEIGHT: f32 = 32.0;

/// Height of a section title row.
const SECTION_TITLE_HEIGHT: f32 = 28.0;

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
/// and a version label. Click on a nav item emits `Selected { id, index }`.
pub struct SidebarNavWidget {
    id: WidgetId,
    sections: Vec<NavSection>,
    active_page: usize,
    version: String,
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
            item_fg: theme.fg_primary,
            active_fg: theme.accent,
            active_bg: theme.accent_bg_strong,
            hover_bg: theme.bg_hover,
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

        // Background.
        let bg = if is_active {
            self.style.active_bg
        } else if self.hovered_item == Some(flat_idx) {
            log::info!(
                "sidebar paint: item {flat_idx} hovered, bg={:?}",
                self.style.hover_bg
            );
            self.style.hover_bg
        } else {
            Color::TRANSPARENT
        };
        if bg.a > 0.001 {
            ctx.draw_list.push_rect(item_rect, RectStyle::filled(bg));
        }

        // Icon.
        let text_x = if let Some(icon_id) = item.icon {
            let icon_size = 16_u32;
            let icon_y = y + (ITEM_HEIGHT - icon_size as f32) / 2.0;
            if let Some(icons) = ctx.icons {
                if let Some(resolved) = icons.get(icon_id, icon_size) {
                    let c = if is_active {
                        self.style.active_fg
                    } else {
                        self.style.item_fg.with_alpha(0.6)
                    };
                    ctx.draw_list.push_icon(
                        Rect::new(x + 8.0, icon_y, icon_size as f32, icon_size as f32),
                        resolved.atlas_page,
                        resolved.uv,
                        c,
                    );
                }
            }
            x + 32.0
        } else {
            x + 8.0
        };

        // Label.
        let fg = if is_active {
            self.style.active_fg
        } else {
            self.style.item_fg
        };
        let style = TextStyle {
            size: 13.0,
            ..TextStyle::default()
        };
        let label_y = y + (ITEM_HEIGHT - 13.0) / 2.0;
        let shaped = ctx.measurer.shape(&item.label, &style, item_w);
        ctx.draw_list
            .push_text(Point::new(text_x, label_y), shaped, fg);
    }

    /// Paints the version label at the bottom of the sidebar.
    fn paint_version_label(&self, ctx: &mut DrawCtx<'_>, x: f32, item_w: f32) {
        if self.version.is_empty() {
            return;
        }
        let style = TextStyle {
            size: 10.0,
            ..TextStyle::default()
        };
        let y = ctx.bounds.y() + ctx.bounds.height() - 24.0;
        let shaped = ctx.measurer.shape(&self.version, &style, item_w);
        ctx.draw_list
            .push_text(Point::new(x + 6.0, y), shaped, self.style.version_fg);
    }

    /// Hit-tests a local Y coordinate to a flat item index.
    fn hit_test_item(&self, local_y: f32) -> Option<usize> {
        let mut y = 0.0;
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
        ctx.draw_list
            .push_rect(bounds, RectStyle::filled(self.style.bg));
        let border_rect = Rect::new(
            bounds.x() + bounds.width() - 1.0,
            bounds.y(),
            1.0,
            bounds.height(),
        );
        ctx.draw_list
            .push_rect(border_rect, RectStyle::filled(self.style.border));

        let mut y = bounds.y() + SIDEBAR_PADDING_Y;
        let x = bounds.x() + SIDEBAR_PADDING_X;
        let item_w = bounds.width() - SIDEBAR_PADDING_X * 2.0;
        let mut flat_idx = 0;

        for section in &self.sections {
            // Section title.
            let title_style = TextStyle {
                size: 10.0,
                weight: FontWeight::Bold,
                ..TextStyle::default()
            };
            let title_text = section.title.to_uppercase();
            let shaped = ctx.measurer.shape(&title_text, &title_style, item_w);
            ctx.draw_list
                .push_text(Point::new(x + 6.0, y), shaped, self.style.section_title_fg);
            y += SECTION_TITLE_HEIGHT;

            for item in &section.items {
                let item_rect = Rect::new(x, y, item_w, ITEM_HEIGHT);
                self.paint_nav_item(ctx, item, item_rect, flat_idx);
                y += ITEM_HEIGHT;
                flat_idx += 1;
            }
        }

        self.paint_version_label(ctx, x, item_w);
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        if let WidgetAction::Selected { index, .. } = action {
            if *index != self.active_page {
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
                let prev = self.hovered_item;
                self.hovered_item = self.hit_test_item(local_y);
                if self.hovered_item != prev {
                    log::info!(
                        "sidebar hover: pos=({:.0},{:.0}) bounds=({:.0},{:.0},{:.0},{:.0}) local_y={:.0} item={:?}",
                        pos.x,
                        pos.y,
                        bounds.x(),
                        bounds.y(),
                        bounds.width(),
                        bounds.height(),
                        local_y,
                        self.hovered_item,
                    );
                }
                super::OnInputResult::handled()
            }
            // Route click events to nav items by position.
            InputEvent::MouseDown { pos, .. } => {
                let local_y = pos.y - bounds.y() - SIDEBAR_PADDING_Y;
                log::info!(
                    "sidebar click: pos=({:.0},{:.0}) bounds=({:.0},{:.0},{:.0},{:.0}) local_y={:.0}",
                    pos.x,
                    pos.y,
                    bounds.x(),
                    bounds.y(),
                    bounds.width(),
                    bounds.height(),
                    local_y,
                );
                if let Some(flat_idx) = self.hit_test_item(local_y) {
                    if let Some(page_idx) = self.page_for_flat_index(flat_idx) {
                        log::info!("sidebar click: flat_idx={flat_idx} page_idx={page_idx}");
                        return super::OnInputResult::handled().with_action(
                            WidgetAction::Selected {
                                id: self.id,
                                index: page_idx,
                            },
                        );
                    }
                }
                log::info!("sidebar click: no item hit");
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
