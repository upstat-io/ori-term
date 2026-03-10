//! Dialog window lifecycle: open, close, render, and event handling.
//!
//! Coordinates dialog OS window creation with the window manager, platform
//! native ops, and GPU renderer. Each dialog is a real OS window with its
//! own surface — moveable independently of the parent terminal window.

use std::cell::Cell;
use std::sync::Arc;
use std::time::Instant;

use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use oriterm_ui::geometry::Rect;
use oriterm_ui::scale::ScaleFactor;
use oriterm_ui::widgets::settings_panel::SettingsPanel;
use oriterm_ui::widgets::{DrawCtx, Widget};

use super::App;
use super::dialog_context::{DialogContent, DialogWindowContext};
use super::settings_overlay::form_builder;
use crate::font::UiFontMeasurer;
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;
use crate::window_manager::platform::platform_ops;
use crate::window_manager::types::{DialogKind, ManagedWindow, WindowKind};

impl App {
    /// Open a settings dialog as a real OS window.
    ///
    /// Creates a frameless window centered on the parent, with platform-native
    /// ownership and shadow. The dialog uses a `UiOnly` renderer — no terminal
    /// grid or pane state.
    pub(super) fn open_settings_dialog(&mut self, event_loop: &ActiveEventLoop) {
        let parent_wid = match self.find_dialog_parent() {
            Some(id) => id,
            None => return,
        };

        // Prevent duplicate settings dialogs.
        if self.has_dialog_of_kind(DialogKind::Settings) {
            log::info!("settings dialog already open");
            // Focus the existing settings dialog.
            if let Some((&wid, _)) = self
                .dialogs
                .iter()
                .find(|(_, ctx)| ctx.kind == DialogKind::Settings)
            {
                if let Some(ctx) = self.dialogs.get(&wid) {
                    ctx.window.focus_window();
                }
            }
            return;
        }

        let kind = DialogKind::Settings;
        let (width, height) = kind.default_size();

        // Center on parent window.
        let position = self.center_on_parent(parent_wid, width, height);

        let window_config = oriterm_ui::window::WindowConfig {
            title: kind.title().into(),
            inner_size: oriterm_ui::geometry::Size::new(width as f32, height as f32),
            transparent: false,
            blur: false,
            opacity: 1.0,
            position: Some(oriterm_ui::geometry::Point::new(
                position.0 as f32,
                position.1 as f32,
            )),
            resizable: kind.is_resizable(),
        };

        let window = match oriterm_ui::window::create_window(event_loop, &window_config) {
            Ok(w) => w,
            Err(e) => {
                log::error!("failed to create settings dialog window: {e}");
                return;
            }
        };
        let winit_id = window.id();

        // Apply platform native ops: ownership, shadow, type hints.
        if let Some(parent_ctx) = self.windows.get(&parent_wid) {
            let parent_win = parent_ctx.window.window();
            let ops = platform_ops();
            ops.set_owner(&window, parent_win);
            ops.enable_shadow(&window);
            ops.set_window_type(&window, &WindowKind::Dialog(kind));
        }

        // Set min inner size for settings dialog.
        window.set_min_inner_size(Some(winit::dpi::LogicalSize::new(600u32, 400u32)));

        // Create GPU surface.
        let Some(gpu) = self.gpu.as_ref() else {
            log::error!("dialog: no GPU state");
            return;
        };
        let Some(pipelines) = self.pipelines.as_ref() else {
            log::error!("dialog: no GPU pipelines");
            return;
        };
        let (surface, surface_config) = match gpu.create_surface(&window) {
            Ok(s) => s,
            Err(e) => {
                log::error!("failed to create dialog GPU surface: {e}");
                return;
            }
        };

        // Create UiOnly renderer.
        let renderer = self.create_dialog_renderer(&window, gpu, pipelines);

        // Build settings form content.
        let content = self.build_settings_content(&window, renderer.as_ref());

        let scale_factor = ScaleFactor::new(window.scale_factor());
        let ctx = DialogWindowContext::new(
            window.clone(),
            surface,
            surface_config,
            renderer,
            kind,
            content,
            scale_factor,
            &self.ui_theme,
        );

        // Register with window manager.
        self.window_manager.register(ManagedWindow::with_parent(
            winit_id,
            WindowKind::Dialog(kind),
            parent_wid,
        ));

        // Store context.
        self.dialogs.insert(winit_id, ctx);

        // Render first frame, then show.
        self.render_dialog(winit_id);
        if let Some(ctx) = self.dialogs.get(&winit_id) {
            ctx.window.set_visible(true);
        }

        log::info!("settings dialog opened: {winit_id:?}, parent: {parent_wid:?}");
    }

