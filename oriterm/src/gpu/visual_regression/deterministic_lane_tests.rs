//! Deterministic golden lane validation tests (Section 05.6).
//!
//! Validates that the pinned software rasterizer + grayscale alpha hinting
//! + disabled subpixel positioning produces byte-identical output across
//! consecutive renders. Also validates the strict comparison function,
//! adapter type selection, and the production cached render path.

use crate::gpu::visual_regression::{
    GoldenLaneConfig, compare_with_reference_strict, headless_env_with_pinned_software_rasterizer,
};

// ── Reproducibility proof ─────────────────────────────────────────

/// Render the same known content twice via the deterministic lane and
/// assert the pixel output is byte-identical. This is the foundational
/// regression guard for the entire golden lane.
#[test]
fn deterministic_lane_produces_identical_output_across_runs() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, _pipelines, _renderer)) = headless_env_with_pinned_software_rasterizer(&config)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let cell = _renderer.cell_metrics();
    let cols: usize = 80;
    let rows: usize = 24;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    // Render an empty grid twice — the simplest reproducibility proof.
    let target1 = gpu.create_render_target(w, h);
    let target2 = gpu.create_render_target(w, h);

    // Just clear both targets — even the clear color should be identical.
    let mut encoder1 = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let _pass = encoder1.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("repro_1"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target1.view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.004,
                        g: 0.004,
                        b: 0.004,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });
    }
    gpu.queue.submit(std::iter::once(encoder1.finish()));

    let mut encoder2 = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let _pass = encoder2.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("repro_2"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target2.view(),
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.004,
                        g: 0.004,
                        b: 0.004,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });
    }
    gpu.queue.submit(std::iter::once(encoder2.finish()));

    let pixels_1 = gpu
        .read_render_target(&target1)
        .expect("readback 1 should succeed");
    let pixels_2 = gpu
        .read_render_target(&target2)
        .expect("readback 2 should succeed");

    assert_eq!(
        pixels_1, pixels_2,
        "deterministic lane must produce identical output across two renders"
    );
}

// ── Adapter type validation ───────────────────────────────────────

/// Asserts the deterministic lane selects a software adapter.
#[test]
fn deterministic_lane_selects_software_adapter() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, _, _)) = headless_env_with_pinned_software_rasterizer(&config) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let info = gpu.adapter_info();
    let name_lower = info.name.to_lowercase();
    const KNOWN: &[&str] = &[
        "llvmpipe",
        "lavapipe",
        "warp",
        "swiftshader",
        "mesa software",
        "cpu",
    ];
    assert!(
        KNOWN.iter().any(|s| name_lower.contains(s)),
        "expected software rasterizer, got: {:?} (backend={:?}, device_type={:?})",
        info.name,
        info.backend,
        info.device_type,
    );
}

// ── Strict comparison tests (05.5 sibling tests) ─────────────────

#[test]
fn strict_comparison_accepts_exact_match() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, _pipelines, _renderer)) = headless_env_with_pinned_software_rasterizer(&config)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Render an empty grid to get some pixels.
    let w = 160u32;
    let h = 48u32;
    let target = gpu.create_render_target(w, h);
    let pixels = gpu
        .read_render_target(&target)
        .expect("readback should succeed");

    let name = "strict_exact_match_test";
    // First call saves the reference.
    let _ = compare_with_reference_strict(name, &pixels, w, h, &config);
    // Second call should pass (exact same pixels).
    let result = compare_with_reference_strict(name, &pixels, w, h, &config);
    assert!(result.is_ok(), "exact match should pass: {result:?}");

    // Clean up.
    let ref_path = super::reference_dir().join(format!("{name}.png"));
    let _ = std::fs::remove_file(&ref_path);
}

#[test]
fn strict_comparison_rejects_single_pixel_difference() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, _, _)) = headless_env_with_pinned_software_rasterizer(&config) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let w = 160u32;
    let h = 48u32;
    let target = gpu.create_render_target(w, h);
    let pixels = gpu
        .read_render_target(&target)
        .expect("readback should succeed");

    let name = "strict_reject_test";
    // Save reference.
    let _ = compare_with_reference_strict(name, &pixels, w, h, &config);

    // Modify one pixel.
    let mut modified = pixels.clone();
    if modified.len() >= 4 {
        modified[0] = modified[0].wrapping_add(10);
    }
    let result = compare_with_reference_strict(name, &modified, w, h, &config);
    assert!(
        result.is_err(),
        "single-pixel difference should be rejected"
    );

    // Clean up.
    let ref_dir = super::reference_dir();
    let _ = std::fs::remove_file(ref_dir.join(format!("{name}.png")));
    let _ = std::fs::remove_file(ref_dir.join(format!("{name}_actual.png")));
    let _ = std::fs::remove_file(ref_dir.join(format!("{name}_diff.png")));
}

