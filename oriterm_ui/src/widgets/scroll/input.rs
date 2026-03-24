//! Input handling for `ScrollWidget`.
//!
//! Keyboard scroll, scrollbar thumb drag, track click-to-jump,
//! and hover detection for both vertical and horizontal axes.

use crate::geometry::{Point, Rect};
use crate::input::Key;

use super::super::scrollbar::{
    ScrollbarAxis, ScrollbarMetrics, compute_rects, drag_delta_to_offset, pointer_to_offset,
};
use super::{ScrollWidget, ScrollbarPolicy};

impl ScrollWidget {
    /// Handles scroll-related key presses (arrows, PageUp/Down, Home/End).
    pub(super) fn handle_scroll_key(&mut self, key: Key, content_h: f32, view_h: f32) -> bool {
        match key {
            Key::ArrowUp => {
                self.scroll_by(-self.line_height, content_h, view_h);
                true
            }
            Key::ArrowDown => {
                self.scroll_by(self.line_height, content_h, view_h);
                true
            }
            Key::PageUp => {
                self.scroll_by(-view_h, content_h, view_h);
                true
            }
            Key::PageDown => {
                self.scroll_by(view_h, content_h, view_h);
                true
            }
            Key::Home => {
                self.scroll_offset = 0.0;
                true
            }
            Key::End => {
                self.scroll_offset = (content_h - view_h).max(0.0);
                true
            }
            _ => false,
        }
    }

    /// Handles mouse-down on scrollbar (thumb drag start or track click).
    pub(super) fn handle_scrollbar_down(
        &mut self,
        pos: Point,
        viewport: Rect,
        content_w: f32,
        content_h: f32,
    ) -> bool {
        if self.scrollbar_policy == ScrollbarPolicy::Hidden {
            return false;
        }

        let view_w = viewport.width();
        let view_h = viewport.height();
        let v_visible = self.has_vertical() && content_h > view_h;
        let h_visible = self.has_horizontal() && content_w > view_w;
        let reserve = self.reserve_far_edge(v_visible && h_visible);

        // Check vertical bar.
        if v_visible {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Vertical,
                content_extent: content_h,
                view_extent: view_h,
                scroll_offset: self.scroll_offset,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            if rects.thumb_hit.contains(pos) {
                self.v_bar.dragging = true;
                self.v_bar.drag_start_pointer = pos.y;
                self.v_bar.drag_start_offset = self.scroll_offset;
                return true;
            }
            if rects.track_hit.contains(pos) {
                let offset = pointer_to_offset(pos.y, &rects, &m);
                self.scroll_offset = offset;
                return true;
            }
        }

        // Check horizontal bar.
        if h_visible {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Horizontal,
                content_extent: content_w,
                view_extent: view_w,
                scroll_offset: self.scroll_offset_x,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            if rects.thumb_hit.contains(pos) {
                self.h_bar.dragging = true;
                self.h_bar.drag_start_pointer = pos.x;
                self.h_bar.drag_start_offset = self.scroll_offset_x;
                return true;
            }
            if rects.track_hit.contains(pos) {
                let offset = pointer_to_offset(pos.x, &rects, &m);
                self.scroll_offset_x = offset;
                return true;
            }
        }

        false
    }

    /// Handles mouse-move for scrollbar drag and hover detection.
    pub(super) fn handle_scrollbar_move(
        &mut self,
        pos: Point,
        viewport: Rect,
        content_w: f32,
        content_h: f32,
    ) -> bool {
        let view_w = viewport.width();
        let view_h = viewport.height();
        let v_visible = self.has_vertical() && content_h > view_h;
        let h_visible = self.has_horizontal() && content_w > view_w;
        let reserve = self.reserve_far_edge(v_visible && h_visible);

        let mut changed = false;

        // Vertical drag.
        if self.v_bar.dragging {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Vertical,
                content_extent: content_h,
                view_extent: view_h,
                scroll_offset: self.scroll_offset,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            let delta = pos.y - self.v_bar.drag_start_pointer;
            let offset_delta = drag_delta_to_offset(delta, &rects, &m);
            let max = (content_h - view_h).max(0.0);
            self.scroll_offset = (self.v_bar.drag_start_offset + offset_delta).clamp(0.0, max);
            return true;
        }

        // Horizontal drag.
        if self.h_bar.dragging {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Horizontal,
                content_extent: content_w,
                view_extent: view_w,
                scroll_offset: self.scroll_offset_x,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            let delta = pos.x - self.h_bar.drag_start_pointer;
            let offset_delta = drag_delta_to_offset(delta, &rects, &m);
            let max = (content_w - view_w).max(0.0);
            self.scroll_offset_x = (self.h_bar.drag_start_offset + offset_delta).clamp(0.0, max);
            return true;
        }

        // Hover detection for vertical bar.
        if v_visible {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Vertical,
                content_extent: content_h,
                view_extent: view_h,
                scroll_offset: self.scroll_offset,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            let was = self.v_bar.track_hovered;
            self.v_bar.track_hovered = rects.track_hit.contains(pos);
            self.v_bar.thumb_hovered = rects.thumb_hit.contains(pos);
            changed |= was != self.v_bar.track_hovered;
        }

        // Hover detection for horizontal bar.
        if h_visible {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Horizontal,
                content_extent: content_w,
                view_extent: view_w,
                scroll_offset: self.scroll_offset_x,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            let was = self.h_bar.track_hovered;
            self.h_bar.track_hovered = rects.track_hit.contains(pos);
            self.h_bar.thumb_hovered = rects.thumb_hit.contains(pos);
            changed |= was != self.h_bar.track_hovered;
        }

        changed
    }
}
