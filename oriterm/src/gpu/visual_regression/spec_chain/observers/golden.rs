//! Golden image observer (rung 8).
//!
//! Compares rendered pixels against a committed reference PNG via
//! `compare_with_reference_strict()`. The golden name is taken from
//! the expectation or falls back to the scenario's catalog row ID.

use oriterm_test_support::spec_chain::{GoldenExpectation, RungName, RungResult};

use crate::gpu::visual_regression::GoldenLaneConfig;
use crate::gpu::visual_regression::compare::compare_with_reference_strict;

use super::texture::RenderedPixels;

/// Observe the golden-image rung: do rendered pixels match the reference PNG?
pub fn observe_golden_image(
    rendered: &RenderedPixels<'_>,
    golden_name: &str,
    expected: &GoldenExpectation,
    config: &GoldenLaneConfig,
) -> RungResult {
    let name = expected.golden_name.unwrap_or(golden_name);

    match compare_with_reference_strict(
        name,
        rendered.pixels,
        rendered.width,
        rendered.height,
        config,
    ) {
        Ok(()) => RungResult::pass(RungName::GoldenImage),
        Err(msg) => RungResult::fail(RungName::GoldenImage, msg),
    }
}
