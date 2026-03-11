//! One-shot application startup: window → GPU → mux → fonts → renderer → tab.

use winit::event_loop::ActiveEventLoop;

use oriterm_mux::domain::SpawnConfig;
use oriterm_ui::window::WindowConfig;

use super::window_context::WindowContext;
use super::{App, DEFAULT_DPI};
use crate::app::config_reload;
use crate::font::{FontCollection, FontSet, GlyphFormat, HintingMode};
use crate::gpu::{GpuPipelines, GpuState, WindowRenderer};
use crate::widgets::terminal_grid::TerminalGridWidget;
use crate::window::TermWindow;
use crate::window_manager::types::{ManagedWindow, WindowKind};

impl App {
    /// Run the one-shot startup sequence: window → GPU → fonts → renderer → tab.
    ///
    /// Returns `Err` with a displayable message on any failure. The caller
    /// logs the error and exits the event loop.
    #[expect(
        clippy::too_many_lines,
        reason = "one-shot startup sequence: window → GPU → fonts → renderer → tab → show"
    )]
    pub(super) fn try_init(
        &mut self,
        event_loop: &ActiveEventLoop,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let t_start = std::time::Instant::now();

        // Build UI window config from the user's config.
        let opacity = self.config.window.effective_opacity();
        let window_config = WindowConfig {
            title: "ori".into(),
            transparent: opacity < 1.0,
            blur: self.config.window.blur && opacity < 1.0,
            opacity,
            ..WindowConfig::default()
        };

        // 1. Create window (invisible) for GPU surface capability probing.
        let window_arc = oriterm_ui::window::create_window(event_loop, &window_config)?;
        let t_window = t_start.elapsed();

        // 2. Spawn font discovery on a background thread (no GPU dependency).
        let font_handle = self.spawn_font_discovery()?;

        // 3. Init GPU on main thread (requires window Arc, runs concurrently with fonts).
        let t_gpu_start = std::time::Instant::now();
        let gpu = GpuState::new(&window_arc, window_config.transparent)?;
        let t_gpu = t_gpu_start.elapsed();

        // 4. Allocate a GUI-local window ID (mux is a flat pane server).
        //    In daemon mode, the window may already be claimed via `--window`.
        let session_wid = if let Some(claimed) = self.active_window {
            claimed
        } else {
            self.session.alloc_window_id()
        };

        // Register window in local session.
        self.session
            .add_window(crate::session::Window::new(session_wid));

        // 5. Wrap the same window into TermWindow (creates surface, applies effects).
        let window = TermWindow::from_window(window_arc, &window_config, &gpu, session_wid)?;

        // 6. Join font thread (GPU init + surface setup ran concurrently).
        let (mut font_collection, user_fb_count, t_fonts) = match font_handle.join() {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => return Err(e.into()),
            Err(_) => return Err("font discovery thread panicked".into()),
        };

        // 6b. Rescale fonts to physical DPI so glyph bitmaps match the
        // physical surface resolution. At 1.5x scaling: 96 * 1.5 = 144 DPI,
        // producing glyphs that are 1.5x larger in pixels — exactly matching
        // the physical surface. Cell metrics become physical pixels.
        let scale = window.scale_factor().factor();
        let physical_dpi = DEFAULT_DPI * scale as f32;
        if let Err(e) = font_collection.set_size(self.config.font.size, physical_dpi) {
            log::error!("font set_size failed: {e}");
        }

        // 6c. Adjust hinting and subpixel mode for the actual display scale factor.
        // Config overrides take priority over auto-detection.
        let hinting = config_reload::resolve_hinting(&self.config.font, scale);
        font_collection.set_hinting(hinting);
        let subpixel_format =
            config_reload::resolve_subpixel_mode(&self.config.font, scale).glyph_format();
        font_collection.set_format(subpixel_format);

        // 6d. Apply font config: features, per-fallback metadata, codepoint map.
        config_reload::apply_font_config(&mut font_collection, &self.config.font, user_fb_count);

        // 7a. Create shared pipelines (once).
        let t_renderer_start = std::time::Instant::now();
        let pipelines = GpuPipelines::new(&gpu);

        // 7b. Cache FontSet for new windows. Re-load from config (the
        // font_set was consumed when creating font_collection above).
        let cached_font_set = {
            let mut fs = FontSet::load(
                self.config.font.family.as_deref(),
                self.config.font.effective_weight(),
            )?;
            let user_fb_families: Vec<&str> = self
                .config
                .font
                .fallback
                .iter()
                .map(|f| f.family.as_str())
                .collect();
            fs.prepend_user_fallbacks(&user_fb_families);
            fs
        };

        // 7c. UI font discovery + cache.
        let ui_font_set = discover_ui_font_set();
        let ui_fc = ui_font_set.as_ref().and_then(|fs| {
            FontCollection::new(
                fs.clone(),
                11.0,
                physical_dpi,
                subpixel_format,
                400,
                hinting,
            )
            .ok()
        });

        // 7d. Create per-window renderer.
        let renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, ui_fc);
        let t_renderer = t_renderer_start.elapsed();

        // 8. Create tab bar widget and apply platform effects.
        let (w, h) = window.size_px();
        let tab_bar_widget = self.create_tab_bar_widget(&window);

        // 9. Compute grid dimensions from viewport, offset by chrome height.
        let tab_bar_h = oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT;
        let cell = renderer.cell_metrics();
        let scale = window.scale_factor().factor() as f32;
        let origin_y = super::chrome::grid_origin_y(tab_bar_h, scale);
        let chrome_px = origin_y as u32;
        let grid_h = h.saturating_sub(chrome_px);
        let cols = cell.columns(w).max(1);
        let rows = cell.rows(grid_h).max(1);

        // 10. Create grid widget with cell metrics and initial grid size.
        let grid_widget = TerminalGridWidget::new(cell.width, cell.height, cols, rows);
        grid_widget.set_bounds(oriterm_ui::geometry::Rect::new(
            0.0,
            origin_y,
            cols as f32 * cell.width,
            rows as f32 * cell.height,
        ));

        // 11. Create initial tab + pane (skip if daemon mode with a claimed window).
        let t_mux_start = std::time::Instant::now();
        let is_daemon = self.mux.as_ref().is_some_and(|m| m.is_daemon_mode());
        let is_claimed = is_daemon && self.active_window.is_some();
        if !is_claimed {
            self.create_initial_tab(session_wid, rows as u16, cols as u16)?;
        }
        let t_mux = t_mux_start.elapsed();

        let t_total = t_start.elapsed();
        log::info!(
            "app: startup — window={t_window:?} gpu={t_gpu:?} fonts={t_fonts:?} \
             renderer={t_renderer:?} mux={t_mux:?} total={t_total:?}",
        );
        log::info!(
            "app: initialized — {w}x{h} px, {cols} cols × {rows} rows, \
             chrome={tab_bar_h}px, font={} {:.1}pt",
            renderer.family_name(),
            self.config.font.size,
        );

        // Clear frame with theme background before showing (prevents white flash).
        let theme = self
            .config
            .colors
            .resolve_theme(crate::platform::theme::system_theme);
        let palette = config_reload::build_palette_from_config(&self.config.colors, theme);
        gpu.clear_surface(window.surface(), palette.background(), opacity);
        window.set_visible(true);
        // On Linux (X11/Wayland), a newly created window is not guaranteed to
        // receive input focus. Explicitly request it so the terminal is
        // immediately interactive.
        window.window().focus_window();

        let winit_id = window.window_id();
        let ctx = WindowContext::new(window, tab_bar_widget, grid_widget, Some(renderer));
        self.gpu = Some(gpu);
        self.pipelines = Some(pipelines);
        self.font_set = Some(cached_font_set);
        self.ui_font_set = ui_font_set;
        self.user_fb_count = user_fb_count;
        self.windows.insert(winit_id, ctx);
        self.window_manager
            .register(ManagedWindow::new(winit_id, WindowKind::Main));
        self.window_manager.set_focused(Some(winit_id));
        self.focused_window_id = Some(winit_id);
        self.active_window = Some(session_wid);
        Ok(())
    }

    /// Spawn font discovery on a background thread.
    #[expect(
        clippy::type_complexity,
        reason = "thread join handle with font discovery result — not worth a type alias"
    )]
    fn spawn_font_discovery(
        &self,
    ) -> Result<
        std::thread::JoinHandle<
            Result<(FontCollection, usize, std::time::Duration), crate::font::FontError>,
        >,
        Box<dyn std::error::Error>,
    > {
        let font_weight = self.config.font.effective_weight();
        let font_size_pt = self.config.font.size;
        let font_config = self.config.font.clone();
        let font_dpi = DEFAULT_DPI;

        std::thread::Builder::new()
            .name("font-discovery".into())
            .spawn(move || {
                let t0 = std::time::Instant::now();
                let mut font_set = FontSet::load(font_config.family.as_deref(), font_weight)?;

                // Prepend user-configured fallback fonts.
                let user_fb_families: Vec<&str> = font_config
                    .fallback
                    .iter()
                    .map(|f| f.family.as_str())
                    .collect();
                let user_fb_count = font_set.prepend_user_fallbacks(&user_fb_families);

                // Default to Full hinting + Alpha format; adjusted after window
                // creation once the actual display scale factor is known.
                let fc = FontCollection::new(
                    font_set,
                    font_size_pt,
                    font_dpi,
                    GlyphFormat::Alpha,
                    font_weight,
                    HintingMode::Full,
                )?;
                Ok((fc, user_fb_count, t0.elapsed()))
            })
            .map_err(|e| -> Box<dyn std::error::Error> {
                format!("failed to spawn font discovery thread: {e}").into()
            })
    }

    /// Create a tab bar widget and install platform window chrome.
    ///
    /// The tab bar is the sole chrome bar (unified tab-in-titlebar).
    /// Chrome installation (Aero Snap on Windows, no-op on other platforms)
    /// goes through [`NativeChromeOps`] — no `#[cfg]` blocks needed.
    pub(super) fn create_tab_bar_widget(
        &self,
        window: &TermWindow,
    ) -> oriterm_ui::widgets::tab_bar::TabBarWidget {
        let (w, _) = window.size_px();
        let scale = window.scale_factor().factor() as f32;
        let logical_w = w as f32 / scale;
        let tab_bar_h = oriterm_ui::widgets::tab_bar::constants::TAB_BAR_HEIGHT;

        // Install platform chrome (Aero Snap subclass on Windows, no-op elsewhere).
        // Empty rects — the tab bar widget is created next.
        super::chrome::install_chrome(
            window.window(),
            crate::window_manager::platform::ChromeMode::Main,
            &[],
            tab_bar_h,
            scale,
        );

        let mut tab_bar_widget =
            oriterm_ui::widgets::tab_bar::TabBarWidget::with_theme(logical_w, &self.ui_theme);

        // Reserve space for macOS traffic light buttons on the left.
        #[cfg(target_os = "macos")]
        tab_bar_widget
            .set_left_inset(oriterm_ui::widgets::tab_bar::constants::MACOS_TRAFFIC_LIGHT_WIDTH);

        tab_bar_widget.set_tabs(vec![oriterm_ui::widgets::tab_bar::TabEntry::new("")]);

        // Set initial platform hit test rects from the tab bar.
        super::chrome::refresh_chrome(
            window.window(),
            &tab_bar_widget.interactive_rects(),
            tab_bar_h,
            scale,
            true,
        );

        tab_bar_widget
    }

    /// Create an initial tab with one pane in the given mux window.
    ///
    /// The mux backend and window must already exist. The pane is stored
    /// inside the backend.
    pub(super) fn create_initial_tab(
        &mut self,
        session_wid: crate::session::WindowId,
        rows: u16,
        cols: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let theme = self
            .config
            .colors
            .resolve_theme(crate::platform::theme::system_theme);

        let config = SpawnConfig {
            cols,
            rows,
            scrollback: self.config.terminal.scrollback,
            shell_integration: self.config.behavior.shell_integration,
            ..SpawnConfig::default()
        };

        let palette = config_reload::build_palette_from_config(&self.config.colors, theme);

        let mux = self.mux.as_mut().ok_or("mux backend missing")?;
        let pane_id = mux.spawn_pane(&config, theme)?;

        // Apply color scheme + user overrides to the pane's terminal palette.
        mux.set_pane_theme(pane_id, theme, palette);

        // Apply image protocol config.
        mux.set_image_config(pane_id, self.config.terminal.image_config());

        // Local tab creation.
        let tab_id = self.session.alloc_tab_id();
        let tab = crate::session::Tab::new(tab_id, pane_id);
        self.session.add_tab(tab);
        if let Some(win) = self.session.get_window_mut(session_wid) {
            win.add_tab(tab_id);
        }

        Ok(())
    }
}

/// Discover the system UI font (proportional sans-serif) for tab bar and overlays.
///
/// Returns `None` if no suitable font is found — the terminal font is used
/// as a fallback in that case.
fn discover_ui_font_set() -> Option<FontSet> {
    let discovery = crate::font::discovery::discover_ui_fonts();
    FontSet::from_discovery(&discovery).ok()
}
