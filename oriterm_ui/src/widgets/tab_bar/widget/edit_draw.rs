//! Inline editing overlay for tab title rename.
//!
//! Draws selection highlight and cursor behind the tab title text
//! when the user is actively editing via double-click.

use crate::draw::RectStyle;
use crate::geometry::Rect;
use crate::text::TextStyle;
use crate::text::editing::TextEditingState;
use crate::widgets::DrawCtx;

/// Layout parameters for the editing overlay.
pub(super) struct EditOverlayParams<'a> {
    pub editing: &'a TextEditingState,
    pub text_style: &'a TextStyle,
    pub text_x: f32,
    pub text_y: f32,
    pub max_w: f32,
    pub line_h: f32,
}

/// Draw selection highlight and cursor for inline tab title editing.
///
/// Called before the text is drawn so selection appears behind it.
pub(super) fn draw_editing_overlay(ctx: &mut DrawCtx<'_>, p: &EditOverlayParams<'_>) {
    let raw = p.editing.text();
    let color = p.text_style.color;

    // Selection highlight.
    if let Some((ss, se)) = p.editing.selection_range() {
        if ss != se && !raw.is_empty() {
            let pre = raw.get(..ss).unwrap_or(raw);
            let sel = raw.get(ss..se).unwrap_or("");
            let pre_w = ctx.measurer.measure(pre, p.text_style, f32::INFINITY).width;
            let sel_w = ctx.measurer.measure(sel, p.text_style, f32::INFINITY).width;
            let sel_rect = Rect::new(p.text_x + pre_w, p.text_y, sel_w.min(p.max_w), p.line_h);
            let sel_color = ctx.theme.accent_bg.with_alpha(0.4);
            ctx.scene.push_quad(sel_rect, RectStyle::filled(sel_color));
        }
    }

    // Cursor line.
    let cursor_pos = p.editing.cursor();
    let cursor_x = if raw.is_empty() {
        0.0
    } else {
        let pre = raw.get(..cursor_pos).unwrap_or(raw);
        ctx.measurer.measure(pre, p.text_style, f32::INFINITY).width
    };
    let cursor_rect = Rect::new(p.text_x + cursor_x, p.text_y, 1.0, p.line_h);
    ctx.scene.push_quad(cursor_rect, RectStyle::filled(color));
}
