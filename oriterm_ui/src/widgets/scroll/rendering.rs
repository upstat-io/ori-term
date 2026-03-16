//! Rendering logic for `ScrollWidget`.
//!
//! Contains the `draw()` body as an inherent method, delegated from
//! the `Widget` trait impl in `mod.rs`.

use crate::geometry::Rect;
use crate::widgets::DrawCtx;

use super::ScrollWidget;

impl ScrollWidget {
    /// Draws the scroll container: clips viewport, translates by scroll
    /// offset, draws child, pops transforms, then draws scrollbar on top.
    pub(super) fn draw_impl(&self, ctx: &mut DrawCtx<'_>) {
        let (content_w, content_h) = self.child_natural_size(ctx.measurer, ctx.theme, ctx.bounds);

        // Clip to the viewport — emitted before the translate so it stays
        // in viewport space (the scroll container's visible area).
        ctx.draw_list.push_clip(ctx.bounds);

        // Apply scroll offset as a translate transform. The child draws at
        // its natural (unscrolled) position — bounds stay stable so the
        // child's SceneNode cache key (bounds) doesn't change on scroll.
        ctx.draw_list
            .push_translate(-self.scroll_offset_x, -self.scroll_offset);

        let child_bounds = Rect::new(ctx.bounds.x(), ctx.bounds.y(), content_w, content_h);
        let mut child_ctx = DrawCtx {
            measurer: ctx.measurer,
            draw_list: ctx.draw_list,
            bounds: child_bounds,
            focused_widget: ctx.focused_widget,
            now: ctx.now,
            animations_running: ctx.animations_running,
            theme: ctx.theme,
            icons: ctx.icons,
            scene_cache: ctx.scene_cache.as_deref_mut(),
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        self.child.paint(&mut child_ctx);

        ctx.draw_list.pop_translate();
        ctx.draw_list.pop_clip();

        // Draw scrollbar on top of content (outside translate).
        self.draw_scrollbar(ctx, content_h, ctx.bounds.height());
    }
}
