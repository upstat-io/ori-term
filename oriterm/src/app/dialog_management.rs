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
use oriterm_ui::widgets::dialog::{DialogButton, DialogButtons, DialogWidget};
use oriterm_ui::widgets::settings_panel::SettingsPanel;

use super::App;
use super::dialog_context::{DialogContent, DialogWindowContext};
use super::settings_overlay::form_builder;
use crate::event::ConfirmationRequest;
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

        // Install platform chrome subclass for hit testing (close button,
        // caption drag, resize edges). Without this on Windows, the OS
        // default WM_NCHITTEST treats the window edges as resize borders,
        // intercepting cursor events before they reach our hover handler.
        self.install_dialog_chrome(winit_id);

        // Render first frame, then show.
        self.render_dialog(winit_id);
        if let Some(ctx) = self.dialogs.get(&winit_id) {
            ctx.window.set_visible(true);
        }

        log::info!("settings dialog opened: {winit_id:?}, parent: {parent_wid:?}");
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
        let parent_wid = match self.find_dialog_parent() {
            Some(id) => id,
            None => return,
        };

        // Prevent duplicate confirmation dialogs.
        if self.has_dialog_of_kind(DialogKind::Confirmation) {
            log::info!("confirmation dialog already open");
            if let Some((&wid, _)) = self
                .dialogs
                .iter()
                .find(|(_, ctx)| ctx.kind == DialogKind::Confirmation)
            {
                if let Some(ctx) = self.dialogs.get(&wid) {
                    ctx.window.focus_window();
                }
            }
            return;
        }

        let kind = DialogKind::Confirmation;
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
                log::error!("failed to create confirmation dialog window: {e}");
                return;
            }
        };
        let winit_id = window.id();

        // Apply platform native ops: ownership, type hints, modal.
        if let Some(parent_ctx) = self.windows.get(&parent_wid) {
            let parent_win = parent_ctx.window.window();
            let ops = platform_ops();
            ops.set_owner(&window, parent_win);
            ops.set_window_type(&window, &WindowKind::Dialog(kind));
            ops.set_modal(&window, parent_win);
        }

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
                log::error!("failed to create confirmation dialog GPU surface: {e}");
                return;
            }
        };

        // Create UiOnly renderer.
        let renderer = self.create_dialog_renderer(&window, gpu, pipelines);

        // Build confirmation content.
        let content = Self::build_confirmation_content(request);

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

        // Install platform chrome subclass for hit testing.
        self.install_dialog_chrome(winit_id);

        // Render first frame, then show.
        self.render_dialog(winit_id);
        if let Some(ctx) = self.dialogs.get(&winit_id) {
            ctx.window.set_visible(true);
        }

        log::info!("confirmation dialog opened: {winit_id:?}, parent: {parent_wid:?}");
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
            let measurer = UiFontMeasurer::new(renderer.active_ui_collection(), scale);
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
