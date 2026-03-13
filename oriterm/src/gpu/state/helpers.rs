//! Free-standing GPU helpers: adapter selection, format/alpha/present mode
//! negotiation, surface config building, and GPU validation.

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
/// Requests `PIPELINE_CACHE` feature if the adapter supports it.
pub(super) fn request_device(adapter: &wgpu::Adapter) -> Option<(wgpu::Device, wgpu::Queue)> {
    let mut features = wgpu::Features::empty();
    if adapter.features().contains(wgpu::Features::PIPELINE_CACHE) {
        features |= wgpu::Features::PIPELINE_CACHE;
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

/// Build a [`wgpu::SurfaceConfiguration`] from the resolved GPU parameters.
///
/// Single source of truth for surface config — called from both `try_init()`
/// (initial probe) and `create_surface()` (per-window).
#[expect(
    clippy::too_many_arguments,
    reason = "wgpu SurfaceConfiguration: format, alpha mode, present mode, viewport dimensions"
)]
pub(super) fn build_surface_config(
    surface_format: wgpu::TextureFormat,
    render_format: wgpu::TextureFormat,
    alpha_mode: wgpu::CompositeAlphaMode,
    supports_view_formats: bool,
    present_mode: wgpu::PresentMode,
    width: u32,
    height: u32,
) -> wgpu::SurfaceConfiguration {
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
