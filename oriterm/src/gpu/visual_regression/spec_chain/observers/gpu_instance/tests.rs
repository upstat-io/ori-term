//! Tests for the GPU instance observer (rung 6).

use oriterm_core::Rgb;
use oriterm_test_support::spec_chain::GpuInstanceExpectation;

use crate::gpu::frame_input::ViewportSize;
use crate::gpu::prepared_frame::{ImageQuad, PreparedFrame};

use super::observe_gpu_instance;

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

/// Regression: [TPR-04-001-codex-r7] — image quad observer added by
/// Section 04 Phase 1b. Tests pin positive/negative behavior for
/// `min_image_quads` across below/above quad layers.
#[test]
fn image_quads_below_present_passes() {
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

/// Regression: [TPR-04-001-codex-r7] — above-layer image quad.
#[test]
fn image_quads_above_present_passes() {
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

/// Regression: [TPR-04-001-codex-r7] — negative pin: empty image lists.
#[test]
fn image_quads_empty_fails() {
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

/// Regression: [TPR-04-001-codex-r7] — split-count: below + above.
#[test]
fn image_quads_both_layers_counted() {
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
