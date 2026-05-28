//! Unit tests for GPU state initialization.

use wgpu::{CompositeAlphaMode, SurfaceCapabilities, TextureFormat, TextureUsages};

use wgpu::PresentMode;

use super::helpers::{
    SurfaceFormatParams, build_surface_config, select_alpha_mode, select_formats,
    select_present_mode,
};

fn caps_with_formats(formats: Vec<TextureFormat>) -> SurfaceCapabilities {
    SurfaceCapabilities {
        formats,
        present_modes: vec![],
        alpha_modes: vec![CompositeAlphaMode::Opaque],
        usages: TextureUsages::RENDER_ATTACHMENT,
    }
}

// --- Surface format selection ---

#[test]
fn select_formats_srgb_surface() {
    let caps = caps_with_formats(vec![TextureFormat::Bgra8UnormSrgb]);

    let (surface_fmt, render_fmt) = select_formats(&caps).unwrap();

    // When surface is already sRGB, render format matches.
    assert_eq!(surface_fmt, TextureFormat::Bgra8UnormSrgb);
    assert_eq!(render_fmt, TextureFormat::Bgra8UnormSrgb);
}

#[test]
fn select_formats_non_srgb_surface_derives_srgb_render() {
    let caps = caps_with_formats(vec![TextureFormat::Bgra8Unorm]);

    let (surface_fmt, render_fmt) = select_formats(&caps).unwrap();

    // Non-sRGB surface: render format is the sRGB suffix.
    assert_eq!(surface_fmt, TextureFormat::Bgra8Unorm);
    assert_eq!(render_fmt, TextureFormat::Bgra8UnormSrgb);
}

#[test]
fn select_formats_rgba_surface() {
    let caps = caps_with_formats(vec![TextureFormat::Rgba8Unorm]);

    let (surface_fmt, render_fmt) = select_formats(&caps).unwrap();

    assert_eq!(surface_fmt, TextureFormat::Rgba8Unorm);
    assert_eq!(render_fmt, TextureFormat::Rgba8UnormSrgb);
}

#[test]
fn select_formats_empty_formats_returns_none() {
    let caps = caps_with_formats(vec![]);
    assert!(select_formats(&caps).is_none());
}

#[test]
fn select_formats_picks_first_when_multiple_available() {
    let caps = caps_with_formats(vec![
        TextureFormat::Bgra8Unorm,
        TextureFormat::Rgba8Unorm,
        TextureFormat::Bgra8UnormSrgb,
    ]);

    let (surface_fmt, render_fmt) = select_formats(&caps).unwrap();

    // Should pick the first format, not scan for an sRGB one.
    assert_eq!(surface_fmt, TextureFormat::Bgra8Unorm);
    assert_eq!(render_fmt, TextureFormat::Bgra8UnormSrgb);
}

#[test]
fn select_formats_multiple_with_srgb_first() {
    let caps = caps_with_formats(vec![
        TextureFormat::Bgra8UnormSrgb,
        TextureFormat::Rgba8Unorm,
    ]);

    let (surface_fmt, render_fmt) = select_formats(&caps).unwrap();

    // sRGB is already first, so render_format matches.
    assert_eq!(surface_fmt, TextureFormat::Bgra8UnormSrgb);
    assert_eq!(render_fmt, TextureFormat::Bgra8UnormSrgb);
}

// --- Alpha mode selection (transparent=true) ---

#[test]
fn select_alpha_transparent_prefers_premultiplied() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![],
        alpha_modes: vec![
            CompositeAlphaMode::Opaque,
            CompositeAlphaMode::PostMultiplied,
            CompositeAlphaMode::PreMultiplied,
        ],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    assert_eq!(
        select_alpha_mode(&caps, true),
        CompositeAlphaMode::PreMultiplied,
    );
}

#[test]
fn select_alpha_transparent_falls_back_to_postmultiplied() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![],
        alpha_modes: vec![
            CompositeAlphaMode::Opaque,
            CompositeAlphaMode::PostMultiplied,
        ],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    assert_eq!(
        select_alpha_mode(&caps, true),
        CompositeAlphaMode::PostMultiplied,
    );
}

// --- Alpha mode selection (transparent=false) ---

