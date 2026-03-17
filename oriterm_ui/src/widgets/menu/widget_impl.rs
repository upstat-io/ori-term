//! Widget trait implementation for `MenuWidget`.
//!
//! Handles layout, drawing (with scroll clipping and scrollbar), and event
//! dispatch (mouse, keyboard, hover). Scroll support activates when
//! `MenuStyle::max_height` is set and content exceeds the limit.

use crate::color::Color;
use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::input::{
    HoverEvent, Key, KeyEvent, MouseButton, MouseEvent, MouseEventKind, ScrollDelta,
};
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;

use super::super::{EventCtx, LayoutCtx, Widget, WidgetAction, WidgetResponse};
use super::{MenuEntry, MenuWidget, SCROLL_LINE_HEIGHT, SCROLLBAR_MIN_THUMB, SCROLLBAR_WIDTH};

use super::DrawCtx;

impl Widget for MenuWidget {
    fn id(&self) -> crate::widget_id::WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let style = self.text_style();
        let left_margin = self.label_left_margin();

        let max_label_w: f32 = self
            .entries
            .iter()
            .filter_map(|e| e.label())
            .map(|label| ctx.measurer.measure(label, &style, f32::INFINITY).width)
            .fold(0.0_f32, f32::max);

        let width = (left_margin + max_label_w + self.style.extra_width).max(self.style.min_width);
        let height = self.visible_height();

        LayoutBox::leaf(width, height).with_widget_id(self.id)
    }

    fn sense(&self) -> Sense {
        Sense::click().union(Sense::focusable())
    }

    fn controllers(&self) -> &[Box<dyn crate::controllers::EventController>] {
        &self.controllers
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn crate::controllers::EventController>] {
        &mut self.controllers
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let bounds = ctx.bounds;
        let s = &self.style;
        let scrollable = self.is_scrollable();

        self.draw_chrome(ctx, bounds);

        // Clip content area when scrolling.
        if scrollable {
            let inset = s.border_width;
            let clip = Rect::new(
                bounds.x() + inset,
                bounds.y() + inset,
                bounds.width() - inset * 2.0,
                bounds.height() - inset * 2.0,
            );
            ctx.draw_list.push_clip(clip);
        }

        self.draw_entries(ctx, bounds);

        if scrollable {
            ctx.draw_list.pop_clip();
            self.draw_scrollbar(ctx, bounds);
        }

        ctx.draw_list.pop_layer();
    }

    fn on_input(&mut self, event: &crate::input::InputEvent, bounds: Rect) -> bool {
        match event {
            crate::input::InputEvent::MouseMove { pos, .. } => {
                let rel_y = pos.y - bounds.y();
                let new_hover = self.entry_at_y(rel_y);
                if new_hover != self.hovered {
                    self.hovered = new_hover;
                }
                true
            }
            crate::input::InputEvent::Scroll { delta, pos, .. } => {
                let delta_y = match *delta {
                    ScrollDelta::Pixels { y, .. } => -y,
                    ScrollDelta::Lines { y, .. } => -y * SCROLL_LINE_HEIGHT,
                };
                if self.scroll_by(delta_y) {
                    let rel_y = pos.y - bounds.y();
                    self.hovered = self.entry_at_y(rel_y);
                }
                true
            }
            _ => false,
        }
    }

    fn on_action(&mut self, action: WidgetAction, _bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::Clicked(_) => {
                // Transform generic click into item selection based on hover.
                if let Some(idx) = self.hovered {
                    if self.entries[idx].is_clickable() {
                        return Some(WidgetAction::Selected {
                            id: self.id,
                            index: idx,
                        });
                    }
                }
                None
            }
            other => Some(other),
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        match event.kind {
            MouseEventKind::Move => {
                let rel_y = event.pos.y - ctx.bounds.y();
                let new_hover = self.entry_at_y(rel_y);
                if new_hover != self.hovered {
                    self.hovered = new_hover;
                    return WidgetResponse::paint();
                }
                WidgetResponse::handled()
            }
            MouseEventKind::Down(MouseButton::Left) => WidgetResponse::handled(),
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(idx) = self.hovered {
                    if self.entries[idx].is_clickable() {
                        return WidgetResponse::paint().with_action(WidgetAction::Selected {
                            id: self.id,
                            index: idx,
                        });
                    }
                }
                WidgetResponse::handled()
            }
            MouseEventKind::Scroll(delta) => {
                // Negate: winit positive y = wheel-up, but positive
                // delta_y means "scroll down" in our scroll_by convention.
                let delta_y = match delta {
                    ScrollDelta::Pixels { y, .. } => -y,
                    ScrollDelta::Lines { y, .. } => -y * SCROLL_LINE_HEIGHT,
                };
                if self.scroll_by(delta_y) {
                    let rel_y = event.pos.y - ctx.bounds.y();
                    self.hovered = self.entry_at_y(rel_y);
                    return WidgetResponse::paint();
                }
                WidgetResponse::handled()
            }
            _ => WidgetResponse::ignored(),
        }
    }

    fn handle_hover(&mut self, event: HoverEvent, _ctx: &EventCtx<'_>) -> WidgetResponse {
        match event {
            HoverEvent::Enter => WidgetResponse::handled(),
            HoverEvent::Leave => {
                if self.hovered.is_some() {
                    self.hovered = None;
                    WidgetResponse::paint()
                } else {
                    WidgetResponse::handled()
                }
            }
        }
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        if !ctx.is_focused {
            return WidgetResponse::ignored();
        }
        match event.key {
            Key::ArrowDown => {
                if self.navigate(true) {
                    if let Some(idx) = self.hovered {
                        self.ensure_visible(idx);
                    }
                    WidgetResponse::paint()
                } else {
                    WidgetResponse::handled()
                }
            }
            Key::ArrowUp => {
                if self.navigate(false) {
                    if let Some(idx) = self.hovered {
                        self.ensure_visible(idx);
                    }
                    WidgetResponse::paint()
                } else {
                    WidgetResponse::handled()
                }
            }
            Key::Enter | Key::Space => {
                if let Some(idx) = self.hovered {
                    if self.entries[idx].is_clickable() {
                        return WidgetResponse::paint().with_action(WidgetAction::Selected {
                            id: self.id,
                            index: idx,
                        });
                    }
                }
                WidgetResponse::handled()
            }
            Key::Escape => {
                WidgetResponse::paint().with_action(WidgetAction::DismissOverlay(self.id))
            }
            _ => WidgetResponse::ignored(),
        }
    }
}

