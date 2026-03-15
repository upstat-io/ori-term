//! Dialog window event handling.
//!
//! Routes winit `WindowEvent` variants to dialog-specific handlers.
//! Chrome events (close, drag) are handled inline. Content area events
//! are routed to the settings panel widget tree. Widget actions are
//! dispatched via `content_actions.rs`. Mouse click/scroll routing lives
//! in the `mouse` submodule.

mod mouse;

use std::time::Instant;

use oriterm_ui::geometry::{Point, Rect};
use oriterm_ui::input::{EventResponse, HoverEvent, MouseEvent, MouseEventKind};
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{EventCtx, Widget, WidgetAction};
use winit::event::WindowEvent;
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowId;

use crate::font::{CachedTextMeasurer, UiFontMeasurer};
use crate::keybindings;

use oriterm_ui::surface::SurfaceLifecycle;

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
        // Lifecycle guard: suppress events outside the Visible state.
        // Resized and ScaleFactorChanged must still be handled in CreatedHidden
        // (the surface needs reconfiguration before the first render).
        if let Some(ctx) = self.dialogs.get(&window_id) {
            let lifecycle = ctx.lifecycle;
            if !matches!(lifecycle, SurfaceLifecycle::Visible) {
                match event {
                    WindowEvent::Resized(..)
                    | WindowEvent::ScaleFactorChanged { .. }
                    | WindowEvent::CloseRequested => {}
                    _ => return,
                }
            }
        }

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
                        ctx.scene_cache.clear();
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
            interaction: None,
            widget_id: None,
        };

        log::trace!(
            "dialog cursor: phys=({:.0},{:.0}) log=({:.1},{:.1}) s={scale:.2} ch={chrome_h:.1} \
             rects={:?}",
            position.x,
            position.y,
            logical_pos.x,
            logical_pos.y,
            ctx.chrome.interactive_rects(),
        );

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
                interaction: None,
                widget_id: None,
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
}