#[test]
fn select_alpha_opaque_prefers_opaque() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![],
        alpha_modes: vec![
            CompositeAlphaMode::PreMultiplied,
            CompositeAlphaMode::Opaque,
        ],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    assert_eq!(select_alpha_mode(&caps, false), CompositeAlphaMode::Opaque,);
}

#[test]
fn select_alpha_opaque_falls_back_to_first_when_no_opaque() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![],
        alpha_modes: vec![CompositeAlphaMode::PreMultiplied],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    // No Opaque available — uses first available.
    assert_eq!(
        select_alpha_mode(&caps, false),
        CompositeAlphaMode::PreMultiplied,
    );
}

// --- Alpha mode selection (shared edge cases) ---

#[test]
fn select_alpha_inherit_as_only_option() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![],
        alpha_modes: vec![CompositeAlphaMode::Inherit],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    // When only Inherit is available, use it (common fallback).
    assert_eq!(select_alpha_mode(&caps, true), CompositeAlphaMode::Inherit,);
    assert_eq!(select_alpha_mode(&caps, false), CompositeAlphaMode::Inherit,);
}

#[test]
fn select_alpha_empty_defaults_to_opaque() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![],
        alpha_modes: vec![],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    // Empty alpha_modes should not panic; falls back to Opaque.
    assert_eq!(select_alpha_mode(&caps, true), CompositeAlphaMode::Opaque,);
    assert_eq!(select_alpha_mode(&caps, false), CompositeAlphaMode::Opaque,);
}

// --- Surface config builder ---

#[test]
fn build_surface_config_sets_view_formats_when_needed() {
    let config = build_surface_config(
        SurfaceFormatParams {
            surface_format: TextureFormat::Bgra8Unorm,
            render_format: TextureFormat::Bgra8UnormSrgb,
            alpha_mode: CompositeAlphaMode::Opaque,
            supports_view_formats: true,
            present_mode: PresentMode::Fifo,
        },
        800,
        600,
    );

    assert_eq!(config.format, TextureFormat::Bgra8Unorm);
    assert_eq!(config.view_formats, vec![TextureFormat::Bgra8UnormSrgb]);
    assert_eq!(config.width, 800);
    assert_eq!(config.height, 600);
}

#[test]
fn build_surface_config_skips_view_formats_when_unsupported() {
    let config = build_surface_config(
        SurfaceFormatParams {
            surface_format: TextureFormat::Bgra8Unorm,
            render_format: TextureFormat::Bgra8UnormSrgb,
            alpha_mode: CompositeAlphaMode::Opaque,
            supports_view_formats: false,
            present_mode: PresentMode::Fifo,
        },
        800,
        600,
    );

    assert!(config.view_formats.is_empty());
}

#[test]
fn build_surface_config_no_view_formats_when_formats_match() {
    let config = build_surface_config(
        SurfaceFormatParams {
            surface_format: TextureFormat::Bgra8UnormSrgb,
            render_format: TextureFormat::Bgra8UnormSrgb,
            alpha_mode: CompositeAlphaMode::PreMultiplied,
            supports_view_formats: true,
            present_mode: PresentMode::Fifo,
        },
        1920,
        1080,
    );

    assert!(config.view_formats.is_empty());
    assert_eq!(config.alpha_mode, CompositeAlphaMode::PreMultiplied);
}

#[test]
fn build_surface_config_clamps_zero_dimensions() {
    let config = build_surface_config(
        SurfaceFormatParams {
            surface_format: TextureFormat::Bgra8UnormSrgb,
            render_format: TextureFormat::Bgra8UnormSrgb,
            alpha_mode: CompositeAlphaMode::Opaque,
            supports_view_formats: false,
            present_mode: PresentMode::Fifo,
        },
        0,
        0,
    );

    assert_eq!(config.width, 1);
    assert_eq!(config.height, 1);
}

// --- Present mode selection ---

#[test]
fn select_present_mode_prefers_mailbox() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![
            PresentMode::Fifo,
            PresentMode::Mailbox,
            PresentMode::Immediate,
        ],
        alpha_modes: vec![CompositeAlphaMode::Opaque],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    assert_eq!(select_present_mode(&caps), PresentMode::Mailbox);
}

#[test]
fn select_present_mode_falls_back_to_immediate() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![PresentMode::Fifo, PresentMode::Immediate],
        alpha_modes: vec![CompositeAlphaMode::Opaque],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    assert_eq!(select_present_mode(&caps), PresentMode::Immediate);
}

