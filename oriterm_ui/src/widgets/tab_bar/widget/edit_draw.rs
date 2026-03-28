//! Tab title inline editor rendering.
//!
//! Draws the editable text field (with cursor and selection highlight)
//! when a tab is being renamed via double-click.

use crate::draw::RectStyle;
use crate::geometry::{Point, Rect};
use crate::text::{TextOverflow, TextStyle};
use crate::widgets::DrawCtx;

use super::TabBarWidget;
use super::draw::TabStrip;

/// Safely slice text at a byte position that is guaranteed to be a char boundary.
///
/// `TextEditingState` maintains this invariant, but Clippy warns about
/// direct string indexing. This helper satisfies the lint.
fn safe_slice(text: &str, end: usize) -> &str {
    text.get(..end).unwrap_or(text)
}

/// Safely slice a range within text.
fn safe_range(text: &str, start: usize, end: usize) -> &str {
    text.get(start..end).unwrap_or("")
}

impl TabBarWidget {
    /// Draws the inline text editor for a tab being renamed.
    ///
    /// Renders the editing text with cursor and optional selection highlight
    /// in place of the normal tab label.
    pub(super) fn draw_tab_editor(&self, ctx: &mut DrawCtx<'_>, x: f32, strip: &TabStrip) {
        let color = strip.text_color;
        let text = self.editing.text();
        let display_text = if text.is_empty() { "Terminal" } else { text };

        let max_w = self.layout.max_text_width().max(0.0);
        let text_style = TextStyle::new(ctx.theme.font_size_small, color);
        let shaped = ctx.measurer.shape(display_text, &text_style, f32::INFINITY);
        let text_x = x + self.metrics.tab_padding;
        let text_y = strip.y + (strip.h - shaped.height) / 2.0;

        // Measure cursor and selection positions before consuming text_style.
        let cursor_pos = self.editing.cursor();
        let cursor_x_offset = if text.is_empty() {
            0.0
        } else {
            ctx.measurer
                .measure(safe_slice(text, cursor_pos), &text_style, f32::INFINITY)
                .width
        };

        // Selection highlight.
        if let Some((sel_start, sel_end)) = self.editing.selection_range() {
            if sel_start != sel_end && !text.is_empty() {
                let pre_sel =
                    ctx.measurer
                        .measure(safe_slice(text, sel_start), &text_style, f32::INFINITY);
                let sel_text = ctx.measurer.measure(
                    safe_range(text, sel_start, sel_end),
                    &text_style,
                    f32::INFINITY,
                );
                let sel_rect = Rect::new(
                    text_x + pre_sel.width,
                    text_y,
                    sel_text.width.min(max_w),
                    shaped.height,
                );
                let sel_color = ctx.theme.accent_bg.with_alpha(0.4);
                ctx.scene.push_quad(sel_rect, RectStyle::filled(sel_color));
            }
        }

        // Clamp shaped text to available width.
        let clamped = ctx.measurer.shape(
            display_text,
            &text_style.with_overflow(TextOverflow::Ellipsis),
            max_w,
        );
        ctx.scene
            .push_text(Point::new(text_x, text_y), clamped, color);

        // Cursor (thin vertical line).
        let cursor_rect = Rect::new(text_x + cursor_x_offset, text_y, 1.0, shaped.height);
        ctx.scene.push_quad(cursor_rect, RectStyle::filled(color));
    }
}
