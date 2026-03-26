//! Sidebar geometry — layout metrics and hit testing.
//!
//! All sidebar spacing constants are centralized here, derived from the
//! mockup CSS (`mockups/settings-brutal.html`). Both `paint.rs` and
//! `input.rs` use these values so layout and hit testing stay in sync.

use crate::geometry::Rect;
use crate::icons::SIDEBAR_NAV_ICON_SIZE;

use super::SidebarNavWidget;

// -- Sidebar padding --

/// Vertical padding inside the sidebar (`padding: 16px 0`).
pub(super) const SIDEBAR_PADDING_Y: f32 = 16.0;

// -- Search field --

/// Horizontal inset for the search container (`.sidebar-search { padding: 0 10px }`).
pub(super) const SEARCH_PADDING_X: f32 = 10.0;

/// Search field height.
pub(super) const SEARCH_FIELD_H: f32 = 28.0;

/// Gap between search field bottom and first section title (`margin-bottom: 12px`).
pub(super) const SEARCH_BOTTOM_GAP: f32 = 12.0;

/// Total vertical space the search area consumes (field + gap).
pub(super) const SEARCH_AREA_H: f32 = SEARCH_FIELD_H + SEARCH_BOTTOM_GAP;

// -- Section titles --

/// Horizontal padding for section titles (`.sidebar-title { padding: 0 16px }`).
pub(super) const TITLE_PADDING_X: f32 = 16.0;

/// Bottom margin below a section title (`margin-bottom: 8px`).
pub(super) const TITLE_BOTTOM_MARGIN: f32 = 8.0;

/// Top margin above a non-first section title (`:not(:first-child) { margin-top: 20px }`).
pub(super) const TITLE_TOP_MARGIN: f32 = 20.0;

/// Estimated height of a section title text line (10px font, ~1.4 line height).
pub(super) const TITLE_TEXT_H: f32 = 14.0;

// -- Nav items --

/// Active indicator border width (`.nav-item { border-left: 3px solid ... }`).
pub(super) const INDICATOR_WIDTH: f32 = 3.0;

/// Nav item vertical padding (`.nav-item { padding: 7px 16px }`).
pub(super) const NAV_ITEM_PADDING_Y: f32 = 7.0;

/// Nav item horizontal padding inside the indicator border.
pub(super) const NAV_ITEM_PADDING_X: f32 = 16.0;

/// Nav item vertical margin (`.nav-item { margin: 1px 0 }`).
pub(super) const NAV_ITEM_MARGIN_Y: f32 = 1.0;

/// Nav item text/content height (13px font).
pub(super) const NAV_ITEM_CONTENT_H: f32 = 13.0;

/// Derived nav item outer height: margin + padding + content + padding + margin.
pub(super) const NAV_ITEM_HEIGHT: f32 = NAV_ITEM_MARGIN_Y
    + NAV_ITEM_PADDING_Y
    + NAV_ITEM_CONTENT_H
    + NAV_ITEM_PADDING_Y
    + NAV_ITEM_MARGIN_Y;

/// Gap between nav item icon and label text.
pub(super) const ICON_TEXT_GAP: f32 = 10.0;

// -- Footer --

/// Horizontal padding for footer text (`.sidebar-footer { padding: 8px 16px }`).
pub(super) const FOOTER_PADDING_X: f32 = 16.0;

/// Vertical padding at the bottom of the footer.
pub(super) const FOOTER_PADDING_Y: f32 = 8.0;

/// Gap between version row and config path row.
pub(super) const FOOTER_ROW_GAP: f32 = 4.0;

/// Gap between version text and update link on the same line.
pub(super) const FOOTER_INLINE_GAP: f32 = 6.0;

// -- Geometry helpers --

/// Returns the search field rect in absolute coordinates.
///
/// The search field is inset by `SEARCH_PADDING_X` from each side of the
/// sidebar, matching `.sidebar-search { padding: 0 10px }`.
pub(super) fn search_field_rect(bounds: Rect) -> Rect {
    let x = bounds.x() + SEARCH_PADDING_X;
    let y = bounds.y() + SIDEBAR_PADDING_Y;
    let w = bounds.width() - SEARCH_PADDING_X * 2.0;
    Rect::new(x, y, w, SEARCH_FIELD_H)
}

/// X position for nav item icons: `sidebar_x` + 3px border + 16px padding.
pub(super) fn nav_icon_x(bounds: &Rect) -> f32 {
    bounds.x() + INDICATOR_WIDTH + NAV_ITEM_PADDING_X
}

/// X position for nav item label text (after icon + gap).
///
/// With icon: `sidebar_x` + 3 + 16 + 16(icon) + 10(gap) = `sidebar_x` + 45.
/// Without icon: same as `nav_icon_x` (`sidebar_x` + 19).
pub(super) fn nav_text_x(bounds: &Rect, has_icon: bool) -> f32 {
    if has_icon {
        nav_icon_x(bounds) + SIDEBAR_NAV_ICON_SIZE as f32 + ICON_TEXT_GAP
    } else {
        nav_icon_x(bounds)
    }
}

/// X position for section title and footer text content.
///
/// Matches `.sidebar-title { padding: 0 16px }` and
/// `.sidebar-footer { padding: 8px 16px }`.
pub(super) fn content_text_x(bounds: &Rect) -> f32 {
    bounds.x() + TITLE_PADDING_X
}

/// Y advance for a section title area (top margin + text + bottom margin).
///
/// First section has no top margin; subsequent sections have `TITLE_TOP_MARGIN`.
pub(super) fn title_y_advance(is_first: bool) -> f32 {
    let top = if is_first { 0.0 } else { TITLE_TOP_MARGIN };
    top + TITLE_TEXT_H + TITLE_BOTTOM_MARGIN
}

impl SidebarNavWidget {
    /// Hit-tests a local Y coordinate (relative to sidebar top + padding)
    /// to a flat item index, respecting search filtering.
    pub(super) fn hit_test_item(&self, local_y: f32) -> Option<usize> {
        let query = self.search_query();
        let mut y = SEARCH_AREA_H;
        let mut flat_idx = 0;
        let mut is_first_visible = true;
        for section in &self.sections {
            // Skip section if no items visible.
            if let Some(ref q) = query {
                if !self.section_visible(section, q) {
                    continue;
                }
            }
            // Title area: top margin (non-first) + text + bottom margin.
            y += title_y_advance(is_first_visible);
            is_first_visible = false;

            for item in &section.items {
                // Skip filtered-out items.
                if let Some(ref q) = query {
                    if !self.item_visible(item, &section.title, q) {
                        continue;
                    }
                }
                if local_y >= y && local_y < y + NAV_ITEM_HEIGHT {
                    return Some(flat_idx);
                }
                y += NAV_ITEM_HEIGHT;
                flat_idx += 1;
            }
        }
        None
    }
}
