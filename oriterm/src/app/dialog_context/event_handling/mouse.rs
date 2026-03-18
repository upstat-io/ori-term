//! Dialog mouse event routing: click dispatch, overlay forwarding, and scroll.

use std::time::Instant;

use oriterm_ui::controllers::ControllerRequests;
use oriterm_ui::geometry::Rect;
use oriterm_ui::input::dispatch::tree::deliver_event_to_tree;
use oriterm_ui::input::{InputEvent, MouseButton, MouseEvent, MouseEventKind, ScrollDelta};
use oriterm_ui::layout::compute_layout;
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{LayoutCtx, WidgetAction};
use winit::window::WindowId;

use crate::app::App;
use crate::app::widget_pipeline::apply_dispatch_requests;
use crate::font::{CachedTextMeasurer, UiFontMeasurer};

use super::super::DialogWindowContext;
use super::DialogClickResult;

/// Returns cached layout or recomputes. Avoids expensive tree walk per event.
fn cached_content_layout(
    ctx: &mut DialogWindowContext,
    scale: f32,
    ui_theme: &oriterm_ui::theme::UiTheme,
    local_viewport: Rect,
) -> Option<std::rc::Rc<oriterm_ui::layout::LayoutNode>> {
    if let Some((vp, node)) = &ctx.cached_layout {
        if *vp == local_viewport {
            return Some(std::rc::Rc::clone(node));
        }
    }
    let renderer = ctx.renderer.as_ref()?;
    let measurer = CachedTextMeasurer::new(
        UiFontMeasurer::new(renderer.active_ui_collection(), scale),
        &ctx.text_cache,
        scale,
    );
    let layout_ctx = LayoutCtx {
        measurer: &measurer,
        theme: ui_theme,
    };
    let layout_box = ctx.content.content_widget().layout(&layout_ctx);
    let node = std::rc::Rc::new(compute_layout(&layout_box, local_viewport));
    ctx.cached_layout = Some((local_viewport, std::rc::Rc::clone(&node)));
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

        // Chrome and content area.
        let result = self.route_dialog_click(window_id, kind, button, state);
        match result {
            DialogClickResult::Close => self.close_dialog(window_id),
            DialogClickResult::Drag => {
                if let Some(ctx) = self.dialogs.get(&window_id) {
                    let _ = ctx.window.drag_window();
                }
            }
            DialogClickResult::Action(action) => {
                self.handle_dialog_content_action(window_id, action);
            }
            DialogClickResult::None => {}
        }
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
        if ctx.overlays.is_empty() {
            return None;
        }
        let scale = ctx.scale_factor.factor() as f32;
        let renderer = ctx.renderer.as_ref()?;
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let mouse_event = MouseEvent {
            kind,
            pos: ctx.last_cursor_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let result = ctx.overlays.process_mouse_event(
            &mouse_event,
            &measurer,
            &ui_theme,
            None,
            &mut ctx.layer_tree,
            &mut ctx.layer_animator,
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

    /// Route a click to either chrome or content area.
    fn route_dialog_click(
        &mut self,
        window_id: WindowId,
        kind: MouseEventKind,
        button: winit::event::MouseButton,
        state: winit::event::ElementState,
    ) -> DialogClickResult {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return DialogClickResult::None;
        };
        let scale = ctx.scale_factor.factor() as f32;
        let logical_pos = ctx.last_cursor_pos;
        let chrome_h = ctx.chrome.caption_height();
        let mouse_event = MouseEvent {
            kind,
            pos: logical_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let now = Instant::now();

        if logical_pos.y < chrome_h {
            // Chrome area: dispatch to control button controllers.
            let input_event = InputEvent::from_mouse_event(&mouse_event);
            let result = ctx.chrome.dispatch_input(&input_event, now);

            // Apply interaction state changes (active/focus).
            apply_dispatch_requests(
                result.requests,
                result.source,
                &mut ctx.interaction,
                &mut ctx.focus,
            );

            if result.requests.contains(ControllerRequests::PAINT) {
                ctx.request_urgent_redraw();
            }

            // Map controller actions to dialog results.
            for action in &result.actions {
                if let WidgetAction::Clicked(id) = action {
                    if let Some(window_action) = ctx.chrome.action_for_widget(*id) {
                        if window_action == WidgetAction::WindowClose {
                            return DialogClickResult::Close;
                        }
                        return DialogClickResult::Action(window_action);
                    }
                }
            }

            // No button clicked — check if we should initiate window drag.
            if button == winit::event::MouseButton::Left
                && state == winit::event::ElementState::Pressed
                && !ctx
                    .chrome
                    .interactive_rects()
                    .iter()
                    .any(|r| r.contains(logical_pos))
            {
                DialogClickResult::Drag
            } else {
                DialogClickResult::None
            }
        } else {
            // Content area: dispatch through the controller pipeline.
            let w = ctx.surface_config.width as f32 / scale;
            let h = ctx.surface_config.height as f32 / scale;
            let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
            let local_viewport =
                Rect::new(0.0, 0.0, content_bounds.width(), content_bounds.height());
            let Some(layout_node) = cached_content_layout(ctx, scale, &ui_theme, local_viewport)
            else {
                return DialogClickResult::None;
            };
            let input_event = InputEvent::from_mouse_event(&mouse_event);
            let active = ctx.interaction.active_widget();
            let result = deliver_event_to_tree(
                ctx.content.content_widget_mut(),
                &input_event,
                content_bounds,
                Some(&layout_node),
                active,
                &[],
                now,
            );

            // Apply interaction state changes.
            apply_dispatch_requests(
                result.requests,
                result.source,
                &mut ctx.interaction,
                &mut ctx.focus,
            );

            if result.requests.contains(ControllerRequests::PAINT) {
                ctx.request_urgent_redraw();
            }

            // Transform Clicked(id) through the content widget's on_action
            // (e.g., SettingsPanel maps Clicked(save_id) → SaveSettings).
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
        let chrome_h = ctx.chrome.caption_height();
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
        let local_viewport = Rect::new(0.0, 0.0, content_bounds.width(), content_bounds.height());
        let Some(layout_node) = cached_content_layout(ctx, scale, &ui_theme, local_viewport) else {
            return;
        };
        let input_event = InputEvent::from_mouse_event(&mouse_event);
        let now = Instant::now();
        let active = ctx.interaction.active_widget();
        let result = deliver_event_to_tree(
            ctx.content.content_widget_mut(),
            &input_event,
            content_bounds,
            Some(&layout_node),
            active,
            &[],
            now,
        );
        apply_dispatch_requests(
            result.requests,
            result.source,
            &mut ctx.interaction,
            &mut ctx.focus,
        );
        if result.handled {
            // Scroll offset changed — invalidate cached layout and scene
            // cache so the next render repaints scrolled content.
            ctx.cached_layout = None;
            ctx.invalidation.invalidate_all();
            ctx.request_urgent_redraw();
        }
    }
}