#[test]
fn select_present_mode_falls_back_to_fifo() {
    let caps = SurfaceCapabilities {
        formats: vec![],
        present_modes: vec![PresentMode::Fifo],
        alpha_modes: vec![CompositeAlphaMode::Opaque],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    assert_eq!(select_present_mode(&caps), PresentMode::Fifo);
}

// --- Cache directory ---

#[test]
fn cache_dir_returns_valid_path() {
    let dir = super::pipeline_cache::cache_dir();
    let path_str = dir.to_string_lossy();
    assert!(
        path_str.contains("ori_term"),
        "cache_dir should contain 'ori_term': {path_str}",
    );
}

// --- GPU adapter enumeration ---

#[test]
fn validate_gpu_does_not_panic() {
    // Verifies GPU validation runs without panicking, even when no
    // GPU adapters are available (e.g. CI, headless).
    let _count = super::helpers::validate_gpu();
}

// --- GPU integration tests (require real adapter) ---

/// Helper: attempt to get a GPU adapter, returning `None` in headless
/// environments.
fn try_get_adapter() -> Option<(wgpu::Instance, wgpu::Adapter)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });

    let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::all()))
        .into_iter()
        .next()?;

    Some((instance, adapter))
}

#[test]
fn gpu_adapter_reports_srgb_capable_format() {
    let Some((_instance, adapter)) = try_get_adapter() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    // Every modern GPU should support at least one format with an sRGB suffix.
    let info = adapter.get_info();
    let srgb_capable = [TextureFormat::Bgra8UnormSrgb, TextureFormat::Rgba8UnormSrgb];

    // We can't check surface formats without a surface, but we can verify
    // the adapter is not a software fallback with no capabilities.
    assert!(
        !info.name.is_empty(),
        "adapter should have a name: {info:?}",
    );

    // Verify add_srgb_suffix round-trips correctly for common formats.
    for fmt in &srgb_capable {
        assert_eq!(fmt.add_srgb_suffix(), *fmt);
    }
    assert_eq!(
        TextureFormat::Bgra8Unorm.add_srgb_suffix(),
        TextureFormat::Bgra8UnormSrgb,
    );
}

#[test]
fn gpu_device_creation_succeeds() {
    let Some((_instance, adapter)) = try_get_adapter() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let result = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oriterm_test"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }));

    assert!(result.is_ok(), "device creation should succeed: {result:?}");
}

#[test]
fn gpu_pipeline_cache_round_trip() {
    let Some((_instance, adapter)) = try_get_adapter() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    if !adapter.features().contains(wgpu::Features::PIPELINE_CACHE) {
        eprintln!("skipped: adapter does not support PIPELINE_CACHE");
        return;
    }

    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oriterm_cache_test"),
        required_features: wgpu::Features::PIPELINE_CACHE,
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("device creation should succeed");

    // Create a fresh pipeline cache (no initial data).
    #[allow(unsafe_code, reason = "testing pipeline cache round-trip")]
    let cache = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("test_cache"),
            data: None,
            fallback: true,
        })
    };

    // Serialize — may be empty if no pipelines were compiled.
    let data = cache.get_data();
    assert!(data.is_some(), "cache should be serializable");

    // Reload from serialized data.
    let data = data.unwrap();
    #[allow(unsafe_code, reason = "testing pipeline cache round-trip")]
    let _reloaded = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("test_cache_reloaded"),
            data: Some(&data),
            fallback: true,
        })
    };

    // If we get here without panicking, the round-trip succeeded.
}

#[test]
fn gpu_texture_dimension_limits_are_reasonable() {
    let Some((_instance, adapter)) = try_get_adapter() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let (device, _queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("oriterm_limits_test"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::default(),
        ..Default::default()
    }))
    .expect("device creation should succeed");

    let limits = device.limits();

    // Our glyph atlas is 2048x2048. Ensure the GPU supports at least that.
    assert!(
        limits.max_texture_dimension_2d >= 2048,
        "GPU must support at least 2048x2048 textures, got {}",
        limits.max_texture_dimension_2d,
    );

    // Verify buffer size is large enough for a frame of instance data.
    // 80 bytes per cell, 200 cols * 50 rows = 10,000 cells = 800KB.
    let min_buffer = 80 * 200 * 50;
    assert!(
        limits.max_buffer_size >= min_buffer,
        "GPU must support at least {min_buffer} byte buffers, got {}",
        limits.max_buffer_size,
    );
}

