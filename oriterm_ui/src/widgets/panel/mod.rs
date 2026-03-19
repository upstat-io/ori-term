//! Panel widget — a visual container with background, border, and shadow.
//!
//! Wraps a single child widget with configurable styling. Used for card-style
//! layouts, dialog backgrounds, and settings panels.

use std::cell::RefCell;
use std::rc::Rc;

use crate::color::Color;
use crate::draw::{RectStyle, Shadow};
use crate::geometry::Insets;
use crate::layout::{LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::widget_id::WidgetId;

use crate::theme::UiTheme;

use super::{DrawCtx, LayoutCtx, Widget};

/// Visual style for a [`PanelWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct PanelStyle {
    /// Background fill color.
    pub bg: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width in logical pixels.
    pub border_width: f32,
    /// Uniform corner radius.
    pub corner_radius: f32,
    /// Inner padding between panel edge and child.
    pub padding: Insets,
    /// Optional drop shadow.
    pub shadow: Option<Shadow>,
}

impl PanelStyle {
    /// Derives a panel style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        Self {
            bg: theme.bg_card,
            border_color: theme.border,
            border_width: 1.0,
            corner_radius: theme.corner_radius * 2.0,
            padding: Insets::all(12.0),
            shadow: Some(Shadow {
                offset_x: 0.0,
                offset_y: 2.0,
                blur_radius: 8.0,
                spread: 0.0,
                color: theme.shadow,
            }),
        }
    }
}

impl Default for PanelStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}

/// A styled container wrapping a single child widget.
///
/// Draws a background rectangle (with optional border, radius, and shadow)
/// behind the child. The child is positioned within the panel's padding area.
pub struct PanelWidget {
    id: WidgetId,
    child: Box<dyn Widget>,
    style: PanelStyle,
    /// Cached layout result, keyed by bounds.
    cached_layout: RefCell<Option<(crate::geometry::Rect, Rc<LayoutNode>)>>,
}

impl PanelWidget {
    /// Creates a panel wrapping the given child widget.
    pub fn new(child: Box<dyn Widget>) -> Self {
        Self {
            id: WidgetId::next(),
            child,
            style: PanelStyle::default(),
            cached_layout: RefCell::new(None),
        }
    }

    /// Sets the panel style.
    #[must_use]
    pub fn with_style(mut self, style: PanelStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets the background color.
    #[must_use]
    pub fn with_bg(mut self, bg: Color) -> Self {
        self.style.bg = bg;
        self
    }

    /// Sets the corner radius.
    #[must_use]
    pub fn with_corner_radius(mut self, radius: f32) -> Self {
        self.style.corner_radius = radius;
        self
    }

    /// Sets the inner padding.
    #[must_use]
    pub fn with_padding(mut self, padding: Insets) -> Self {
        self.style.padding = padding;
        self
    }

    /// Sets the drop shadow.
    #[must_use]
    pub fn with_shadow(mut self, shadow: Shadow) -> Self {
        self.style.shadow = Some(shadow);
        self
    }

    /// Returns cached layout if bounds match, otherwise recomputes.
    fn get_or_compute_layout(
        &self,
        measurer: &dyn super::TextMeasurer,
        theme: &UiTheme,
        bounds: crate::geometry::Rect,
    ) -> Rc<LayoutNode> {
        {
            let cached = self.cached_layout.borrow();
            if let Some((ref cb, ref node)) = *cached {
                if *cb == bounds {
                    return Rc::clone(node);
                }
            }
        }
        let ctx = LayoutCtx { measurer, theme };
        let child_box = self.child.layout(&ctx);
        let wrapper = LayoutBox::flex(crate::layout::Direction::Column, vec![child_box])
            .with_padding(self.style.padding)
            .with_width(SizeSpec::Fill)
            .with_height(SizeSpec::Fill)
            .with_widget_id(self.id);
        let node = Rc::new(compute_layout(&wrapper, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }
}

impl Widget for PanelWidget {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let child_box = self.child.layout(ctx);
        LayoutBox::flex(crate::layout::Direction::Column, vec![child_box])
            .with_padding(self.style.padding)
            .with_widget_id(self.id)
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        // Invalidate cache each frame so children with changed intrinsic sizes
        // get fresh layout.
        *self.cached_layout.borrow_mut() = None;

        // Layer captures the panel bg for subpixel text compositing.
        ctx.scene.push_layer_bg(self.style.bg);

        // Draw panel background.
        let mut rect_style = RectStyle::filled(self.style.bg).with_radius(self.style.corner_radius);
        if self.style.border_width > 0.0 {
            rect_style = rect_style.with_border(self.style.border_width, self.style.border_color);
        }
        if let Some(shadow) = self.style.shadow {
            rect_style = rect_style.with_shadow(shadow);
        }
        ctx.scene.push_quad(ctx.bounds, rect_style);

        // Compute child layout and draw child.
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(child_node) = layout.children.first() {
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                scene: ctx.scene,
                bounds: child_node.content_rect,
                now: ctx.now,
                theme: ctx.theme,
                icons: ctx.icons,
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            self.child.paint(&mut child_ctx);
        }

        ctx.scene.pop_layer_bg();
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(self.child.as_mut());
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.child.focusable_children()
    }
}

#[cfg(test)]
mod tests;
