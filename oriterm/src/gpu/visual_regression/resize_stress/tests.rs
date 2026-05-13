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

// -- Resize-grow uncovered-region clearing tests --
//
// When the render destination is LARGER than the prepared viewport
// (the resize-grow case — winit `Resized` fires between `prepare()` and
// `render_to_surface()`), the partial cache copy fills only the
// `vp × vp` upper-left sub-rect. The overlay-pass `LoadOp::Load`
// then exposes uninitialized memory along the new edge. Fix: pre-clear
// the destination to `clear_color()` when `dst > vp` on either axis.

/// Shared helper: prepare + render through `render_frame_cached` with a
/// caller-controlled palette background + opacity, then read back pixels.
///
/// Returns `(pixels, target_w)` for `pixel_rgba_at(...)` lookups.
#[allow(
    clippy::too_many_arguments,
    reason = "test matrix helper — each parameter is a deliberate axis the \
    cached_path_* tests vary independently (viewport/target dimensions, palette \
    background, opacity); collapsing into a struct would obscure the matrix shape"
)]
fn prepare_and_render_cached_with_clear(
    gpu: &crate::gpu::state::GpuState,
    pipelines: &crate::gpu::pipelines::GpuPipelines,
    renderer: &mut crate::gpu::window_renderer::WindowRenderer,
    prep_w: u32,
    prep_h: u32,
    target_w: u32,
    target_h: u32,
    palette_bg: oriterm_core::Rgb,
    opacity: f32,
) -> (Vec<u8>, u32) {
    let cell = renderer.cell_metrics();
    let wl = compute_window_layout(prep_w, prep_h, &cell, 1.0, true, 0.0, 0.0, 0.0);
    let mut input = FrameInput::test_grid(wl.cols, wl.rows, "test");
    input.viewport = ViewportSize::new(prep_w, prep_h);
    input.cell_size = cell;
    input.content.cursor.visible = false;
    input.palette.background = palette_bg;
    input.palette.opacity = opacity;

    let origin = (wl.grid_rect.x(), wl.grid_rect.y());
    renderer.prepare(&input, gpu, pipelines, origin, 1.0, true);
    let target = renderer.render_frame_cached(gpu, pipelines, target_w, target_h, true);
    let pixels = gpu
        .read_render_target(&target)
        .expect("pixel readback should succeed");
    (pixels, target_w)
}

/// Fetch the RGBA byte tuple at `(x, y)` in a tightly-packed pixel buffer.
fn pixel_rgba_at(pixels: &[u8], x: u32, y: u32, target_w: u32) -> [u8; 4] {
    let i = ((y * target_w + x) * 4) as usize;
    [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
}

/// Approximate-equal channel comparison absorbing 8-bit quantization.
fn rgba_approx_eq(a: [u8; 4], b: [u8; 4], epsilon: u8) -> bool {
    a.iter()
        .zip(b.iter())
        .all(|(x, y)| x.abs_diff(*y) <= epsilon)
}

/// Regression: BUG-06-052 — semantic pin on both axes; pixels outside the
/// prepared viewport equal `clear_color()`.
///
/// Uses a MID-TONE color (Rgb(128, 64, 200)) rather than saturated extremes —
/// saturated colors mask sRGB round-trip bugs because 0 and 255 map identically
/// in sRGB and linear space. Mid-tones exercise the sRGB→linear→sRGB round
/// trip in the clear-pass + readback pipeline.
#[test]
fn cached_path_grow_both_axes_clears_uncovered_to_clear_color() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    let (pixels, w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        1200,
        800,
        bg,
        1.0,
    );
    let outside = pixel_rgba_at(&pixels, 1100, 700, w);
    assert!(
        rgba_approx_eq(outside, [128, 64, 200, 255], 4),
        "uncovered pixel at (1100,700) should be mid-tone purple (128,64,200,255) \
         after sRGB round-trip, got {outside:?}"
    );
}

/// Regression: BUG-06-052 — horizontal-only grow; clear pass must fire.
#[test]
fn cached_path_grow_horizontal_only_clears_uncovered_strip() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    let (pixels, w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        1200,
        600,
        bg,
        1.0,
    );
    let outside = pixel_rgba_at(&pixels, 1100, 300, w);
    assert!(
        rgba_approx_eq(outside, [128, 64, 200, 255], 4),
        "uncovered pixel at (1100,300) should be mid-tone purple (128,64,200,255), got {outside:?}"
    );
}

