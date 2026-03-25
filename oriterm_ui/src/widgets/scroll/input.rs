//! Input handling for `ScrollWidget`.
//!
//! Keyboard scroll, scrollbar hover detection, and controller-dispatched
//! drag handling for both vertical and horizontal axes.
//!
//! Press/drag/release flows through `ScrollbarCaptureController` →
//! `on_action()`. Hover and scroll wheel remain in `on_input()`.

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

    /// Updates shared hit zones for the scrollbar capture controller.
    ///
    /// Called during paint so the controller has current geometry for
    /// the next event cycle.
    pub(super) fn update_hit_zones(&self, viewport: Rect, content_w: f32, content_h: f32) {
        if self.scrollbar_policy == ScrollbarPolicy::Hidden {
            *self.hit_zones.borrow_mut() = super::super::scrollbar::ScrollbarHitZones::default();
            return;
        }

        let view_w = viewport.width();
        let view_h = viewport.height();
        let v_visible = self.has_vertical() && content_h > view_h;
        let h_visible = self.has_horizontal() && content_w > view_w;
        let reserve = self.reserve_far_edge(v_visible && h_visible);

        let mut zones = self.hit_zones.borrow_mut();

        if v_visible {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Vertical,
                content_extent: content_h,
                view_extent: view_h,
                scroll_offset: self.scroll_offset,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            zones.v_thumb_hit = Some(rects.thumb_hit);
            zones.v_track_hit = Some(rects.track_hit);
        } else {
            zones.v_thumb_hit = None;
            zones.v_track_hit = None;
        }

        if h_visible {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Horizontal,
                content_extent: content_w,
                view_extent: view_w,
                scroll_offset: self.scroll_offset_x,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            zones.h_thumb_hit = Some(rects.thumb_hit);
            zones.h_track_hit = Some(rects.track_hit);
        } else {
            zones.h_thumb_hit = None;
            zones.h_track_hit = None;
        }
    }

    /// Handles `DragStart` from the controller — thumb drag or track click.
    pub(super) fn handle_drag_start(
        &mut self,
        pos: Point,
        viewport: Rect,
        content_w: f32,
        content_h: f32,
    ) {
        if self.scrollbar_policy == ScrollbarPolicy::Hidden {
            return;
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
                return;
            }
            if rects.track_hit.contains(pos) {
                let offset = pointer_to_offset(pos.y, &rects, &m);
                self.scroll_offset = offset;
                return;
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
                return;
            }
            if rects.track_hit.contains(pos) {
                let offset = pointer_to_offset(pos.x, &rects, &m);
                self.scroll_offset_x = offset;
            }
        }
    }

    /// Handles `DragUpdate` from the controller — thumb drag tracking.
    pub(super) fn handle_drag_update(
        &mut self,
        total_delta: Point,
        viewport: Rect,
        content_w: f32,
        content_h: f32,
    ) {
        let view_w = viewport.width();
        let view_h = viewport.height();
        let v_visible = self.has_vertical() && content_h > view_h;
        let h_visible = self.has_horizontal() && content_w > view_w;
        let reserve = self.reserve_far_edge(v_visible && h_visible);

        if self.v_bar.dragging {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Vertical,
                content_extent: content_h,
                view_extent: view_h,
                scroll_offset: self.scroll_offset,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            let delta = total_delta.y;
            let offset_delta = drag_delta_to_offset(delta, &rects, &m);
            let max = (content_h - view_h).max(0.0);
            self.scroll_offset = (self.v_bar.drag_start_offset + offset_delta).clamp(0.0, max);
        } else if self.h_bar.dragging {
            let m = ScrollbarMetrics {
                axis: ScrollbarAxis::Horizontal,
                content_extent: content_w,
                view_extent: view_w,
                scroll_offset: self.scroll_offset_x,
            };
            let rects = compute_rects(viewport, &m, &self.scrollbar_style, reserve);
            let delta = total_delta.x;
            let offset_delta = drag_delta_to_offset(delta, &rects, &m);
            let max = (content_w - view_w).max(0.0);
            self.scroll_offset_x = (self.h_bar.drag_start_offset + offset_delta).clamp(0.0, max);
        } else {
            // Neither axis is dragging — no-op.
        }
    }

    /// Handles mouse-move for hover detection (track/thumb hover state).
    ///
    /// Does NOT handle drag — that flows through the controller.
    pub(super) fn handle_scrollbar_hover(
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
