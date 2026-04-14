//! Tests for the texture render observer (rung 7).

use oriterm_test_support::spec_chain::{RungName, TextureExpectation};

use crate::gpu::visual_regression::{
    GoldenLaneConfig, headless_env_with_pinned_software_rasterizer,
};

use super::RenderedPixels;
use super::observe_texture_render;

#[test]
fn texture_dimensions_match_passes() {
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
fn texture_wrong_width_fails() {
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
fn texture_wrong_height_fails() {
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
fn texture_all_zero_pixels_insufficient_fails() {
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
fn texture_none_expectations_passes() {
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
