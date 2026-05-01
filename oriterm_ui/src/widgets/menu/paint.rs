//! Drawing helpers for [`MenuWidget`].
//!
//! Holds the chrome / entries / query-row / no-match / item / separator /
//! scrollbar drawing code plus the shared `scrollbar_context` +
//! `update_scrollbar_hover` geometry helpers. Kept as an `impl MenuWidget`
//! block in a sibling module so `mod.rs` and `widget_impl.rs` stay under
//! the 500-line file budget.

use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::text::TextStyle;

use super::super::DrawCtx;
use super::super::scrollbar::{
    ScrollbarAxis, ScrollbarMetrics, compute_rects, draw_overlay, should_show,
};
use super::{MenuEntry, MenuWidget};

impl MenuWidget {
    /// Draws shadow, background layer, and border.
    pub(super) fn draw_chrome(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let s = &self.style;

        if s.shadow_color.a > 0.0 {
            let shadow_rect = Rect::new(
                bounds.x() + 2.0,
                bounds.y() + 2.0,
                bounds.width(),
                bounds.height(),
            );
            ctx.scene.push_quad(
                shadow_rect,
                RectStyle::filled(s.shadow_color).with_radius(s.corner_radius),
            );
        }

        ctx.scene.push_layer_bg(s.bg);

        ctx.scene.push_quad(
            bounds,
            RectStyle::filled(s.bg)
                .with_border(s.border_width, s.border_color)
                .with_radius(s.corner_radius),
        );
    }

    /// Draws all visible entries, accounting for scroll offset and any
    /// pinned search row above the entries region.
    pub(super) fn draw_entries(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let s = &self.style;
        let left_margin = self.label_left_margin();
        let text_style = self.text_style();
        let entries_top = bounds.y() + s.padding_y + self.query_row_height();

        if self.searchable && self.display_indices.is_empty() {
            self.draw_no_matches_row(ctx, bounds, entries_top);
            return;
        }

        let mut y = entries_top - self.scroll_offset;

        for &i in &self.display_indices {
            let entry = &self.entries[i];
            let item_h = match entry {
                MenuEntry::Separator => s.separator_height,
                _ => s.item_height,
            };

            if y + item_h < bounds.y() {
                y += item_h;
                continue;
            }
            if y > bounds.bottom() {
                break;
            }

            match entry {
                MenuEntry::Separator => {
                    self.draw_separator(ctx, bounds, y);
                }
                MenuEntry::Item { label } | MenuEntry::Check { label, .. } => {
                    self.draw_item(ctx, i, entry, label, &text_style, left_margin, bounds, y);
                }
            }
            y += item_h;
        }
    }

    /// Draws the pinned type-to-filter row at the top of a searchable menu.
    pub(super) fn draw_query_row(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let s = &self.style;
        let qh = self.query_row_height();
        let inset = s.border_width;
        let row = Rect::new(
            bounds.x() + inset,
            bounds.y() + inset,
            bounds.width() - inset * 2.0,
            qh,
        );
        ctx.scene.push_quad(
            row,
            RectStyle::filled(s.bg).with_radius(s.corner_radius * 0.5),
        );

        let placeholder = self.query.is_empty();
        let display_text = if placeholder {
            "Search\u{2026}"
        } else {
            self.query.as_str()
        };
        let fg = if placeholder {
            s.no_match_text_color
        } else {
            s.fg
        };
        let text_style = TextStyle::new(s.font_size, fg);
        let inner_w = (row.width() - s.padding_x * 2.0).max(0.0);
        let shaped = ctx.measurer.shape(display_text, &text_style, inner_w);
        let tx = row.x() + s.padding_x;
        let ty = row.y() + (row.height() - shaped.height) / 2.0;
        ctx.scene.push_text(Point::new(tx, ty), shaped, fg);
    }

    /// Draws the "No matches" indicator row when a searchable menu has no
    /// visible entries.
    fn draw_no_matches_row(&self, ctx: &mut DrawCtx<'_>, bounds: Rect, entries_top: f32) {
        let s = &self.style;
        let text_style = TextStyle::new(s.font_size, s.no_match_text_color);
        let inner_w = (bounds.width() - self.label_left_margin() - s.padding_x).max(0.0);
        let shaped = ctx.measurer.shape("No matches", &text_style, inner_w);
        let tx = bounds.x() + self.label_left_margin();
        let ty = entries_top + (s.item_height - shaped.height) / 2.0;
        ctx.scene
            .push_text(Point::new(tx, ty), shaped, s.no_match_text_color);
    }

