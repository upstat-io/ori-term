//! Dialog mouse event routing: click dispatch, overlay forwarding, and scroll.

use std::time::Instant;

use oriterm_ui::geometry::Rect;
use oriterm_ui::input::dispatch::tree::deliver_event_to_tree;
use oriterm_ui::input::{InputEvent, MouseButton, MouseEvent, MouseEventKind, ScrollDelta};
use oriterm_ui::layout::compute_layout;
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{LayoutCtx, WidgetAction};
use winit::window::WindowId;

use crate::app::App;
use crate::app::widget_pipeline::apply_dispatch_requests;
use crate::font::CachedTextMeasurer;

use super::super::DialogWindowContext;
use super::DialogClickResult;

/// Computes the content layout tree fresh from the current widget state.
///
/// Used by click and scroll handlers where no cached layout is available.
/// Cursor-move events avoid this by caching the layout computed during
/// hit testing (see `handle_dialog_cursor_move`).
pub(super) fn compute_content_layout(
    ctx: &DialogWindowContext,
    scale: f32,
    ui_theme: &oriterm_ui::theme::UiTheme,
    local_viewport: Rect,
) -> Option<std::rc::Rc<oriterm_ui::layout::LayoutNode>> {
    let renderer = ctx.renderer.as_ref()?;
    let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
    let layout_ctx = LayoutCtx {
        measurer: &measurer,
        theme: ui_theme,
    };
    let layout_box = ctx.content.content_widget().layout(&layout_ctx);
    let node = std::rc::Rc::new(compute_layout(&layout_box, local_viewport));
    Some(node)
}

impl App {
    /// Handle mouse button events within a dialog window.
    pub(in crate::app) fn handle_dialog_mouse_input(
        &mut self,
        window_id: WindowId,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) {
        let ui_button = match button {
            winit::event::MouseButton::Left => MouseButton::Left,
            winit::event::MouseButton::Right => MouseButton::Right,
            winit::event::MouseButton::Middle => MouseButton::Middle,
            _ => return,
        };
        let kind = match state {
            winit::event::ElementState::Pressed => MouseEventKind::Down(ui_button),
            winit::event::ElementState::Released => MouseEventKind::Up(ui_button),
        };

        // Route to overlay manager first (dropdown popup interactions).
        if let Some(result) = self.try_dialog_overlay_mouse(window_id, kind) {
            self.handle_dialog_overlay_result(window_id, result);
            return;
        }

        // Header drag: left mouse down in the caption area (above the header
        // height) but outside the close button → initiate window drag.
        // On Windows, WM_NCHITTEST already handles this at the OS level so
        // the event never reaches here for the caption region. This path is
        // needed for Linux (Wayland/X11) and macOS.
        if matches!(kind, MouseEventKind::Down(MouseButton::Left)) {
            if self.try_dialog_header_drag(window_id) {
                return;
            }
        }

        // Content area.
        let result = self.route_dialog_click(window_id, kind);
        match result {
            DialogClickResult::Action(action) => {
                self.handle_dialog_content_action(window_id, action);
            }
            DialogClickResult::None => {}
        }
    }

    /// Check if the cursor is in the dialog caption drag area and initiate drag.
    ///
    /// The caption area is the top `DIALOG_DRAG_CAPTION_HEIGHT` pixels of
    /// the dialog, excluding the sidebar (which contains the search field
    /// and nav items). Returns `true` if `drag_window()` was called.
    ///
    /// On Windows, `WM_NCHITTEST` handles this at the OS level so this
    /// code path is primarily needed for Linux and macOS.
    fn try_dialog_header_drag(&self, window_id: WindowId) -> bool {
        use oriterm_ui::widgets::sidebar_nav::SIDEBAR_WIDTH;

        let Some(ctx) = self.dialogs.get(&window_id) else {
            return false;
        };
        let pos = ctx.last_cursor_pos;
        // Must be within the caption strip at the top of the dialog.
        if pos.y >= crate::app::dialog_management::DIALOG_DRAG_CAPTION_HEIGHT {
            return false;
        }
        // Sidebar is excluded — search field and nav items must remain clickable.
        if pos.x < SIDEBAR_WIDTH {
            return false;
        }
        if let Err(e) = ctx.window.drag_window() {
            log::warn!("dialog drag_window failed: {e}");
        }
        true
    }

