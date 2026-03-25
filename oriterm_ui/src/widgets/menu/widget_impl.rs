//! Widget trait implementation for `MenuWidget`.
//!
//! Handles layout, drawing (with scroll clipping and scrollbar), and event
//! dispatch (mouse, keyboard, hover). Scroll support activates when
//! `MenuStyle::max_height` is set and content exceeds the limit.
//!
//! Press/drag input flows through `ScrubController` → `on_action()` with
//! zone discrimination (scrollbar thumb, scrollbar track, or menu item).
//! Idle hover and scroll wheel remain in `on_input()`.

use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::input::{InputEvent, ScrollDelta};
use crate::interaction::LifecycleEvent;
use crate::layout::LayoutBox;
use crate::sense::Sense;
use crate::text::TextStyle;

use super::super::scrollbar::{
    ScrollbarAxis, ScrollbarMetrics, compute_rects, drag_delta_to_offset, draw_overlay,
    pointer_to_offset, should_show,
};
use super::super::{LayoutCtx, LifecycleCtx, OnInputResult, Widget, WidgetAction};
use super::{DragMode, MenuEntry, MenuWidget, SCROLL_LINE_HEIGHT};

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
        Sense::click_and_drag()
    }

    fn controllers(&self) -> &[Box<dyn crate::controllers::EventController>] {
        &self.controllers
    }

    fn controllers_mut(&mut self) -> &mut [Box<dyn crate::controllers::EventController>] {
        &mut self.controllers
    }

    fn lifecycle(&mut self, event: &LifecycleEvent, _ctx: &mut LifecycleCtx<'_>) {
        if let LifecycleEvent::HotChanged { is_hot: false, .. } = event {
            self.scrollbar_state.track_hovered = false;
            self.scrollbar_state.thumb_hovered = false;
        }
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
            ctx.scene.push_clip(clip);
        }

        self.draw_entries(ctx, bounds);

        if scrollable {
            ctx.scene.pop_clip();
            self.draw_scrollbar(ctx, bounds);
        }

        ctx.scene.pop_layer_bg();
    }

    fn on_action(&mut self, action: WidgetAction, bounds: Rect) -> Option<WidgetAction> {
        match action {
            WidgetAction::DragStart { pos, .. } => {
                self.drag_origin = Some(pos);
                self.handle_drag_start(pos, bounds);
                None
            }
            WidgetAction::DragUpdate { total_delta, .. } => {
                self.handle_drag_update(total_delta, bounds);
                None
            }
            WidgetAction::DragEnd { .. } => {
                let result = self.handle_drag_end();
                self.drag_origin = None;
                self.drag_mode = None;
                result
            }
            other => Some(other),
        }
    }

    fn on_input(&mut self, event: &InputEvent, bounds: Rect) -> OnInputResult {
        match event {
            // Idle hover — ScrubController does not consume MouseMove when idle.
            InputEvent::MouseMove { pos, .. } => {
                if self.is_scrollable() {
                    self.update_scrollbar_hover(*pos, bounds);
                }
                // Clear entry hover when cursor is on the scrollbar.
                if self.scrollbar_state.track_hovered {
                    self.hovered = None;
                } else {
                    let rel_y = pos.y - bounds.y();
                    let new_hover = self.entry_at_y(rel_y);
                    if new_hover != self.hovered {
                        self.hovered = new_hover;
                    }
                }
                OnInputResult::handled()
            }
            InputEvent::Scroll { delta, pos, .. } => {
                let delta_y = match *delta {
                    ScrollDelta::Pixels { y, .. } => -y,
                    ScrollDelta::Lines { y, .. } => -y * SCROLL_LINE_HEIGHT,
                };
                if self.scroll_by(delta_y) {
                    // Only update hover if the cursor is on a menu item, not the
                    // scrollbar — same guard as the MouseMove path above.
                    if !self.scrollbar_state.track_hovered {
                        let rel_y = pos.y - bounds.y();
                        self.hovered = self.entry_at_y(rel_y);
                    }
                }
                OnInputResult::handled()
            }
            _ => OnInputResult::ignored(),
        }
    }

    fn key_context(&self) -> Option<&'static str> {
        Some("Menu")
    }

    fn handle_keymap_action(
        &mut self,
        action: &dyn crate::action::KeymapAction,
        _bounds: Rect,
    ) -> Option<WidgetAction> {
        match action.name() {
            "widget::NavigateDown" => {
                self.navigate_keyboard(true);
                None
            }
            "widget::NavigateUp" => {
                self.navigate_keyboard(false);
                None
            }
            "widget::Confirm" => {
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
            "widget::Dismiss" => Some(WidgetAction::DismissOverlay(self.id)),
            _ => None,
        }
    }
}

