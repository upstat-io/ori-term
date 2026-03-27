//! Dialog content key dispatch — routes keyboard events to dialog widgets.

use std::rc::Rc;
use std::time::Instant;

use oriterm_ui::geometry::Rect;
use oriterm_ui::interaction::build_parent_map;
use oriterm_ui::layout::compute_layout;
use oriterm_ui::widgets::{LayoutCtx, WidgetAction};
use winit::window::{CursorIcon, WindowId};

use crate::app::App;
use crate::app::widget_pipeline::{apply_dispatch_requests, collect_focusable_ids};
use crate::font::CachedTextMeasurer;

impl App {
    /// Route a key event through the controller pipeline to dialog content.
    ///
    /// Converts the winit key event to an `InputEvent`, then dispatches via
    /// `deliver_event_to_tree`. Reuses the cached layout tree when the
    /// viewport hasn't changed, avoiding a full layout + parent-map +
    /// focus-order rebuild on every keystroke.
    pub(in crate::app) fn dispatch_dialog_content_key(
        &mut self,
        window_id: WindowId,
        event: &winit::event::KeyEvent,
    ) -> Option<WidgetAction> {
        let input_event = super::key_conversion::winit_key_to_input_event(event, self.modifiers)?;

        let ctx = self.dialogs.get_mut(&window_id)?;
        let scale = ctx.scale_factor.factor() as f32;

        let chrome_h = ctx.chrome.caption_height();
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
        let local_viewport = Rect::new(0.0, 0.0, content_bounds.width(), content_bounds.height());

        // Reuse cached layout if viewport matches (avoids full tree rebuild
        // on every keystroke — the parent map and focus order are already
        // set from the prior layout pass). Fall back to fresh computation
        // on cache miss (resize, page switch, scroll invalidation).
        let layout_node = if let Some((vp, node)) = &ctx.cached_layout {
            if *vp == local_viewport {
                Rc::clone(node)
            } else {
                self.rebuild_dialog_layout(window_id, local_viewport)?
            }
        } else {
            self.rebuild_dialog_layout(window_id, local_viewport)?
        };
        let ctx = self.dialogs.get_mut(&window_id)?;

        #[cfg(debug_assertions)]
        let layout_ids = {
            let mut ids = std::collections::HashSet::new();
            oriterm_ui::pipeline::collect_layout_widget_ids(&layout_node, &mut ids);
            ids
        };

        // Build focus path for keyboard routing.
        let focus_path = ctx.root.interaction().focus_ancestor_path();
        let active = ctx.root.interaction().active_widget();
        let now = Instant::now();

        // Dispatch via keymap (for KeyDown) or fall through to controllers.
        let result = super::keymap_dispatch::dispatch_dialog_key_event(
            &input_event,
            ctx,
            &focus_path,
            active,
            content_bounds,
            &layout_node,
            now,
            #[cfg(debug_assertions)]
            &layout_ids,
        );

        // Apply interaction state changes (focus cycling, active) and mark dirty.
        let changed = {
            let (interaction, focus) = ctx.root.interaction_and_focus_mut();
            apply_dispatch_requests(result.requests, result.source, interaction, focus)
        };
        ctx.root.mark_widgets_prepaint_dirty(&changed);

        // Request redraw when the event was handled (widget mutated local
        // state, e.g. sidebar search text), interaction state changed
        // (focus cycling via Tab, active state via Enter/Space), or
        // controllers requested repaint.
        if super::needs_content_redraw(result.handled, !changed.is_empty(), result.requests) {
            ctx.request_urgent_redraw();
        }

        // Transform Clicked(id) through the content widget's on_action
        // (e.g., SettingsPanel maps Clicked(save_id) → SaveSettings).
        result.actions.into_iter().next().map(|a| {
            if let WidgetAction::Clicked(id) = a {
                ctx.content
                    .content_widget_mut()
                    .on_action(WidgetAction::Clicked(id), content_bounds)
                    .unwrap_or(WidgetAction::Clicked(id))
            } else {
                a
            }
        })
    }

    /// Compute layout, parent map, and focus order for a dialog, caching the result.
    ///
    /// Called on cache miss when the keyboard dispatch path needs a fresh
    /// layout tree. Stores the result in `cached_layout` so subsequent
    /// keypresses can skip the full rebuild.
    fn rebuild_dialog_layout(
        &mut self,
        window_id: WindowId,
        local_viewport: Rect,
    ) -> Option<Rc<oriterm_ui::layout::LayoutNode>> {
        let ui_theme = self.ui_theme;
        let ctx = self.dialogs.get_mut(&window_id)?;
        let renderer = ctx.renderer.as_ref()?;
        let scale = ctx.scale_factor.factor() as f32;
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
        let layout_ctx = LayoutCtx {
            measurer: &measurer,
            theme: &ui_theme,
        };
        let layout_box = ctx.content.content_widget().layout(&layout_ctx);
        let node = Rc::new(compute_layout(&layout_box, local_viewport));

        let parent_map = build_parent_map(&node);
        ctx.root.interaction_mut().set_parent_map(parent_map);

        let mut focusable = Vec::new();
        collect_focusable_ids(ctx.content.content_widget_mut(), &mut focusable);
        ctx.root.sync_focus_order(focusable);

        ctx.cached_layout = Some((local_viewport, Rc::clone(&node)));
        Some(node)
    }

    /// Clear hover state for chrome and content.
    ///
    /// Clears the `InteractionManager`'s hot path (empty = no widget under cursor).
    /// The next `prepare_widget_tree` will deliver `HotChanged(false)` lifecycle
    /// events and the `VisualStateAnimator` transitions back to normal.
    pub(in crate::app) fn clear_dialog_hover(&mut self, window_id: WindowId) {
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let changed = ctx.root.interaction_mut().update_hot_path(&[]);
        ctx.root.mark_widgets_prepaint_dirty(&changed);
        if ctx.last_cursor_icon != CursorIcon::Default {
            ctx.window.set_cursor(CursorIcon::Default);
            ctx.last_cursor_icon = CursorIcon::Default;
        }
        ctx.request_urgent_redraw();
    }
}
