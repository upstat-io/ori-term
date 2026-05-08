//! Integration test for the incremental dispatch path: drive the
//! production dispatch through `WindowRenderer::prepare` +
//! `render_frame_cached`, and assert that Frame N+1 dispatches incremental
//! AND its cached-render output matches Frame N's pixel-for-pixel.
//!
//! Gated behind `gpu-tests` because `render_frame_cached()` requires the
//! feature flag. Skips when no software rasterizer is available.

#![cfg(all(test, feature = "gpu-tests"))]

use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::visual_regression::{
    GoldenLaneConfig, headless_env_with_pinned_software_rasterizer,
};

/// Regression: BUG-06-027 — Frame N+1's `WindowRenderer::prepare` must
/// dispatch through the incremental branch (production fix's
/// pre-dispatch save_terminal_tier publishes Frame N's terminal tier into
/// saved_tier), AND `render_frame_cached` must produce pixel-identical
/// output relative to Frame N for the same input.
///
/// See: bug-tracker/plans/BUG-06-027/
#[test]
fn incremental_dispatch_cached_render_path_matches_fresh_rebuild() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let Some((gpu, pipelines, mut renderer)) =
        headless_env_with_pinned_software_rasterizer(&config)
    else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let cell = renderer.cell_metrics();
    let cols: usize = 20;
    let rows: usize = 8;
    let w = (cell.width * cols as f32).ceil() as u32;
    let h = (cell.height * rows as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(cols, rows, "incremental dispatch cached render test");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.subpixel_positioning = config.subpixel_positioning;
    input.content.cursor.visible = false;

    // Frame 0: full-rebuild. content_changed=true populates terminal tier
    // for the next frame's dispatch.
    renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    assert!(
        !renderer.prepared.was_incremental,
        "Frame 0 must be full-rebuild (saved_tier empty before first prepare)"
    );
    let target0 = renderer.render_frame_cached(&gpu, &pipelines, w, h, true);
    let pixels_0 = gpu
        .read_render_target(&target0)
        .expect("readback frame 0 should succeed");

    // Frame 1: identical input, content_changed=true so the production
    // dispatch predicate runs (cursor-blink fast path is gated on
    // !content_changed). Pre-dispatch save_terminal_tier publishes
    // Frame 0's terminal tier → can_incremental resolves true → frame 1
    // takes the incremental branch.
    input.content.all_dirty = false;
    input.content.damage.clear();
    renderer.prepare(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    assert!(
        renderer.prepared.was_incremental,
        "Frame 1 must dispatch incremental — production fix's pre-dispatch \
         save_terminal_tier should publish Frame 0's terminal tier into saved_tier"
    );

    let target1 = renderer.render_frame_cached(&gpu, &pipelines, w, h, true);
    let pixels_1 = gpu
        .read_render_target(&target1)
        .expect("readback frame 1 should succeed");

    assert_eq!(
        pixels_0, pixels_1,
        "incremental-path cached render must produce pixel-identical output \
         to the prior frame for unchanged input (replay correctness end-to-end)"
    );
}
