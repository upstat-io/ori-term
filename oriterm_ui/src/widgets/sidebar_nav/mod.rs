//! Sidebar navigation widget for settings panel pages.
//!
//! Displays section titles, nav items with icons, an active-page indicator,
//! and a version label. Emits `WidgetAction::Selected` on nav item click.

mod geometry;
mod input;
mod paint;

use std::cell::{Cell, RefCell};

use winit::window::CursorIcon;

use crate::action::WidgetAction;
use crate::color::Color;
use crate::geometry::Rect;
use crate::icons::IconId;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::editing::TextEditingState;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget};

/// Fixed width of the sidebar (logical pixels).
pub(crate) const SIDEBAR_WIDTH: f32 = 200.0;

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

/// Which footer target is currently hovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HoveredFooterTarget {
    /// The "Update Available" link.
    UpdateLink,
    /// The config file path row.
    ConfigPath,
}

/// Target for footer click actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FooterTarget {
    /// The "Update Available" link was clicked, carrying the URL to open (if any).
    UpdateLink(Option<String>),
    /// The config file path row was clicked.
    ConfigPath,
}

/// Cached footer layout computed during paint for hit testing.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct FooterRects {
    /// The update link rect (if visible).
    pub update_link: Option<Rect>,
    /// The config path row rect.
    pub config_path: Option<Rect>,
}

/// Sidebar navigation widget.
///
/// Renders section titles, nav items with hover/active states, icons,
/// a search field, modified-page dots, version label, footer metadata,
/// and config path. Click on a nav item emits `Selected { id, index }`.
pub struct SidebarNavWidget {
    pub(super) id: WidgetId,
    pub(super) sections: Vec<NavSection>,
    pub(super) active_page: usize,
    pub(super) version: String,
    pub(super) config_path: String,
    /// Bitset of page indices that have unsaved modifications.
    pub(super) modified_pages: u64,
    pub(super) hovered_item: HoveredItem,
    pub(super) style: SidebarNavStyle,
    /// Internal search field editing state.
    pub(super) search_state: TextEditingState,
    /// Whether the search field currently has keyboard focus.
    pub(super) search_focused: bool,
    /// Which footer target is currently hovered (if any).
    pub(super) hovered_footer: Option<HoveredFooterTarget>,
    /// Optional "Update Available" link label (e.g. "Update Available").
    pub(super) update_label: Option<String>,
    /// Optional tooltip for the update link (e.g. version number).
    pub(super) update_tooltip: Option<String>,
    /// Optional URL to open when the update link is clicked.
    pub(super) update_url: Option<String>,
    /// Footer rects cached from last paint for hit testing (interior mutability for `&self` paint).
    pub(super) footer_rects: Cell<FooterRects>,
    /// Cached character boundary X-offsets from last paint, for click-to-cursor mapping.
    ///
    /// Each entry is `(byte_position, x_offset)`. Populated during `paint_search_field()`
    /// and read during `position_cursor_at_x()`. Uses `RefCell` for interior mutability
    /// since paint takes `&self`.
    pub(super) search_char_offsets: RefCell<Vec<(usize, f32)>>,
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
            search_state: TextEditingState::new(),
            search_focused: false,
            hovered_footer: None,
            update_label: None,
            update_tooltip: None,
            update_url: None,
            footer_rects: Cell::new(FooterRects::default()),
            search_char_offsets: RefCell::new(Vec::new()),
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

    /// Sets optional "Update Available" footer metadata including a URL to open on click.
    #[must_use]
    pub fn with_update_available(
        mut self,
        label: impl Into<String>,
        tooltip: impl Into<String>,
        url: impl Into<String>,
    ) -> Self {
        self.update_label = Some(label.into());
        self.update_tooltip = Some(tooltip.into());
        self.update_url = Some(url.into());
        self
    }

    /// Returns `true` if an update link has been configured.
    pub fn has_update_link(&self) -> bool {
        self.update_url.is_some()
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
    pub(super) fn is_page_modified(&self, page_index: usize) -> bool {
        self.modified_pages & (1 << page_index) != 0
    }

    /// Returns the total number of nav items across all sections.
    pub(super) fn total_item_count(&self) -> usize {
        self.sections.iter().map(|s| s.items.len()).sum()
    }

    /// Returns the `page_index` for a flat item index (filtered).
    pub(super) fn page_for_flat_index(&self, flat_idx: usize) -> Option<usize> {
        self.visible_items()
            .nth(flat_idx)
            .map(|item| item.page_index)
    }

    /// Returns the search query as a lowercased string, or `None` if empty.
    pub(super) fn search_query(&self) -> Option<String> {
        let q = self.search_state.text().trim();
        if q.is_empty() {
            None
        } else {
            Some(q.to_lowercase())
        }
    }

    /// Returns whether a nav item is visible given the current search query.
    ///
    /// An item is visible if: no query is active, OR the item's label or its
    /// section title matches the query, OR the item is the active page.
    pub(super) fn item_visible(&self, item: &NavItem, section_title: &str, query: &str) -> bool {
        item.page_index == self.active_page
            || item.label.to_lowercase().contains(query)
            || section_title.to_lowercase().contains(query)
    }

    /// Returns whether a section has any visible items given the current query.
    pub(super) fn section_visible(&self, section: &NavSection, query: &str) -> bool {
        section
            .items
            .iter()
            .any(|item| self.item_visible(item, &section.title, query))
    }

    /// Iterates visible items in display order (respecting search filter).
    pub(super) fn visible_items(&self) -> impl Iterator<Item = &NavItem> {
        let query = self.search_query();
        self.sections.iter().flat_map(move |s| {
            let q = query.clone();
            let title = s.title.clone();
            let active = self.active_page;
            s.items.iter().filter(move |item| match &q {
                None => true,
                Some(q) => {
                    item.page_index == active
                        || item.label.to_lowercase().contains(q.as_str())
                        || title.to_lowercase().contains(q.as_str())
                }
            })
        })
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
        Sense::click().union(Sense::focusable())
    }

    fn layout(&self, _ctx: &LayoutCtx<'_>) -> LayoutBox {
        LayoutBox::leaf(SIDEBAR_WIDTH, 0.0)
            .with_width(crate::layout::SizeSpec::Fixed(SIDEBAR_WIDTH))
            .with_height(crate::layout::SizeSpec::Fill)
            .with_widget_id(self.id)
            .with_cursor_icon(CursorIcon::Pointer)
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        self.paint_sidebar(ctx);
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        match action {
            WidgetAction::Selected { id, index } => {
                // Only react to our own nav item selections, not external
                // Selected actions from other widgets (SchemeCard, CursorPicker).
                if *id == self.id && *index != self.active_page {
                    self.set_active_page(*index);
                    return true;
                }
            }
            WidgetAction::PageDirty { page, dirty } => {
                self.set_page_modified(*page, *dirty);
                return true;
            }
            _ => {}
        }
        false
    }

    fn on_input(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> super::OnInputResult {
        self.handle_input(event, bounds)
    }

    fn lifecycle(
        &mut self,
        event: &crate::interaction::LifecycleEvent,
        ctx: &mut crate::widgets::LifecycleCtx<'_>,
    ) {
        self.handle_lifecycle(event, ctx);
    }
}

#[cfg(test)]
mod tests;