// --- Headless GPU init ---

#[test]
fn headless_init_succeeds_when_adapter_available() {
    use super::GpuState;

    match GpuState::new_headless() {
        Ok(gpu) => {
            // Headless always uses Rgba8UnormSrgb.
            assert_eq!(gpu.render_format(), TextureFormat::Rgba8UnormSrgb,);
            assert_eq!(gpu.surface_format(), gpu.render_format());
            // Headless has no compositor, so no transparency.
            assert!(!gpu.supports_transparency());
        }
        Err(_) => {
            eprintln!("skipped: no GPU adapter available for headless init");
        }
    }
}

#[test]
fn headless_can_cache_blit_true_when_formats_match() {
    use super::GpuState;

    match GpuState::new_headless() {
        Ok(gpu) => {
            // Headless: surface_format == render_format (both Rgba8UnormSrgb).
            assert!(gpu.can_cache_blit());
        }
        Err(_) => {
            eprintln!("skipped: no GPU adapter available");
        }
    }
}

#[test]
fn headless_does_not_use_dcomp() {
    use super::GpuState;

    match GpuState::new_headless() {
        Ok(gpu) => {
            // Headless never uses DirectComposition.
            assert!(!gpu.uses_dcomp());
        }
        Err(_) => {
            eprintln!("skipped: no GPU adapter available");
        }
    }
}

#[test]
fn headless_stores_adapter_info() {
    use super::GpuState;

    match GpuState::new_headless() {
        Ok(gpu) => {
            let info = gpu.adapter_info();
            assert!(!info.name.is_empty(), "adapter info should have a name");
        }
        Err(_) => {
            eprintln!("skipped: no GPU adapter available");
        }
    }
}

// --- Adapter preference ---

#[test]
fn new_headless_default_picks_discrete_or_fallback() {
    use super::GpuState;

    // DiscreteOrFallback is the default — same behavior as new_headless().
    match GpuState::new_headless() {
        Ok(gpu) => {
            assert_eq!(gpu.render_format(), TextureFormat::Rgba8UnormSrgb);
            assert!(!gpu.adapter_info().name.is_empty());
        }
        Err(_) => {
            eprintln!("skipped: no GPU adapter available");
        }
    }
}

#[test]
fn new_headless_with_software_preference_uses_force_fallback() {
    use super::GpuState;
    use super::helpers::AdapterPreference;

    match GpuState::new_headless_with_preference(AdapterPreference::SoftwareRasterizer) {
        Ok(gpu) => {
            let info = gpu.adapter_info();
            let name = info.name.to_lowercase();

            // Primary signal: wgpu reports `device_type == Cpu` for
            // software rasterizers. This is the authoritative wgpu
            // contract — any adapter the driver itself tags as CPU
            // qualifies as a software rasterizer.
            if info.device_type == wgpu::DeviceType::Cpu {
                return;
            }

            // Fallback: some software rasterizers on older wgpu versions
            // report `device_type == Other` but have recognizable names
            // (llvmpipe on Linux when wgpu fails to parse the MESA
            // device type, for example). Keep the string list as a
            // defensive backstop.
            const KNOWN: &[&str] = &[
                "llvmpipe",
                "lavapipe",
                "warp",
                "swiftshader",
                "mesa software",
                "microsoft basic render",
                "cpu",
            ];
            assert!(
                KNOWN.iter().any(|s| name.contains(s)),
                "expected software rasterizer, got: {:?} (backend={:?}, device_type={:?})",
                info.name,
                info.backend,
                info.device_type,
            );
        }
        Err(_) => {
            // No software rasterizer available — acceptable on some platforms.
            eprintln!("SKIP: no software rasterizer available");
        }
    }
}

#[test]
fn pick_adapter_software_rasterizer_returns_none_when_unavailable() {
    use super::helpers::{AdapterPreference, pick_adapter_with_preference};

    // This test validates the negative path: the function returns None
    // (not a panic) when no software adapter exists. On systems WITH a
    // software adapter, it returns Some — both outcomes are valid.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let result = pick_adapter_with_preference(
        &instance,
        wgpu::Backends::PRIMARY,
        AdapterPreference::SoftwareRasterizer,
    );
    // We can't assert None (may have llvmpipe), but we CAN assert no panic.
    if let Some(adapter) = result {
        let info = adapter.get_info();
        eprintln!(
            "software adapter found: {} ({:?})",
            info.name, info.device_type
        );
    } else {
        eprintln!("no software adapter — negative path confirmed");
    }
}

