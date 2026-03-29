//! Layout construction helpers for `ContainerWidget`.
//!
//! Extracted from `mod.rs` to keep the main file under 500 lines.

use std::rc::Rc;

use crate::geometry::Rect;
use crate::layout::{LayoutBox, LayoutNode, compute_layout};
use crate::theme::UiTheme;

use super::{ContainerWidget, LayoutCtx, TextMeasurer};

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
}
