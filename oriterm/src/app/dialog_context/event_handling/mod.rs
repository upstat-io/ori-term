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
use oriterm_ui::input::dispatch::tree::deliver_event_to_tree;
use oriterm_ui::input::{InputEvent, MouseEvent, MouseEventKind};
use oriterm_ui::overlay::OverlayEventResult;
use oriterm_ui::widgets::{LayoutCtx, Widget, WidgetAction};
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
                        ctx.root.invalidation_mut().invalidate_all();
                        ctx.root.damage_mut().reset();
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
                removed = ctx.root.clear_popups();
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
    /// All other keys are dispatched through the controller pipeline to
    /// content widgets. Global keybindings are checked as a fallback.
    fn handle_dialog_keyboard(&mut self, window_id: WindowId, event: &winit::event::KeyEvent) {
        // Only handle Escape inline — all other keys go through the pipeline.
        if event.state == winit::event::ElementState::Pressed
            && event.logical_key == Key::Named(NamedKey::Escape)
        {
            if self.dialog_has_overlay(window_id) {
                self.dismiss_dialog_overlay(window_id);
                return;
            }
            self.close_dialog(window_id);
            return;
        }

        // Route through the controller pipeline (handles Tab focus cycling,
        // Enter/Space activation, and any other widget keyboard input).
        let action = self.dispatch_dialog_content_key(window_id, event);
        if let Some(action) = action {
            self.handle_dialog_content_action(window_id, action);
            return;
        }

        // Global keybindings: actions that work from any window.
        if event.state == winit::event::ElementState::Pressed {
            let mods = self.modifiers.into();
            if let Some(binding_key) = keybindings::key_to_binding_key(&event.logical_key) {
                if let Some(action) = keybindings::find_binding(&self.bindings, &binding_key, mods)
                {
                    if action.is_global() {
                        let action = action.clone();
                        self.execute_action(&action);
                    }
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

        // Route to overlay manager first (dropdown popup hover).
        if !ctx.root.overlays().is_empty() {
            let move_event = MouseEvent {
                kind: MouseEventKind::Move,
                pos: logical_pos,
                modifiers: oriterm_ui::input::Modifiers::NONE,
            };
            let result = ctx.root.process_overlay_mouse_event(
                &move_event,
                &measurer,
                &ui_theme,
                None,
                Instant::now(),
            );
            if matches!(result, OverlayEventResult::Delivered { .. }) {
                ctx.request_urgent_redraw();
                return;
            }
        }

        log::trace!(
            "dialog cursor: phys=({:.0},{:.0}) log=({:.1},{:.1}) s={scale:.2} ch={chrome_h:.1} \
             rects={:?}",
            position.x,
            position.y,
            logical_pos.x,
            logical_pos.y,
            ctx.chrome.interactive_rects(),
        );

        // Build the hot path from cursor position.
        // The InteractionManager uses this to generate HotChanged lifecycle
        // events, which are delivered during prepare_widget_tree.
        let mut hot_path = Vec::new();
        if logical_pos.y < chrome_h {
            // Chrome: check which control button is under the cursor.
            if let Some(btn_id) = ctx.chrome.widget_at_point(logical_pos) {
                hot_path.push(ctx.chrome.id());
                hot_path.push(btn_id);
            }
        } else {
            // Content area: hit test the layout tree (cached).
            let w = ctx.surface_config.width as f32 / scale;
            let h = ctx.surface_config.height as f32 / scale;
            let content_bounds: Rect = Rect::new(0.0, chrome_h, w, h - chrome_h);
            let local_viewport: Rect =
                Rect::new(0.0, 0.0, content_bounds.width(), content_bounds.height());
            let layout_node = match &ctx.cached_layout {
                Some((vp, node)) if *vp == local_viewport => std::rc::Rc::clone(node),
                _ => {
                    let layout_ctx = LayoutCtx {
                        measurer: &measurer,
                        theme: &ui_theme,
                    };
                    let layout_box = ctx.content.content_widget().layout(&layout_ctx);
                    let node = std::rc::Rc::new(oriterm_ui::layout::compute_layout(
                        &layout_box,
                        local_viewport,
                    ));
                    ctx.cached_layout = Some((local_viewport, std::rc::Rc::clone(&node)));
                    node
                }
            };
            let local = Point::new(
                logical_pos.x - content_bounds.x(),
                logical_pos.y - content_bounds.y(),
            );
            let hit = oriterm_ui::input::layout_hit_test_path(&layout_node, local);
            for entry in &hit.path {
                hot_path.push(entry.widget_id);
            }
        }

        // Update the InteractionManager's hot path. HotChanged lifecycle events
        // are stored internally and delivered during the next prepare_widget_tree.
        ctx.root.interaction_mut().update_hot_path(&hot_path);

        // Dispatch MouseMove to content widgets for per-item hover tracking.
        if logical_pos.y >= chrome_h {
            log::info!(
                "dialog cursor dispatch: pos=({:.0},{:.0})",
                logical_pos.x,
                logical_pos.y
            );
            self.dispatch_dialog_content_move(window_id, logical_pos);
        }

        // Always request a redraw after a cursor move so the next
        // prepare_widget_tree delivers any pending HotChanged events and
        // updates the VisualStateAnimator accordingly.
        if let Some(ctx) = self.dialogs.get_mut(&window_id) {
            ctx.request_urgent_redraw();
        }
    }

    /// Dispatch a `MouseMove` input event to the dialog content widget tree.
    ///
    /// Allows widgets like `SidebarNavWidget` to track per-item hover state
    /// using `on_input(MouseMove)`.
    fn dispatch_dialog_content_move(&mut self, window_id: WindowId, logical_pos: Point) {
        let ui_theme = self.ui_theme;
        let Some(ctx) = self.dialogs.get_mut(&window_id) else {
            log::info!("dispatch_dialog_content_move: no dialog context");
            return;
        };
        let scale = ctx.scale_factor.factor() as f32;
        let chrome_h = ctx.chrome.caption_height();
        let w = ctx.surface_config.width as f32 / scale;
        let h = ctx.surface_config.height as f32 / scale;
        let content_bounds = Rect::new(0.0, chrome_h, w, h - chrome_h);
        let local_viewport = Rect::new(0.0, 0.0, content_bounds.width(), content_bounds.height());
        let Some(layout_node) = mouse::cached_content_layout(ctx, scale, &ui_theme, local_viewport)
        else {
            return;
        };
        #[cfg(debug_assertions)]
        let layout_ids = {
            let mut ids = std::collections::HashSet::new();
            oriterm_ui::pipeline::collect_layout_widget_ids(&layout_node, &mut ids);
            ids
        };
        let move_event = MouseEvent {
            kind: MouseEventKind::Move,
            pos: logical_pos,
            modifiers: oriterm_ui::input::Modifiers::NONE,
        };
        let input_event = InputEvent::from_mouse_event(&move_event);
        let active = ctx.root.interaction().active_widget();
        let now = Instant::now();
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
        if result.handled
            || result
                .requests
                .contains(oriterm_ui::controllers::ControllerRequests::PAINT)
        {
            ctx.request_urgent_redraw();
        }
    }
}