// Drag action handlers.
impl MenuWidget {
    /// Determines the press zone and starts the appropriate interaction.
    fn handle_drag_start(&mut self, pos: Point, bounds: Rect) {
        if self.is_scrollable() {
            let (m, inner) = self.scrollbar_context(bounds);
            if should_show(&m) {
                let rects = compute_rects(inner, &m, &self.style.scrollbar, 0.0);
                if rects.thumb_hit.contains(pos) {
                    self.scrollbar_state.dragging = true;
                    self.scrollbar_state.drag_start_offset = self.scroll_offset;
                    self.drag_mode = Some(DragMode::ScrollbarThumb);
                    return;
                }
                if rects.track_hit.contains(pos) {
                    self.scroll_offset = pointer_to_offset(pos.y, &rects, &m);
                    self.drag_mode = Some(DragMode::ScrollbarTrack);
                    return;
                }
            }
        }
        // Update hover from the press position so a click without prior
        // MouseMove (e.g. menu opens under a stationary cursor) selects
        // the correct item instead of silently no-oping.
        let rel_y = pos.y - bounds.y();
        self.hovered = self.entry_at_y(rel_y);
        self.drag_mode = Some(DragMode::ItemPress);
    }

    /// Updates state during an active drag.
    fn handle_drag_update(&mut self, total_delta: Point, bounds: Rect) {
        match self.drag_mode {
            Some(DragMode::ScrollbarThumb) => {
                let (m, inner) = self.scrollbar_context(bounds);
                let rects = compute_rects(inner, &m, &self.style.scrollbar, 0.0);
                let offset_delta = drag_delta_to_offset(total_delta.y, &rects, &m);
                let max = self.max_scroll();
                self.scroll_offset =
                    (self.scrollbar_state.drag_start_offset + offset_delta).clamp(0.0, max);
                self.hovered = None;
            }
            Some(DragMode::ItemPress) => {
                // Update item hover based on current absolute position.
                if let Some(origin) = self.drag_origin {
                    let cur_y = origin.y + total_delta.y;
                    self.hovered = self.entry_at_y(cur_y - bounds.y());
                }
            }
            Some(DragMode::ScrollbarTrack) | None => {}
        }
    }

    /// Finalizes the drag and optionally emits a Selected action.
    fn handle_drag_end(&mut self) -> Option<WidgetAction> {
        match self.drag_mode {
            Some(DragMode::ScrollbarThumb) => {
                self.scrollbar_state.dragging = false;
                None
            }
            Some(DragMode::ItemPress) => {
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
            Some(DragMode::ScrollbarTrack) | None => None,
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

        // Selected-item accent tint (beneath hover).
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

        // Hover highlight.
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
        ctx.scene
            .push_text(Point::new(text_x, text_y), shaped, s.fg);
    }

    /// Draws a thin overlay scrollbar on the right edge using the shared helper.
    fn draw_scrollbar(&self, ctx: &mut DrawCtx<'_>, bounds: Rect) {
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
}

// Scrollbar helpers.
impl MenuWidget {
    /// Scrollbar metrics and border-inset viewport for shared geometry helpers.
    fn scrollbar_context(&self, bounds: Rect) -> (ScrollbarMetrics, Rect) {
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
    fn update_scrollbar_hover(&mut self, pos: Point, bounds: Rect) {
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
