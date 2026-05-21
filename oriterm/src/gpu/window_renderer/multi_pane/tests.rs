//! GPU-gated multi-pane chrome tests.
//!
//! These tests call the real production methods on a `WindowRenderer`
//! constructed via `headless_env()` and verify instance buffer counts.

#![cfg(feature = "gpu-tests")]

use std::sync::Arc;

use crate::gpu::ViewportSize;
use crate::gpu::frame_input::FrameInput;
use crate::gpu::prepared_frame::PreparedFrame;
use crate::gpu::visual_regression::headless_env;
use crate::session::compute::DividerLayout;
use crate::session::rect::Rect;
use crate::session::split_tree::SplitDirection;
use oriterm_core::Rgb;
use oriterm_core::image::ImageId;
use oriterm_mux::PaneId;

/// Helper: construct a test `DividerLayout`.
fn test_divider(x: f32, y: f32, w: f32, h: f32) -> DividerLayout {
    DividerLayout {
        rect: Rect {
            x,
            y,
            width: w,
            height: h,
        },
        direction: SplitDirection::Horizontal,
        pane_before: PaneId::from_raw(0),
        pane_after: PaneId::from_raw(1),
    }
}

#[test]
fn divider_empty_list_pushes_nothing() {
    let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

    let color = Rgb {
        r: 42,
        g: 42,
        b: 54,
    };
    let hover = Rgb {
        r: 109,
        g: 155,
        b: 224,
    };
    renderer.append_dividers(&[], color, hover, None);

    assert_eq!(renderer.prepared.backgrounds.len(), 0);
}

#[test]
fn divider_multiple_only_one_hovered() {
    let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

    let color = Rgb {
        r: 42,
        g: 42,
        b: 54,
    };
    let hover_color = Rgb {
        r: 109,
        g: 155,
        b: 224,
    };
    let d1 = test_divider(100.0, 0.0, 2.0, 600.0);
    let d2 = test_divider(300.0, 0.0, 2.0, 600.0);
    let d3 = test_divider(500.0, 0.0, 2.0, 600.0);
    renderer.append_dividers(&[d1, d2, d3], color, hover_color, Some(d2));

    assert_eq!(renderer.prepared.backgrounds.len(), 3);
}

#[test]
fn focus_border_pushes_four_rects() {
    let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

    let color = Rgb {
        r: 109,
        g: 155,
        b: 224,
    };
    let rect = Rect {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 150.0,
    };
    renderer.append_focus_border(&rect, color, 2.0);

    assert_eq!(renderer.prepared.cursors.len(), 4);
}

#[test]
fn focus_border_scaled_width() {
    let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

    let color = Rgb {
        r: 109,
        g: 155,
        b: 224,
    };
    let rect = Rect {
        x: 100.0,
        y: 100.0,
        width: 200.0,
        height: 150.0,
    };
    renderer.append_focus_border(&rect, color, 4.0); // 2x DPI

    assert_eq!(renderer.prepared.cursors.len(), 4);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn window_border_pushes_four_rects() {
    let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

    let color = Rgb {
        r: 58,
        g: 58,
        b: 72,
    };
    renderer.append_window_border(800, 600, color, 2.0);

    assert_eq!(renderer.prepared.cursors.len(), 4);
}

#[cfg(not(target_os = "macos"))]
#[test]
fn window_border_scaled() {
    let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);

    let color = Rgb {
        r: 58,
        g: 58,
        b: 72,
    };
    renderer.append_window_border(800, 600, color, 4.0); // 2x DPI

    assert_eq!(renderer.prepared.cursors.len(), 4);
}

// Helper: build a `FrameInput` carrying a single 2x2 RGBA image. The
// upload path (`ensure_pane_images_uploaded`) iterates
// `content.image_data` directly, so a matching `RenderablePlacement` is
// not required for the upload-side tests below.
fn input_with_image(image_id: ImageId) -> FrameInput {
    // Solid red 2x2 RGBA image (16 bytes).
    let pixels: Vec<u8> = vec![
        255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255,
    ];
    let mut input = FrameInput::test_grid(8, 4, "image-test");
    input
        .content
        .image_data
        .push(oriterm_core::RenderableImageData {
            id: image_id,
            data: Arc::new(pixels),
            width: 2,
            height: 2,
            pixel_generation: 0,
        });
    input
}

