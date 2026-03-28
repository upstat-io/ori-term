//! Sidebar paint — background, search field, titles, items, footer.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::icons::{IconId, SIDEBAR_NAV_ICON_SIZE};
use crate::text::{FontWeight, TextStyle, TextTransform};
use crate::widgets::DrawCtx;

use super::geometry::{
    self, FOOTER_INLINE_GAP, FOOTER_PADDING_X, FOOTER_PADDING_Y, FOOTER_ROW_GAP, INDICATOR_WIDTH,
    NAV_ITEM_HEIGHT, NAV_ITEM_PADDING_Y, SEARCH_AREA_H, SEARCH_PADDING_X, SIDEBAR_PADDING_Y,
    TITLE_TOP_MARGIN,
};
use super::{FooterRects, HoveredFooterTarget, NavItem, SidebarNavWidget};

/// Truncates a string with an ellipsis if it exceeds the available width.
///
/// Measures the full text first. If it fits, returns it unchanged. Otherwise
/// walks character boundaries to find the longest prefix that fits with the
/// appended ellipsis character (U+2026, width ~1 glyph).
#[expect(
    clippy::string_slice,
    reason = "char_indices ensures slicing on char boundaries"
)]
fn truncate_with_ellipsis(
    text: &str,
    style: &TextStyle,
    max_width: f32,
    ctx: &DrawCtx<'_>,
) -> String {
    let full_w = ctx.measurer.measure(text, style, f32::INFINITY).width;
    if full_w <= max_width {
        return text.to_owned();
    }
    // Reserve space for ellipsis.
    let ellipsis = "\u{2026}";
    let ellipsis_w = ctx.measurer.measure(ellipsis, style, f32::INFINITY).width;
    let target_w = max_width - ellipsis_w;
    if target_w <= 0.0 {
        return ellipsis.to_owned();
    }
    // Walk char boundaries to find longest prefix that fits.
    let mut last_good = 0;
    for (i, _) in text.char_indices() {
        let prefix_w = ctx.measurer.measure(&text[..i], style, f32::INFINITY).width;
        if prefix_w > target_w {
            break;
        }
        last_good = i;
    }
    let mut result = text[..last_good].to_owned();
    result.push_str(ellipsis);
    result
}

