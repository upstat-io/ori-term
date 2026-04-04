//! Window creation helpers.
//!
//! Creates OS windows with GPU surfaces, renderers, chrome/tab bar widgets,
//! and grid widgets. Extracted from `window_management/mod.rs` to keep it
//! under the 500-line limit.

use winit::event_loop::ActiveEventLoop;
use winit::window::WindowId;

use crate::session::WindowId as SessionWindowId;

use super::super::App;
use super::super::window_context::WindowContext;
use crate::widgets::terminal_grid::TerminalGridWidget;
use crate::window::TermWindow;
use crate::window_manager::types::{ManagedWindow, WindowKind};

impl App {
    /// Create a new terminal window with an initial tab and pane.
    ///
    /// Reuses the existing GPU device, pipelines, and mux. Creates a new winit
    /// window with its own surface, renderer, chrome/tab bar widgets, and mux
    /// window. An initial tab with one pane is spawned in the new window.
    ///
    /// Returns the winit [`WindowId`] of the new window, or `None` on failure.
    pub(in crate::app) fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Option<WindowId> {
        let (winit_id, session_wid) = self.create_window_bare(event_loop)?;

        // Extract geometry from the new window's per-window renderer
        // (scoped to release the borrow before mux operations).
        let (cols, rows) = {
            let ctx = self.windows.get(&winit_id)?;
            let renderer = ctx.renderer.as_ref()?;
            let (w, h) = ctx.window.size_px();
            let cell = renderer.cell_metrics();
            let scale = ctx.window.scale_factor().factor() as f32;
            let hidden =
                self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
            let tb_h = ctx.tab_bar.metrics().height;
            let sb_h = if self.config.window.show_status_bar {
                oriterm_ui::widgets::status_bar::STATUS_BAR_HEIGHT
            } else {
                0.0
            };
            let wl = super::super::chrome::compute_window_layout(
                w, h, &cell, scale, hidden, tb_h, sb_h, 0.0,
            );
            (wl.cols, wl.rows)
        };

        let mux = self.mux.as_mut()?;
        let theme = self
            .config
            .colors
            .resolve_theme(crate::platform::theme::system_theme);
        let spawn_config = oriterm_mux::domain::SpawnConfig {
            cols: cols as u16,
            rows: rows as u16,
            scrollback: self.config.terminal.scrollback,
            shell_integration: self.config.behavior.shell_integration,
            shell: self.config.terminal.shell.clone(),
            ..oriterm_mux::domain::SpawnConfig::default()
        };
        let palette =
            crate::app::config_reload::build_palette_from_config(&self.config.colors, theme);
        let clear_palette = palette.clone();
        let pane_id = match mux.spawn_pane(&spawn_config, theme) {
            Ok(pid) => {
                mux.set_pane_theme(pid, theme, palette);
                mux.discard_notifications();
                pid
            }
            Err(e) => {
                log::error!("failed to create initial tab for new window: {e}");
                mux.discard_notifications();
                self.session.remove_window(session_wid);
                self.windows.remove(&winit_id);
                return None;
            }
        };

        // Local tab creation.
        let tab_id = self.session.alloc_tab_id();
        let tab = crate::session::Tab::new(tab_id, pane_id);
        self.session.add_tab(tab);
        if let Some(win) = self.session.get_window_mut(session_wid) {
            win.add_tab(tab_id);
        }

        // Clear frame and show. Clamp opacity when the surface lacks alpha
        // support so the first frame matches the steady-state render path.
        let palette = clear_palette;
        let opacity = self.config.window.effective_opacity();
        if let Some(gpu) = self.gpu.as_ref() {
            let clear_opacity = if gpu.supports_transparency() {
                opacity
            } else {
                1.0
            };
            if let Some(ctx) = self.windows.get(&winit_id) {
                gpu.clear_surface(ctx.window.surface(), palette.background(), clear_opacity);
            }
        }
        if let Some(ctx) = self.windows.get(&winit_id) {
            ctx.window.set_visible(true);
        }

        // Register with window manager.
        self.window_manager
            .register(ManagedWindow::new(winit_id, WindowKind::Main));
        self.window_manager.set_focused(Some(winit_id));

        // Focus the new window.
        self.focused_window_id = Some(winit_id);
        self.active_window = Some(session_wid);

        log::info!("window created: {winit_id:?} → session {session_wid:?}");

        Some(winit_id)
    }