    /// Draws a separator line.
    fn draw_separator(&self, ctx: &mut DrawCtx<'_>, bounds: Rect, y: f32) {
        let s = &self.style;
        let sep_y = y + s.separator_height / 2.0;
        ctx.scene.push_line(
            Point::new(bounds.x() + s.hover_inset, sep_y),
            Point::new(bounds.right() - s.hover_inset, sep_y),
            1.0,
            s.separator_color,
        );
    }

    /// Draws a single clickable item (Item or Check).
    #[expect(clippy::too_many_arguments, reason = "drawing: entry state + layout")]
    fn draw_item(
        &self,
        ctx: &mut DrawCtx<'_>,
        index: usize,
        entry: &MenuEntry,
        label: &str,
        text_style: &TextStyle,
        left_margin: f32,
        bounds: Rect,
        y: f32,
    ) {
        let s = &self.style;

        if self.selected_index == Some(index) && self.hovered != Some(index) {
            let rect = Rect::new(
                bounds.x() + s.hover_inset,
                y,
                bounds.width() - s.hover_inset * 2.0,
                s.item_height,
            );
            ctx.scene.push_quad(
                rect,
                RectStyle::filled(s.selected_bg).with_radius(s.hover_radius),
            );
        }

        if self.hovered == Some(index) {
            let rect = Rect::new(
                bounds.x() + s.hover_inset,
                y,
                bounds.width() - s.hover_inset * 2.0,
                s.item_height,
            );
            ctx.scene.push_quad(
                rect,
                RectStyle::filled(s.hover_bg).with_radius(s.hover_radius),
            );
        }

        if let MenuEntry::Check { checked: true, .. } = entry {
            let check_x = bounds.x() + s.padding_x;
            let check_y = y + (s.item_height - s.checkmark_size) / 2.0;
            self.draw_checkmark(ctx, check_x, check_y);
        }

        let text_x = bounds.x() + left_margin;
        let text_w = bounds.width() - left_margin - s.padding_x;
        let shaped = ctx.measurer.shape(label, text_style, text_w);
        let text_y = y + (s.item_height - shaped.height) / 2.0;
        ctx.scene
            .push_text(Point::new(text_x, text_y), shaped, s.fg);
    }

    /// Draws a thin overlay scrollbar on the right edge using the shared helper.
    pub(super) fn draw_scrollbar(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let (m, inner) = self.scrollbar_context(bounds);
        if !should_show(&m) {
            return;
        }
        let rects = compute_rects(inner, &m, &self.style.scrollbar, 0.0);
        draw_overlay(
            ctx.scene,
            &rects,
            &self.style.scrollbar,
            self.scrollbar_state.visual_state(),
        );
    }

    /// Scrollbar metrics and border-inset viewport for shared geometry helpers.
    pub(super) fn scrollbar_context(&self, bounds: Rect) -> (ScrollbarMetrics, Rect) {
        let bw = self.style.border_width;
        let inner = Rect::new(
            bounds.x() + bw,
            bounds.y() + bw,
            bounds.width() - bw * 2.0,
            bounds.height() - bw * 2.0,
        );
        let m = ScrollbarMetrics {
            axis: ScrollbarAxis::Vertical,
            content_extent: self.total_height(),
            view_extent: self.visible_height(),
            scroll_offset: self.scroll_offset,
        };
        (m, inner)
    }

    /// Updates scrollbar hover state for idle mouse moves (no drag active).
    pub(super) fn update_scrollbar_hover(&mut self, pos: Point, bounds: Rect) {
        let (m, inner) = self.scrollbar_context(bounds);
        if !should_show(&m) {
            self.scrollbar_state.track_hovered = false;
            self.scrollbar_state.thumb_hovered = false;
            return;
        }
        let rects = compute_rects(inner, &m, &self.style.scrollbar, 0.0);
        self.scrollbar_state.track_hovered = rects.track_hit.contains(pos);
        self.scrollbar_state.thumb_hovered = rects.thumb_hit.contains(pos);
    }
}
