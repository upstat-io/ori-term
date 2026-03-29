//! Dialog cursor resolution helpers.
//!
//! Extracted from `event_handling/mod.rs` to keep that file under 500 lines.

use std::time::Instant;

use oriterm_ui::geometry::{Point, Rect};
use oriterm_ui::input::{MouseEvent, MouseEventKind, layout_hit_test_disabled_at};
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::LayoutCtx;
use winit::window::CursorIcon;

use crate::app::App;
use crate::font::CachedTextMeasurer;

use super::super::DialogWindowContext;

impl App {
    /// Routes a cursor move to the overlay manager. Returns `true` if consumed.
    pub(super) fn route_overlay_hover(
        ctx: &mut DialogWindowContext,
        logical_pos: Point,
        scale: f32,
        theme: &oriterm_ui::theme::UiTheme,
    ) -> bool {
        if ctx.root.overlays().is_empty() {
            return false;
        }
        let Some(renderer) = ctx.renderer.as_ref() else {
            return false;
        };
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
        let move_event = MouseEvent {
            kind: MouseEventKind::Move,
            pos: logical_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let result = ctx.root.process_overlay_mouse_event(
            &move_event,
            &measurer,
            theme,
            None,
            Instant::now(),
        );
        if matches!(result, OverlayEventResult::Delivered { .. }) {
            let hot_ids = ctx.root.interaction_mut().update_hot_path(&[]);
            ctx.root.mark_widgets_prepaint_dirty(&hot_ids);
            let cursor = ctx
                .root
                .overlays()
                .cursor_icon_at(logical_pos)
                .unwrap_or(CursorIcon::Default);
            if cursor != ctx.last_cursor_icon {
                ctx.window.set_cursor(cursor);
                ctx.last_cursor_icon = cursor;
            }
            ctx.request_urgent_redraw();
            return true;
        }
        false
    }

    /// Hit-tests dialog content, resolves cursor icon, and populates `hot_path`.
    pub(super) fn hit_test_content(
        ctx: &mut DialogWindowContext,
        logical_pos: Point,
        scale: f32,
        theme: &oriterm_ui::theme::UiTheme,
        hot_path: &mut Vec<oriterm_ui::widget_id::WidgetId>,
    ) {
        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let viewport = Rect::new(0.0, 0.0, w, h);
        let layout_node = {
            let layout_ctx = LayoutCtx {
                measurer: &measurer,
                theme,
            };
            let layout_box = ctx.content.content_widget().layout(&layout_ctx);
            std::rc::Rc::new(oriterm_ui::layout::compute_layout(&layout_box, viewport))
        };
        let hit = oriterm_ui::input::layout_hit_test_path(&layout_node, logical_pos);
        for entry in &hit.path {
            hot_path.push(entry.widget_id);
        }

        // Resolve cursor from hit path leaf, falling back to disabled scan.
        let leaf_cursor = hit
            .path
            .last()
            .map_or(CursorIcon::Default, |e| e.cursor_icon);
        let cursor = if leaf_cursor != CursorIcon::Default {
            leaf_cursor
        } else if layout_hit_test_disabled_at(&layout_node, logical_pos) {
            CursorIcon::NotAllowed
        } else {
            CursorIcon::Default
        };
        if cursor != ctx.last_cursor_icon {
            ctx.window.set_cursor(cursor);
            ctx.last_cursor_icon = cursor;
        }

        ctx.cached_layout = Some((viewport, layout_node));
    }
}
