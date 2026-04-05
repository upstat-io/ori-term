//! Resize stress tests for the GPU rendering pipeline.
//!
//! Simulates rapid window resize by rendering the same GPU renderer at
//! different viewport sizes in sequence — exactly what happens when a user
//! drags the window border. Catches crashes from mismatched dimensions,
//! stale instance buffers, atlas invalidation, and render target size changes.
//!
//! The `cached_path_*` tests exercise the production `render_cached` code
//! path where content is rendered to an offscreen cache texture, then copied
//! to the output surface. When the surface is reconfigured to a smaller size
//! between `prepare()` and `render_to_surface()`, the copy extent can
//! overrun the destination texture — the exact crash seen during vertical
//! window resize on Windows.

#![cfg(all(test, feature = "gpu-tests"))]

use crate::app::compute_window_layout;
use crate::gpu::frame_input::{FrameInput, ViewportSize};
use crate::gpu::visual_regression::headless_env;

/// Render a frame at the given viewport size, returning the pixel buffer.
///
/// Uses `compute_window_layout` to derive grid cols/rows from viewport
/// dimensions — the same path as production `handle_resize()`.
fn render_at_size(
    gpu: &crate::gpu::state::GpuState,
    pipelines: &crate::gpu::pipelines::GpuPipelines,
    renderer: &mut crate::gpu::window_renderer::WindowRenderer,
    width: u32,
    height: u32,
    text: &str,
) -> Vec<u8> {
    let cell = renderer.cell_metrics();
    let scale = 1.0;
    let wl = compute_window_layout(width, height, &cell, scale, true, 0.0, 0.0, 0.0);

    let mut input = FrameInput::test_grid(wl.cols, wl.rows, text);
    input.viewport = ViewportSize::new(width, height);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let origin = (wl.grid_rect.x(), wl.grid_rect.y());
    renderer.prepare(&input, gpu, pipelines, origin, 1.0, true);

    let target = gpu.create_render_target(width, height);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

/// Render a frame at the given viewport size with tab bar and status bar.
///
/// Exercises the full layout path including chrome.
fn render_at_size_with_chrome(
    gpu: &crate::gpu::state::GpuState,
    pipelines: &crate::gpu::pipelines::GpuPipelines,
    renderer: &mut crate::gpu::window_renderer::WindowRenderer,
    width: u32,
    height: u32,
    text: &str,
) -> Vec<u8> {
    let cell = renderer.cell_metrics();
    let scale = 1.0;
    let tab_bar_h = 36.0;
    let status_bar_h = 22.0;
    let border_inset = 2.0;
    let wl = compute_window_layout(
        width,
        height,
        &cell,
        scale,
        false,
        tab_bar_h,
        status_bar_h,
        border_inset,
    );

    let mut input = FrameInput::test_grid(wl.cols, wl.rows, text);
    input.viewport = ViewportSize::new(width, height);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let origin = (wl.grid_rect.x(), wl.grid_rect.y());
    renderer.prepare(&input, gpu, pipelines, origin, 1.0, true);

    let target = gpu.create_render_target(width, height);
    renderer.render_frame(gpu, pipelines, target.view());
    gpu.read_render_target(&target)
        .expect("pixel readback should succeed")
}

// -- Stress tests --

#[test]
fn resize_stress_rapid_dimension_changes() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let text = "The quick brown fox jumps over the lazy dog. 0123456789";

    let sizes: &[(u32, u32)] = &[
        (800, 600),
        (801, 601),
        (850, 640),
        (900, 700),
        (400, 300),
        (100, 100),
        (50, 50),
        (1920, 1080),
        (80, 24),
        (800, 600),
    ];

    for &(w, h) in sizes {
        let pixels = render_at_size(&gpu, &pipelines, &mut renderer, w, h, text);
        assert_eq!(
            pixels.len(),
            (w * h * 4) as usize,
            "pixel buffer size mismatch at {w}x{h}"
        );
    }
}

#[test]
fn resize_stress_tiny_to_large() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let text = "content";

    // Grow from tiny to large.
    for size in (50..=1000).step_by(50) {
        let pixels = render_at_size(&gpu, &pipelines, &mut renderer, size, size, text);
        assert_eq!(pixels.len(), (size * size * 4) as usize);
    }
}

#[test]
fn resize_stress_large_to_tiny() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let text = "content";

    // Shrink from large to tiny.
    for size in (50..=1000).rev().step_by(50) {
        let pixels = render_at_size(&gpu, &pipelines, &mut renderer, size, size, text);
        assert_eq!(pixels.len(), (size * size * 4) as usize);
    }
}