    /// Try routing a mouse event through a dialog's overlay manager.
    ///
    /// Returns `Some(result)` if an overlay consumed the event.
    fn try_dialog_overlay_mouse(
        &mut self,
        window_id: WindowId,
        kind: MouseEventKind,
    ) -> Option<OverlayEventResult> {
        let ui_theme = self.ui_theme;
        let ctx = self.dialogs.get_mut(&window_id)?;
        if !ctx.root.has_overlays() {
            return None;
        }
        let scale = ctx.scale_factor.factor() as f32;
        let renderer = ctx.renderer.as_ref()?;
        let measurer = CachedTextMeasurer::new(renderer.ui_measurer(scale), &ctx.text_cache, scale);
        let mouse_event = MouseEvent {
            kind,
            pos: ctx.last_cursor_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let result = ctx.root.process_overlay_mouse_event(
            &mouse_event,
            &measurer,
            &ui_theme,
            None,
            Instant::now(),
        );
        match &result {
            OverlayEventResult::Delivered { .. } | OverlayEventResult::Dismissed(_) => {
                ctx.request_urgent_redraw();
                Some(result)
            }
            _ => None,
        }
    }

    /// Route a click to the dialog content area.
    fn route_dialog_click(
        &mut self,
        window_id: WindowId,
        kind: MouseEventKind,
    ) -> DialogClickResult {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return DialogClickResult::None;
        };
        let logical_pos = ctx.last_cursor_pos;
        let mouse_event = MouseEvent {
            kind,
            pos: logical_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let now = Instant::now();
        Self::dispatch_dialog_content_click(ctx, &mouse_event, &ui_theme, now)
    }

    /// Dispatch a click within the dialog content area.
    fn dispatch_dialog_content_click(
        ctx: &mut DialogWindowContext,
        mouse_event: &MouseEvent,
        ui_theme: &oriterm_ui::theme::UiTheme,
        now: Instant,
    ) -> DialogClickResult {
        let scale = ctx.scale_factor.factor() as f32;
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, 0.0, w, h);
        let local_viewport = Rect::new(0.0, 0.0, w, h);
        let Some(layout_node) = compute_content_layout(ctx, scale, ui_theme, local_viewport) else {
            return DialogClickResult::None;
        };
        #[cfg(debug_assertions)]
        let layout_ids = {
            let mut ids = std::collections::HashSet::new();
            oriterm_ui::pipeline::collect_layout_widget_ids(&layout_node, &mut ids);
            ids
        };
        let input_event = InputEvent::from_mouse_event(mouse_event);
        let active = ctx.root.interaction().active_widget();
        let result = deliver_event_to_tree(
            ctx.content.content_widget_mut(),
            &input_event,
            content_bounds,
            Some(&layout_node),
            active,
            &[],
            now,
            #[cfg(debug_assertions)]
            Some(&layout_ids),
            #[cfg(not(debug_assertions))]
            None,
        );

        // Apply interaction state changes and mark dirty.
        let changed = {
            let (interaction, focus) = ctx.root.interaction_and_focus_mut();
            apply_dispatch_requests(result.requests, result.source, interaction, focus)
        };
        ctx.root.mark_widgets_prepaint_dirty(&changed);

        // Redraw when event was handled (widget mutated local state),
        // interaction state changed, or controllers requested repaint.
        if super::super::needs_content_redraw(result.handled, !changed.is_empty(), result.requests)
        {
            ctx.request_urgent_redraw();
        }

        // Transform Clicked(id) through the content widget's on_action
        // (e.g., SettingsPanel maps Clicked(save_id) -> SaveSettings).
        // Other actions (OpenDropdown, Toggled, etc.) pass through unchanged.
        match result.actions.into_iter().next() {
            Some(WidgetAction::Clicked(id)) => {
                let action = ctx
                    .content
                    .content_widget_mut()
                    .on_action(WidgetAction::Clicked(id), content_bounds)
                    .unwrap_or(WidgetAction::Clicked(id));
                DialogClickResult::Action(action)
            }
            Some(action) => DialogClickResult::Action(action),
            None => DialogClickResult::None,
        }
    }

    /// Handle mouse wheel events within a dialog window.
    ///
    /// Routes through the dialog's overlay manager first so popup menus
    /// (e.g. dropdown lists) receive scroll events before the underlying
    /// content's `ScrollWidget` can consume them.
    pub(in crate::app) fn handle_dialog_scroll(
        &mut self,
        window_id: WindowId,
        delta: winit::event::MouseScrollDelta,
    ) {
        // Build the scroll delta for the UI event.
        let scale = self
            .dialogs
            .get(&window_id)
            .map_or(1.0, |ctx| ctx.scale_factor.factor() as f32);
        let scroll_delta = match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines { x, y },
            winit::event::MouseScrollDelta::PixelDelta(pos) => ScrollDelta::Pixels {
                x: pos.x as f32 / scale,
                y: pos.y as f32 / scale,
            },
        };

        // Try overlay routing first (dropdown popups get scroll priority).
        if let Some(result) =
            self.try_dialog_overlay_mouse(window_id, MouseEventKind::Scroll(scroll_delta))
        {
            self.handle_dialog_overlay_result(window_id, result);
            return;
        }

        // No overlay consumed the scroll — fall through to content widget.
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let mouse_event = MouseEvent {
            kind: MouseEventKind::Scroll(scroll_delta),
            pos: ctx.last_cursor_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, 0.0, w, h);
        let local_viewport = Rect::new(0.0, 0.0, w, h);
        let Some(layout_node) = compute_content_layout(ctx, scale, &ui_theme, local_viewport)
        else {
            return;
        };
        #[cfg(debug_assertions)]
        let layout_ids = {
            let mut ids = std::collections::HashSet::new();
            oriterm_ui::pipeline::collect_layout_widget_ids(&layout_node, &mut ids);
            ids
        };
        let input_event = InputEvent::from_mouse_event(&mouse_event);
        let now = Instant::now();
        let active = ctx.root.interaction().active_widget();
        let result = deliver_event_to_tree(
            ctx.content.content_widget_mut(),
            &input_event,
            content_bounds,
            Some(&layout_node),
            active,
            &[],
            now,
            #[cfg(debug_assertions)]
            Some(&layout_ids),
            #[cfg(not(debug_assertions))]
            None,
        );
        let changed = {
            let (interaction, focus) = ctx.root.interaction_and_focus_mut();
            apply_dispatch_requests(result.requests, result.source, interaction, focus)
        };
        ctx.root.mark_widgets_prepaint_dirty(&changed);
        if result.handled {
            ctx.request_urgent_redraw();
        }
    }
}