/// multi-pane prepare_pane_into was missing
/// Phase D (upload_image_textures), so kitty / sixel images extracted
/// from snapshots reached the frame but never reached the GPU.
#[test]
fn multi_pane_prepare_uploads_image_textures() {
    let (gpu, pipelines, mut renderer) = headless_env().expect("GPU available");
    let image_id = ImageId::from_raw(1);
    let input = input_with_image(image_id);
    let bg = Rgb { r: 0, g: 0, b: 0 };

    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
    let mut target = PreparedFrame::new(ViewportSize::new(800, 600), bg, 1.0);
    renderer.prepare_pane_into(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, &mut target);
    renderer.finish_multi_pane_frame();

    assert!(
        renderer
            .image_texture_cache_for_test()
            .get_bind_group(image_id)
            .is_some(),
        "multi-pane Phase D must upload image_data textures to GPU"
    );
}

/// image_texture_cache frame_counter must advance
/// exactly once per visual frame in multi-pane mode, regardless of pane
/// count. Naive per-pane upload would advance N times, tightening the
/// effective evict_unused retention window to THRESHOLD/pane_count.
#[test]
fn multi_pane_frame_counter_advances_once_per_frame_regardless_of_panes() {
    let (gpu, pipelines, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };

    let before = renderer.image_texture_cache_for_test().frame_counter();
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
    // Three panes — naive begin_frame-per-pane would advance counter to before+3.
    for i in 0..3 {
        let mut target = PreparedFrame::new(ViewportSize::new(800, 600), bg, 1.0);
        let input = input_with_image(ImageId::from_raw(10 + i));
        renderer.prepare_pane_into(&input, &gpu, &pipelines, (0.0, 0.0), 1.0, &mut target);
    }
    renderer.finish_multi_pane_frame();
    let after = renderer.image_texture_cache_for_test().frame_counter();

    assert_eq!(
        after - before,
        1,
        "image_texture_cache.frame_counter must advance exactly once per visual frame, \
 not N times where N = pane count"
    );
}

/// Regression: pane-cache invariant — a pane served from
/// PaneRenderCache skips prepare_pane_into. Without touch_cached_pane_images,
/// the image texture last_frame never advances and evict_unused(THRESHOLD)
/// drops it after THRESHOLD cached frames, leaving cached image quads with
/// None bind groups.
#[test]
fn touch_cached_pane_images_refreshes_last_frame() {
    use crate::gpu::prepared_frame::ImageQuad;

    let (gpu, pipelines, mut renderer) = headless_env().expect("GPU available");
    let image_id = ImageId::from_raw(1);
    let input = input_with_image(image_id);
    let bg = Rgb { r: 0, g: 0, b: 0 };

    // Frame 1: prepare into a target — the image gets uploaded to GPU.
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
    let mut cached_target = PreparedFrame::new(ViewportSize::new(800, 600), bg, 1.0);
    renderer.prepare_pane_into(
        &input,
        &gpu,
        &pipelines,
        (0.0, 0.0),
        1.0,
        &mut cached_target,
    );
    renderer.finish_multi_pane_frame();
    assert!(
        renderer
            .image_texture_cache_for_test()
            .get_bind_group(image_id)
            .is_some(),
        "first frame must upload the image"
    );

    // The fill_frame_shaped emitter requires a `RenderablePlacement` in
    // `content.images` to emit image quads — but for THIS test we only
    // care about the touch path. Inject an ImageQuad referencing the
    // same image_id directly so `touch_cached_pane_images` has work to do.
    cached_target.image_quads_above.push(ImageQuad {
        image_id,
        x: 0.0,
        y: 0.0,
        w: 2.0,
        h: 2.0,
        uv_x: 0.0,
        uv_y: 0.0,
        uv_w: 1.0,
        uv_h: 1.0,
        opacity: 1.0,
    });

    // Frames 2..N: simulate cache_hit — extend_from + touch_cached_pane_images,
    // NEVER re-running prepare_pane_into. Without touch, `last_frame` stales.
    // Run more than IMAGE_TEXTURE_EVICT_FRAME_THRESHOLD (60) frames.
    for _ in 0..70 {
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
        renderer.touch_cached_pane_images(&cached_target);
        renderer.finish_multi_pane_frame();
    }

    assert!(
        renderer
            .image_texture_cache_for_test()
            .get_bind_group(image_id)
            .is_some(),
        "image must remain in cache across pane-cache hits"
    );
}