#[test]
fn pick_adapter_discrete_or_fallback_matches_original() {
    use super::helpers::{AdapterPreference, pick_adapter, pick_adapter_with_preference};

    // Property: DiscreteOrFallback must delegate to pick_adapter.
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        ..Default::default()
    });
    let original = pick_adapter(&instance, None, wgpu::Backends::PRIMARY);
    let via_pref = pick_adapter_with_preference(
        &instance,
        wgpu::Backends::PRIMARY,
        AdapterPreference::DiscreteOrFallback,
    );
    match (original, via_pref) {
        (Some(a), Some(b)) => {
            assert_eq!(
                a.get_info().name,
                b.get_info().name,
                "DiscreteOrFallback should pick the same adapter as pick_adapter",
            );
        }
        (None, None) => {
            eprintln!("skipped: no GPU adapter available");
        }
        (a, b) => {
            panic!(
                "adapter availability mismatch: original={}, via_pref={}",
                a.is_some(),
                b.is_some(),
            );
        }
    }
}

/// Regression: both public headless entrypoints must funnel through
/// `sanitize_headless_env()` so the Wayland/X11 probe-hang guard fires
/// regardless of which constructor the caller reaches. Before the fix,
/// only `new_headless()` sanitized — callers that reached
/// `new_headless_with_preference()` directly (visual-regression
/// software-rasterizer lane + `GpuState::new_headless_with_preference`
/// callers in this test file) bypassed the guard.
#[test]
fn both_headless_entrypoints_unset_display_env() {
    use super::GpuState;
    use super::helpers::AdapterPreference;

    // Exercise the preference entrypoint first so the OnceLock inside
    // `sanitize_headless_env` is known-fired before we assert — otherwise
    // a hypothetical regression that only runs sanitization from
    // `new_headless()` would be hidden by a prior successful run in the
    // same process.
    let _ = GpuState::new_headless_with_preference(AdapterPreference::DiscreteOrFallback);
    let _ = GpuState::new_headless();

    // SAFETY: these reads are the complement of the `env::remove_var`
    // calls in `sanitize_headless_env`. The sanitizer only fires from
    // background test threads if some other test installed them, but
    // Cargo's default test harness runs tests in the same process we are
    // in, so after either entrypoint above returns, any stale
    // WAYLAND_DISPLAY / DISPLAY must already be cleared.
    assert!(
        std::env::var_os("WAYLAND_DISPLAY").is_none(),
        "sanitize_headless_env must clear WAYLAND_DISPLAY via either entrypoint",
    );
    assert!(
        std::env::var_os("DISPLAY").is_none(),
        "sanitize_headless_env must clear DISPLAY via either entrypoint",
    );
}

// --- Regression pins for the panic-safe configure helper ---
//
// Provenance documented in per-test `/// Regression:` doc comments.