impl SidebarNavWidget {
    /// Paints the entire sidebar: background, search, titles, items, footer.
    pub(super) fn paint_sidebar(&self, ctx: &mut DrawCtx<'_>) {
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

        let search_x = bounds.x() + SEARCH_PADDING_X;
        let search_w = bounds.width() - SEARCH_PADDING_X * 2.0;
        let mut y = bounds.y() + SIDEBAR_PADDING_Y;

        // Search field at top.
        self.paint_search_field(ctx, search_x, y, search_w);
        y += SEARCH_AREA_H;

        let query = self.search_query();
        let mut flat_idx = 0;
        let mut is_first_visible = true;
        for section in &self.sections {
            // Skip entire section if no items are visible under the query.
            if let Some(ref q) = query {
                if !self.section_visible(section, q) {
                    continue;
                }
            }

            // Top margin for non-first visible sections.
            if !is_first_visible {
                y += TITLE_TOP_MARGIN;
            }
            is_first_visible = false;

            // Section title — uppercase with wide letter spacing.
            let title_style = TextStyle {
                size: 10.0,
                weight: FontWeight::NORMAL,
                letter_spacing: 1.5,
                text_transform: TextTransform::Uppercase,
                ..TextStyle::default()
            };
            let title_text = format!("// {}", section.title);
            let title_x = geometry::content_text_x(&bounds);
            let shaped = ctx
                .measurer
                .shape(&title_text, &title_style, bounds.width());
            ctx.scene
                .push_text(Point::new(title_x, y), shaped, self.style.section_title_fg);
            y += geometry::TITLE_TEXT_H + geometry::TITLE_BOTTOM_MARGIN;

            for item in &section.items {
                // Skip items that don't match the search query.
                if let Some(ref q) = query {
                    if !self.item_visible(item, &section.title, q) {
                        continue;
                    }
                }
                // Nav items span the full sidebar width.
                let item_rect = Rect::new(bounds.x(), y, bounds.width(), NAV_ITEM_HEIGHT);
                self.paint_nav_item(ctx, item, item_rect, flat_idx);
                y += NAV_ITEM_HEIGHT;
                flat_idx += 1;
            }
        }

        self.paint_footer(ctx, &bounds);
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

        // Background — full row width (matching CSS box model).
        let bg = if is_active {
            self.style.active_bg
        } else if self.hovered_item == Some(flat_idx) {
            self.style.hover_bg
        } else {
            Color::TRANSPARENT
        };
        if bg.a > 0.001 {
            ctx.scene.push_quad(item_rect, RectStyle::filled(bg));
        }

        // 3px left border: transparent for inactive, accent for active.
        if is_active {
            let indicator = Rect::new(x, y, INDICATOR_WIDTH, NAV_ITEM_HEIGHT);
            ctx.scene
                .push_quad(indicator, RectStyle::filled(self.style.active_fg));
        }

        // Icon (at sidebar_x + 3 + 16 = sidebar_x + 19).
        let text_x = if let Some(icon_id) = item.icon {
            let icon_size = SIDEBAR_NAV_ICON_SIZE;
            let icon_x = geometry::nav_icon_x(&item_rect);
            let icon_y = y + (NAV_ITEM_HEIGHT - icon_size as f32) / 2.0;
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
                        Rect::new(icon_x, icon_y, icon_size as f32, icon_size as f32),
                        resolved.atlas_page,
                        resolved.uv,
                        c,
                    );
                }
            }
            geometry::nav_text_x(&item_rect, true)
        } else {
            geometry::nav_text_x(&item_rect, false)
        };

        // Label.
        let fg = if is_active {
            self.style.active_fg
        } else if self.hovered_item == Some(flat_idx) {
            self.style.hover_fg
        } else {
            self.style.item_fg
        };
        let weight = if is_active {
            FontWeight::MEDIUM
        } else {
            FontWeight::NORMAL
        };
        let style = TextStyle {
            size: 13.0,
            weight,
            ..TextStyle::default()
        };
        let label_y = y + NAV_ITEM_PADDING_Y;
        let shaped = ctx.measurer.shape(&item.label, &style, item_w);
        ctx.scene.push_text(Point::new(text_x, label_y), shaped, fg);

        // Modified dot (6px square, warning color, right-aligned with 16px margin).
        if self.is_page_modified(item.page_index) {
            let dot_size = 6.0;
            let dot_x = item_rect.right() - 16.0;
            let dot_y = y + (NAV_ITEM_HEIGHT - dot_size) / 2.0;
            let dot_rect = Rect::new(dot_x, dot_y, dot_size, dot_size);
            ctx.scene
                .push_quad(dot_rect, RectStyle::filled(ctx.theme.warning));
        }
    }

    /// Paints the sidebar footer: version label + optional update link + config path.
    ///
    /// Builds bottom-up from the sidebar bottom edge. Stores rects in `Cell`
    /// for hit testing (interior mutability since `paint` takes `&self`).
    fn paint_footer(&self, ctx: &mut DrawCtx<'_>, bounds: &Rect) {
        let text_x = bounds.x() + FOOTER_PADDING_X;
        let avail_w = bounds.width() - FOOTER_PADDING_X * 2.0;
        // Sidebar bottom padding (16px) + footer internal padding (8px).
        let mut y = bounds.bottom() - SIDEBAR_PADDING_Y - FOOTER_PADDING_Y;
        let mut rects = FooterRects::default();

        // Config path (bottom-most, faint + smaller, with ellipsis truncation).
        if !self.config_path.is_empty() {
            let style = TextStyle {
                size: 10.0,
                ..TextStyle::default()
            };
            let display_text = truncate_with_ellipsis(&self.config_path, &style, avail_w, ctx);
            let shaped = ctx.measurer.shape(&display_text, &style, f32::INFINITY);
            y -= shaped.height;

            let is_hovered = self.hovered_footer == Some(HoveredFooterTarget::ConfigPath);
            let fg = if is_hovered {
                ctx.theme.accent
            } else {
                self.style.version_fg.with_alpha(0.7)
            };
            rects.config_path = Some(Rect::new(text_x, y, avail_w, shaped.height));
            ctx.scene.push_text(Point::new(text_x, y), shaped, fg);
            y -= FOOTER_ROW_GAP;
        }

        // Version label + optional update link on the same line.
        if !self.version.is_empty() {
            let ver_style = TextStyle {
                size: 11.0,
                ..TextStyle::default()
            };
            let ver_shaped = ctx.measurer.shape(&self.version, &ver_style, avail_w);
            y -= ver_shaped.height;
            ctx.scene
                .push_text(Point::new(text_x, y), ver_shaped, self.style.version_fg);

            // Update link (adjacent on the same line, 6px gap).
            if let Some(ref label) = self.update_label {
                let ver_w = ctx
                    .measurer
                    .measure(&self.version, &ver_style, f32::INFINITY)
                    .width;
                let link_x = text_x + ver_w + FOOTER_INLINE_GAP;
                let link_style = TextStyle {
                    size: 10.0,
                    weight: FontWeight::MEDIUM,
                    ..TextStyle::default()
                };
                let link_shaped = ctx.measurer.shape(label, &link_style, avail_w);
                let link_h = link_shaped.height;
                let is_hovered = self.hovered_footer == Some(HoveredFooterTarget::UpdateLink);
                let fg = if is_hovered {
                    ctx.theme.accent_hover
                } else {
                    ctx.theme.accent
                };
                let link_w = ctx
                    .measurer
                    .measure(label, &link_style, f32::INFINITY)
                    .width;
                rects.update_link = Some(Rect::new(link_x, y, link_w, link_h));
                ctx.scene.push_text(Point::new(link_x, y), link_shaped, fg);

                // Manual underline on hover (1px line, 2px below baseline).
                if is_hovered {
                    let underline_y = y + link_h + 2.0;
                    let underline = Rect::new(link_x, underline_y, link_w, 1.0);
                    ctx.scene.push_quad(underline, RectStyle::filled(fg));
                }
            }
        }

        self.footer_rects.set(rects);
    }

    /// Paints the search field at the top of the sidebar.
    ///
    /// Mockup CSS: height 28px, bg `--bg-surface`, border 2px, padding
    /// `6px 8px 6px 26px` (26px left includes the search icon). When
    /// focused, uses accent border and draws a text cursor.
    #[expect(
        clippy::string_slice,
        reason = "selection bounds always on char boundaries"
    )]
    fn paint_search_field(&self, ctx: &mut DrawCtx<'_>, x: f32, y: f32, w: f32) {
        let field_h = 28.0;
        let field_rect = Rect::new(x, y, w, field_h);
        let bg = ctx.theme.bg_primary;

        let border_color = if self.search_focused {
            ctx.theme.accent
        } else {
            ctx.theme.border
        };
        let bg_style = RectStyle::filled(bg).with_border(2.0, border_color);

        // Layer captures the field bg for subpixel text compositing.
        ctx.scene.push_layer_bg(bg);
        ctx.scene.push_quad(field_rect, bg_style);

        // Inner area for text (left 26px includes icon space, right 8px).
        let text_x = x + 26.0;
        let text_w = w - 26.0 - 8.0;
        let inner = Rect::new(text_x, y + 2.0, text_w, field_h - 4.0);
        ctx.scene.push_clip(inner);

        let style = TextStyle {
            size: 12.0,
            ..TextStyle::default()
        };
        let search_text = self.search_state.text();

        // Cache character boundary X-offsets for click-to-cursor mapping.
        {
            let mut offsets = self.search_char_offsets.borrow_mut();
            offsets.clear();
            for (i, _) in search_text.char_indices() {
                let x_off = ctx
                    .measurer
                    .measure(&search_text[..i], &style, f32::INFINITY)
                    .width;
                offsets.push((i, x_off));
            }
            let end_x = ctx
                .measurer
                .measure(search_text, &style, f32::INFINITY)
                .width;
            offsets.push((search_text.len(), end_x));
        }

        if search_text.is_empty() {
            // Placeholder text.
            let shaped = ctx.measurer.shape("Search settings...", &style, text_w);
            let text_y = y + (field_h - shaped.height) / 2.0;
            ctx.scene
                .push_text(Point::new(text_x, text_y), shaped, ctx.theme.fg_faint);
        } else {
            // Selection highlight (behind text).
            if let Some((sel_start, sel_end)) = self.search_state.selection_range() {
                if sel_start != sel_end {
                    let prefix_w = ctx
                        .measurer
                        .measure(&search_text[..sel_start], &style, f32::INFINITY)
                        .width;
                    let sel_w = ctx
                        .measurer
                        .measure(&search_text[sel_start..sel_end], &style, f32::INFINITY)
                        .width;
                    let sel_rect = Rect::new(text_x + prefix_w, inner.y(), sel_w, inner.height());
                    ctx.scene.push_quad(
                        sel_rect,
                        RectStyle::filled(ctx.theme.accent.with_alpha(0.3)),
                    );
                }
            }

            // Search text.
            let shaped = ctx.measurer.shape(search_text, &style, f32::INFINITY);
            let text_y = y + (field_h - shaped.height) / 2.0;
            ctx.scene
                .push_text(Point::new(text_x, text_y), shaped, ctx.theme.fg_primary);
        }

        // Cursor (only when focused).
        if self.search_focused {
            let cursor = self.search_state.cursor();
            let prefix = &search_text[..cursor];
            let cursor_px = ctx.measurer.measure(prefix, &style, f32::INFINITY).width;
            let cursor_rect = Rect::new(text_x + cursor_px, inner.y(), 1.0, inner.height());
            ctx.scene
                .push_quad(cursor_rect, RectStyle::filled(ctx.theme.fg_primary));
        }

        ctx.scene.pop_clip();

        // Search icon (12px, centered at x+8, vertically centered).
        let icon_size = 12u32;
        let icon_x = x + 8.0;
        let icon_y = y + (field_h - icon_size as f32) / 2.0;
        if let Some(icons) = ctx.icons {
            if let Some(resolved) = icons.get(IconId::Search, icon_size) {
                let icon_color = if self.search_focused {
                    ctx.theme.fg_secondary
                } else {
                    ctx.theme.fg_faint
                };
                ctx.scene.push_icon(
                    Rect::new(icon_x, icon_y, icon_size as f32, icon_size as f32),
                    resolved.atlas_page,
                    resolved.uv,
                    icon_color,
                );
            }
        }

        ctx.scene.pop_layer_bg();
    }
}