// Drawing helpers.
impl MenuWidget {
    /// Draws shadow, background layer, and border.
    fn draw_chrome(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let s = &self.style;

        if s.shadow_color.a > 0.0 {
            let shadow_rect = Rect::new(
                bounds.x() + 2.0,
                bounds.y() + 2.0,
                bounds.width(),
                bounds.height(),
            );
            ctx.draw_list.push_rect(
                shadow_rect,
                RectStyle::filled(s.shadow_color).with_radius(s.corner_radius),
            );
        }

        ctx.draw_list.push_layer(s.bg);

        ctx.draw_list.push_rect(
            bounds,
            RectStyle::filled(s.bg)
                .with_border(s.border_width, s.border_color)
                .with_radius(s.corner_radius),
        );
    }

    /// Draws all visible entries, accounting for scroll offset.
    fn draw_entries(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let s = &self.style;
        let left_margin = self.label_left_margin();
        let text_style = self.text_style();
        let mut y = bounds.y() + s.padding_y - self.scroll_offset;

        for (i, entry) in self.entries.iter().enumerate() {
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

    /// Draws a separator line.
    fn draw_separator(&self, ctx: &mut DrawCtx<'_>, bounds: Rect, y: f32) {
        let s = &self.style;
        let sep_y = y + s.separator_height / 2.0;
        ctx.draw_list.push_line(
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

        // Selected-item accent tint (beneath hover).
        if self.selected_index == Some(index) && self.hovered != Some(index) {
            let rect = Rect::new(
                bounds.x() + s.hover_inset,
                y,
                bounds.width() - s.hover_inset * 2.0,
                s.item_height,
            );
            ctx.draw_list.push_rect(
                rect,
                RectStyle::filled(s.selected_bg).with_radius(s.hover_radius),
            );
        }

        // Hover highlight.
        if self.hovered == Some(index) {
            let rect = Rect::new(
                bounds.x() + s.hover_inset,
                y,
                bounds.width() - s.hover_inset * 2.0,
                s.item_height,
            );
            ctx.draw_list.push_rect(
                rect,
                RectStyle::filled(s.hover_bg).with_radius(s.hover_radius),
            );
        }

        // Checkmark (Check entries only).
        if let MenuEntry::Check { checked: true, .. } = entry {
            let check_x = bounds.x() + s.padding_x;
            let check_y = y + (s.item_height - s.checkmark_size) / 2.0;
            self.draw_checkmark(ctx, check_x, check_y);
        }

        // Label text.
        let text_x = bounds.x() + left_margin;
        let text_w = bounds.width() - left_margin - s.padding_x;
        let shaped = ctx.measurer.shape(label, text_style, text_w);
        let text_y = y + (s.item_height - shaped.height) / 2.0;
        ctx.draw_list
            .push_text(Point::new(text_x, text_y), shaped, s.fg);
    }

    /// Draws a thin overlay scrollbar on the right edge.
    fn draw_scrollbar(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
        let total = self.total_height();
        let visible = self.visible_height();
        if total <= visible {
            return;
        }

        let track_x = bounds.right() - SCROLLBAR_WIDTH - self.style.border_width - 1.0;
        let track_y = bounds.y() + self.style.border_width;
        let track_h = bounds.height() - self.style.border_width * 2.0;

        let ratio = visible / total;
        let thumb_h = (track_h * ratio).max(SCROLLBAR_MIN_THUMB).min(track_h);
        let max_scroll = self.max_scroll();
        let scroll_ratio = if max_scroll > 0.0 {
            self.scroll_offset / max_scroll
        } else {
            0.0
        };
        let thumb_y = track_y + scroll_ratio * (track_h - thumb_h);

        let thumb_rect = Rect::new(track_x, thumb_y, SCROLLBAR_WIDTH, thumb_h);
        let thumb_color = Color::WHITE.with_alpha(0.25);
        ctx.draw_list.push_rect(
            thumb_rect,
            RectStyle::filled(thumb_color).with_radius(SCROLLBAR_WIDTH / 2.0),
        );
    }
}
