//! Rendering logic for `ScrollWidget`.
//!
//! Contains the `draw()` body as an inherent method, delegated from
//! the `Widget` trait impl in `mod.rs`.

use crate::geometry::Rect;
use crate::widgets::DrawCtx;

use super::ScrollWidget;

impl ScrollWidget {
    /// Draws the scroll container: clips viewport, translates by scroll
    /// offset, draws child, pops transforms, then draws scrollbars on top.
    pub(super) fn draw_impl(&self, ctx: &mut DrawCtx<'_>) {
        let (content_w, content_h) = self.child_natural_size(ctx.measurer, ctx.theme, ctx.bounds);

        ctx.scene.push_clip(ctx.bounds);
        ctx.scene
            .push_offset(-self.scroll_offset_x, -self.scroll_offset);

        let child_bounds = Rect::new(ctx.bounds.x(), ctx.bounds.y(), content_w, content_h);
        let mut child_ctx = DrawCtx {
            measurer: ctx.measurer,
            scene: ctx.scene,
            bounds: child_bounds,
            now: ctx.now,
            theme: ctx.theme,
            icons: ctx.icons,
            interaction: ctx.interaction,
            widget_id: None,
            frame_requests: ctx.frame_requests,
        };
        self.child.paint(&mut child_ctx);

        ctx.scene.pop_offset();
        ctx.scene.pop_clip();

        // Draw scrollbars on top of content (outside translate/clip).
        self.draw_scrollbars(ctx, content_w, content_h);
    }
}