    /// Close a dialog window, discarding any pending changes.
    pub(super) fn close_dialog(&mut self, winit_id: WindowId) {
        // Clear platform modal state if applicable.
        if let Some(ctx) = self.dialogs.get(&winit_id) {
            if let Some(managed) = self.window_manager.get(winit_id) {
                if let Some(parent_wid) = managed.parent {
                    if let Some(parent_ctx) = self.windows.get(&parent_wid) {
                        let ops = platform_ops();
                        ops.clear_modal(&ctx.window, parent_ctx.window.window());
                    }
                }
            }
        }

        // Unregister from window manager (cascades to children).
        self.window_manager.unregister(winit_id);

        // Remove context (drops GPU resources).
        self.dialogs.remove(&winit_id);

        log::info!(
            "dialog closed: {winit_id:?}, {} dialogs remaining",
            self.dialogs.len()
        );
    }

    /// Render a dialog window's content to its GPU surface.
    pub(super) fn render_dialog(&mut self, winit_id: WindowId) {
        let Some(gpu) = self.gpu.as_ref() else { return };
        let Some(pipelines) = self.pipelines.as_ref() else {
            return;
        };
        let ui_theme = self.ui_theme;

        let Some(ctx) = self.dialogs.get_mut(&winit_id) else {
            return;
        };
        if !ctx.has_surface_area() {
            return;
        }
        let Some(renderer) = ctx.renderer.as_mut() else {
            return;
        };

        let w = ctx.surface_config.width;
        let h = ctx.surface_config.height;
        let scale = ctx.scale_factor.factor() as f32;

        // Prepare the UI-only frame.
        let bg = oriterm_core::Rgb {
            r: 30,
            g: 30,
            b: 46,
        };
        renderer.prepare_ui_frame(w, h, bg, 1.0);

        // Resolve icons.
        renderer.resolve_icons(gpu, scale);

        let logical_w = (w as f32 / scale).round();
        let logical_h = (h as f32 / scale).round();
        let chrome_h = ctx.chrome.caption_height();

        ctx.draw_list.clear();
        let animations_running = Cell::new(false);
        let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
        let icons = renderer.resolved_icons();

        // Draw the chrome title bar.
        let chrome_bounds = Rect::new(0.0, 0.0, logical_w, chrome_h);
        {
            let mut draw_ctx = DrawCtx {
                measurer: &measurer,
                draw_list: &mut ctx.draw_list,
                bounds: chrome_bounds,
                focused_widget: None,
                now: Instant::now(),
                animations_running: &animations_running,
                theme: &ui_theme,
                icons: Some(icons),
            };
            ctx.chrome.draw(&mut draw_ctx);
        }

        // Draw the dialog content below the chrome.
        let content_bounds = Rect::new(0.0, chrome_h, logical_w, logical_h - chrome_h);
        {
            let mut draw_ctx = DrawCtx {
                measurer: &measurer,
                draw_list: &mut ctx.draw_list,
                bounds: content_bounds,
                focused_widget: None,
                now: Instant::now(),
                animations_running: &animations_running,
                theme: &ui_theme,
                icons: Some(icons),
            };
            let DialogContent::Settings { panel, .. } = &ctx.content;
            panel.draw(&mut draw_ctx);
        }

        // Convert draw list to GPU instances.
        renderer.append_ui_draw_list_with_text(&ctx.draw_list, scale, 1.0, gpu);

        // Draw overlay popups (dropdown lists) on top.
        let overlay_count = ctx.overlays.draw_count();
        if overlay_count > 0 {
            let overlay_bounds = Rect::new(0.0, 0.0, logical_w, logical_h);
            {
                let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
                ctx.overlays.layout_overlays(&measurer, &ui_theme);
            }
            for i in 0..overlay_count {
                ctx.draw_list.clear();
                let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
                let icons = renderer.resolved_icons();
                let mut overlay_draw_ctx = DrawCtx {
                    measurer: &measurer,
                    draw_list: &mut ctx.draw_list,
                    bounds: overlay_bounds,
                    focused_widget: None,
                    now: Instant::now(),
                    animations_running: &animations_running,
                    theme: &ui_theme,
                    icons: Some(icons),
                };
                let opacity =
                    ctx.overlays
                        .draw_overlay_at(i, &mut overlay_draw_ctx, &ctx.layer_tree);
                renderer.append_overlay_draw_list_with_text(&ctx.draw_list, scale, opacity, gpu);
            }
        }

        // Render to surface.
        let result = renderer.render_to_surface(gpu, pipelines, &ctx.surface);
        match result {
            Ok(()) => {}
            Err(crate::gpu::SurfaceError::Lost) => {
                log::warn!("dialog surface lost, reconfiguring");
                ctx.resize_surface(w, h, gpu);
            }
            Err(e) => log::error!("dialog render error: {e}"),
        }
    }

