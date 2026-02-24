//! Drawing and layout-caching methods for the dialog widget.

use std::rc::Rc;

use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::layout::{LayoutNode, compute_layout};
use crate::theme::UiTheme;

use super::{DialogContent, DialogWidget, DrawCtx, LayoutCtx};
use crate::widgets::Widget;

impl DialogWidget {
    /// Returns cached layout if bounds match, otherwise recomputes.
    pub(super) fn get_or_compute_layout(
        &self,
        measurer: &dyn super::super::TextMeasurer,
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
        let layout_box = self.build_layout(&ctx);
        let node = Rc::new(compute_layout(&layout_box, bounds));
        *self.cached_layout.borrow_mut() = Some((bounds, Rc::clone(&node)));
        node
    }

    /// Draw the buttons inside the footer layout node.
    pub(super) fn draw_buttons(&self, ctx: &mut DrawCtx<'_>, footer_node: &LayoutNode) {
        for (i, btn_node) in footer_node.children.iter().enumerate() {
            let (button, btn_kind) = self.button_at_index_ref(i);
            let focused_id = if self.focus_visible && self.focused_button == btn_kind {
                Some(button.id())
            } else {
                None
            };
            let mut btn_ctx = DrawCtx {
                measurer: ctx.measurer,
                draw_list: ctx.draw_list,
                bounds: btn_node.content_rect,
                focused_widget: focused_id,
                now: ctx.now,
                animations_running: ctx.animations_running,
                theme: ctx.theme,
            };
            button.draw(&mut btn_ctx);
        }
    }

    /// Draw the content zone: title, message, and optional preview.
    pub(super) fn draw_content(&self, ctx: &mut DrawCtx<'_>, content_zone: &LayoutNode) {
        // Title.
        if let Some(title_node) = content_zone.children.first() {
            if !self.title.is_empty() {
                let s = self.title_style();
                let shaped = ctx
                    .measurer
                    .shape(&self.title, &s, title_node.content_rect.width());
                ctx.draw_list.push_text(
                    Point::new(title_node.content_rect.x(), title_node.content_rect.y()),
                    shaped,
                    self.style.title_fg,
                );
            }
        }

        // Message.
        if let Some(msg_node) = content_zone.children.get(1) {
            if !self.message.is_empty() {
                let s = self.message_style();
                let shaped = ctx
                    .measurer
                    .shape(&self.message, &s, msg_node.content_rect.width());
                ctx.draw_list.push_text(
                    Point::new(msg_node.content_rect.x(), msg_node.content_rect.y()),
                    shaped,
                    self.style.message_fg,
                );
            }
        }

        // Optional preview block.
        if let Some(ref content) = self.content {
            if let Some(preview_node) = content_zone.children.get(2) {
                self.draw_preview(ctx, preview_node, content);
            }
        }
    }

    /// Draw the preview block: background rect, per-line text, overflow ellipsis.
    fn draw_preview(&self, ctx: &mut DrawCtx<'_>, node: &LayoutNode, content: &DialogContent) {
        let rect = node.content_rect;

        // Preview background (darker, rounded).
        let preview_rect_style =
            RectStyle::filled(self.style.preview_bg).with_radius(self.style.preview_radius);
        ctx.draw_list.push_rect(rect, preview_rect_style);

        // Clip to prevent overflow, push layer for subpixel compositing.
        ctx.draw_list.push_clip(rect);
        ctx.draw_list.push_layer(self.style.preview_bg);

        let s = self.preview_text_style();
        let inner_w = rect.width() - self.style.preview_padding.width();
        let x = rect.x() + self.style.preview_padding.left;
        let y_start = rect.y() + self.style.preview_padding.top;
        let y_limit = rect.bottom() - self.style.preview_padding.bottom;

        // Measure single line height.
        let line_m = ctx.measurer.measure("X", &s, inner_w);
        let line_h = line_m.height;

        // Draw each line, stopping when we'd overflow the preview box.
        let mut y = y_start;
        let mut overflowed = false;
        for line in content.text.lines() {
            if y + line_h > y_limit {
                overflowed = true;
                break;
            }
            let shaped = ctx.measurer.shape(line, &s, inner_w);
            ctx.draw_list
                .push_text(Point::new(x, y), shaped, self.style.message_fg);
            y += line_h;
        }

        // Draw ellipsis indicator when content overflows the preview box.
        if overflowed {
            let ellipsis_y = y_limit - line_h;
            let shaped = ctx.measurer.shape("\u{2026}", &s, inner_w);
            ctx.draw_list
                .push_text(Point::new(x, ellipsis_y), shaped, self.style.message_fg);
        }

        ctx.draw_list.pop_layer();
        ctx.draw_list.pop_clip();
    }

    /// Draw the footer zone: separator, background, buttons.
    ///
    /// The footer bg is drawn as a sharp rect (radius 0) because the GPU
    /// shader only supports uniform corner radius. The dialog's base rect
    /// (drawn in `footer_bg` with full radius) provides the rounded bottom
    /// corners; this sharp rect sits inside it without leaking.
    pub(super) fn draw_footer(&self, ctx: &mut DrawCtx<'_>, footer_node: &LayoutNode) {
        let sep_y = footer_node.rect.y();

        // 1px separator line at the top of the footer zone.
        ctx.draw_list.push_line(
            Point::new(ctx.bounds.x(), sep_y),
            Point::new(ctx.bounds.right(), sep_y),
            1.0,
            self.style.separator_color,
        );

        // Footer background — inset from the dialog edges so the base
        // rect's border and rounded bottom corners remain visible.
        let r = self.style.corner_radius;
        let bw = self.style.border_width;
        let footer_rect = Rect::new(
            ctx.bounds.x() + bw,
            sep_y,
            ctx.bounds.width() - bw * 2.0,
            (ctx.bounds.bottom() - sep_y - r).max(0.0),
        );
        ctx.draw_list.push_layer(self.style.footer_bg);
        ctx.draw_list
            .push_rect(footer_rect, RectStyle::filled(self.style.footer_bg));

        self.draw_buttons(ctx, footer_node);

        ctx.draw_list.pop_layer();
    }
}
