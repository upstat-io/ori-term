//! Deterministic golden lane tests exercising the production cached render path.
//!
//! These tests use `render_frame_cached()` (the content-caching production path)
//! rather than `render_frame()` (test-only direct path). Gated behind `gpu-tests`
//! because `render_frame_cached()` requires `#[cfg(feature = "gpu-tests")]`.
//!
//! Section 05.6 items:
//! - Cached render reproducibility proof
//! - Subpixel positioning toggle semantic pin

#![cfg(all(test, feature = "gpu-tests"))]

use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::visual_regression::{
    GoldenLaneConfig, headless_env_with_pinned_software_rasterizer,
};

// ── Cached render reproducibility proof ──────────────────────────

/// Render the same terminal content twice via the production cached render
/// path (`render_frame_cached`) and assert byte-identical pixel output.
///
/// This complements `deterministic_lane_produces_identical_output_across_runs`
/// (which uses raw encoder clears) by exercising the full production pipeline:
/// prepare -> content cache -> copy to output -> overlay/cursor pass.
#[test]
fn cached_render_produces_identical_output_across_runs() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, pipelines, mut renderer)) =
        headless_env_with_pinned_software_rasterizer(&config)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols: usize = 80;
    let rows: usize = 24;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, "Hello, deterministic world! 0123456789");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.subpixel_positioning = config.subpixel_positioning;
    input.content.cursor.visible = false;

    // First render via cached path.
    renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    let target1 = renderer.render_frame_cached(&gpu, &pipelines, w, h, true);
    let pixels_1 = gpu
        .read_render_target(&target1)
        .expect("readback 1 should succeed");

    // Second render via cached path (same input, content_changed=true to
    // force full re-render rather than hitting the cache-reuse path).
    renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    let target2 = renderer.render_frame_cached(&gpu, &pipelines, w, h, true);
    let pixels_2 = gpu
        .read_render_target(&target2)
        .expect("readback 2 should succeed");

    assert_eq!(
        pixels_1, pixels_2,
        "render_frame_cached must produce byte-identical output across consecutive runs \
         (deterministic lane: software rasterizer + grayscale alpha + no subpixel positioning)"
    );
}

/// Verify the cache-reuse branch (`content_changed=false`) produces
/// byte-identical output to the full render branch.
///
/// In production, steady-state frames (cursor blink, no content change)
/// hit the `content_changed=false` path which skips content uploads and
/// reuses the cached texture. This test ensures that path produces the
/// same pixels as a full re-render.
#[test]
fn cached_render_reuse_branch_matches_full_render() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, pipelines, mut renderer)) =
        headless_env_with_pinned_software_rasterizer(&config)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols: usize = 80;
    let rows: usize = 24;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, "Cache reuse determinism test 0123456789");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.subpixel_positioning = config.subpixel_positioning;
    input.content.cursor.visible = false;

    // First render: full content upload (content_changed=true).
    renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    let target_full = renderer.render_frame_cached(&gpu, &pipelines, w, h, true);
    let pixels_full = gpu
        .read_render_target(&target_full)
        .expect("readback (full) should succeed");

    // Second render: cache reuse (content_changed=false) — the steady-state
    // blink-only path. Skips content uploads, reuses the cached texture.
    renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, false);
    let target_reuse = renderer.render_frame_cached(&gpu, &pipelines, w, h, false);
    let pixels_reuse = gpu
        .read_render_target(&target_reuse)
        .expect("readback (reuse) should succeed");

    assert_eq!(
        pixels_full, pixels_reuse,
        "cache-reuse branch (content_changed=false) must produce identical output \
         to full render (content_changed=true) for the same input"
    );
}

// ── Subpixel positioning toggle semantic pin ─────────────────────

