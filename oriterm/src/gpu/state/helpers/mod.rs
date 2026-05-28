//! Free-standing GPU helpers: adapter selection, format/alpha/present mode
//! negotiation, surface config building, and GPU validation.

/// Adapter selection preference for headless GPU initialization.
///
/// Controls how `pick_adapter_with_preference()` selects the wgpu adapter.
/// `DiscreteOrFallback` preserves existing behavior (discrete GPU preferred).
/// `SoftwareRasterizer` pins the software fallback for deterministic golden
/// image comparison across machines.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[allow(
    dead_code,
    reason = "SoftwareRasterizer used only from headless test paths"
)]
pub(crate) enum AdapterPreference {
    /// Current default — discrete GPU preferred, any fallback.
    DiscreteOrFallback,
    /// Software rasterizer (llvmpipe / WARP / swiftshader).
    ///
    /// PRIMARY: `force_fallback_adapter: true` via `instance.request_adapter()`.
    /// SECONDARY: `enumerate_adapters()` + `DeviceType::Cpu` filter.
    /// Returns `None` if neither mechanism finds a software adapter.
    SoftwareRasterizer,
}

/// Select an adapter according to the given preference.
///
/// For `DiscreteOrFallback`, delegates to [`pick_adapter`]. For
/// `SoftwareRasterizer`, uses `force_fallback_adapter: true` as the
/// primary mechanism (wgpu-level contract, reliable across drivers),
/// falling back to `enumerate_adapters()` + `DeviceType::Cpu` filter
/// (unreliable on some drivers — WARP on Windows may report as
/// `DiscreteGpu`/`Other`).
#[allow(dead_code, reason = "used from headless test paths")]
pub(crate) fn pick_adapter_with_preference(
    instance: &wgpu::Instance,
    backends: wgpu::Backends,
    preference: AdapterPreference,
) -> Option<wgpu::Adapter> {
    match preference {
        AdapterPreference::DiscreteOrFallback => pick_adapter(instance, None, backends),
        AdapterPreference::SoftwareRasterizer => {
            // PRIMARY: force_fallback_adapter is a wgpu-level contract.
            let primary =
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::LowPower,
                    force_fallback_adapter: true,
                    compatible_surface: None,
                }));
            if let Ok(adapter) = primary {
                return Some(adapter);
            }
            // SECONDARY: enumerate + DeviceType::Cpu filter (unreliable
            // on some drivers, but catches edge cases).
            for a in pollster::block_on(instance.enumerate_adapters(backends)) {
                if a.get_info().device_type == wgpu::DeviceType::Cpu {
                    return Some(a);
                }
            }
            None
        }
    }
}

/// Enumerate adapters and pick the best one.
///
/// When `surface` is `Some`, only considers surface-compatible adapters.
/// Prefers discrete GPUs over integrated, falling back to any adapter.
pub(super) fn pick_adapter(
    instance: &wgpu::Instance,
    surface: Option<&wgpu::Surface<'_>>,
    backends: wgpu::Backends,
) -> Option<wgpu::Adapter> {
    let mut discrete: Option<wgpu::Adapter> = None;
    let mut fallback: Option<wgpu::Adapter> = None;

    for a in pollster::block_on(instance.enumerate_adapters(backends)) {
        if let Some(s) = surface {
            if !a.is_surface_supported(s) {
                continue;
            }
        }
        if a.get_info().device_type == wgpu::DeviceType::DiscreteGpu {
            discrete = Some(a);
            break;
        }
        if fallback.is_none() {
            fallback = Some(a);
        }
    }

    discrete.or(fallback)
}

/// Request a device and queue from the adapter.
///
/// Requests optional features if the adapter supports them:
/// - `PIPELINE_CACHE` — shader compilation caching across sessions.
/// - `DUAL_SOURCE_BLENDING` — per-channel LCD subpixel compositing
///   without requiring CPU-side background color knowledge.
pub(super) fn request_device(adapter: &wgpu::Adapter) -> Option<(wgpu::Device, wgpu::Queue)> {
    let mut features = wgpu::Features::empty();
    if adapter.features().contains(wgpu::Features::PIPELINE_CACHE) {
        features |= wgpu::Features::PIPELINE_CACHE;
    }
    if adapter
        .features()
        .contains(wgpu::Features::DUAL_SOURCE_BLENDING)
    {
        features |= wgpu::Features::DUAL_SOURCE_BLENDING;
    }

    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oriterm"),
        required_features: features,
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .map_err(|e| log::error!("GPU device request failed: {e}"))
    .ok()
}

/// Select surface format and derive sRGB render format.
///
/// Returns `None` if `caps.formats` is empty (incompatible surface).
pub(super) fn select_formats(
    caps: &wgpu::SurfaceCapabilities,
) -> Option<(wgpu::TextureFormat, wgpu::TextureFormat)> {
    let surface_format = *caps.formats.first()?;
    let render_format = surface_format.add_srgb_suffix();
    Some((surface_format, render_format))
}

/// Select the best composite alpha mode.
///
/// When `transparent` is true, prefers non-opaque modes so the compositor
/// can see transparent pixels and show blur/acrylic through them.
/// When `transparent` is false, prefers `Opaque` to avoid click-through
/// issues on compositors (e.g. Wayland/WSLg) that treat non-opaque surfaces
/// as having a live alpha channel.
pub(super) fn select_alpha_mode(
    caps: &wgpu::SurfaceCapabilities,
    transparent: bool,
) -> wgpu::CompositeAlphaMode {
    if transparent {
        // Transparency requested: prefer composited alpha.
        if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PostMultiplied)
        {
            wgpu::CompositeAlphaMode::PostMultiplied
        } else {
            caps.alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Opaque)
        }
    } else {
        // Opaque window: prefer Opaque to prevent click-through.
        if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            wgpu::CompositeAlphaMode::Opaque
        } else {
            caps.alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Opaque)
        }
    }
}

