//! Page container widget — shows one child page at a time.
//!
//! Holds a vector of page widgets and displays only the active page.
//! Switches pages in response to `WidgetAction::Selected` from a paired
//! navigation widget (e.g., `SidebarNav`).

use crate::layout::{Direction, LayoutBox, SizeSpec};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// A container that shows one child page at a time.
///
/// Pages are stored as `Box<dyn Widget>`. Only the active page participates
/// in layout and paint. Switches pages when it receives a `Selected` action
/// that no child widget handles (typically from a `SidebarNavWidget`).
pub struct PageContainerWidget {
    id: WidgetId,
    pages: Vec<Box<dyn Widget>>,
    active_page: usize,
}

impl PageContainerWidget {
    /// Creates a page container with the given pages.
    pub fn new(pages: Vec<Box<dyn Widget>>) -> Self {
        Self {
            id: WidgetId::next(),
            pages,
            active_page: 0,
        }
    }

    /// Returns the active page index.
    pub fn active_page(&self) -> usize {
        self.active_page
    }

    /// Switches to the given page index.
    ///
    /// Does nothing if the index is out of range.
    pub fn set_active_page(&mut self, index: usize) {
        if index < self.pages.len() {
            self.active_page = index;
        }
    }

    /// Returns the number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }
}

impl Widget for PageContainerWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        if let Some(page) = self.pages.get(self.active_page) {
            let child = page.layout(ctx);
            LayoutBox::flex(Direction::Column, vec![child])
                .with_width(SizeSpec::Fill)
                .with_height(SizeSpec::Fill)
                .with_widget_id(self.id)
        } else {
            LayoutBox::leaf(0.0, 0.0).with_widget_id(self.id)
        }
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let Some(page) = self.pages.get(self.active_page) else {
            return;
        };
        let mut child_ctx = DrawCtx {
            measurer: ctx.measurer,
            draw_list: ctx.draw_list,
            bounds: ctx.bounds,
            now: ctx.now,
            theme: ctx.theme,
            icons: ctx.icons,
            scene_cache: ctx.scene_cache.as_deref_mut(),
            interaction: None,
            widget_id: None,
            frame_requests: None,
        };
        page.paint(&mut child_ctx);
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        for page in &mut self.pages {
            visitor(page.as_mut());
        }
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        // Propagate to the active page first — its widgets may handle it.
        if let Some(page) = self.pages.get_mut(self.active_page) {
            if page.accept_action(action) {
                return true;
            }
        }
        // No child handled it — check for page switch.
        if let WidgetAction::Selected { index, .. } = action {
            if *index < self.pages.len() && *index != self.active_page {
                self.active_page = *index;
                // Reset scroll on the newly-active page.
                self.pages[self.active_page].reset_scroll();
                return true;
            }
        }
        false
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.pages
            .get(self.active_page)
            .map_or_else(Vec::new, |p| p.focusable_children())
    }
}

#[cfg(test)]
mod tests;