/// Regression: BUG-06-052 — vertical-only grow; clear pass must fire.
#[test]
fn cached_path_grow_vertical_only_clears_uncovered_strip() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    let (pixels, w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        800,
        800,
        bg,
        1.0,
    );
    let outside = pixel_rgba_at(&pixels, 400, 700, w);
    assert!(
        rgba_approx_eq(outside, [128, 64, 200, 255], 4),
        "uncovered pixel at (400,700) should be mid-tone purple (128,64,200,255), got {outside:?}"
    );
}

/// Regression: BUG-06-052 — mixed-axis: width grows, height shrinks. `||`
/// gate must fire when EITHER grows.
#[test]
fn cached_path_grow_h_shrink_v_clears_horizontal_grow_region() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    let (pixels, w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        1200,
        400,
        bg,
        1.0,
    );
    let outside = pixel_rgba_at(&pixels, 1100, 200, w);
    assert!(
        rgba_approx_eq(outside, [128, 64, 200, 255], 4),
        "uncovered horizontal-grow pixel at (1100,200) should be mid-tone purple (128,64,200,255), got {outside:?}"
    );
}

/// Regression: BUG-06-052 — mixed-axis: width shrinks, height grows;
/// inverse-axis coverage for the `||` gate.
#[test]
fn cached_path_shrink_h_grow_v_clears_vertical_grow_region() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    let (pixels, w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        600,
        800,
        bg,
        1.0,
    );
    let outside = pixel_rgba_at(&pixels, 400, 700, w);
    assert!(
        rgba_approx_eq(outside, [128, 64, 200, 255], 4),
        "uncovered vertical-grow pixel at (400,700) should be mid-tone purple (128,64,200,255), got {outside:?}"
    );
}

/// Regression: BUG-06-052 — common-path guard: `dst == vp` must continue
/// to work post-fix. Pins both readback length AND non-zero rendered
/// output (rejects a hypothetical no-op render that would also satisfy
/// length but produce all-zero pixels).
#[test]
fn cached_path_dst_eq_vp_no_extra_clear() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    let (pixels, _w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        800,
        600,
        bg,
        1.0,
    );
    assert_eq!(pixels.len(), (800 * 600 * 4) as usize);
    assert!(
        pixels.iter().any(|&b| b != 0),
        "rendered pixels must not be all zero — no-op render would satisfy length only"
    );
}

/// Regression: BUG-06-052 — opacity dimension: opacity < 1.0 with non-zero
/// background. Verifies the premultiplied-alpha clear value reaches the
/// uncovered region.
#[test]
fn cached_path_grow_clears_with_semi_transparent_clear() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    // Saturated magenta deliberately chosen for the opacity-channel pin —
    // R/B channels saturate, G zeroes out, alpha pinned at 0.5.
    let bg = oriterm_core::Rgb {
        r: 255,
        g: 0,
        b: 255,
    };
    let (pixels, w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        1200,
        800,
        bg,
        0.5,
    );
    let outside = pixel_rgba_at(&pixels, 1100, 700, w);
    // Premultiplied: linear-space r/b ≈ 0.5, sRGB-encoded back to ≈188 for
    // pure magenta at half opacity. Alpha ≈ 128.
    assert!(
        outside[3].abs_diff(128) <= 4,
        "uncovered pixel alpha should be ~128 (opacity=0.5), got {outside:?}"
    );
    assert!(
        outside[0] > 100 && outside[2] > 100,
        "uncovered pixel R/B should be roughly half-magenta, got {outside:?}"
    );
    assert!(
        outside[1] < 16,
        "uncovered pixel G should be ~0, got {outside:?}"
    );
}

/// Regression: BUG-06-052 — deterministic pin against wgpu zero-init
/// masking. Two-frame sequence with DIFFERENT clear colors; the uncovered
/// region after the second (grown) frame MUST be the CURRENT clear color,
/// NEVER black ([0,0,0,0]) and NEVER the prior frame's clear color
/// (defends against any cache-reuse path that could surface prior cache
/// content into the grown region). `render_frame_cached` builds a fresh
/// `RenderTarget` per call, so the destination texture itself cannot
/// retain prior-frame contents — but the CACHE texture is reused across
/// the two renderer.prepare() invocations on this single renderer
/// instance.
#[test]
fn cached_path_grow_uncovered_region_takes_current_clear_not_zero_init() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let green = oriterm_core::Rgb { r: 0, g: 255, b: 0 };
    let red = oriterm_core::Rgb { r: 255, g: 0, b: 0 };

    // Frame 1: small render with green clear color.
    let _ = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        800,
        600,
        green,
        1.0,
    );

    // Frame 2: larger render with red clear color. The uncovered region
    // must be red — NOT green (prior frame), NOT zero (lazy zero-init).
    let (pixels, w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        1200,
        800,
        red,
        1.0,
    );
    let outside = pixel_rgba_at(&pixels, 1100, 700, w);
    assert!(
        outside[0] > 200 && outside[1] < 32 && outside[2] < 32,
        "uncovered pixel at (1100,700) must be RED (current clear), \
         got {outside:?} — green ({:?}) or zero would indicate the \
         uncovered region inherited stale state instead of current clear",
        [0, 255, 0, 255]
    );
}