    /// Check if a dialog of the given kind is already open.
    fn has_dialog_of_kind(&self, kind: DialogKind) -> bool {
        self.dialogs.values().any(|ctx| ctx.kind == kind)
    }

    /// Find the best parent window ID for a new dialog.
    fn find_dialog_parent(&self) -> Option<WindowId> {
        // Prefer the focused main/tear-off window.
        if let Some(wid) = self.focused_window_id {
            if self.windows.contains_key(&wid) {
                return Some(wid);
            }
        }
        // Fall back to any main window.
        self.windows.keys().next().copied()
    }

    /// Compute the position to center a dialog on its parent window.
    fn center_on_parent(&self, parent_wid: WindowId, width: u32, height: u32) -> (i32, i32) {
        let Some(ctx) = self.windows.get(&parent_wid) else {
            return (100, 100);
        };
        let parent_pos = ctx.window.window().outer_position().unwrap_or_default();
        let parent_size = ctx.window.window().outer_size();
        let x = parent_pos.x + (parent_size.width as i32 - width as i32) / 2;
        let y = parent_pos.y + (parent_size.height as i32 - height as i32) / 2;
        (x, y)
    }

    /// Create a `UiOnly` renderer for a dialog window.
    fn create_dialog_renderer(
        &self,
        window: &Arc<winit::window::Window>,
        gpu: &GpuState,
        pipelines: &crate::gpu::GpuPipelines,
    ) -> Option<WindowRenderer> {
        let scale = window.scale_factor() as f32;
        let physical_dpi = super::DEFAULT_DPI * scale;
        let hinting = super::config_reload::resolve_hinting(&self.config.font, f64::from(scale));
        let format =
            super::config_reload::resolve_subpixel_mode(&self.config.font, f64::from(scale))
                .glyph_format();

        let ui_fc = self.ui_font_set.as_ref().and_then(|fs| {
            crate::font::FontCollection::new(fs.clone(), 11.0, physical_dpi, format, 400, hinting)
                .ok()
        })?;

        Some(WindowRenderer::new_ui_only(gpu, pipelines, ui_fc))
    }

    /// Build dialog content for the settings panel.
    fn build_settings_content(
        &self,
        window: &Arc<winit::window::Window>,
        renderer: Option<&WindowRenderer>,
    ) -> DialogContent {
        let (mut form, ids) = form_builder::build_settings_form(&self.config);

        // Compute label widths for aligned form layout.
        if let Some(renderer) = renderer {
            let scale = window.scale_factor() as f32;
            let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
            form.compute_label_widths(&measurer, &self.ui_theme);
        }

        DialogContent::Settings {
            panel: SettingsPanel::embedded(form),
            ids,
            pending_config: self.config.clone(),
            original_config: self.config.clone(),
        }
    }
}
