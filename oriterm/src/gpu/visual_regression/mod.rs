//! Visual regression test infrastructure.
//!
//! Renders known terminal content to offscreen targets, reads back pixels,
//! and compares against reference PNGs with per-pixel fuzzy tolerance.
//!
//! # Workflow
//!
//! 1. First run (no reference): renders and saves as the reference PNG.
//! 2. Subsequent runs: renders, compares against reference with tolerance.
//! 3. On mismatch: saves `*_actual.png` and `*_diff.png` for inspection.
//! 4. `ORITERM_UPDATE_GOLDEN=1`: overwrites references with current output.
//!    **CI guard:** panics if both `CI` and `ORITERM_UPDATE_GOLDEN` are set.
//!
//! # Running
//!
//! ```text
//! cargo test -p oriterm -- visual_regression
//! ```

mod compare;
mod core_tests;
mod cursor_opacity_tests;
mod decoration_tests;
mod deterministic_lane_tests;
mod dialog_helpers;
mod edge_case_tests;
mod frame_input_helper;
mod golden_lane_config;
pub(crate) use golden_lane_config::GoldenLaneConfig;
mod main_window;
mod meta_tests;
mod multi_size;
mod reference_tests;
mod resize_stress;
mod settings_dialog;
#[cfg(test)]
mod spec_chain;
mod status_bar;
mod tab_bar_brutal;
mod tab_bar_icons;
mod tack;
mod text_blink_tests;
mod vttest;
mod weight_tests;

use std::path::PathBuf;

use super::frame_input::FrameInput;
use super::pipelines::GpuPipelines;
use super::state::{AdapterPreference, GpuState};
use super::window_renderer::WindowRenderer;
use crate::font::{FontCollection, FontSet, GlyphFormat, HintingMode};

/// Per-channel tolerance for pixel comparison. Accounts for anti-aliasing
/// differences and minor rasterization variance across GPU drivers.
pub(super) const PIXEL_TOLERANCE: u8 = 2;

/// Maximum percentage of pixels allowed to differ before a test fails.
/// 99.5% of pixels must match (at most 0.5% may differ).
pub(super) const MAX_MISMATCH_PERCENT: f64 = 0.5;

/// Default test font parameters.
const TEST_FONT_WEIGHT: u16 = 400;
const TEST_FONT_SIZE_PT: f32 = 12.0;
const TEST_DPI: f32 = 96.0;

/// Directory for reference PNG files, relative to the crate root.
pub(super) fn reference_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/references")
}

/// **Legacy non-deterministic** headless rendering environment.
///
/// Uses `FontSet::embedded()` and any available GPU adapter. Adapter
/// selection is non-deterministic across machines (discrete GPU preferred).
/// Defaults to `HintingMode::Full` with `subpixel_positioning: true`.
///
/// New spec-conformance goldens MUST use
/// [`headless_env_with_pinned_software_rasterizer`] instead — it pins
/// the software rasterizer, hinting mode, and subpixel positioning for
/// reproducible output.
pub(crate) fn headless_env() -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_with_config(TEST_FONT_SIZE_PT, TEST_DPI)
}

/// Headless rendering environment with configurable font size and DPI.
pub(super) fn headless_env_with_config(
    size_pt: f32,
    dpi: f32,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_full(size_pt, dpi, GlyphFormat::Alpha)
}

/// Headless rendering environment with configurable font size, DPI, and glyph format.
pub(super) fn headless_env_full(
    size_pt: f32,
    dpi: f32,
    format: GlyphFormat,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    headless_env_with_hinting(size_pt, dpi, format, HintingMode::Full)
}

/// Headless rendering environment with full control over font size, DPI,
/// glyph format, and hinting mode.
pub(super) fn headless_env_with_hinting(
    size_pt: f32,
    dpi: f32,
    format: GlyphFormat,
    hinting: HintingMode,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless().ok()?;
    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        FontSet::embedded(),
        size_pt,
        dpi,
        format,
        TEST_FONT_WEIGHT,
        550,
        hinting,
    )
    .ok()?;
    let renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, None);
    Some((gpu, pipelines, renderer))
}

