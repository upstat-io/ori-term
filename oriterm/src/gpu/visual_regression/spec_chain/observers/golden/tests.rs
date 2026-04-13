//! Tests for the golden image observer (rung 8).

use oriterm_test_support::spec_chain::{GoldenExpectation, RungName};

use crate::gpu::visual_regression::{
    GoldenLaneConfig, headless_env_with_pinned_software_rasterizer,
};

use super::super::texture::RenderedPixels;
use super::observe_golden_image;

#[test]
fn golden_exact_match_passes() {
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
fn golden_pixel_mismatch_fails() {
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
fn golden_catalog_row_id_fallback_used_when_no_golden_name() {
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