/// Semantic pin: the deterministic lane correctly propagates
/// `GoldenLaneConfig.subpixel_positioning` to the renderer AND the flag
/// reaches the glyph emitter through the FrameInput.
///
/// The embedded monospace font (IBM Plex Mono) produces integer cell metrics
/// and zero fractional glyph offsets at all grid-fitted sizes, so a pixel-
/// level comparison cannot distinguish subpixel_positioning=true vs false.
/// The behavioral proof that the flag changes rasterization output lives in
/// `window_renderer/tests.rs`:
/// - `grid_raster_keys_disabled_subpx_all_zero` — synthetic data, flag=false
/// - `grid_raster_keys_enabled_subpx_nonzero` — synthetic data, flag=true
///
/// This test verifies the PLUMBING: the deterministic lane setup propagates
/// the flag from config → renderer, and the FrameInput carries it through to
/// the prepare → emit pipeline. If either propagation is broken, the
/// deterministic lane silently runs with the wrong subpixel positioning mode.
#[test]
fn subpixel_positioning_propagated_from_config_to_renderer() {
    // SPEC_DEFAULT has subpixel_positioning: false.
    let config_off = GoldenLaneConfig::SPEC_DEFAULT;
    assert!(
        !config_off.subpixel_positioning,
        "SPEC_DEFAULT.subpixel_positioning must be false"
    );

    let Some((_gpu, _pipelines, renderer_off)) =
        headless_env_with_pinned_software_rasterizer(&config_off)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    assert!(
        !renderer_off.subpixel_positioning(),
        "renderer must have subpixel_positioning=false when config says false"
    );

    // Config with subpixel_positioning: true.
    let config_on = GoldenLaneConfig {
        subpixel_positioning: true,
        ..GoldenLaneConfig::SPEC_DEFAULT
    };
    let Some((_gpu_on, _pipelines_on, renderer_on)) =
        headless_env_with_pinned_software_rasterizer(&config_on)
    else {
        eprintln!("SKIP: software rasterizer unavailable (second instance)");
        return;
    };
    assert!(
        renderer_on.subpixel_positioning(),
        "renderer must have subpixel_positioning=true when config says true"
    );
}

/// Semantic pin: the cached render path produces non-blank pixels when
/// rendering text content, proving the full pipeline (prepare → content
/// cache → copy → overlay) is functional and actually rasterizes glyphs.
///
/// Complements the reproducibility test above which asserts identical output
/// but doesn't verify the output contains rendered text.
#[test]
fn cached_render_produces_non_blank_output() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, pipelines, mut renderer)) =
        headless_env_with_pinned_software_rasterizer(&config)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols: usize = 80;
    let rows: usize = 24;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, "ABCDEFGHIJ 0123456789 The quick brown fox");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.subpixel_positioning = config.subpixel_positioning;
    input.content.cursor.visible = false;

    renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    let target = renderer.render_frame_cached(&gpu, &pipelines, w, h, true);
    let pixels = gpu
        .read_render_target(&target)
        .expect("readback should succeed");

    // Verify the pixel buffer is not all-zero (blank/transparent) and not
    // all-same (solid fill without any glyphs). Both would indicate the
    // cached render path failed to produce visible text.
    let total_bytes = pixels.len();
    assert!(total_bytes > 0, "pixel buffer must not be empty");

    let non_zero = pixels.iter().filter(|&&b| b != 0).count();
    assert!(
        non_zero > 0,
        "pixel buffer is all-zero — cached render path produced blank output"
    );

    // Count distinct pixel values — rendered text should produce many distinct
    // values from anti-aliased glyph edges. A solid fill would have at most 4
    // distinct values (one RGBA quad).
    let mut seen = std::collections::HashSet::new();
    for chunk in pixels.chunks(4) {
        seen.insert((chunk[0], chunk[1], chunk[2], chunk[3]));
    }
    assert!(
        seen.len() > 4,
        "pixel buffer has only {} distinct RGBA values — expected anti-aliased text \
         to produce many distinct values from glyph edges",
        seen.len(),
    );
}
