//! Dialog window lifecycle: open, close, and content creation.
//!
//! Coordinates dialog OS window creation with the window manager, platform
//! native ops, and GPU renderer. Each dialog is a real OS window with its
//! own surface — moveable independently of the parent terminal window.
//! Rendering lives in `dialog_rendering.rs`.

use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use oriterm_ui::scale::ScaleFactor;
use oriterm_ui::surface::SurfaceLifecycle;
use oriterm_ui::widgets::dialog::{DialogButton, DialogButtons, DialogWidget};
use oriterm_ui::widgets::settings_panel::SettingsPanel;

use super::App;
use super::dialog_context::{DialogContent, DialogWindowContext};
use super::settings_overlay::form_builder;
use crate::event::ConfirmationRequest;
use crate::font::{CachedTextMeasurer, TextShapeCache, UiFontMeasurer};
use crate::gpu::state::GpuState;
use crate::gpu::window_renderer::WindowRenderer;
use crate::window_manager::platform::platform_ops;
use crate::window_manager::types::{DialogKind, ManagedWindow, WindowKind};

/// Intermediate state from creating a dialog OS window and GPU surface.
///
/// Produced by [`App::create_dialog_window`], consumed by
/// [`App::finalize_dialog`]. Separates OS window setup (common to all
/// dialog kinds) from content creation (kind-specific).
struct DialogWindowParts {
    window: Arc<winit::window::Window>,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    renderer: Option<WindowRenderer>,
    parent_wid: WindowId,
}

impl App {
    /// Open a settings dialog as a real OS window.
    ///
    /// Creates a frameless window centered on the parent, with platform-native
    /// ownership and shadow. The dialog uses a `UiOnly` renderer — no terminal
    /// grid or pane state.
    pub(super) fn open_settings_dialog(&mut self, event_loop: &ActiveEventLoop) {
        let kind = DialogKind::Settings;
        let Some(parts) = self.create_dialog_window(kind, event_loop) else {
            return;
        };

        // Set min inner size for settings dialog.
        parts
            .window
            .set_min_inner_size(Some(winit::dpi::LogicalSize::new(600u32, 400u32)));

        let content = self.build_settings_content(&parts.window, parts.renderer.as_ref());
        self.finalize_dialog(parts, kind, content);
    }

    /// Open a confirmation dialog as a real OS window.
    ///
    /// The dialog is modal: it blocks input to its parent window until
    /// dismissed. The `request.kind` determines what action is taken when
    /// the user clicks OK.
    pub(super) fn open_confirmation_dialog(
        &mut self,
        event_loop: &ActiveEventLoop,
        request: ConfirmationRequest,
    ) {
        let kind = DialogKind::Confirmation;
        let Some(parts) = self.create_dialog_window(kind, event_loop) else {
            return;
        };

        // Modal: disable parent window input.
        if let Some(parent_ctx) = self.windows.get(&parts.parent_wid) {
            platform_ops().set_modal(&parts.window, parent_ctx.window.window());
        }

        let content = Self::build_confirmation_content(request);
        self.finalize_dialog(parts, kind, content);
    }

