//! Scrollbar rendering for `ScrollWidget`.
//!
//! Handles scrollbar drawing (track + thumb), visibility policy, and
//! geometry calculations (track rect, thumb rect).

use crate::draw::RectStyle;
use crate::geometry::Rect;

use super::{DrawCtx, ScrollWidget, ScrollbarPolicy};

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
    pub(super) fn scrollbar_track_rect(&self, viewport: Rect) -> Rect {
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
    pub(super) fn scrollbar_thumb_rect(
        &self,
        viewport: Rect,
        content_height: f32,
        view_height: f32,
    ) -> Rect {
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
            ctx.scene.push_quad(track, track_style);
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
        ctx.scene.push_quad(thumb, style);
    }
}
