//! One-shot application startup: window → GPU → mux → fonts → renderer → tab.

use winit::event_loop::ActiveEventLoop;

use oriterm_mux::domain::SpawnConfig;
use oriterm_ui::window::WindowConfig;

use super::window_context::WindowContext;
use super::{App, DEFAULT_DPI};
use crate::app::config_reload;
use crate::font::{FontByteCache, FontCollection, FontSet, GlyphFormat, HintingMode};
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
        // DComp transparency (WS_EX_NOREDIRECTIONBITMAP) only works on DX12.
        // Vulkan has no DComp path — setting it makes the window invisible.
        let dcomp_available = matches!(
            self.config.rendering.gpu_backend,
            crate::config::GpuBackend::Auto | crate::config::GpuBackend::DirectX12
        );
        let window_config = WindowConfig {
            title: "ori".into(),
            transparent: opacity < 1.0,
            blur: self.config.window.blur && opacity < 1.0,
            opacity,
            decoration: decoration_to_mode(self.config.window.decorations),
            use_compositor_surface: dcomp_available && opacity < 1.0,
            ..WindowConfig::default()
        };

        // 1. Create window (invisible) for GPU surface capability probing.
        let window_arc = oriterm_ui::window::create_window(event_loop, &window_config)?;
        let t_window = t_start.elapsed();

        // 2. Spawn font discovery on a background thread (no GPU dependency).
        let font_handle = self.spawn_font_discovery()?;

        // 3. Init GPU on main thread (requires window Arc, runs concurrently with fonts).
        let t_gpu_start = std::time::Instant::now();
        let gpu = GpuState::new(
            &window_arc,
            window_config.transparent,
            self.config.rendering.gpu_backend,
        )?;
        let t_gpu = t_gpu_start.elapsed();

        // If the window was created for DComp but the GPU fell back to a
        // non-DComp backend, remove WS_EX_NOREDIRECTIONBITMAP so the window
        // is visible. Without this, Vulkan or plain DX12 inherit a compositor-
        // surface window they cannot present to.
        if window_config.use_compositor_surface && !gpu.uses_dcomp() {
            log::warn!(
                "GPU did not use DirectComposition — clearing compositor surface flag \
                 to prevent invisible window"
            );
            oriterm_ui::window::clear_compositor_surface_flag(&window_arc);
        }

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
        let (mut font_collection, cached_font_set, font_cache, fallback_map, t_fonts) =
            match font_handle.join() {
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
        let subpixel_format = config_reload::resolve_subpixel_mode(
            &self.config.font,
            scale,
            f64::from(self.config.window.effective_opacity()),
        )
        .glyph_format();
        font_collection.set_format(subpixel_format);

        // 6d. Apply font config: features, per-fallback metadata, codepoint map.
        config_reload::apply_font_config(&mut font_collection, &self.config.font, &fallback_map);

        // 7a. Create shared pipelines (once).
        let t_renderer_start = std::time::Instant::now();
        let pipelines = GpuPipelines::new(&gpu);

        // 7b. FontSet cached from thread (Arc-cloned before FontCollection
        // consumed it — zero disk reads).

        // 7c. UI font registry: exact-size collections for all UI text sizes.
        // Uses embedded IBM Plex Mono with forced grayscale + no hinting so
        // the settings dialog renders identically on all platforms.
        drop(font_cache);
        let ui_font_set = FontSet::ui_embedded();
        let ui_sizes = crate::font::UiFontSizes::new(
            ui_font_set,
            physical_dpi,
            GlyphFormat::Alpha,
            HintingMode::None,
            400,
            crate::font::ui_font_sizes::PRELOAD_SIZES,
        )
        .ok()
        .map(|mut sizes| {
            config_reload::apply_font_config_to_ui_sizes(
                &mut sizes,
                &self.config.font,
                &fallback_map,
            );
            sizes
        });

        // 7d. Create per-window renderer.
        let mut renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, ui_sizes);
        let subpx_pos = config_reload::resolve_subpixel_positioning(&self.config.font, scale);
        renderer.set_subpixel_positioning(subpx_pos);
        let atlas_filter = config_reload::resolve_atlas_filtering(&self.config.font, scale);
        renderer.set_atlas_filtering(atlas_filter, &gpu, &pipelines.atlas_layout);
        let t_renderer = t_renderer_start.elapsed();

        // 8. Create tab bar widget and apply platform effects.
        let (w, h) = window.size_px();
        let tab_bar_widget = self.create_tab_bar_widget(&window);

        // 9. Compute grid dimensions via layout engine (Column { TabBar, Grid }).
        let cell = renderer.cell_metrics();
        let scale = window.scale_factor().factor() as f32;
        let hidden = self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
        let tb_h = tab_bar_widget.metrics().height;
        let sb_h = if self.config.window.show_status_bar {
            oriterm_ui::widgets::status_bar::STATUS_BAR_HEIGHT
        } else {
            0.0
        };
        let wl = super::chrome::compute_window_layout(w, h, &cell, scale, hidden, tb_h, sb_h, 0.0);

        // 10. Create grid widget with cell metrics and layout-computed size.
        let grid_widget = TerminalGridWidget::new(cell.width, cell.height, wl.cols, wl.rows);
        grid_widget.set_bounds(wl.grid_rect);

        // 11. Create initial tab + pane (skip if daemon mode with a claimed window).
        let t_mux_start = std::time::Instant::now();
        let is_daemon = self.mux.as_ref().is_some_and(|m| m.is_daemon_mode());
        let is_claimed = is_daemon && self.active_window.is_some();
        if !is_claimed {
            self.create_initial_tab(session_wid, wl.rows as u16, wl.cols as u16)?;
        }
        let t_mux = t_mux_start.elapsed();

        let t_total = t_start.elapsed();
        log::info!(
            "app: startup — window={t_window:?} gpu={t_gpu:?} fonts={t_fonts:?} \
             renderer={t_renderer:?} mux={t_mux:?} total={t_total:?}",
        );
        let tab_bar_h = if hidden { 0.0 } else { tb_h };
        log::info!(
            "app: initialized — {w}x{h} px, {} cols × {} rows, \
             chrome={tab_bar_h}px, font={} {:.1}pt",
            wl.cols,
            wl.rows,
            renderer.family_name(),
            self.config.font.size,
        );

        // Clear frame with theme background before showing (prevents white flash).
        let theme = self
            .config
            .colors
            .resolve_theme(crate::platform::theme::system_theme);
        let palette = config_reload::build_palette_from_config(&self.config.colors, theme);
        // Clamp opacity to 1.0 when the surface doesn't support alpha.
        // On Vulkan/opaque fallback, sub-1.0 opacity would produce a
        // broken first frame before the steady-state render path clamps it.
        let clear_opacity = if gpu.supports_transparency() {
            opacity
        } else {
            1.0
        };
        gpu.clear_surface(window.surface(), palette.background(), clear_opacity);
        window.set_visible(true);
        // On Linux (X11/Wayland), a newly created window is not guaranteed to
        // receive input focus. Explicitly request it so the terminal is
        // immediately interactive.
        window.window().focus_window();

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
        self.gpu = Some(gpu);
        self.pipelines = Some(pipelines);
        self.font_set = Some(cached_font_set);
        self.user_fallback_map = fallback_map;
        self.windows.insert(winit_id, ctx);
        self.window_manager
            .register(ManagedWindow::new(winit_id, WindowKind::Main));
        self.window_manager.set_focused(Some(winit_id));
        self.focused_window_id = Some(winit_id);
        self.active_window = Some(session_wid);
        Ok(())
    }

    /// Spawn font discovery on a background thread.
    ///
    /// Returns `(FontCollection, FontSet, FontByteCache, fallback_map, elapsed)`.
    /// The `FontSet` is an `Arc`-cloned copy preserved before `FontCollection`
    /// consumes the original — zero additional disk reads. The `FontByteCache`
    /// is returned so the caller can reuse it for UI font loading.
    #[expect(
        clippy::type_complexity,
        reason = "thread join handle with font discovery result — not worth a type alias"
    )]
    fn spawn_font_discovery(
        &self,
    ) -> Result<
        std::thread::JoinHandle<
            Result<
                (
                    FontCollection,
                    FontSet,
                    FontByteCache,
                    Vec<usize>,
                    std::time::Duration,
                ),
                crate::font::FontError,
            >,
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
                let mut cache = FontByteCache::new();
                let mut font_set =
                    FontSet::load_cached(font_config.family.as_deref(), font_weight, &mut cache)?;

                // Prepend user-configured fallback fonts.
                let user_fb_families: Vec<&str> = font_config
                    .fallback
                    .iter()
                    .map(|f| f.family.as_str())
                    .collect();
                let fallback_map = font_set.prepend_user_fallbacks(&user_fb_families, &mut cache);

                // Clone before FontCollection consumes the FontSet (Arc clone, no disk I/O).
                let cached_set = font_set.clone();

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
                Ok((fc, cached_set, cache, fallback_map, t0.elapsed()))
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
        let metrics = metrics_from_style(self.config.window.tab_bar_style);

        // When the tab bar is hidden, chrome should report zero caption height
        // so macOS traffic lights and Windows Aero Snap use the correct geometry
        // from the start, not just after the first relayout.
        let hidden = self.config.window.tab_bar_position == crate::config::TabBarPosition::Hidden;
        let chrome_h = if hidden { 0.0 } else { metrics.height };

        // Publish the active tab bar height so macOS fullscreen notification
        // callbacks can center traffic lights at the correct height.
        #[cfg(target_os = "macos")]
        crate::window_manager::platform::macos::set_tab_bar_height(chrome_h);

        // Install platform chrome (Aero Snap subclass on Windows, no-op elsewhere).
        // Empty rects — the tab bar widget is created next.
        super::chrome::install_chrome(
            window.window(),
            crate::window_manager::platform::ChromeMode::Main,
            &[],
            chrome_h,
            scale,
        );
        let mut tab_bar_widget = oriterm_ui::widgets::tab_bar::TabBarWidget::with_theme_and_metrics(
            logical_w,
            &self.ui_theme,
            metrics,
        );

        // Reserve space for macOS traffic light buttons on the left.
        #[cfg(target_os = "macos")]
        tab_bar_widget
            .set_left_inset(oriterm_ui::widgets::tab_bar::constants::MACOS_TRAFFIC_LIGHT_WIDTH);

        tab_bar_widget.set_tabs(vec![oriterm_ui::widgets::tab_bar::TabEntry::new("")]);

        // Set initial platform hit test rects from the tab bar.
        super::chrome::refresh_chrome(
            window.window(),
            &tab_bar_widget.interactive_rects(),
            chrome_h,
            scale,
            true,
        );

        tab_bar_widget
    }
}

