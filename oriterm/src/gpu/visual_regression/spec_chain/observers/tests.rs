//! Tests for visual rung observers (rungs 5-8).
//!
//! Uses the deterministic golden lane from Section 05 — NOT the legacy
//! `headless_env_full()` entry point.

use oriterm_core::Rgb;
use oriterm_test_support::spec_chain::{
    GoldenExpectation, GpuInstanceExpectation, RungName, TextureExpectation,
};

use crate::gpu::frame_input::ViewportSize;
use crate::gpu::prepared_frame::{ImageQuad, PreparedFrame};
use crate::gpu::visual_regression::{
    GoldenLaneConfig, headless_env_with_pinned_software_rasterizer,
};

use super::texture::RenderedPixels;
use super::{observe_golden_image, observe_gpu_instance, observe_texture_render};

// ── Texture observer tests ───────────────────────────────────────

#[test]
fn texture_observer_passes_on_matching_dimensions() {
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

    let rendered = RenderedPixels {
        pixels: &pixels,
        width: w,
        height: h,
    };
    let expected = TextureExpectation {
        width: Some(w),
        height: Some(h),
        min_non_zero_pixels: None,
    };

    let result = observe_texture_render(&rendered, &expected);
    assert!(
        result.passed,
        "matching dimensions should pass: {:?}",
        result.failure
    );
    assert_eq!(result.rung_name, RungName::TextureRender);
}

#[test]
fn texture_observer_fails_on_wrong_width() {
    let pixels = vec![0u8; 160 * 48 * 4];
    let rendered = RenderedPixels {
        pixels: &pixels,
        width: 160,
        height: 48,
    };
    let expected = TextureExpectation {
        width: Some(320),
        height: None,
        min_non_zero_pixels: None,
    };

    let result = observe_texture_render(&rendered, &expected);
    assert!(!result.passed, "wrong width should fail");
    assert!(
        result.failure.as_deref().unwrap_or("").contains("width"),
        "failure message should mention width: {:?}",
        result.failure
    );
}

#[test]
fn texture_observer_fails_on_wrong_height() {
    let pixels = vec![0u8; 160 * 48 * 4];
    let rendered = RenderedPixels {
        pixels: &pixels,
        width: 160,
        height: 48,
    };
    let expected = TextureExpectation {
        width: None,
        height: Some(96),
        min_non_zero_pixels: None,
    };

    let result = observe_texture_render(&rendered, &expected);
    assert!(!result.passed, "wrong height should fail");
    assert!(
        result.failure.as_deref().unwrap_or("").contains("height"),
        "failure message should mention height: {:?}",
        result.failure
    );
}

#[test]
fn texture_observer_fails_on_insufficient_non_zero_pixels() {
    // All-zero pixel buffer.
    let pixels = vec![0u8; 100 * 100 * 4];
    let rendered = RenderedPixels {
        pixels: &pixels,
        width: 100,
        height: 100,
    };
    let expected = TextureExpectation {
        width: None,
        height: None,
        min_non_zero_pixels: Some(1),
    };

    let result = observe_texture_render(&rendered, &expected);
    assert!(
        !result.passed,
        "all-zero pixels should fail min_non_zero check"
    );
}

#[test]
fn texture_observer_passes_none_expectations() {
    let pixels = vec![0u8; 16];
    let rendered = RenderedPixels {
        pixels: &pixels,
        width: 2,
        height: 2,
    };
    let expected = TextureExpectation::default();

    let result = observe_texture_render(&rendered, &expected);
    assert!(result.passed, "all-None expectations should pass");
}

// ── Golden observer tests ────────────────────────────────────────

#[test]
fn golden_observer_passes_on_exact_match() {
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

    let name = "observer_golden_exact_test";
    let rendered = RenderedPixels {
        pixels: &pixels,
        width: w,
        height: h,
    };
    let expected = GoldenExpectation {
        golden_name: Some(name),
    };

    // First call saves the reference.
    let _ = observe_golden_image(&rendered, name, &expected, &config);
    // Second call should match exactly.
    let result = observe_golden_image(&rendered, name, &expected, &config);
    assert!(
        result.passed,
        "exact match should pass: {:?}",
        result.failure
    );
    assert_eq!(result.rung_name, RungName::GoldenImage);

    // Clean up.
    let ref_path = crate::gpu::visual_regression::reference_dir().join(format!("{name}.png"));
    let _ = std::fs::remove_file(&ref_path);
}