    /// Create an OS window without spawning any tabs.
    ///
    /// Allocates a GUI-local window ID, creates the OS window + GPU surface,
    /// per-window renderer, chrome/tab bar widgets, and grid widget. The
    /// window starts hidden. The caller is responsible for moving or
    /// creating tabs, clearing the surface, and showing the window.
    ///
    /// Returns `(winit_id, session_window_id)` or `None` on failure.
    pub(in crate::app) fn create_window_bare(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Option<(WindowId, SessionWindowId)> {
        let gpu = self.gpu.as_ref()?;
        let pipelines = self.pipelines.as_ref()?;
        let font_set = self.font_set.as_ref()?.clone();

        let opacity = self.config.window.effective_opacity();
        // Use the actual GPU backend's DComp status, not the config's requested
        // backend. If the GPU fell back from DX12+DComp to Vulkan during init,
        // new windows must not set WS_EX_NOREDIRECTIONBITMAP either.
        let dcomp_active = gpu.uses_dcomp();
        let window_config = oriterm_ui::window::WindowConfig {
            title: "ori".into(),
            transparent: opacity < 1.0,
            blur: self.config.window.blur && opacity < 1.0,
            opacity,
            decoration: super::super::init::decoration_to_mode(self.config.window.decorations),
            use_compositor_surface: dcomp_active && opacity < 1.0,
            ..oriterm_ui::window::WindowConfig::default()
        };

        // Allocate a GUI-local window ID (mux is a flat pane server).
        let session_wid = self.session.alloc_window_id();
        self.session
            .add_window(crate::session::Window::new(session_wid));

        let window = match TermWindow::new(event_loop, &window_config, gpu, session_wid) {
            Ok(w) => w,
            Err(e) => {
                log::error!("failed to create window: {e}");
                self.session.remove_window(session_wid);
                return None;
            }
        };

        // Chrome + tab bar widgets.
        let tab_bar_widget = self.create_tab_bar_widget(&window);

        let Some(renderer) = self.create_window_renderer(&window, gpu, pipelines, font_set) else {
            self.session.remove_window(session_wid);
            return None;
        };

        // Compute grid dimensions via layout engine (Column { TabBar, Grid }).
        let (w, h) = window.size_px();
        let cell = renderer.cell_metrics();
        let scale = window.scale_factor().factor() as f32;
        let hidden = self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
        let tb_h = tab_bar_widget.metrics().height;
        let sb_h = if self.config.window.show_status_bar {
            oriterm_ui::widgets::status_bar::STATUS_BAR_HEIGHT
        } else {
            0.0
        };
        let wl = super::super::chrome::compute_window_layout(
            w, h, &cell, scale, hidden, tb_h, sb_h, 0.0,
        );

        // Terminal grid widget.
        let cols = wl.cols;
        let rows = wl.rows;
        let grid_widget = TerminalGridWidget::new(cell.width, cell.height, cols, rows);
        grid_widget.set_bounds(wl.grid_rect);

        // Status bar widget (bottom metadata bar).
        let status_bar_widget =
            oriterm_ui::widgets::status_bar::StatusBarWidget::new(w as f32 / scale, &self.ui_theme);

        let winit_id = window.window_id();
        let ctx = WindowContext::new(
            window,
            tab_bar_widget,
            status_bar_widget,
            grid_widget,
            Some(renderer),
        );
        self.windows.insert(winit_id, ctx);

        log::info!(
            "bare window created: {winit_id:?} → session {session_wid:?}, \
             {w}x{h} px, {cols}x{rows} cells"
        );

        Some((winit_id, session_wid))
    }

    /// Build a per-window renderer for the given window's DPI and font config.
    fn create_window_renderer(
        &self,
        window: &TermWindow,
        gpu: &crate::gpu::GpuState,
        pipelines: &crate::gpu::GpuPipelines,
        font_set: crate::font::FontSet,
    ) -> Option<crate::gpu::WindowRenderer> {
        let scale = window.scale_factor().factor() as f32;
        let physical_dpi = super::super::DEFAULT_DPI * scale;
        let hinting =
            super::super::config_reload::resolve_hinting(&self.config.font, f64::from(scale));
        let opacity = f64::from(self.config.window.effective_opacity());
        let format = super::super::config_reload::resolve_subpixel_mode(
            &self.config.font,
            f64::from(scale),
            opacity,
        )
        .glyph_format();
        let weight = self.config.font.effective_weight();
        let bold_weight = self.config.font.effective_bold_weight();

        let mut font_collection = match crate::font::FontCollection::new(
            font_set,
            self.config.font.size,
            physical_dpi,
            format,
            weight,
            bold_weight,
            hinting,
        ) {
            Ok(fc) => fc,
            Err(e) => {
                log::error!("failed to create font collection for new window: {e}");
                return None;
            }
        };
        super::super::config_reload::apply_font_config(
            &mut font_collection,
            &self.config.font,
            &self.user_fallback_map,
        );

        // UI font registry: embedded IBM Plex Mono with forced grayscale + no hinting.
        let ui_sizes = crate::font::UiFontSizes::new(
            crate::font::FontSet::ui_embedded(),
            physical_dpi,
            crate::font::GlyphFormat::Alpha,
            crate::font::HintingMode::None,
            400,
            600,
            crate::font::ui_font_sizes::PRELOAD_SIZES,
        )
        .ok()
        .map(|mut sizes| {
            super::super::config_reload::apply_font_config_to_ui_sizes(
                &mut sizes,
                &self.config.font,
                &self.user_fallback_map,
            );
            sizes
        });

        let mut renderer =
            crate::gpu::WindowRenderer::new(gpu, pipelines, font_collection, ui_sizes);
        let scale_f64 = f64::from(scale);
        let subpx_pos =
            super::super::config_reload::resolve_subpixel_positioning(&self.config.font, scale_f64);
        renderer.set_subpixel_positioning(subpx_pos);
        let atlas_filter =
            super::super::config_reload::resolve_atlas_filtering(&self.config.font, scale_f64);
        renderer.set_atlas_filtering(atlas_filter, gpu, &pipelines.atlas_layout);
        Some(renderer)
    }
}
