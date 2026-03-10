//! Scrollbar rendering and interaction for `ScrollWidget`.
//!
//! Handles scrollbar drawing (track + thumb), thumb drag interaction,
//! track click-to-jump, and hover state tracking.

use crate::draw::RectStyle;
use crate::geometry::Rect;
use crate::input::{MouseButton, MouseEvent, MouseEventKind};

use super::{DrawCtx, ScrollWidget, ScrollbarPolicy, WidgetResponse};

impl ScrollWidget {
    /// Returns whether the scrollbar should be visible.
    pub(super) fn should_show_scrollbar(&self, content_height: f32, view_height: f32) -> bool {
        match self.scrollbar_policy {
            ScrollbarPolicy::Auto => content_height > view_height,
            ScrollbarPolicy::Always => true,
            ScrollbarPolicy::Hidden => false,
        }
    }

    /// Computes the scrollbar track rect.
    fn scrollbar_track_rect(&self, viewport: Rect) -> Rect {
        let w = if self.scrollbar.track_hovered || self.scrollbar.dragging {
            self.scrollbar_style.width * 1.5
        } else {
            self.scrollbar_style.width
        };
        Rect::new(
            viewport.right() - w - 2.0,
            viewport.y(),
            w,
            viewport.height(),
        )
    }

    /// Computes the scrollbar thumb rect.
    fn scrollbar_thumb_rect(&self, viewport: Rect, content_height: f32, view_height: f32) -> Rect {
        let track = self.scrollbar_track_rect(viewport);
        let ratio = view_height / content_height;
        let thumb_h = (track.height() * ratio)
            .max(self.scrollbar_style.min_thumb_height)
            .min(track.height());
        let scroll_range = (content_height - view_height).max(0.0);
        let scroll_ratio = if scroll_range > 0.0 {
            self.scroll_offset / scroll_range
        } else {
            0.0
        };
        let thumb_y = track.y() + scroll_ratio * (track.height() - thumb_h);
        Rect::new(track.x(), thumb_y, track.width(), thumb_h)
    }

    /// Draws the vertical scrollbar.
    pub(super) fn draw_scrollbar(
        &self,
        ctx: &mut DrawCtx<'_>,
        content_height: f32,
        view_height: f32,
    ) {
        if !self.should_show_scrollbar(content_height, view_height) {
            return;
        }

        let s = &self.scrollbar_style;

        // Draw track background when hovered/dragging.
        if self.scrollbar.track_hovered || self.scrollbar.dragging {
            let track = self.scrollbar_track_rect(ctx.bounds);
            let track_style =
                RectStyle::filled(s.track_color.with_alpha(0.15)).with_radius(s.thumb_radius);
            ctx.draw_list.push_rect(track, track_style);
        }

        // Draw thumb.
        let thumb = self.scrollbar_thumb_rect(ctx.bounds, content_height, view_height);
        let thumb_color = if self.scrollbar.dragging {
            s.thumb_color.with_alpha(0.6)
        } else if self.scrollbar.track_hovered {
            s.thumb_color.with_alpha(0.4)
        } else {
            s.thumb_color
        };
        let style = RectStyle::filled(thumb_color).with_radius(s.thumb_radius);
        ctx.draw_list.push_rect(thumb, style);
    }

    /// Handles mouse events on the scrollbar. Returns `Some(response)` if handled.
    pub(super) fn handle_scrollbar_mouse(
        &mut self,
        event: &MouseEvent,
        viewport: Rect,
        content_height: f32,
        view_height: f32,
    ) -> Option<WidgetResponse> {
        if self.scrollbar_policy == ScrollbarPolicy::Hidden {
            return None;
        }

        let track = self.scrollbar_track_rect(viewport);
        let thumb = self.scrollbar_thumb_rect(viewport, content_height, view_height);

        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if thumb.contains(event.pos) {
                    self.scrollbar.dragging = true;
                    self.scrollbar.drag_start_y = event.pos.y;
                    self.scrollbar.drag_start_offset = self.scroll_offset;
                    return Some(WidgetResponse::handled());
                }
                // Click on track (not thumb) — jump to position.
                if track.contains(event.pos) {
                    let ratio = (event.pos.y - track.y()) / track.height();
                    let max = (content_height - view_height).max(0.0);
                    self.scroll_offset = (ratio * max).clamp(0.0, max);
                    return Some(WidgetResponse::paint());
                }
                None
            }
            MouseEventKind::Move => {
                if self.scrollbar.dragging {
                    let delta_y = event.pos.y - self.scrollbar.drag_start_y;
                    let track_h = track.height();
                    let max = (content_height - view_height).max(0.0);
                    let scroll_delta = if track_h > 0.0 {
                        delta_y * max / track_h
                    } else {
                        0.0
                    };
                    self.scroll_offset =
                        (self.scrollbar.drag_start_offset + scroll_delta).clamp(0.0, max);
                    return Some(WidgetResponse::paint());
                }
                // Track hover detection.
                let was_hovered = self.scrollbar.track_hovered;
                self.scrollbar.track_hovered = track.contains(event.pos);
                if was_hovered != self.scrollbar.track_hovered {
                    return Some(WidgetResponse::paint());
                }
                None
            }
            MouseEventKind::Up(_) => {
                if self.scrollbar.dragging {
                    self.scrollbar.dragging = false;
                    return Some(WidgetResponse::paint());
                }
                None
            }
            _ => None,
        }
    }
}