/// touch_cached_pane_images returns false when
/// a referenced image has been evicted between cache write and now.
/// Caller must invalidate the pane cache and fall through to re-prepare.
#[test]
fn touch_cached_pane_images_returns_false_when_image_evicted() {
    use crate::gpu::prepared_frame::ImageQuad;

    let (_gpu, _pip, mut renderer) = headless_env().expect("GPU available");
    let bg = Rgb { r: 0, g: 0, b: 0 };
    // Construct a cached PreparedFrame with an ImageQuad whose image_id
    // is NOT in `image_texture_cache` (simulating eviction between the
    // cache write and the cache_hit touch call).
    let mut cached_target = PreparedFrame::new(ViewportSize::new(800, 600), bg, 1.0);
    cached_target.image_quads_above.push(ImageQuad {
        image_id: ImageId::from_raw(999),
        x: 0.0,
        y: 0.0,
        w: 2.0,
        h: 2.0,
        uv_x: 0.0,
        uv_y: 0.0,
        uv_w: 1.0,
        uv_h: 1.0,
        opacity: 1.0,
    });

    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
    let all_present = renderer.touch_cached_pane_images(&cached_target);
    renderer.finish_multi_pane_frame();

    assert!(
        !all_present,
        "touch_cached_pane_images must return false when any referenced image is missing — \
 caller relies on this to invalidate the pane cache and fall through to cache-miss"
    );
}

/// touch_cached_pane_images returns true when
/// every referenced image is present in the cache.
#[test]
fn touch_cached_pane_images_returns_true_when_all_present() {
    use crate::gpu::prepared_frame::ImageQuad;

    let (gpu, pipelines, mut renderer) = headless_env().expect("GPU available");
    let image_id = ImageId::from_raw(7);
    let input = input_with_image(image_id);
    let bg = Rgb { r: 0, g: 0, b: 0 };

    // Frame 1: upload via prepare_pane_into so the image lives in cache.
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
    let mut cached_target = PreparedFrame::new(ViewportSize::new(800, 600), bg, 1.0);
    renderer.prepare_pane_into(
        &input,
        &gpu,
        &pipelines,
        (0.0, 0.0),
        1.0,
        &mut cached_target,
    );
    cached_target.image_quads_above.push(ImageQuad {
        image_id,
        x: 0.0,
        y: 0.0,
        w: 2.0,
        h: 2.0,
        uv_x: 0.0,
        uv_y: 0.0,
        uv_w: 1.0,
        uv_h: 1.0,
        opacity: 1.0,
    });
    let all_present = renderer.touch_cached_pane_images(&cached_target);
    renderer.finish_multi_pane_frame();

    assert!(
        all_present,
        "touch_cached_pane_images must return true when every referenced image is in cache"
    );
}

/// Regression: pane-cache eviction without touch — WITHOUT the
/// touch call, the same scenario MUST result in the image being evicted.
/// Proves the touch is load-bearing, not coincidental.
#[test]
fn touch_cached_pane_images_negative_pin_eviction_without_touch() {
    use crate::gpu::prepared_frame::ImageQuad;

    let (gpu, pipelines, mut renderer) = headless_env().expect("GPU available");
    let image_id = ImageId::from_raw(2);
    let input = input_with_image(image_id);
    let bg = Rgb { r: 0, g: 0, b: 0 };

    // Frame 1: upload the image.
    renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
    let mut cached_target = PreparedFrame::new(ViewportSize::new(800, 600), bg, 1.0);
    renderer.prepare_pane_into(
        &input,
        &gpu,
        &pipelines,
        (0.0, 0.0),
        1.0,
        &mut cached_target,
    );
    renderer.finish_multi_pane_frame();
    cached_target.image_quads_above.push(ImageQuad {
        image_id,
        x: 0.0,
        y: 0.0,
        w: 2.0,
        h: 2.0,
        uv_x: 0.0,
        uv_y: 0.0,
        uv_w: 1.0,
        uv_h: 1.0,
        opacity: 1.0,
    });

    // 70 cache-hit frames WITHOUT touch — image must age out.
    for _ in 0..70 {
        renderer.begin_multi_pane_frame(ViewportSize::new(800, 600), bg, 1.0);
        // NOTE: deliberately omit renderer.touch_cached_pane_images(&cached_target);
        renderer.finish_multi_pane_frame();
    }

    assert!(
        renderer
            .image_texture_cache_for_test()
            .get_bind_group(image_id)
            .is_none(),
        "without touch_cached_pane_images, eviction must fire after THRESHOLD frames"
    );
}
