//! GPU device, adapter, queue, and surface management.
//!
//! [`GpuState`] is shared across all windows and owns the wgpu device lifetime.
//! It handles backend selection (Vulkan preferred, DX12+`DirectComposition` for
//! Windows transparency), adapter enumeration (discrete GPU preferred), sRGB
//! surface format derivation, and Vulkan pipeline cache persistence.

mod helpers;
mod pipeline_cache;

use helpers::{
    build_surface_config, pick_adapter, request_device, select_alpha_mode, select_formats,
    select_present_mode,
};

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use winit::window::Window;

/// Error returned when GPU initialization fails on all backends.
#[derive(Debug)]
pub struct GpuInitError;

impl fmt::Display for GpuInitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("failed to initialize GPU with any backend")
    }
}

impl std::error::Error for GpuInitError {}

/// GPU state shared across all windows.
pub struct GpuState {
    instance: wgpu::Instance,
    /// Shared device for all render commands.
    pub(super) device: wgpu::Device,
    /// Shared queue for all render submissions.
    pub(super) queue: wgpu::Queue,
    /// Native surface format (used for surface configuration).
    surface_format: wgpu::TextureFormat,
    /// sRGB format for render passes and pipelines. May differ from
    /// `surface_format` when the surface doesn't natively support sRGB
    /// (e.g. DX12 `DirectComposition`).
    render_format: wgpu::TextureFormat,
    /// Composite alpha mode negotiated with the compositor.
    surface_alpha_mode: wgpu::CompositeAlphaMode,
    /// Whether the backend supports `view_formats` for sRGB reinterpretation.
    supports_view_formats: bool,
    /// Presentation mode negotiated with the surface. Prefers non-blocking
    /// modes (`Mailbox`) over vsync-blocking (`Fifo`) to keep the event
    /// loop responsive during rendering.
    present_mode: wgpu::PresentMode,
    /// Vulkan pipeline cache (compiled shaders cached to disk across sessions).
    pub(super) pipeline_cache: Option<wgpu::PipelineCache>,
    pipeline_cache_path: Option<PathBuf>,
}

impl GpuState {
    /// Initialize GPU: create instance, surface, adapter, device, and queue.
    ///
    /// When `transparent` is true on Windows, uses DX12 with `DirectComposition`
    /// (the only path that gives `PreMultiplied` alpha on Windows HWND
    /// swapchains). Otherwise prefers Vulkan (supports pipeline caching for
    /// faster subsequent launches).
    pub fn new(window: &Arc<Window>, transparent: bool) -> Result<Self, GpuInitError> {
        // On Windows with transparency, DX12+DComp is the only path for
        // PreMultiplied alpha.
        #[cfg(target_os = "windows")]
        if transparent {
            if let Some(state) = Self::try_init(window, wgpu::Backends::DX12, true, transparent) {
                return Ok(state);
            }
            log::warn!("DX12 DirectComposition init failed, falling back to Vulkan");
        }

        // Prefer Vulkan — it supports pipeline caching (compiled shaders
        // persisted to disk).
        if let Some(state) = Self::try_init(window, wgpu::Backends::VULKAN, false, transparent) {
            return Ok(state);
        }

        // Fall back to other primary backends (DX12, Metal).
        if let Some(state) = Self::try_init(window, wgpu::Backends::PRIMARY, false, transparent) {
            return Ok(state);
        }

        // Last resort: secondary backends (GL, etc.).
        Self::try_init(window, wgpu::Backends::SECONDARY, false, transparent).ok_or(GpuInitError)
    }

    /// Initialize GPU in headless mode (no window or surface required).
    ///
    /// Used for testing and offscreen rendering. Picks any available adapter
    /// (including software rasterizers) and uses `Rgba8UnormSrgb` as the
    /// default format for render target compatibility.
    #[allow(dead_code, reason = "headless GPU for testing")]
    pub fn new_headless() -> Result<Self, GpuInitError> {
        Self::try_init_headless(wgpu::Backends::PRIMARY)
            .or_else(|| Self::try_init_headless(wgpu::Backends::SECONDARY))
            .ok_or(GpuInitError)
    }