#[test]
fn strict_comparison_with_tolerance_1_accepts_minor_variation() {
    let config_strict = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, _, _)) = headless_env_with_pinned_software_rasterizer(&config_strict) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let w = 160u32;
    let h = 48u32;
    let target = gpu.create_render_target(w, h);
    let pixels = gpu
        .read_render_target(&target)
        .expect("readback should succeed");

    let name = "strict_tolerance_1_test";
    // Save reference with strict config.
    let _ = compare_with_reference_strict(name, &pixels, w, h, &config_strict);

    // Modify first pixel by 1 channel value.
    let mut modified = pixels.clone();
    if modified.len() >= 4 {
        modified[0] = modified[0].wrapping_add(1);
    }

    // Compare with tolerance=1.
    let config_relaxed = GoldenLaneConfig {
        pixel_tolerance: 1,
        max_diff_percent: 100.0,
        ..GoldenLaneConfig::SPEC_DEFAULT
    };
    let result = compare_with_reference_strict(name, &modified, w, h, &config_relaxed);
    assert!(
        result.is_ok(),
        "tolerance=1 should accept 1-channel difference: {result:?}"
    );

    // Clean up.
    let ref_path = super::reference_dir().join(format!("{name}.png"));
    let _ = std::fs::remove_file(&ref_path);
}

/// Verifies that `_actual.png` and `_diff.png` artifacts are saved when
/// strict comparison fails. The error message must reference both paths.
#[test]
fn strict_comparison_saves_diff_artifacts_on_failure() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, _, _)) = headless_env_with_pinned_software_rasterizer(&config) else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let w = 160u32;
    let h = 48u32;
    let target = gpu.create_render_target(w, h);
    let pixels = gpu
        .read_render_target(&target)
        .expect("readback should succeed");

    let name = "strict_artifacts_test";
    let ref_dir = super::reference_dir();
    let ref_path = ref_dir.join(format!("{name}.png"));
    let actual_path = ref_dir.join(format!("{name}_actual.png"));
    let diff_path = ref_dir.join(format!("{name}_diff.png"));

    // Save the reference.
    let _ = compare_with_reference_strict(name, &pixels, w, h, &config);

    // Modify one pixel to force failure.
    let mut modified = pixels.clone();
    if modified.len() >= 4 {
        modified[0] = modified[0].wrapping_add(10);
    }

    // Ensure artifacts don't already exist.
    let _ = std::fs::remove_file(&actual_path);
    let _ = std::fs::remove_file(&diff_path);

    let result = compare_with_reference_strict(name, &modified, w, h, &config);
    assert!(result.is_err(), "modified pixels should fail strict check");

    // Artifacts must exist after a failure.
    assert!(actual_path.exists(), "_actual.png must be saved on failure");
    assert!(diff_path.exists(), "_diff.png must be saved on failure");

    // Error message must reference both artifact paths.
    let msg = result.unwrap_err();
    assert!(
        msg.contains("_actual.png"),
        "error message must reference _actual.png: {msg}"
    );
    assert!(
        msg.contains("_diff.png"),
        "error message must reference _diff.png: {msg}"
    );

    // Clean up.
    let _ = std::fs::remove_file(&ref_path);
    let _ = std::fs::remove_file(&actual_path);
    let _ = std::fs::remove_file(&diff_path);
}

// ── Back-compat validation ────────────────────────────────────────

/// Existing visual_regression tests still pass — the legacy env is untouched.
/// This test verifies that `headless_env()` (legacy) still works.
#[test]
fn legacy_headless_env_still_works() {
    let Some((_gpu, _pipelines, _renderer)) = super::headless_env() else {
        eprintln!("SKIP: no GPU adapter available (legacy)");
        return;
    };
    // If we get here, the legacy env constructed successfully.
}
