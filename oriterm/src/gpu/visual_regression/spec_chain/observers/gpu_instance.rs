//! GPU instance observer (rung 6).
//!
//! Asserts that the prepared GPU instance buffers contain the expected
//! number and kind of instances after `WindowRenderer::prepare()`.

use oriterm_test_support::spec_chain::RungResult;
use oriterm_test_support::spec_chain::{GpuInstanceExpectation, RungName};

use crate::gpu::prepared_frame::PreparedFrame;

/// Observe the GPU instance rung: do instance buffers match expected?
pub fn observe_gpu_instance(
    prepared: &PreparedFrame,
    expected: &GpuInstanceExpectation,
) -> RungResult {
    if let Some(min_bg) = expected.min_backgrounds {
        let actual = prepared.backgrounds.len();
        if actual < min_bg {
            return RungResult::fail(
                RungName::GpuInstance,
                format!("GPU backgrounds: expected >= {min_bg}, got {actual}"),
            );
        }
    }

    if let Some(min_glyphs) = expected.min_glyphs {
        let actual =
            prepared.glyphs.len() + prepared.subpixel_glyphs.len() + prepared.color_glyphs.len();
        if actual < min_glyphs {
            return RungResult::fail(
                RungName::GpuInstance,
                format!("GPU total glyphs: expected >= {min_glyphs}, got {actual}"),
            );
        }
    }

    if let Some(has_cursor) = expected.has_cursor {
        let actual = !prepared.cursors.is_empty();
        if actual != has_cursor {
            return RungResult::fail(
                RungName::GpuInstance,
                format!("GPU cursor instance: expected {has_cursor}, got {actual}"),
            );
        }
    }

    RungResult::pass(RungName::GpuInstance)
}
