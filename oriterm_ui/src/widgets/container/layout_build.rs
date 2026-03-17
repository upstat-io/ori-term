//! Layout construction and cache management for `ContainerWidget`.
//!
//! Extracted from `mod.rs` to keep the main file under 500 lines.

use std::rc::Rc;

use crate::geometry::Rect;
use crate::layout::{LayoutBox, LayoutNode, compute_layout};
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{ContainerWidget, DrawCtx, LayoutCtx, TextMeasurer};

impl ContainerWidget {
    /// Returns cached layout if bounds match and layout is clean, otherwise recomputes.
    pub(super) fn get_or_compute_layout(
        &self,
        measurer: &dyn TextMeasurer,
        theme: &UiTheme,
        bounds: Rect,
    ) -> Rc<LayoutNode> {
        if !self.needs_layout {
            let cached = self.cached_layout.borrow();
            if let Some((ref cb, ref node)) = *cached {
                if *cb == bounds {
                    return Rc::clone(node);
                }
            }
        }
        let ctx = LayoutCtx { measurer, theme };
        let layout_box = self.build_layout_box(&ctx);
        let node = Rc::new(compute_layout(&layout_box, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }

    /// Builds the `LayoutBox` descriptor tree from children.
    pub(super) fn build_layout_box(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let child_boxes: Vec<LayoutBox> = self.children.iter().map(|c| c.layout(ctx)).collect();
        self.layout_mode
            .build(child_boxes)
            .with_padding(self.padding)
            .with_width(self.width)
            .with_height(self.height)
            .with_widget_id(self.id)
            .with_clip(self.clip_children)
    }

    // Scene cache helpers

    /// Tries to replay cached draw commands for a child widget.
    ///
    /// Returns `true` if the cache hit and commands were replayed.
    pub(super) fn try_replay_cached(
        ctx: &mut DrawCtx<'_>,
        child_id: WidgetId,
        bounds: Rect,
    ) -> bool {
        let cache = match ctx.scene_cache.as_ref() {
            Some(c) => c,
            None => return false,
        };
        let node = match cache.get(child_id) {
            Some(n) if n.is_valid() && n.bounds() == bounds => n,
            _ => return false,
        };
        ctx.draw_list.extend_from_cache(node.commands());
        true
    }

    /// Stores a child's draw output in the scene cache for future reuse.
    ///
    /// `log_start` is the store-log position captured before the child's
    /// draw. All IDs stored between then and now are recorded as contained
    /// descendants of this child's cache entry.
    pub(super) fn store_in_cache(
        ctx: &mut DrawCtx<'_>,
        child_id: WidgetId,
        bounds: Rect,
        start: usize,
        log_start: usize,
    ) {
        if let Some(cache) = ctx.scene_cache.as_deref_mut() {
            let commands = ctx.draw_list.commands()[start..].to_vec();
            cache.store(child_id, commands, bounds, log_start);
        }
    }
}
