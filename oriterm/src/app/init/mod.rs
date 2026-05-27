//! One-shot application startup: window → GPU → mux → fonts → renderer → tab.

mod boot;
mod tab_creation;

pub(in crate::app) use boot::{decoration_to_mode, metrics_from_style};

use winit::event_loop::ActiveEventLoop;

use base64::{Engine as _, engine::general_purpose};

use oriterm_ui::window::WindowConfig;

use super::window_context::WindowContext;
use super::{App, DEFAULT_DPI};
use crate::app::config_reload;
use crate::font::{FontSet, GlyphFormat, HintingMode};
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
            position: self
                .initial_position
                .map(|(x, y)| oriterm_ui::geometry::Point::new(x as f32, y as f32)),
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

        // Register the device-lost callback so wgpu signals back into the
        // event loop when the underlying device dies. The callback fires
        // exactly once per device — `App::recover_gpu()` (5.16.4) will
        // re-register on the new device after every recreate. This is the
        // SSOT for the registration call site for the *initial* device.
        //
        // The closure also bumps `device_lost_signal` so the post-present
        // check in `finish_render` can detect a loss that occurred between
        // submit and present, before the queued `TermEvent::GpuDeviceLost`
        // has been dispatched by the event loop.
        let proxy = self.event_proxy.clone();
        let signal = std::sync::Arc::clone(&self.device_lost_signal);
        gpu.register_device_lost_callback(move |reason, message| {
            signal.fetch_add(1, std::sync::atomic::Ordering::Release);
            proxy.send(crate::event::TermEvent::GpuDeviceLost {
                reason: crate::gpu::recovery::GpuLossReason::from_wgpu(reason),
                message,
            });
        });

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
        // In daemon mode, the window may already be claimed via `--window`.
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
            crate::font::FontRasterConfig {
                format: GlyphFormat::Alpha,
                weight: 400,
                bold_weight: 600,
                hinting: HintingMode::None,
            },
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
        let wl = super::chrome::compute_window_layout(
            w,
            h,
            &cell,
            scale,
            super::chrome::ChromeLayout {
                tab_bar_hidden: hidden,
                tab_bar_height: tb_h,
                status_bar_height: sb_h,
                border_inset: 0.0,
            },
        );

        // 10. Create grid widget with cell metrics and layout-computed size.
        let grid_widget = TerminalGridWidget::new(cell.width, cell.height, wl.cols, wl.rows);
        grid_widget.set_bounds(wl.grid_rect);

        // 11. Create initial tab + pane (skip if daemon mode with a claimed window or tabs).
        let t_mux_start = std::time::Instant::now();
        let is_daemon = self.mux.as_ref().is_some_and(|m| m.is_daemon_mode());
        let has_claimed_tabs = self.claimed_tabs.is_some();
        let is_claimed = is_daemon && (self.active_window.is_some() || has_claimed_tabs);

        // Section 03.9 Phase 4 — Windows handoff path: take any pending
        // HandoffData from `App::new_handoff` and adopt the pre-existing
        // PTY handles instead of spawning a fresh shell.
        // Capture cell metrics before create_*_tab so we can seed the
        // IO-thread Term with real values instead of its `8x16` default.
        let cell_w_u16 = cell.width.round().max(1.0) as u16;
        let cell_h_u16 = cell.height.round().max(1.0) as u16;

        #[cfg(target_os = "windows")]
        let used_handoff = if let Some(handoff) = self.handoff_pending.take() {
            self.create_handoff_tab(session_wid, handoff, cell_w_u16, cell_h_u16)?;
            true
        } else {
            false
        };
        #[cfg(not(target_os = "windows"))]
        let used_handoff = false;

        // If we have claimed tabs, decode and adopt them.
        if let Some(json) = self.claimed_tabs.take() {
            if let Ok(tab_json) = general_purpose::STANDARD.decode(json) {
                if let Ok(tab) = serde_json::from_slice::<crate::session::Tab>(&tab_json) {
                    let pane_ids = tab.all_panes();
                    let mux = self.mux.as_mut().ok_or("mux backend missing")?;
                    for pid in pane_ids {
                        let _ = mux.subscribe(pid);
                    }
                    self.session.add_tab(tab.clone());
                    if let Some(win) = self.session.get_window_mut(session_wid) {
                        win.add_tab(tab.id());
                    }
                    log::info!(
                        "claimed tab {} with {} panes",
                        tab.id(),
                        tab.all_panes().len()
                    );
                } else {
                    log::error!("failed to deserialize claimed tab JSON");
                }
            } else {
                log::error!("failed to decode claimed tabs base64");
            }
        } else if !is_claimed && !used_handoff {
            self.create_initial_tab(
                session_wid,
                tab_creation::InitialTabGeometry {
                    rows: wl.rows as u16,
                    cols: wl.cols as u16,
                    cell_w: cell_w_u16,
                    cell_h: cell_h_u16,
                },
            )?;
        } else {
            // Daemon mode with a claimed window or handoff already populated
            // the session — nothing to do here.
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
}