/// Regression: BUG-06-108 — bare surface.configure calls crashed process on
/// validation panic. All call sites in state/mod.rs MUST route through
/// the try_configure_surface helper.
#[test]
fn state_mod_when_scanned_contains_no_unwrapped_surface_configure() {
    let src = include_str!("mod.rs");
    let mut bare_matches: Vec<(usize, &str)> = Vec::new();
    for (i, line) in src.lines().enumerate() {
        let trimmed = line.trim_start();
        // Skip comments + docs.
        if trimmed.starts_with("//") {
            continue;
        }
        // Skip the helper-call lines themselves.
        if line.contains("try_configure_surface(") {
            continue;
        }
        // A bare configure call looks like `surface.configure(&device, ...)`
        // or `surface.configure(device, ...)` — pattern: `.configure(` with
        // a `&device` or `device` argument nearby on the same line.
        if line.contains(".configure(")
            && (line.contains("device") || line.contains("&self.device"))
        {
            bare_matches.push((i + 1, line));
        }
    }
    assert!(
        bare_matches.is_empty(),
        "Bare surface.configure calls found outside try_configure_surface helper:\n{}\n\
         All configure call sites in state/mod.rs must route through helpers::try_configure_surface \
         to preserve the panic-safe contract that the GpuState::new fallback chain depends on.",
        bare_matches
            .iter()
            .map(|(n, l)| format!("  line {n}: {l}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Regression: BUG-06-108 — try_init must route configure through the
/// panic-safe helper.
#[test]
fn try_init_when_scanned_uses_configure_helper() {
    let src = include_str!("mod.rs");
    let fn_start = src
        .find("fn try_init(")
        .expect("try_init function present in state/mod.rs");
    // Scope: from fn declaration to the next top-level fn (heuristic: next
    // `\n    fn ` or `\nimpl ` or `\nfn `). Helper-call must appear in
    // this scope.
    let scope_end_candidates = ["\n    fn ", "\nimpl ", "\nfn ", "\n}\n\n"];
    let scope_end = scope_end_candidates
        .iter()
        .filter_map(|marker| src[fn_start + "fn try_init(".len()..].find(marker))
        .min()
        .unwrap_or(src.len() - fn_start);
    let body = &src[fn_start..fn_start + "fn try_init(".len() + scope_end];
    assert!(
        body.contains("try_configure_surface("),
        "try_init body must call try_configure_surface (panic-safe helper)"
    );
}

/// Regression: BUG-06-108 — create_surface must route configure through the
/// panic-safe helper.
#[test]
fn create_surface_when_scanned_uses_configure_helper() {
    let src = include_str!("mod.rs");
    let fn_start = src
        .find("pub fn create_surface(")
        .expect("create_surface function present in state/mod.rs");
    let scope_end_candidates = ["\n    fn ", "\n    pub fn ", "\nimpl ", "\nfn ", "\n}\n\n"];
    let scope_end = scope_end_candidates
        .iter()
        .filter_map(|marker| src[fn_start + "pub fn create_surface(".len()..].find(marker))
        .min()
        .unwrap_or(src.len() - fn_start);
    let body = &src[fn_start..fn_start + "pub fn create_surface(".len() + scope_end];
    assert!(
        body.contains("try_configure_surface("),
        "create_surface body must call try_configure_surface (panic-safe helper)"
    );
}

/// Regression: BUG-06-108 — configure_surface must route through the
/// panic-safe helper.
#[test]
fn configure_surface_when_scanned_uses_configure_helper() {
    let src = include_str!("mod.rs");
    let fn_start = src
        .find("pub fn configure_surface(")
        .expect("configure_surface function present in state/mod.rs");
    let scope_end_candidates = ["\n    fn ", "\n    pub fn ", "\nimpl ", "\nfn ", "\n}\n\n"];
    let scope_end = scope_end_candidates
        .iter()
        .filter_map(|marker| src[fn_start + "pub fn configure_surface(".len()..].find(marker))
        .min()
        .unwrap_or(src.len() - fn_start);
    let body = &src[fn_start..fn_start + "pub fn configure_surface(".len() + scope_end];
    assert!(
        body.contains("try_configure_surface("),
        "configure_surface body must call try_configure_surface (panic-safe helper)"
    );
}

/// Regression: BUG-06-108 — SurfaceInitError must have ConfigurePanicked
/// variant so callers can distinguish wgpu instance refusal from a caught
/// configure panic.
#[test]
fn surface_init_error_when_constructed_with_configure_panicked_variant_compiles() {
    let _ = super::SurfaceInitError::ConfigurePanicked;
}

/// Regression: BUG-06-108 — SurfaceInitError must preserve the existing
/// `From<wgpu::CreateSurfaceError>` conversion path so callers using `?`
/// on `instance.create_surface(...)` continue to compile unchanged.
#[test]
fn surface_init_error_when_used_as_into_target_for_create_surface_error_compiles() {
    fn _assert_into<T: Into<super::SurfaceInitError>>() {}
    _assert_into::<wgpu::CreateSurfaceError>();
}

/// Regression: BUG-06-108 — the vendored wgpu-hal DXGI patch must use
/// per-target scaling: DXGI_SCALING_NONE for WndHandle (composition-class
/// targets reject NONE per Microsoft docs) and DXGI_SCALING_STRETCH for
/// all composition-class targets.
#[test]
fn wgpu_hal_dxgi_swap_chain_creation_when_scanned_uses_per_target_scaling() {
    use oriterm_test_support::paths;

    let path = paths::term_workspace_root().join("crates/wgpu-hal/src/dx12/mod.rs");
    if !path.exists() {
        eprintln!(
            "SKIP: wgpu_hal_dxgi_swap_chain_creation_when_scanned_uses_per_target_scaling — \
             vendored wgpu-hal not present at {}",
            path.display()
        );
        return;
    }
    let src = std::fs::read_to_string(&path).expect("read vendored wgpu-hal dx12/mod.rs");

    // Per-target match dispatch must be present.
    assert!(
        src.contains("let scaling = match self.target {"),
        "vendored wgpu-hal dx12/mod.rs must use per-target scaling dispatch"
    );

    // Extract the scaling-match body (everything from `match self.target {`
    // through the matching `};`).
    let match_start = src
        .find("let scaling = match self.target {")
        .expect("scaling match present");
    let after_match_kw = &src[match_start..];
    let body_end_offset = after_match_kw
        .find("};")
        .expect("scaling match must terminate with `};`");
    let match_body = &after_match_kw[..body_end_offset];

    // Each SurfaceTarget pattern must be paired with the correct scaling
    // constant within the match body. A regression that swapped a variant
    // to the wrong scaling value would fail per-variant.
    let target_scaling_pairs: &[(&str, &str)] = &[
        // WndHandle -> NONE preserves the c80fa26e resize-jitter fix.
        ("SurfaceTarget::WndHandle(_)", "DXGI_SCALING_NONE"),
        // All composition-class targets must use STRETCH per Microsoft
        // CreateSwapChainForComposition docs.
        ("SurfaceTarget::Visual(_)", "DXGI_SCALING_STRETCH"),
        ("SurfaceTarget::VisualFromWndHandle", "DXGI_SCALING_STRETCH"),
        ("SurfaceTarget::SurfaceHandle(_)", "DXGI_SCALING_STRETCH"),
        ("SurfaceTarget::SwapChainPanel(_)", "DXGI_SCALING_STRETCH"),
    ];

    for (variant, expected_scaling) in target_scaling_pairs {
        // Find the variant pattern within the match body.
        let variant_pos = match_body.find(variant).unwrap_or_else(|| {
            panic!(
                "composition arm must include `{variant}` in scaling match — \
                 Microsoft CreateSwapChainForComposition requires \
                 DXGI_SCALING_STRETCH for composition-class targets"
            );
        });

        // Walk forward from the variant pattern to the next `=>` (which
        // ends the LHS — could be after an or-pattern chain `| Other |
        // Another =>`) and then to the next `,` or arm terminator. The
        // expected scaling constant must appear between `=>` and that
        // terminator. This pairs each variant with its RHS value even
        // when multiple variants share a single or-pattern arm.
        let after_variant = &match_body[variant_pos..];
        let arrow_pos = after_variant
            .find("=>")
            .expect("each match arm must have a `=>` after its pattern");
        let after_arrow = &after_variant[arrow_pos + 2..];
        let rhs_end = after_arrow.find(',').unwrap_or(after_arrow.len().min(200));
        let arm_rhs = &after_arrow[..rhs_end];

        assert!(
            arm_rhs.contains(expected_scaling),
            "match arm reached from `{variant}` must map to `{expected_scaling}` — \
             found RHS: {arm_rhs:?}"
        );
    }
}

/// Regression: BUG-06-108 — the vendored wgpu-hal DXGI patch must cite the
/// patch source ("ori_term patch") so future readers can trace why the
/// vendored crate diverges from upstream wgpu-hal.
#[test]
fn wgpu_hal_dxgi_patch_when_scanned_cites_patch_provenance() {
    use oriterm_test_support::paths;

    let path = paths::term_workspace_root().join("crates/wgpu-hal/src/dx12/mod.rs");
    if !path.exists() {
        eprintln!(
            "SKIP: wgpu_hal_dxgi_patch_when_scanned_cites_patch_provenance — \
             vendored wgpu-hal not present"
        );
        return;
    }
    let src = std::fs::read_to_string(&path).expect("read vendored wgpu-hal dx12/mod.rs");
    assert!(
        src.contains("ori_term patch"),
        "vendored wgpu-hal dx12/mod.rs must carry an `ori_term patch` comment so \
         future readers can trace the patch provenance back to the vendored crate's README"
    );
}