/// Select the best non-blocking present mode from surface capabilities.
///
/// Prefers `Mailbox` (non-blocking, no tearing, latest frame always shown)
/// over `Fifo` (vsync-blocking, freezes event loop for up to one refresh
/// interval). Falls back to `Fifo` which is universally supported.
pub(super) fn select_present_mode(caps: &wgpu::SurfaceCapabilities) -> wgpu::PresentMode {
    let modes = &caps.present_modes;

    // Mailbox: non-blocking, replaces queued frame with latest.
    // Keeps the event loop free to process input events immediately.
    if modes.contains(&wgpu::PresentMode::Mailbox) {
        return wgpu::PresentMode::Mailbox;
    }

    // Immediate: non-blocking, may tear. Acceptable fallback.
    if modes.contains(&wgpu::PresentMode::Immediate) {
        return wgpu::PresentMode::Immediate;
    }

    // Fifo is always supported per the spec.
    wgpu::PresentMode::Fifo
}

/// Resolved surface format parameters for [`build_surface_config`]: surface
/// and render formats, alpha mode, view-format support, and present mode.
#[derive(Clone, Copy)]
pub(super) struct SurfaceFormatParams {
    pub surface_format: wgpu::TextureFormat,
    pub render_format: wgpu::TextureFormat,
    pub alpha_mode: wgpu::CompositeAlphaMode,
    pub supports_view_formats: bool,
    pub present_mode: wgpu::PresentMode,
}

/// Build a [`wgpu::SurfaceConfiguration`] from the resolved GPU parameters.
///
/// Single source of truth for surface config — called from both `try_init()`
/// (initial probe) and `create_surface()` (per-window).
pub(super) fn build_surface_config(
    params: SurfaceFormatParams,
    width: u32,
    height: u32,
) -> wgpu::SurfaceConfiguration {
    let SurfaceFormatParams {
        surface_format,
        render_format,
        alpha_mode,
        supports_view_formats,
        present_mode,
    } = params;
    let needs_view_format = render_format != surface_format;
    let view_formats = if needs_view_format && supports_view_formats {
        vec![render_format]
    } else {
        vec![]
    };

    wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
        format: surface_format,
        width: width.max(1),
        height: height.max(1),
        present_mode,
        alpha_mode,
        view_formats,
        desired_maximum_frame_latency: 2,
    }
}

/// Validate GPU availability by creating an instance and enumerating adapters.
///
/// Logs adapter info for each compatible GPU found. Returns the number of
/// adapters discovered. This is a lightweight check that does not require a
/// window or surface.
#[allow(dead_code, reason = "GPU validation diagnostics")]
pub fn validate_gpu() -> usize {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let adapters: Vec<_> = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY))
        .into_iter()
        .collect();

    for a in &adapters {
        let info = a.get_info();
        log::info!(
            "GPU adapter: {} ({:?}, {:?})",
            info.name,
            info.backend,
            info.device_type,
        );
    }

    if adapters.is_empty() {
        log::warn!("no GPU adapters found");
    }

    adapters.len()
}

/// Marker returned when `wgpu::Surface::configure` panicked and was caught.
///
/// `wgpu::Surface::configure` returns `()` and panics on validation errors.
/// Callers wrap the call in [`try_configure_surface`] so the panic becomes
/// a `Result` the fallback chain in `GpuState::new` can route around
/// instead of aborting the process.
///
/// See: bug-tracker/plans/completed/BUG-06-108/
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigurePanicked;

/// Run `f` under `std::panic::catch_unwind`, returning [`ConfigurePanicked`]
/// on caught panic.
///
/// Factored out of [`try_configure_surface`] so the panic-catch contract can
/// be unit-tested without needing a real GPU surface.
pub(crate) fn catch_panic<F: FnOnce() + std::panic::UnwindSafe>(
    f: F,
) -> Result<(), ConfigurePanicked> {
    std::panic::catch_unwind(f).map_err(|_payload| ConfigurePanicked)
}

/// Panic-safe wrapper around `wgpu::Surface::configure`.
///
/// `wgpu::Surface::configure` is one of the few wgpu APIs that panics on
/// validation errors instead of returning `Result`. This wrapper catches
/// the panic and returns `Err(ConfigurePanicked)` so the GPU init fallback
/// chain advances to the next backend instead of aborting the process.
///
/// Does NOT touch the global panic hook — earlier iterations of this
/// helper swapped to a no-op hook for the duration of the `catch_unwind`
/// block to suppress the scary `[ERROR] PANIC: ...` line emitted by the
/// project's panic hook during normal fallback. That approach was
/// abandoned because the font-discovery thread runs concurrently with
/// GPU init, and a global hook swap would silently swallow any unrelated
/// panic that fired in that window. The panic-hook noise is a UX concern
/// tracked separately in the bug-tracker.
///
/// See: bug-tracker/plans/completed/BUG-06-108/
pub(crate) fn try_configure_surface(
    surface: &wgpu::Surface<'_>,
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> Result<(), ConfigurePanicked> {
    use std::panic::AssertUnwindSafe;
    let result = catch_panic(AssertUnwindSafe(|| surface.configure(device, config)));
    if result.is_err() {
        log::warn!("wgpu::Surface::configure panicked — caught for fallback handling");
    }
    result
}

#[cfg(test)]
mod tests;
