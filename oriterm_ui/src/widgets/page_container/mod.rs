//! Page container widget — shows one child page at a time.
//!
//! Holds a vector of page widgets and displays only the active page.
//! Switches pages in response to `WidgetAction::Selected` from a paired
//! navigation widget (e.g., `SidebarNav`). Only `Selected` actions whose
//! `id` matches the registered navigation source trigger page switches;
//! other `Selected` actions (e.g., from `SchemeCard`, `CursorPicker`) are
//! ignored by the page container.

use crate::layout::{Direction, LayoutBox, SizeSpec};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, Widget, WidgetAction};

/// A container that shows one child page at a time.
///
/// Pages are stored as `Box<dyn Widget>`. Only the active page participates
/// in layout and paint. Switches pages when it receives a `Selected` action
/// whose `id` matches the registered navigation source.
pub struct PageContainerWidget {
    id: WidgetId,
    pages: Vec<Box<dyn Widget>>,
    active_page: usize,
    /// Widget ID of the paired navigation source (e.g., `SidebarNavWidget`).
    /// Only `Selected` actions from this source trigger page switches.
    nav_source_id: Option<WidgetId>,
}

impl PageContainerWidget {
    /// Creates a page container with the given pages.
    pub fn new(pages: Vec<Box<dyn Widget>>) -> Self {
        Self {
            id: WidgetId::next(),
            pages,
            active_page: 0,
            nav_source_id: None,
        }
    }

    /// Registers the navigation source whose `Selected` actions trigger page switches.
    ///
    /// Only `Selected` actions with an `id` matching this source will switch pages.
    /// All other `Selected` actions (from child widgets like `SchemeCard`,
    /// `CursorPicker`, etc.) are passed through without triggering a page switch.
    #[must_use]
    pub fn with_nav_source(mut self, id: WidgetId) -> Self {
        self.nav_source_id = Some(id);
        self
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
            scene: ctx.scene,
            bounds: ctx.bounds,
            now: ctx.now,
            theme: ctx.theme,
            icons: ctx.icons,
            interaction: ctx.interaction,
            widget_id: Some(page.id()),
            frame_requests: ctx.frame_requests,
        };
        page.paint(&mut child_ctx);
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        if let Some(page) = self.pages.get_mut(self.active_page) {
            visitor(page.as_mut());
        }
    }

    fn for_each_child_mut_all(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
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
        // Check for page switch — only from the paired navigation source.
        if let WidgetAction::Selected { id, index } = action {
            let from_nav = self.nav_source_id.is_some_and(|nav_id| nav_id == *id);
            if from_nav && *index < self.pages.len() && *index != self.active_page {
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
