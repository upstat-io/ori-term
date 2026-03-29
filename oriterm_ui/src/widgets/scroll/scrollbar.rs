//! Scrollbar rendering for `ScrollWidget`.
//!
//! Delegates to the shared scrollbar geometry and drawing helpers in
//! [`super::super::scrollbar`], selecting the correct per-axis state.

use super::super::scrollbar::{
    ScrollbarAxis, ScrollbarMetrics, compute_rects, draw_overlay, should_show,
};
use super::{DrawCtx, ScrollWidget, ScrollbarPolicy};

impl ScrollWidget {
    /// Whether a scrollbar on the given axis should be visible.
    fn bar_visible(&self, axis: ScrollbarAxis, content: f32, view: f32) -> bool {
        match self.scrollbar_policy {
            ScrollbarPolicy::Auto => should_show(&ScrollbarMetrics {
                axis,
                content_extent: content,
                view_extent: view,
                scroll_offset: 0.0,
            }),
            ScrollbarPolicy::Always => true,
            ScrollbarPolicy::Hidden => false,
        }
    }

    /// Draws overlay scrollbars for all active axes.
    pub(super) fn draw_scrollbars(&self, ctx: &mut DrawCtx<'_>, content_w: f32, content_h: f32) {
        let vp = ctx.bounds;
        let view_w = vp.width();
        let view_h = vp.height();

        let v_vis =
            self.has_vertical() && self.bar_visible(ScrollbarAxis::Vertical, content_h, view_h);
        let h_vis =
            self.has_horizontal() && self.bar_visible(ScrollbarAxis::Horizontal, content_w, view_w);
        let reserve = self.reserve_far_edge(v_vis && h_vis);

        // Draw each visible axis bar using shared helpers.
        let bars: [(bool, ScrollbarAxis, f32, f32, f32); 2] = [
            (
                v_vis,
                ScrollbarAxis::Vertical,
                content_h,
                view_h,
                self.scroll_offset,
            ),
            (
                h_vis,
                ScrollbarAxis::Horizontal,
                content_w,
                view_w,
                self.scroll_offset_x,
            ),
        ];
        for &(visible, axis, content, view, offset) in &bars {
            if !visible {
                continue;
            }
            let m = ScrollbarMetrics {
                axis,
                content_extent: content,
                view_extent: view,
                scroll_offset: offset,
            };
            let rects = compute_rects(vp, &m, &self.scrollbar_style, reserve);
            let state = match axis {
                ScrollbarAxis::Vertical => self.v_bar.visual_state(),
                ScrollbarAxis::Horizontal => self.h_bar.visual_state(),
            };
            draw_overlay(ctx.scene, &rects, &self.scrollbar_style, state);
        }
    }
}
