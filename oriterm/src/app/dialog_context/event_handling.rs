//! Dialog window event handling.
//!
//! Routes winit `WindowEvent` variants to dialog-specific handlers.
//! Chrome events (close, drag) are handled inline. Content area events
//! are routed to the settings panel widget tree. Widget actions are
//! dispatched via `content_actions.rs`.

use std::time::Instant;

use oriterm_ui::geometry::{Point, Rect};
use oriterm_ui::input::{
    EventResponse, HoverEvent, MouseButton, MouseEvent, MouseEventKind, ScrollDelta,
};
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{EventCtx, Widget, WidgetAction};
use winit::event::WindowEvent;
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use crate::font::{CachedTextMeasurer, UiFontMeasurer};
use crate::keybindings;

use crate::app::App;

/// Result of processing a dialog mouse click.
enum DialogClickResult {
    /// Close the dialog window.
    Close,
    /// Initiate an OS window drag.
    Drag,
    /// A widget action was emitted by the content area.
    Action(WidgetAction),
    /// No action needed.
    None,
}

/// Whether an `EventResponse` indicates a repaint is needed.
fn wants_repaint(resp: EventResponse) -> bool {
    matches!(
        resp,
        EventResponse::RequestPaint | EventResponse::RequestLayout
    )
}

impl App {
    /// Handle a winit `WindowEvent` for a dialog window.
    pub(in crate::app) fn handle_dialog_window_event(
        &mut self,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.close_dialog(window_id);
            }
            WindowEvent::Resized(size) => {
                if let Some(gpu) = self.gpu.as_ref() {
                    if let Some(ctx) = self.dialogs.get_mut(&window_id) {
                        ctx.resize_surface(size.width, size.height, gpu);
                    }
                }
                // Update platform hit test rects after chrome layout recompute.
                self.refresh_dialog_platform_rects(window_id);
                // Render immediately to avoid showing uninitialized surface
                // (light blue flash) between reconfigure and next frame budget.
                self.render_dialog(window_id);
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(ctx) = self.dialogs.get_mut(&window_id) {
                    let new_factor = oriterm_ui::scale::ScaleFactor::new(scale_factor);
                    if ctx.scale_factor != new_factor {
                        ctx.scale_factor = new_factor;
                        // Recompute chrome layout width — the physical surface
                        // size may stay the same, but the logical width changes
                        // when DPI changes (e.g. moving to a different monitor).
                        let scale = new_factor.factor() as f32;
                        let logical_w = ctx.surface_config.width as f32 / scale;
                        ctx.chrome.set_window_width(logical_w);
                        // Invalidate content layout cache — text metrics change
                        // with DPI even when logical bounds stay the same.
                        ctx.content.invalidate_cache();
                        ctx.text_cache.clear();
                        ctx.invalidation.invalidate_all();
                        // TODO: re-rasterize UI fonts at new DPI.
                        ctx.request_urgent_redraw();
                    }
                }
                // Update platform hit test rects for the new DPI.
                self.refresh_dialog_platform_rects(window_id);
            }
            WindowEvent::Focused(focused) => {
                self.handle_dialog_focus(window_id, focused);
            }
            WindowEvent::ModifiersChanged(mods) => {
                self.modifiers = mods.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.handle_dialog_keyboard(window_id, &event);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.handle_dialog_cursor_move(window_id, position);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.handle_dialog_mouse_input(window_id, state, button);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.handle_dialog_scroll(window_id, delta);
            }
            WindowEvent::CursorLeft { .. } => {
                self.clear_dialog_hover(window_id);
            }
            _ => {}
        }
    }

    /// Handle focus change for a dialog window.
    fn handle_dialog_focus(&mut self, window_id: WindowId, focused: bool) {
        if focused {
            self.window_manager.set_focused(Some(window_id));
            self.flush_pending_focus_out();
        } else {
            let mut removed = 0;
            if let Some(ctx) = self.dialogs.get_mut(&window_id) {
                removed = ctx
                    .overlays
                    .clear_popups(&mut ctx.layer_tree, &mut ctx.layer_animator);
                if removed > 0 {
                    ctx.request_urgent_redraw();
                }
            }
            if removed > 0 {
                self.pending_dropdown_id = None;
            }
        }
        if let Some(ctx) = self.dialogs.get_mut(&window_id) {
            ctx.chrome.set_active(focused);
            ctx.request_urgent_redraw();
        }
    }

    /// Handle keyboard input within a dialog window.
    ///
    /// Escape closes the dialog (or dismisses a dropdown popup).
    /// Tab/Enter/Space route to confirmation dialog widgets.
    /// Global keybindings are dispatched normally.
    fn handle_dialog_keyboard(&mut self, window_id: WindowId, event: &winit::event::KeyEvent) {
        if event.state != winit::event::ElementState::Pressed {
            return;
        }

        // Escape: dismiss dropdown popup, or close dialog.
        if event.logical_key == Key::Named(NamedKey::Escape) {
            if self.dialog_has_overlay(window_id) {
                self.dismiss_dialog_overlay(window_id);
                return;
            }
            self.close_dialog(window_id);
            return;
        }

        // Route Tab/Enter/Space to confirmation dialog content widgets.
        let action = self.try_dialog_content_key(window_id, event);
        if let Some(action) = action {
            self.handle_dialog_content_action(window_id, action);
            return;
        }

        // Global keybindings: actions that work from any window.
        let mods = self.modifiers.into();
        if let Some(binding_key) = keybindings::key_to_binding_key(&event.logical_key) {
            if let Some(action) = keybindings::find_binding(&self.bindings, &binding_key, mods) {
                if action.is_global() {
                    let action = action.clone();
                    self.execute_action(&action);
                }
            }
        }
    }

    /// Handle cursor movement within a dialog window.
    fn handle_dialog_cursor_move(
        &mut self,
        window_id: WindowId,
        position: winit::dpi::PhysicalPosition<f64>,
    ) {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let scale = ctx.scale_factor.factor() as f32;
        let logical_pos = Point::new(position.x as f32 / scale, position.y as f32 / scale);
        ctx.last_cursor_pos = logical_pos;

        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let chrome_h = ctx.chrome.caption_height();
        let mut needs_redraw = false;

        // Route to overlay manager first (dropdown popup hover).
        if !ctx.overlays.is_empty() {
            let move_event = MouseEvent {
                kind: MouseEventKind::Move,
                pos: logical_pos,
                modifiers: oriterm_ui::input::Modifiers::NONE,
            };
            let result = ctx.overlays.process_mouse_event(
                &move_event,
                &measurer,
                &ui_theme,
                None,
                &mut ctx.layer_tree,
                &mut ctx.layer_animator,
                Instant::now(),
            );
            if matches!(result, OverlayEventResult::Delivered { .. }) {
                ctx.request_urgent_redraw();
                return;
            }
        }

        let event_ctx = EventCtx {
            measurer: &measurer,
            bounds: Rect::default(),
            is_focused: false,
            focused_widget: None,
            theme: &ui_theme,
        };

        if log::log_enabled!(log::Level::Trace) {
            let zone = if logical_pos.y < chrome_h {
                "chrome"
            } else {
                "content"
            };
            log::trace!(
                "dialog cursor: phys=({:.0},{:.0}) log=({:.1},{:.1}) s={scale:.2} ch={chrome_h:.1} \
                 z={zone} rects={:?}",
                position.x,
                position.y,
                logical_pos.x,
                logical_pos.y,
                ctx.chrome.interactive_rects(),
            );
        }

        if logical_pos.y < chrome_h {
            // Chrome hover (close button highlight).
            let resp = ctx.chrome.update_hover(logical_pos, &event_ctx);
            if wants_repaint(resp.response) {
                resp.mark_tracker(&mut ctx.invalidation);
                needs_redraw = true;
            }
        } else {
            // Content area hover — clear any active chrome hover first.
            let resp = ctx.chrome.handle_hover(HoverEvent::Leave, &event_ctx);
            if wants_repaint(resp.response) {
                resp.mark_tracker(&mut ctx.invalidation);
                needs_redraw = true;
            }

            let w = ctx.surface_config.width as f32 / scale;
            let h = ctx.surface_config.height as f32 / scale;
            let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
            let hover_event = MouseEvent {
                kind: MouseEventKind::Move,
                pos: logical_pos,
                modifiers: oriterm_ui::input::Modifiers::NONE,
            };
            let content_ctx = EventCtx {
                measurer: &measurer,
                bounds: content_bounds,
                is_focused: false,
                focused_widget: None,
                theme: &ui_theme,
            };
            let resp = ctx
                .content
                .content_widget_mut()
                .handle_mouse(&hover_event, &content_ctx);
            if wants_repaint(resp.response) {
                resp.mark_tracker(&mut ctx.invalidation);
                needs_redraw = true;
            }
        }
        if needs_redraw {
            ctx.request_urgent_redraw();
        }
    }

    /// Handle mouse button events within a dialog window.
    fn handle_dialog_mouse_input(
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
        let Some(renderer) = ctx.renderer.as_ref() else {
            return DialogClickResult::None;
        };
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );

        if logical_pos.y < chrome_h {
            let event_ctx = EventCtx {
                measurer: &measurer,
                bounds: Rect::default(),
                is_focused: false,
                focused_widget: None,
                theme: &ui_theme,
            };
            let resp = ctx.chrome.handle_mouse(&mouse_event, &event_ctx);
            if wants_repaint(resp.response) {
                resp.mark_tracker(&mut ctx.invalidation);
                ctx.request_urgent_redraw();
            }
            if resp.action.as_ref() == Some(&WidgetAction::WindowClose) {
                DialogClickResult::Close
            } else if button == winit::event::MouseButton::Left
                && state == winit::event::ElementState::Pressed
                && resp.action.is_none()
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
            let w = ctx.surface_config.width as f32 / scale;
            let h = ctx.surface_config.height as f32 / scale;
            let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
            let event_ctx = EventCtx {
                measurer: &measurer,
                bounds: content_bounds,
                is_focused: false,
                focused_widget: None,
                theme: &ui_theme,
            };
            let resp = ctx
                .content
                .content_widget_mut()
                .handle_mouse(&mouse_event, &event_ctx);
            if wants_repaint(resp.response) {
                resp.mark_tracker(&mut ctx.invalidation);
                ctx.request_urgent_redraw();
            }
            match resp.action {
                Some(action) => DialogClickResult::Action(action),
                None => DialogClickResult::None,
            }
        }
    }

    /// Handle mouse wheel events within a dialog window.
    fn handle_dialog_scroll(&mut self, window_id: WindowId, delta: winit::event::MouseScrollDelta) {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            return;
        };
        let scale = ctx.scale_factor.factor() as f32;
        let scroll_delta = match delta {
            winit::event::MouseScrollDelta::LineDelta(x, y) => ScrollDelta::Lines { x, y: -y },
            winit::event::MouseScrollDelta::PixelDelta(pos) => ScrollDelta::Pixels {
                x: pos.x as f32 / scale,
                y: -(pos.y as f32 / scale),
            },
        };
        let mouse_event = MouseEvent {
            kind: MouseEventKind::Scroll(scroll_delta),
            pos: ctx.last_cursor_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let Some(renderer) = ctx.renderer.as_ref() else {
            return;
        };
        let measurer = CachedTextMeasurer::new(
            UiFontMeasurer::new(renderer.active_ui_collection(), scale),
            &ctx.text_cache,
            scale,
        );
        let chrome_h = ctx.chrome.caption_height();
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
        let event_ctx = EventCtx {
            measurer: &measurer,
            bounds: content_bounds,
            is_focused: false,
            focused_widget: None,
            theme: &ui_theme,
        };
        let resp = ctx
            .content
            .content_widget_mut()
            .handle_mouse(&mouse_event, &event_ctx);
        if wants_repaint(resp.response) {
            resp.mark_tracker(&mut ctx.invalidation);
            ctx.request_urgent_redraw();
        }
    }
}
