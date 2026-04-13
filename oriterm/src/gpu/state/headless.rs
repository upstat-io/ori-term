//! Headless GPU initialization (no window or surface required).
//!
//! Used for testing and offscreen rendering. Extracted from `state/mod.rs`
//! to keep the parent file under the 500-line limit (§05.0 BLOAT split).

use super::helpers::{
    AdapterPreference, pick_adapter, pick_adapter_with_preference, request_device,
};
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
        Self::new_headless_with_preference(AdapterPreference::DiscreteOrFallback)
    }

    /// Initialize GPU in headless mode with an explicit adapter preference.
    ///
    /// `SoftwareRasterizer` selects a software fallback (llvmpipe / WARP /
    /// swiftshader) for deterministic golden image comparison. Returns
    /// `Err(GpuInitError)` if no matching adapter is found.
    #[allow(dead_code, reason = "headless GPU for testing")]
    pub fn new_headless_with_preference(
        preference: AdapterPreference,
    ) -> Result<Self, GpuInitError> {
        Self::try_init_headless(wgpu::Backends::PRIMARY, preference)
            .or_else(|| Self::try_init_headless(wgpu::Backends::SECONDARY, preference))
            .ok_or(GpuInitError)
    }

    /// Adapter metadata retained from initialization.
    ///
    /// Contains name, backend, and device type — used for diagnostics
    /// and for test assertions (e.g. verifying the deterministic lane
    /// selected a software rasterizer).
    #[allow(dead_code, reason = "headless GPU diagnostics")]
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// Try to initialize GPU in headless mode with the given backend set
    /// and adapter preference.
    ///
    /// No surface is created — uses `Rgba8UnormSrgb` as default format.
    #[allow(dead_code, reason = "headless GPU for testing")]
    fn try_init_headless(backends: wgpu::Backends, preference: AdapterPreference) -> Option<Self> {
        let instance = Self::create_instance(backends, false);
        let adapter = match preference {
            AdapterPreference::DiscreteOrFallback => pick_adapter(&instance, None, backends)?,
            AdapterPreference::SoftwareRasterizer => {
                pick_adapter_with_preference(&instance, backends, preference)?
            }
        };

        let (device, queue) = request_device(&adapter)?;

        // Without a surface, use Rgba8UnormSrgb as the default render format.
        // This is universally supported and matches offscreen render targets.
        let surface_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let render_format = surface_format;

        let adapter_info = adapter.get_info();
        log::info!(
            "GPU (headless): adapter={}, backend={:?}, device_type={:?}, format={surface_format:?}",
            adapter_info.name,
            adapter_info.backend,
            adapter_info.device_type,
        );

        let (pipeline_cache, pipeline_cache_path) =
            pipeline_cache::load_pipeline_cache(&device, &adapter_info);
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
            adapter_info,
        })
    }
}
