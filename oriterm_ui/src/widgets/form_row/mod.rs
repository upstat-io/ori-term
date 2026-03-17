//! Form row widget — a label + control pair for settings panels.
//!
//! Each `FormRow` renders a text label on the left at a fixed column width
//! and a control widget on the right, filling the remaining space. Used
//! inside `FormSection` and `FormLayout` for aligned two-column forms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::{Point, Rect};
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::sense::Sense;
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{DrawCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction};

/// A form row containing a label and a control widget.
///
/// The label occupies a fixed-width column on the left, and the control
/// fills the remaining space on the right. The label column width is set
/// by the parent `FormLayout` for cross-row alignment.
pub struct FormRow {
    id: WidgetId,
    label: String,
    control: Box<dyn Widget>,

    /// Fixed label column width (set by parent `FormLayout` for alignment).
    label_width: f32,

    /// Cached layout result, keyed by bounds.
    cached_layout: RefCell<Option<(Rect, Rc<LayoutNode>)>>,
}

impl FormRow {
    /// Creates a form row with a label and control widget.
    pub fn new(label: impl Into<String>, control: Box<dyn Widget>) -> Self {
        Self {
            id: WidgetId::next(),
            label: label.into(),
            control,
            label_width: 100.0,
            cached_layout: RefCell::new(None),
        }
    }

    /// Sets the label column width (called by `FormLayout` for alignment).
    pub fn set_label_width(&mut self, width: f32) {
        self.label_width = width;
    }

    /// Returns the label text (for measurement by `FormLayout`).
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the label text style for the given theme.
    fn label_style(theme: &UiTheme) -> TextStyle {
        TextStyle::new(theme.font_size, theme.fg_secondary)
    }

    /// Measures the label width using the given measurer and theme.
    pub fn measure_label_width(&self, measurer: &dyn TextMeasurer, theme: &UiTheme) -> f32 {
        let style = Self::label_style(theme);
        measurer.measure(&self.label, &style, f32::INFINITY).width
    }
}

// Layout helpers.
impl FormRow {
    /// Returns cached layout if bounds match, otherwise recomputes.
    fn get_or_compute_layout(
        &self,
        measurer: &dyn TextMeasurer,
        theme: &UiTheme,
        bounds: Rect,
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
        let layout_box = self.build_layout_box(&ctx);
        let node = Rc::new(compute_layout(&layout_box, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }

    /// Builds a row layout: fixed-width label + fill control.
    fn build_layout_box(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        let label_style = Self::label_style(ctx.theme);
        let label_metrics = ctx
            .measurer
            .measure(&self.label, &label_style, f32::INFINITY);
        let label_box = LayoutBox::leaf(label_metrics.width, label_metrics.height)
            .with_width(SizeSpec::Fixed(self.label_width));

        let control_box = self.control.layout(ctx);

        LayoutBox::flex(Direction::Row, vec![label_box, control_box])
            .with_align(Align::Center)
            .with_widget_id(self.id)
    }
}

impl Widget for FormRow {
    fn id(&self) -> WidgetId {
        self.id
    }

    fn is_focusable(&self) -> bool {
        false
    }

    fn layout(&self, ctx: &LayoutCtx<'_>) -> LayoutBox {
        self.build_layout_box(ctx)
    }

    fn sense(&self) -> Sense {
        Sense::none()
    }

    fn paint(&self, ctx: &mut DrawCtx<'_>) {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);

        // Draw label text.
        if let Some(label_node) = layout.children.first() {
            if !self.label.is_empty() {
                let style = Self::label_style(ctx.theme);
                let max_w = label_node.content_rect.width();
                let shaped = ctx.measurer.shape(&self.label, &style, max_w);
                let pos = Point::new(label_node.content_rect.x(), label_node.content_rect.y());
                ctx.draw_list.push_text(pos, shaped, ctx.theme.fg_secondary);
            }
        }

        // Draw control.
        if let Some(control_node) = layout.children.get(1) {
            let mut child_ctx = DrawCtx {
                measurer: ctx.measurer,
                draw_list: ctx.draw_list,
                bounds: control_node.content_rect,
                focused_widget: ctx.focused_widget,
                now: ctx.now,
                theme: ctx.theme,
                icons: ctx.icons,
                scene_cache: ctx.scene_cache.as_deref_mut(),
                interaction: None,
                widget_id: None,
                frame_requests: None,
            };
            self.control.paint(&mut child_ctx);
        }
    }

    fn for_each_child_mut(&mut self, visitor: &mut dyn FnMut(&mut dyn Widget)) {
        visitor(self.control.as_mut());
    }

    fn accept_action(&mut self, action: &WidgetAction) -> bool {
        self.control.accept_action(action)
    }

    fn focusable_children(&self) -> Vec<WidgetId> {
        self.control.focusable_children()
    }
}

#[cfg(test)]
mod tests;
