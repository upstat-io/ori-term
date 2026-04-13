//! Headless GPU initialization (no window or surface required).
//!
//! Used for testing and offscreen rendering. Extracted from `state/mod.rs`
//! to keep the parent file under the 500-line limit (§05.0 BLOAT split).
//! Future headless construction methods (e.g. `new_headless_with_preference()`
//! from §05.1) are added here.

use super::helpers::{pick_adapter, request_device};
use super::pipeline_cache;
use super::{GpuInitError, GpuState};

impl GpuState {
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

        let dual_source = device
            .features()
            .contains(wgpu::Features::DUAL_SOURCE_BLENDING);

        Some(Self {
            instance,
            device,
            queue,
            surface_format,
            render_format,
            surface_alpha_mode: wgpu::CompositeAlphaMode::Opaque,
            supports_view_formats: false,
            present_mode: wgpu::PresentMode::Fifo,
            uses_dcomp: false,
            dual_source_blending: dual_source,
            pipeline_cache,
            pipeline_cache_path,
        })
    }
}