#[test]
fn resize_stress_with_chrome() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let text = "Terminal content with chrome";

    let sizes: &[(u32, u32)] = &[
        (800, 600),
        (400, 300),
        (200, 150),
        (1200, 900),
        (600, 400),
        (100, 100),
        (800, 600),
    ];

    for &(w, h) in sizes {
        let pixels = render_at_size_with_chrome(&gpu, &pipelines, &mut renderer, w, h, text);
        assert_eq!(
            pixels.len(),
            (w * h * 4) as usize,
            "pixel buffer size mismatch at {w}x{h} with chrome"
        );
    }
}

#[test]
fn resize_stress_asymmetric_aspect_ratios() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let text = "wide and narrow";

    // Very wide, very tall, square — exercise extreme aspect ratios.
    let sizes: &[(u32, u32)] = &[
        (2000, 100),
        (100, 2000),
        (500, 500),
        (1600, 50),
        (50, 900),
        (800, 600),
    ];

    for &(w, h) in sizes {
        let pixels = render_at_size(&gpu, &pipelines, &mut renderer, w, h, text);
        assert_eq!(
            pixels.len(),
            (w * h * 4) as usize,
            "pixel buffer size mismatch at {w}x{h}"
        );
    }
}

#[test]
fn resize_stress_alternating_grow_shrink() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let text = "alternating resize cycles";

    // Simulate interactive drag: alternate grow/shrink.
    for i in 0..30 {
        let w = if i % 2 == 0 { 400 } else { 1200 };
        let h = if i % 3 == 0 { 300 } else { 800 };
        let pixels = render_at_size(&gpu, &pipelines, &mut renderer, w, h, text);
        assert_eq!(pixels.len(), (w * h * 4) as usize);
    }
}

// -- Cached render path tests --
//
// These test the production render path where content is cached in an
// offscreen texture and copied to the output. The key scenario: prepare()
// runs at viewport size A, then the surface is reconfigured to size B
// (smaller) before render_to_surface(). The copy_texture_to_texture uses
// the stale viewport A dimensions, overrunning the smaller destination.

/// Prepare at 960px height, render to 955px target.
///
/// Reproduces the exact crash from the log:
/// `Copy of Y 0..960 would end up overrunning the bounds of the
///  Destination texture of Y size 955`
#[test]
fn cached_path_vertical_shrink_during_render() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let prep_w = 1280u32;
    let prep_h = 960u32;
    let target_h = 955u32;

    let cell = renderer.cell_metrics();
    let wl = compute_window_layout(prep_w, prep_h, &cell, 1.0, true, 0.0, 0.0, 0.0);

    let mut input = FrameInput::test_grid(wl.cols, wl.rows, "test content");
    input.viewport = ViewportSize::new(prep_w, prep_h);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    let origin = (wl.grid_rect.x(), wl.grid_rect.y());
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);

    // Render cached to a SMALLER target — this is the crash.
    renderer.render_frame_cached(&gpu, &pipelines, prep_w, target_h, true);
}

/// Prepare at 800x600, render to 800x580 — vertical shrink only.
#[test]
fn cached_path_vertical_shrink_20px() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let wl = compute_window_layout(800, 600, &cell, 1.0, true, 0.0, 0.0, 0.0);

    let mut input = FrameInput::test_grid(wl.cols, wl.rows, "content");
    input.viewport = ViewportSize::new(800, 600);
    input.cell_size = cell;
    input.content.cursor.visible = false;

    renderer.prepare(
        &input,
        &gpu,
        &pipelines,
        (wl.grid_rect.x(), wl.grid_rect.y()),
        1.0,
        true,
    );
    renderer.render_frame_cached(&gpu, &pipelines, 800, 580, true);
}

/// Rapid vertical resize through the cached path.
#[test]
fn cached_path_rapid_vertical_resize() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let w = 1280u32;
    let cell = renderer.cell_metrics();

    // Simulate dragging the bottom edge up then down. Each iteration
    // prepares at one height and renders to a slightly different height
    // (as happens when WM_SIZING and surface reconfigure race).
    for prep_h in (400u32..=960).step_by(5) {
        let wl = compute_window_layout(w, prep_h, &cell, 1.0, true, 0.0, 0.0, 0.0);
        let mut input = FrameInput::test_grid(wl.cols, wl.rows, "resize");
        input.viewport = ViewportSize::new(w, prep_h);
        input.cell_size = cell;
        input.content.cursor.visible = false;
        renderer.prepare(
            &input,
            &gpu,
            &pipelines,
            (wl.grid_rect.x(), wl.grid_rect.y()),
            1.0,
            true,
        );

        // Target is a few pixels shorter — the race condition.
        let target_h = prep_h.saturating_sub(5).max(1);
        renderer.render_frame_cached(&gpu, &pipelines, w, target_h, true);
    }
}