/// Regression: BUG-06-052 — exercise the `needs_full_render=false` branch.
/// First frame renders full at `dst == vp` (populates content cache); second
/// frame requests `needs_full_render=false` at `dst > vp` so the cache is
/// REUSED and only the overlay/cursor buffers refresh. The clear pass +
/// partial copy must still fire correctly in the cache-reuse path —
/// pins the gate-correctness across the `if needs_full_render` branch.
#[test]
fn cached_path_grow_with_cache_reuse_clears_uncovered_region() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    // Frame 1: full render at dst == vp populates the cache.
    let cell = renderer.cell_metrics();
    let wl = compute_window_layout(800, 600, &cell, 1.0, true, 0.0, 0.0, 0.0);
    let mut input = FrameInput::test_grid(wl.cols, wl.rows, "frame1");
    input.viewport = ViewportSize::new(800, 600);
    input.cell_size = cell;
    input.content.cursor.visible = false;
    input.palette.background = bg;
    input.palette.opacity = 1.0;
    let origin = (wl.grid_rect.x(), wl.grid_rect.y());
    renderer.prepare(&input, &gpu, &pipelines, origin, 1.0, true);
    let _ = renderer.render_frame_cached(&gpu, &pipelines, 800, 600, true);

    // Frame 2: cache-reuse path at dst > vp. The partial-copy + clear
    // must still apply correctly.
    let target = renderer.render_frame_cached(&gpu, &pipelines, 1200, 800, false);
    let pixels = gpu
        .read_render_target(&target)
        .expect("readback should succeed");
    let outside = pixel_rgba_at(&pixels, 1100, 700, 1200);
    assert!(
        rgba_approx_eq(outside, [128, 64, 200, 255], 4),
        "uncovered pixel at (1100,700) under cache-reuse path should be \
         mid-tone purple, got {outside:?}"
    );
}

/// Regression: BUG-06-052 — boundary: 1x1 destination smaller than 800x600
/// viewport — the `dst > vp` gate stays FALSE on both axes (1 < 800, 1 < 600)
/// so the pre-clear pass does NOT fire; the test exercises the SHRINK-clamp
/// path where `copy_texture_to_texture` writes the minimum extent (1×1)
/// without validation error. Distinct from the grow-clear regression
/// surface — pins the helper's common-path correctness under the
/// minimum non-zero destination size.
#[test]
fn cached_path_shrink_to_1x1_does_not_panic_and_writes_pixel() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };
    // dst smaller than vp on both axes — shrink path; copy clamps to 1x1.
    let (pixels, _w) = prepare_and_render_cached_with_clear(
        &gpu,
        &pipelines,
        &mut renderer,
        800,
        600,
        1,
        1,
        bg,
        1.0,
    );
    assert_eq!(pixels.len(), 4, "1x1 readback must be exactly 4 bytes");
    // The single pixel must be non-zero — proves the cache actually
    // wrote content through to the 1x1 destination, not a no-op render.
    assert!(
        pixels[0] != 0 || pixels[1] != 0 || pixels[2] != 0 || pixels[3] != 0,
        "1x1 pixel must be non-zero, got {pixels:?}"
    );
}

/// Regression: BUG-06-052 — rapid alternation: grow/shrink cycles, uncovered
/// region matches current clear across the resize stream.
#[test]
fn cached_path_rapid_grow_shrink_alternation_clears_each_grow_frame() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };
    let bg = oriterm_core::Rgb {
        r: 128,
        g: 64,
        b: 200,
    };

    for i in 0..10 {
        let target_w = if i % 2 == 0 { 1200 } else { 600 };
        let target_h = if i % 2 == 0 { 800 } else { 400 };
        let (pixels, w) = prepare_and_render_cached_with_clear(
            &gpu,
            &pipelines,
            &mut renderer,
            800,
            600,
            target_w,
            target_h,
            bg,
            1.0,
        );
        if target_w > 800 && target_h > 600 {
            let outside = pixel_rgba_at(&pixels, 1100, 700, w);
            assert!(
                rgba_approx_eq(outside, [128, 64, 200, 255], 4),
                "iter {i}: uncovered pixel at (1100,700) should be mid-tone purple (128,64,200,255), got {outside:?}"
            );
        }
    }
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