    /// Returns the native surface format used for surface configuration.
    #[allow(dead_code, reason = "surface format query for later sections")]
    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_format
    }

    /// Returns the sRGB render format used for pipelines.
    pub fn render_format(&self) -> wgpu::TextureFormat {
        self.render_format
    }

    /// Returns true if the surface alpha mode supports transparency.
    #[allow(dead_code, reason = "transparency query for later sections")]
    pub fn supports_transparency(&self) -> bool {
        !matches!(self.surface_alpha_mode, wgpu::CompositeAlphaMode::Opaque)
    }

    /// Whether the surface uses a present mode that requires client-side
    /// frame budget gating (e.g. `Immediate`).
    ///
    /// `Mailbox` and `Fifo` provide hardware-level pacing.
    /// `Immediate` has no pacing — without a client budget gate, the
    /// renderer would run uncapped during sustained PTY output.
    pub fn needs_frame_budget(&self) -> bool {
        matches!(self.present_mode, wgpu::PresentMode::Immediate)
    }

    /// Create and configure a new surface for a window.
    pub fn create_surface(
        &self,
        window: &Arc<Window>,
    ) -> Result<(wgpu::Surface<'static>, wgpu::SurfaceConfiguration), wgpu::CreateSurfaceError>
    {
        let surface = self.instance.create_surface(window.clone())?;
        let size = window.inner_size();
        let config = build_surface_config(
            self.surface_format,
            self.render_format,
            self.surface_alpha_mode,
            self.supports_view_formats,
            self.present_mode,
            size.width,
            size.height,
        );
        surface.configure(&self.device, &config);
        Ok((surface, config))
    }

    /// Reconfigure an existing surface (e.g. after a window resize).
    ///
    /// Encapsulates device access so callers don't need the raw wgpu `Device`.
    pub fn configure_surface(
        &self,
        surface: &wgpu::Surface<'_>,
        config: &wgpu::SurfaceConfiguration,
    ) {
        surface.configure(&self.device, config);
    }

    /// Render a single clear frame to a surface.
    ///
    /// Fills the framebuffer with `bg` (sRGB bytes) premultiplied by `opacity`,
    /// then presents. Used before `window.set_visible(true)` to prevent a
    /// white/gray flash on startup.
    pub fn clear_surface(&self, surface: &wgpu::Surface<'_>, bg: oriterm_core::Rgb, opacity: f32) {
        let frame = match surface.get_current_texture() {
            Ok(f) => f,
            Err(e) => {
                log::warn!("clear_surface: failed to acquire texture: {e}");
                return;
            }
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor {
            format: Some(self.render_format),
            ..Default::default()
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("clear_surface"),
            });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: f64::from(super::srgb_to_linear(bg.r)) * f64::from(opacity),
                            g: f64::from(super::srgb_to_linear(bg.g)) * f64::from(opacity),
                            b: f64::from(super::srgb_to_linear(bg.b)) * f64::from(opacity),
                            a: f64::from(opacity),
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                ..Default::default()
            });
        }
        self.queue.submit(std::iter::once(encoder.finish()));
        frame.present();

        // Flush the GPU so the clear frame is visible before the window shows.
        if let Err(e) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
            log::warn!("clear_surface: GPU poll failed: {e}");
        }
    }

    /// Save the pipeline cache to disk on a background thread.
    ///
    /// Extracts the cache data synchronously (fast), then spawns a
    /// detached thread for the disk write so the caller doesn't block.
    /// Flush all pending GPU work synchronously.
    ///
    /// Blocks until the device has completed all submitted command buffers.
    /// Used after rendering a first frame to ensure it is visible before
    /// showing a window.
    pub fn poll_device(&self) {
        if let Err(e) = self.device.poll(wgpu::PollType::wait_indefinitely()) {
            log::warn!("poll_device: GPU poll failed: {e}");
        }
    }

    pub fn save_pipeline_cache_async(&self) {
        let (Some(cache), Some(path)) = (&self.pipeline_cache, &self.pipeline_cache_path) else {
            return;
        };
        let Some(data) = cache.get_data() else {
            return;
        };
        let path = path.clone();
        std::thread::Builder::new()
            .name("pipeline-cache-save".into())
            .spawn(move || {
                pipeline_cache::write_pipeline_cache(&path, &data);
            })
            .ok();
    }

    /// Try to initialize GPU with the given backend set and a window surface.
    ///
    /// Returns `None` if no compatible adapter is found or device creation
    /// fails, allowing the caller to fall back to the next backend.
    fn try_init(
        window: &Arc<Window>,
        backends: wgpu::Backends,
        dcomp: bool,
        transparent: bool,
    ) -> Option<Self> {
        let t0 = std::time::Instant::now();
        let instance = Self::create_instance(backends, dcomp);
        let t_instance = t0.elapsed();

        let surface = instance.create_surface(window.clone()).ok()?;
        let t_surface = t0.elapsed();

        let adapter = pick_adapter(&instance, Some(&surface), backends)?;
        let t_adapter = t0.elapsed();

        let (device, queue) = request_device(&adapter)?;
        let t_device = t0.elapsed();

        let caps = surface.get_capabilities(&adapter);
        let downlevel = adapter.get_downlevel_capabilities();
        let (surface_format, render_format) = select_formats(&caps)?;
        let surface_alpha_mode = select_alpha_mode(&caps, transparent);
        let present_mode = select_present_mode(&caps);
        let supports_view_formats = downlevel
            .flags
            .contains(wgpu::DownlevelFlags::SURFACE_VIEW_FORMATS);

        // Configure the initial surface.
        let size = window.inner_size();
        let config = build_surface_config(
            surface_format,
            render_format,
            surface_alpha_mode,
            supports_view_formats,
            present_mode,
            size.width,
            size.height,
        );
        surface.configure(&device, &config);
        drop(config);
        let t_configure = t0.elapsed();

        let info = adapter.get_info();
        let transparency_supported =
            !matches!(surface_alpha_mode, wgpu::CompositeAlphaMode::Opaque);
        log::info!(
            "GPU: adapter={}, backend={:?}, surface={surface_format:?}, \
             render={render_format:?}, alpha={surface_alpha_mode:?} (available: {:?}), \
             present={present_mode:?}, transparency={}, view_formats={}",
            info.name,
            info.backend,
            caps.alpha_modes,
            if transparency_supported { "yes" } else { "no" },
            if supports_view_formats {
                "supported"
            } else {
                "not supported"
            },
        );

        let (pipeline_cache, pipeline_cache_path) =
            pipeline_cache::load_pipeline_cache(&device, &info);
        let t_cache = t0.elapsed();
        drop(adapter);

        log::info!(
            "GPU init breakdown: instance={t_instance:?} surface={t_surface:?} \
             adapter={t_adapter:?} device={t_device:?} configure={t_configure:?} \
             cache={t_cache:?}",
        );

        Some(Self {
            instance,
            device,
            queue,
            surface_format,
            render_format,
            surface_alpha_mode,
            supports_view_formats,
            present_mode,
            pipeline_cache,
            pipeline_cache_path,
        })
    }

    /// Try to initialize GPU in headless mode with the given backend set.
    ///
    /// No surface is created — uses `Rgba8UnormSrgb` as default format.
    #[allow(dead_code, reason = "headless GPU for testing")]
    fn try_init_headless(backends: wgpu::Backends) -> Option<Self> {
        let instance = Self::create_instance(backends, false);
        let adapter = pick_adapter(&instance, None, backends)?;

        let (device, queue) = request_device(&adapter)?;

        // Without a surface, use Rgba8UnormSrgb as the default render format.
        // This is universally supported and matches offscreen render targets.
        let surface_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let render_format = surface_format;

        let info = adapter.get_info();
        log::info!(
            "GPU (headless): adapter={}, backend={:?}, format={surface_format:?}",
            info.name,
            info.backend,
        );

        let (pipeline_cache, pipeline_cache_path) =
            pipeline_cache::load_pipeline_cache(&device, &info);
        drop(adapter);

        Some(Self {
            instance,
            device,
            queue,
            surface_format,
            render_format,
            surface_alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            supports_view_formats: false,
            present_mode: wgpu::PresentMode::Fifo,
            pipeline_cache,
            pipeline_cache_path,
        })
    }

    /// Create a wgpu instance with the specified backends.
    fn create_instance(backends: wgpu::Backends, dcomp: bool) -> wgpu::Instance {
        wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends,
            backend_options: wgpu::BackendOptions {
                dx12: wgpu::Dx12BackendOptions {
                    presentation_system: if dcomp {
                        wgpu::Dx12SwapchainKind::DxgiFromVisual
                    } else {
                        wgpu::Dx12SwapchainKind::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests;