/// Convert a [`Decorations`](crate::config::Decorations) config value into
/// [`DecorationMode`](oriterm_ui::window::DecorationMode).
pub(super) fn decoration_to_mode(
    decorations: crate::config::Decorations,
) -> oriterm_ui::window::DecorationMode {
    match decorations {
        crate::config::Decorations::None => oriterm_ui::window::DecorationMode::Frameless,
        crate::config::Decorations::Full => oriterm_ui::window::DecorationMode::Native,
        crate::config::Decorations::Transparent => {
            oriterm_ui::window::DecorationMode::TransparentTitlebar
        }
        crate::config::Decorations::Buttonless => oriterm_ui::window::DecorationMode::Buttonless,
    }
}

/// Convert a [`TabBarStyle`](crate::config::TabBarStyle) config value into
/// [`TabBarMetrics`](oriterm_ui::widgets::tab_bar::constants::TabBarMetrics).
pub(super) fn metrics_from_style(
    style: crate::config::TabBarStyle,
) -> oriterm_ui::widgets::tab_bar::constants::TabBarMetrics {
    match style {
        crate::config::TabBarStyle::Default => {
            oriterm_ui::widgets::tab_bar::constants::TabBarMetrics::DEFAULT
        }
        crate::config::TabBarStyle::Compact => {
            oriterm_ui::widgets::tab_bar::constants::TabBarMetrics::COMPACT
        }
    }
}

impl App {
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
            shell: self.config.terminal.shell.clone(),
            ..SpawnConfig::default()
        };

        let palette = config_reload::build_palette_from_config(&self.config.colors, theme);

        let mux = self.mux.as_mut().ok_or("mux backend missing")?;
        let pane_id = mux.spawn_pane(&config, theme)?;

        // Apply color scheme + user overrides to the pane's terminal palette.
        mux.set_pane_theme(pane_id, theme, palette);

        // Apply image protocol config.
        mux.set_image_config(pane_id, self.config.terminal.image_config());

        // Apply bold-is-bright config.
        mux.set_bold_is_bright(pane_id, self.config.behavior.bold_is_bright);

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