    /// Create a dialog OS window with GPU surface and renderer.
    ///
    /// Handles duplicate prevention, parent lookup, window creation,
    /// platform ownership, type hints, and GPU setup. Returns `None` if
    /// any step fails (no parent, duplicate, GPU error).
    fn create_dialog_window(
        &self,
        kind: DialogKind,
        event_loop: &ActiveEventLoop,
    ) -> Option<DialogWindowParts> {
        let parent_wid = self.find_dialog_parent()?;

        // Prevent duplicate dialogs of the same kind.
        if self.has_dialog_of_kind(kind) {
            log::info!("{} dialog already open", kind.title());
            if let Some((_, ctx)) = self.dialogs.iter().find(|(_, ctx)| ctx.kind == kind) {
                ctx.window.focus_window();
            }
            return None;
        }

        let (width, height) = kind.default_size();
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
                log::error!("failed to create {} dialog window: {e}", kind.title());
                return None;
            }
        };

        // Platform ownership and type hints.
        if let Some(parent_ctx) = self.windows.get(&parent_wid) {
            let parent_win = parent_ctx.window.window();
            let ops = platform_ops();
            ops.set_owner(&window, parent_win);
            ops.set_window_type(&window, &WindowKind::Dialog(kind));
        }

        // GPU surface and renderer.
        let gpu = self.gpu.as_ref()?;
        let pipelines = self.pipelines.as_ref()?;
        let (surface, surface_config) = match gpu.create_surface(&window) {
            Ok(s) => s,
            Err(e) => {
                log::error!("failed to create dialog GPU surface: {e}");
                return None;
            }
        };
        let renderer = self.create_dialog_renderer(&window, gpu, pipelines);

        Some(DialogWindowParts {
            window,
            surface,
            surface_config,
            renderer,
            parent_wid,
        })
    }

    /// Register, store, install chrome, render first frame, and show a dialog.
    fn finalize_dialog(
        &mut self,
        parts: DialogWindowParts,
        kind: DialogKind,
        content: DialogContent,
    ) {
        let winit_id = parts.window.id();
        let scale_factor = ScaleFactor::new(parts.window.scale_factor());
        let ctx = DialogWindowContext::new(
            parts.window.clone(),
            parts.surface,
            parts.surface_config,
            parts.renderer,
            kind,
            content,
            scale_factor,
            &self.ui_theme,
        );

        self.window_manager.register(ManagedWindow::with_parent(
            winit_id,
            WindowKind::Dialog(kind),
            parts.parent_wid,
        ));
        self.dialogs.insert(winit_id, ctx);
        self.install_dialog_chrome(winit_id);
        self.render_dialog(winit_id);

        // Transition to Primed — the event loop's about_to_wait handler
        // will show the window on the next tick (after the first frame is
        // committed). This prevents any flash of uninitialized content.
        if let Some(ctx) = self.dialogs.get_mut(&winit_id) {
            ctx.lifecycle = ctx.lifecycle.transition(SurfaceLifecycle::Primed);
        }

        log::info!(
            "{} dialog opened: {winit_id:?}, parent: {:?}",
            kind.title(),
            parts.parent_wid,
        );
    }

    /// Close a dialog window, discarding any pending changes.
    ///
    /// Transitions to `Closing`: hides the window, clears modal state,
    /// and suppresses further input. Actual destruction (unregister +
    /// context removal) is deferred to the next `about_to_wait` tick
    /// to avoid mutable borrow issues during event dispatch.
    pub(super) fn close_dialog(&mut self, winit_id: WindowId) {
        // Transition to Closing: hide and clear modal state.
        if let Some(ctx) = self.dialogs.get_mut(&winit_id) {
            if ctx.lifecycle == SurfaceLifecycle::Closing
                || ctx.lifecycle == SurfaceLifecycle::Destroyed
            {
                return; // Already closing/closed.
            }
            ctx.lifecycle = ctx.lifecycle.transition(SurfaceLifecycle::Closing);
            #[cfg(target_os = "windows")]
            oriterm_ui::platform_windows::set_transitions_enabled(&ctx.window, false);
            ctx.window.set_visible(false);
        }

        // Clear platform modal state.
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

        // Defer destruction to the next event loop tick.
        self.pending_destroy.push(winit_id);

        log::info!("dialog closing: {winit_id:?}");
    }

    /// Complete deferred dialog destruction.
    ///
    /// Called from `about_to_wait` to transition `Closing → Destroyed`,
    /// unregister from the window manager, and drop GPU resources.
    pub(super) fn drain_pending_destroy(&mut self) {
        if self.pending_destroy.is_empty() {
            return;
        }
        for wid in self.pending_destroy.drain(..) {
            if let Some(ctx) = self.dialogs.get_mut(&wid) {
                ctx.lifecycle = ctx.lifecycle.transition(SurfaceLifecycle::Destroyed);
            }
            self.window_manager.unregister(wid);
            self.dialogs.remove(&wid);
            log::info!(
                "dialog destroyed: {wid:?}, {} dialogs remaining",
                self.dialogs.len()
            );
        }
    }

    /// Show all Primed dialogs (first frame rendered, ready to be visible).
    ///
    /// Called from `about_to_wait` to transition `Primed → Visible`.
    /// Platform DWM transition suppression is handled here.
    pub(super) fn show_primed_dialogs(&mut self) {
        for ctx in self.dialogs.values_mut() {
            if ctx.lifecycle != SurfaceLifecycle::Primed {
                continue;
            }
            #[cfg(target_os = "windows")]
            oriterm_ui::platform_windows::set_transitions_enabled(&ctx.window, false);
            ctx.window.set_visible(true);
            #[cfg(target_os = "windows")]
            oriterm_ui::platform_windows::set_transitions_enabled(&ctx.window, true);
            ctx.lifecycle = ctx.lifecycle.transition(SurfaceLifecycle::Visible);
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
    ///
    /// All arithmetic is in physical pixels (`outer_position` and `outer_size`
    /// return physical coordinates). The result is converted back to logical
    /// for winit's `LogicalPosition`.
    fn center_on_parent(&self, parent_wid: WindowId, width: u32, height: u32) -> (i32, i32) {
        let Some(ctx) = self.windows.get(&parent_wid) else {
            return (100, 100);
        };
        let parent_win = ctx.window.window();
        let scale = parent_win.scale_factor();
        let parent_pos = parent_win.outer_position().unwrap_or_default();
        let parent_size = parent_win.outer_size();

        // Convert dialog logical size to physical for consistent math.
        let phys_w = (width as f64 * scale).round() as i32;
        let phys_h = (height as f64 * scale).round() as i32;
        let mut x = parent_pos.x + (parent_size.width as i32 - phys_w) / 2;
        let mut y = parent_pos.y + (parent_size.height as i32 - phys_h) / 2;

        // Clamp to the current monitor's physical bounds.
        if let Some(monitor) = parent_win.current_monitor() {
            let mon_pos = monitor.position();
            let mon_size = monitor.size();
            let max_x = mon_pos.x + mon_size.width as i32 - phys_w;
            let max_y = mon_pos.y + mon_size.height as i32 - phys_h;
            x = x.clamp(mon_pos.x, max_x.max(mon_pos.x));
            y = y.clamp(mon_pos.y, max_y.max(mon_pos.y));
        }

        // Convert physical position back to logical for winit's LogicalPosition.
        let lx = (x as f64 / scale).round() as i32;
        let ly = (y as f64 / scale).round() as i32;
        (lx, ly)
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

    /// Build dialog content for a confirmation dialog.
    fn build_confirmation_content(request: ConfirmationRequest) -> DialogContent {
        let mut dialog = DialogWidget::new(&request.title)
            .with_message(request.message)
            .with_buttons(DialogButtons::OkCancel)
            .with_ok_label(request.ok_label)
            .with_cancel_label(request.cancel_label)
            .with_default_button(DialogButton::Ok);
        if let Some(content) = request.content {
            dialog = dialog.with_content(content);
        }
        DialogContent::Confirmation {
            dialog: Box::new(dialog),
            kind: request.kind,
        }
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
            let cache = TextShapeCache::new();
            let measurer = CachedTextMeasurer::new(
                UiFontMeasurer::new(renderer.active_ui_collection(), scale),
                &cache,
                scale,
            );
            form.compute_label_widths(&measurer, &self.ui_theme);
        }

        DialogContent::Settings {
            panel: Box::new(SettingsPanel::embedded(form)),
            ids,
            pending_config: Box::new(self.config.clone()),
            original_config: Box::new(self.config.clone()),
        }
    }

    /// Install platform chrome on a dialog window.
    ///
    /// Enables proper OS-level hit testing so the close button receives
    /// cursor events and the caption area supports drag. Routes through
    /// [`NativeChromeOps`] — no-op on non-Windows platforms.
    fn install_dialog_chrome(&self, winit_id: WindowId) {
        let Some(ctx) = self.dialogs.get(&winit_id) else {
            return;
        };
        let scale = ctx.scale_factor.factor() as f32;
        let mode = crate::window_manager::platform::ChromeMode::Dialog {
            resizable: ctx.kind.is_resizable(),
        };
        super::chrome::install_chrome(
            &ctx.window,
            mode,
            ctx.chrome.interactive_rects(),
            ctx.chrome.caption_height(),
            scale,
        );
    }

    /// Update platform hit test rects and chrome metrics after a dialog resize.
    ///
    /// Routes through [`NativeChromeOps`] — no-op on non-Windows platforms.
    pub(super) fn refresh_dialog_platform_rects(&self, winit_id: WindowId) {
        let Some(ctx) = self.dialogs.get(&winit_id) else {
            return;
        };
        let scale = ctx.scale_factor.factor() as f32;
        super::chrome::refresh_chrome(
            &ctx.window,
            ctx.chrome.interactive_rects(),
            ctx.chrome.caption_height(),
            scale,
            ctx.kind.is_resizable(),
        );
    }
}