/// Create a headless rendering environment pinned to a software rasterizer
/// for deterministic golden image comparison.
///
/// This is the canonical entry point for spec-conformance golden tests.
/// Uses `force_fallback_adapter: true` (primary) to select a software
/// rasterizer (llvmpipe/WARP/swiftshader), reads font parameters from
/// `config`, and derives cell metrics from `FontCollection::cell_metrics()`
/// (the SSOT — no independent cell metric storage).
///
/// Returns `None` if no software rasterizer is available on the current
/// platform. Tests should use the graceful skip protocol:
///
/// ```text
/// let Some((gpu, pipelines, renderer)) =
///     headless_env_with_pinned_software_rasterizer(&GoldenLaneConfig::SPEC_DEFAULT)
/// else {
///     eprintln!("SKIP: software rasterizer unavailable");
///     return;
/// };
/// ```
///
/// **Legacy entry points** (`headless_env`, `headless_env_with_config`,
/// `headless_env_full`, `headless_env_with_hinting`) remain for the existing
/// visual regression test suite. New spec-conformance goldens MUST use this
/// function instead.
#[allow(
    dead_code,
    reason = "consumed by 05.4 VisualSpecHarness + 05.6 validation tests"
)]
pub(crate) fn headless_env_with_pinned_software_rasterizer(
    config: &GoldenLaneConfig,
) -> Option<(GpuState, GpuPipelines, WindowRenderer)> {
    let gpu = GpuState::new_headless_with_preference(AdapterPreference::SoftwareRasterizer).ok()?;

    let info = gpu.adapter_info();
    log::info!(
        "deterministic lane: adapter={}, backend={:?}, device_type={:?}",
        info.name,
        info.backend,
        info.device_type,
    );

    let pipelines = GpuPipelines::new(&gpu);
    let font_collection = FontCollection::new(
        FontSet::embedded(),
        config.font_size_pt,
        config.dpi,
        config.glyph_format,
        TEST_FONT_WEIGHT,
        550,
        config.hinting_mode,
    )
    .ok()?;
    let renderer = WindowRenderer::new(&gpu, &pipelines, font_collection, None);
    Some((gpu, pipelines, renderer))
}

/// Render a `FrameInput` to RGBA pixels via the headless pipeline.
pub(super) fn render_to_pixels(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    input: &FrameInput,
) -> Vec<u8> {
    render_to_pixels_with_origin(gpu, pipelines, renderer, input, (0.0, 0.0))
}

/// Render a `FrameInput` to RGBA pixels with a custom grid origin offset.
///
/// The `origin` shifts all cell positions, simulating chrome height.
pub(super) fn render_to_pixels_with_origin(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    input: &FrameInput,
    origin: (f32, f32),
) -> Vec<u8> {
    let w = input.viewport.width;
    let h = input.viewport.height;
    let target = gpu.create_render_target(w, h);
    renderer.prepare(input, gpu, pipelines, origin, 1.0, true);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

/// Render with a specific cursor opacity (for fade-blink testing).
///
/// Unlike [`render_to_pixels`] which always uses opacity 1.0, this function
/// passes the given `cursor_opacity` to the prepare pipeline, testing the
/// GPU alpha blending path in isolation from `ColorEase`.
pub(super) fn render_to_pixels_with_opacity(
    gpu: &GpuState,
    pipelines: &GpuPipelines,
    renderer: &mut WindowRenderer,
    input: &FrameInput,
    cursor_opacity: f32,
) -> Vec<u8> {
    let w = input.viewport.width;
    let h = input.viewport.height;
    let target = gpu.create_render_target(w, h);
    renderer.prepare(input, gpu, pipelines, (0.0, 0.0), cursor_opacity, true);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

// Re-export comparison functions from the `compare` submodule so sibling
// modules can import them via `super::compare_with_reference` (matching the
// import paths established before the split).
use compare::compare_with_reference;
#[allow(
    unused_imports,
    reason = "consumed by 04.4 golden observer + 05.6 validation tests"
)]
pub(crate) use compare::compare_with_reference_strict;
use compare::pixel_diff;
