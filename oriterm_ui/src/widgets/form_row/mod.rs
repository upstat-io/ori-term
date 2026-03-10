//! Form row widget — a label + control pair for settings panels.
//!
//! Each `FormRow` renders a text label on the left at a fixed column width
//! and a control widget on the right, filling the remaining space. Used
//! inside `FormSection` and `FormLayout` for aligned two-column forms.

use std::cell::RefCell;
use std::rc::Rc;

use crate::geometry::{Point, Rect};
use crate::input::{HoverEvent, KeyEvent, MouseEvent, MouseEventKind};
use crate::layout::{Align, Direction, LayoutBox, LayoutNode, SizeSpec, compute_layout};
use crate::text::TextStyle;
use crate::theme::UiTheme;
use crate::widget_id::WidgetId;

use super::{
    CaptureRequest, DrawCtx, EventCtx, LayoutCtx, TextMeasurer, Widget, WidgetAction,
    WidgetResponse,
};

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
    /// Whether the control widget has active mouse capture (drag in progress).
    captured: bool,
    /// Whether the control widget is currently hovered.
    control_hovered: bool,

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
            captured: false,
            control_hovered: false,
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

// Hover tracking.
impl FormRow {
    /// Updates hover state based on cursor position within the row.
    fn update_control_hover(
        &mut self,
        control_node: &LayoutNode,
        pos: Point,
        ctx: &EventCtx<'_>,
    ) -> WidgetResponse {
        let inside = control_node.rect.contains(pos);
        if inside && !self.control_hovered {
            // Entering the control.
            self.control_hovered = true;
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: control_node.content_rect,
                is_focused: ctx.focused_widget == Some(self.control.id()),
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            self.control.handle_hover(HoverEvent::Enter, &child_ctx);
            return WidgetResponse::paint();
        }
        if !inside && self.control_hovered {
            // Leaving the control.
            self.control_hovered = false;
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: control_node.content_rect,
                is_focused: ctx.focused_widget == Some(self.control.id()),
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            self.control.handle_hover(HoverEvent::Leave, &child_ctx);
            return WidgetResponse::paint();
        }
        WidgetResponse::ignored()
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

    fn draw(&self, ctx: &mut DrawCtx<'_>) {
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
                animations_running: ctx.animations_running,
                theme: ctx.theme,
                icons: ctx.icons,
            };
            self.control.draw(&mut child_ctx);
        }
    }

    fn handle_mouse(&mut self, event: &MouseEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);

        let Some(control_node) = layout.children.get(1) else {
            return WidgetResponse::ignored();
        };

        // Move events: update hover tracking (unless captured).
        if event.kind == MouseEventKind::Move && !self.captured {
            return self.update_control_hover(control_node, event.pos, ctx);
        }

        // During capture, route all events to the control regardless of position.
        let hit = self.captured || control_node.rect.contains(event.pos);
        if hit {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: control_node.content_rect,
                is_focused: ctx.focused_widget == Some(self.control.id()),
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            let resp = self.control.handle_mouse(event, &child_ctx);

            // Update capture state from child's request.
            match resp.capture {
                CaptureRequest::Acquire => self.captured = true,
                CaptureRequest::Release => self.captured = false,
                CaptureRequest::None => {
                    if matches!(event.kind, MouseEventKind::Up(_)) {
                        self.captured = false;
                    }
                }
            }
            return resp;
        }
        WidgetResponse::ignored()
    }

    fn handle_hover(&mut self, event: HoverEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        if event == HoverEvent::Leave {
            self.captured = false;
            if self.control_hovered {
                self.control_hovered = false;
                let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
                if let Some(control_node) = layout.children.get(1) {
                    let child_ctx = EventCtx {
                        measurer: ctx.measurer,
                        bounds: control_node.content_rect,
                        is_focused: ctx.focused_widget == Some(self.control.id()),
                        focused_widget: ctx.focused_widget,
                        theme: ctx.theme,
                    };
                    self.control.handle_hover(HoverEvent::Leave, &child_ctx);
                }
                return WidgetResponse::paint();
            }
        }
        // Enter is handled by Move-based hover tracking — ignore here.
        WidgetResponse::ignored()
    }

    fn handle_key(&mut self, event: KeyEvent, ctx: &EventCtx<'_>) -> WidgetResponse {
        let layout = self.get_or_compute_layout(ctx.measurer, ctx.theme, ctx.bounds);
        if let Some(control_node) = layout.children.get(1) {
            let child_ctx = EventCtx {
                measurer: ctx.measurer,
                bounds: control_node.content_rect,
                is_focused: ctx.focused_widget == Some(self.control.id()),
                focused_widget: ctx.focused_widget,
                theme: ctx.theme,
            };
            return self.control.handle_key(event, &child_ctx);
        }
        WidgetResponse::ignored()
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