#[test]
fn golden_observer_fails_on_pixel_mismatch() {
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

    let name = "observer_golden_mismatch_test";
    let expected = GoldenExpectation {
        golden_name: Some(name),
    };

    // Save reference.
    let rendered = RenderedPixels {
        pixels: &pixels,
        width: w,
        height: h,
    };
    let _ = observe_golden_image(&rendered, name, &expected, &config);

    // Modify a pixel.
    let mut modified = pixels.clone();
    if modified.len() >= 4 {
        modified[0] = modified[0].wrapping_add(10);
    }
    let rendered_modified = RenderedPixels {
        pixels: &modified,
        width: w,
        height: h,
    };

    let result = observe_golden_image(&rendered_modified, name, &expected, &config);
    assert!(!result.passed, "pixel mismatch should fail");

    // Clean up.
    let ref_dir = crate::gpu::visual_regression::reference_dir();
    let _ = std::fs::remove_file(ref_dir.join(format!("{name}.png")));
    let _ = std::fs::remove_file(ref_dir.join(format!("{name}_actual.png")));
    let _ = std::fs::remove_file(ref_dir.join(format!("{name}_diff.png")));
}

#[test]
fn golden_observer_uses_catalog_row_id_as_fallback_name() {
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

    let catalog_id = "OBSERVER-FALLBACK-TEST";
    let expected = GoldenExpectation {
        golden_name: None, // Should fall back to catalog_row_id.
    };
    let rendered = RenderedPixels {
        pixels: &pixels,
        width: w,
        height: h,
    };

    // First call saves reference using catalog_row_id as the name.
    let _ = observe_golden_image(&rendered, catalog_id, &expected, &config);

    // Verify the reference was saved under the catalog ID name.
    let ref_path = crate::gpu::visual_regression::reference_dir().join(format!("{catalog_id}.png"));
    assert!(
        ref_path.exists(),
        "reference should be saved using catalog_row_id when golden_name is None"
    );

    // Clean up.
    let _ = std::fs::remove_file(&ref_path);
}

// ── GPU instance observer: image quad tests ──────────────────────

fn dummy_image_quad() -> ImageQuad {
    ImageQuad {
        image_id: oriterm_core::image::ImageId::from_raw(1),
        x: 0.0,
        y: 0.0,
        w: 10.0,
        h: 10.0,
        uv_x: 0.0,
        uv_y: 0.0,
        uv_w: 1.0,
        uv_h: 1.0,
        opacity: 1.0,
    }
}

#[test]
fn gpu_instance_image_quads_passes_when_below_present() {
    let vp = ViewportSize::new(100, 100);
    let mut frame = PreparedFrame::new(vp, Rgb { r: 0, g: 0, b: 0 }, 1.0);
    frame.image_quads_below.push(dummy_image_quad());

    let expected = GpuInstanceExpectation::at_least(0, 0).with_images(1);
    let result = observe_gpu_instance(&frame, &expected);
    assert!(
        result.passed,
        "should pass with 1 below quad: {:?}",
        result.failure
    );
}

#[test]
fn gpu_instance_image_quads_passes_when_above_present() {
    let vp = ViewportSize::new(100, 100);
    let mut frame = PreparedFrame::new(vp, Rgb { r: 0, g: 0, b: 0 }, 1.0);
    frame.image_quads_above.push(dummy_image_quad());

    let expected = GpuInstanceExpectation::at_least(0, 0).with_images(1);
    let result = observe_gpu_instance(&frame, &expected);
    assert!(
        result.passed,
        "should pass with 1 above quad: {:?}",
        result.failure
    );
}

#[test]
fn gpu_instance_image_quads_fails_when_empty() {
    let vp = ViewportSize::new(100, 100);
    let frame = PreparedFrame::new(vp, Rgb { r: 0, g: 0, b: 0 }, 1.0);

    let expected = GpuInstanceExpectation::at_least(0, 0).with_images(1);
    let result = observe_gpu_instance(&frame, &expected);
    assert!(!result.passed, "should fail with 0 image quads");
    assert!(
        result
            .failure
            .as_deref()
            .unwrap_or("")
            .contains("image quads"),
        "failure should mention image quads: {:?}",
        result.failure
    );
}

#[test]
fn gpu_instance_image_quads_counts_both_layers() {
    let vp = ViewportSize::new(100, 100);
    let mut frame = PreparedFrame::new(vp, Rgb { r: 0, g: 0, b: 0 }, 1.0);
    frame.image_quads_below.push(dummy_image_quad());
    frame.image_quads_above.push(dummy_image_quad());

    let expected = GpuInstanceExpectation::at_least(0, 0).with_images(2);
    let result = observe_gpu_instance(&frame, &expected);
    assert!(
        result.passed,
        "should pass with 1 below + 1 above = 2 total: {:?}",
        result.failure
    );
}
