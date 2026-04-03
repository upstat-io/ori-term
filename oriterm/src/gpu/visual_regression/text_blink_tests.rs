//! GPU text blink tests — frame-by-frame verification using the real
//! `CursorBlink` timer.
//!
//! Simulates a full blink cycle at 60fps by backdating the timer's epoch,
//! rendering each frame through the GPU pipeline, and asserting that BLINK
//! cell brightness follows the expected pattern: visible plateau → smooth
//! fade-out → hidden plateau → smooth fade-in.

use std::time::Duration;

use oriterm_core::CellFlags;
use oriterm_ui::animation::CursorBlink;

use crate::gpu::frame_input::FrameInput;

use super::{headless_env, render_to_pixels};

/// Grid dimensions for text blink tests.
const COLS: usize = 10;
const ROWS: usize = 3;

/// Column of the BLINK cell.
const BLINK_COL: usize = 0;
/// Column of the non-BLINK reference cell.
const NORMAL_COL: usize = 5;

/// Build a test frame with one BLINK cell at col 0 and one normal cell at col 5.
fn blink_input(cell: crate::font::CellMetrics, text_blink_opacity: f32) -> FrameInput {
    use crate::gpu::frame_input::ViewportSize;

    let w = (cell.width * COLS as f32).ceil() as u32;
    let h = (cell.height * ROWS as f32).ceil() as u32;

    let mut input = FrameInput::test_grid(COLS, ROWS, "");
    input.viewport = ViewportSize::new(w, h);
    input.cell_size = cell;
    input.text_blink_opacity = text_blink_opacity;
    input.content.cursor.visible = false;

    // Place 'A' in the BLINK cell and the normal cell.
    input.content.cells[BLINK_COL].ch = 'A';
    input.content.cells[BLINK_COL].flags = CellFlags::BLINK;
    input.content.cells[NORMAL_COL].ch = 'A';

    input
}

/// Extract the RGBA value at the center of a cell.
fn cell_pixel(pixels: &[u8], width: u32, col: usize, cell_w: f32, cell_h: f32) -> [u8; 4] {
    let cx = (col as f32 * cell_w + cell_w / 2.0) as u32;
    let cy = (cell_h / 2.0) as u32;
    let idx = ((cy * width + cx) * 4) as usize;
    [
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    ]
}

/// Frame-by-frame blink cycle test using the real `CursorBlink` timer.
///
/// Steps through one full blink cycle (1060ms at 500ms interval) at ~60fps
/// (16ms steps = ~66 frames). For each frame:
/// 1. Backdate the timer epoch to simulate elapsed time.
/// 2. Read `intensity()` — the actual value the runtime would use.
/// 3. Render through the full GPU pipeline with that opacity.
/// 4. Measure BLINK cell brightness.
///
/// Then assert the brightness sequence matches the expected blink pattern:
/// - Visible plateau: brightness near max, stable.
/// - Fade-out: brightness decreases smoothly frame by frame.
/// - Hidden plateau: brightness near zero, stable.
/// - Fade-in: brightness increases smoothly frame by frame.
/// - Non-BLINK cell: constant brightness throughout.
#[test]
fn text_blink_full_cycle_frame_by_frame() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let interval = Duration::from_millis(500);
    let frame_dt = Duration::from_millis(16);
    let cycle = interval * 2; // 1000ms full cycle.
    let num_frames = (cycle.as_millis() / frame_dt.as_millis()) as usize;

    let mut blink_br: Vec<u32> = Vec::with_capacity(num_frames);
    let mut normal_br: Vec<u32> = Vec::with_capacity(num_frames);
    let mut opacities: Vec<f32> = Vec::with_capacity(num_frames);

    for frame_idx in 0..num_frames {
        let elapsed = frame_dt * frame_idx as u32;

        // Create a fresh timer and backdate it to simulate elapsed time.
        let mut timer = CursorBlink::new(interval);
        timer.backdate(elapsed);
        let opacity = timer.intensity();
        opacities.push(opacity);

        // Render through the GPU pipeline.
        let input = blink_input(cell, opacity);
        let w = input.viewport.width;
        let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);

        let blink_px = cell_pixel(&pixels, w, BLINK_COL, cell.width, cell.height);
        let normal_px = cell_pixel(&pixels, w, NORMAL_COL, cell.width, cell.height);

        blink_br.push(blink_px[0] as u32 + blink_px[1] as u32 + blink_px[2] as u32);
        normal_br.push(normal_px[0] as u32 + normal_px[1] as u32 + normal_px[2] as u32);
    }

    // --- Assertions ---

    // 1. Non-BLINK cell must be constant across ALL frames.
    let normal_ref = normal_br[0];
    for (i, &br) in normal_br.iter().enumerate() {
        let diff = (br as i32 - normal_ref as i32).abs();
        assert!(
            diff < 5,
            "non-BLINK cell must be constant: frame {i} br={br} vs frame 0 br={normal_ref}",
        );
    }

    // 2. BLINK cell must reach full brightness (visible plateau) —
    //    should match the non-BLINK cell.
    let max_br = *blink_br.iter().max().unwrap();
    assert!(
        (max_br as i32 - normal_ref as i32).abs() < 10,
        "BLINK cell at full opacity should match normal cell: max={max_br} normal={normal_ref}",
    );

    // 3. BLINK cell at hidden plateau must be near the cell background
    //    brightness (not the glyph). Cell bg is RGB(30,30,46) = brightness 106.
    let min_br = *blink_br.iter().min().unwrap();
    let bg_brightness = 30 + 30 + 46; // test_grid cell bg
    assert!(
        (min_br as i32 - bg_brightness).abs() < 15,
        "BLINK cell at opacity 0 should match cell bg ({bg_brightness}): got {min_br}",
    );

    // 4. The dynamic range must span from near-normal to near-background.
    let range = max_br as i32 - min_br as i32;
    assert!(
        range > 150,
        "BLINK cell brightness range too small: {max_br} → {min_br} (range={range}). \
         Blink is not actually changing the glyph visibility.",
    );

    // 5. Opacity must hit both extremes.
    let max_opacity = opacities.iter().copied().fold(0.0_f32, f32::max);
    let min_opacity = opacities.iter().copied().fold(1.0_f32, f32::min);
    assert!(
        (max_opacity - 1.0).abs() < 0.01,
        "timer never reached opacity 1.0: max={max_opacity}",
    );
    assert!(
        min_opacity < 0.01,
        "timer never reached opacity 0.0: min={min_opacity}",
    );

    // 6. Opacity must have intermediate values (proving fade, not binary).
    let has_intermediate = opacities.iter().any(|&o| o > 0.1 && o < 0.9);
    assert!(
        has_intermediate,
        "no intermediate opacity values found — blink is binary, not fading. \
         opacities: {opacities:?}",
    );

    // 7. During fade-out, brightness must decrease monotonically frame by frame.
    let fade_out_start = opacities.iter().position(|&o| o < 0.95).unwrap();
    let fade_out_end = opacities.iter().position(|&o| o < 0.05).unwrap();
    if fade_out_end > fade_out_start + 2 {
        for i in fade_out_start..fade_out_end - 1 {
            assert!(
                blink_br[i] >= blink_br[i + 1],
                "fade-out not monotonic: frame {i} br={} > frame {} br={}",
                blink_br[i],
                i + 1,
                blink_br[i + 1],
            );
        }
    }

    // 8. During fade-in, brightness must increase monotonically.
    let fade_in_start = opacities.iter().rposition(|&o| o < 0.05).unwrap();
    let fade_in_end = opacities.iter().rposition(|&o| o > 0.95).unwrap();
    if fade_in_end > fade_in_start + 2 {
        for i in fade_in_start..fade_in_end - 1 {
            assert!(
                blink_br[i] <= blink_br[i + 1],
                "fade-in not monotonic: frame {i} br={} < frame {} br={}",
                blink_br[i],
                i + 1,
                blink_br[i + 1],
            );
        }
    }
}

/// Verify the fast path (`content_changed = false`) correctly rebuilds
/// instances when `text_blink_opacity` changes between frames.
///
/// Uses the real `CursorBlink` timer at two time points (visible plateau
/// and hidden plateau) to get opacity values, then renders through
/// `WindowRenderer::prepare()` with `content_changed = false` on the
/// second frame. The BLINK cell must be dimmer in frame 2.
#[test]
fn text_blink_fast_path_with_timer() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();
    let interval = Duration::from_millis(500);

    // Frame 1: visible plateau (elapsed = 0ms, opacity = 1.0).
    let timer_visible = CursorBlink::new(interval);
    let opacity_visible = timer_visible.intensity();
    assert!(
        opacity_visible > 0.99,
        "fresh timer should be at full opacity: {opacity_visible}",
    );

    let input1 = blink_input(cell, opacity_visible);
    let w = input1.viewport.width;
    let h = input1.viewport.height;
    let target1 = gpu.create_render_target(w, h);
    renderer.prepare(&input1, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    renderer.render_frame(&gpu, &pipelines, target1.view());
    let pixels1 = gpu.read_render_target(&target1).expect("readback");

    // Frame 2: hidden plateau (elapsed = 600ms, opacity ≈ 0.0).
    let mut timer_hidden = CursorBlink::new(interval);
    timer_hidden.backdate(Duration::from_millis(600));
    let opacity_hidden = timer_hidden.intensity();
    assert!(
        opacity_hidden < 0.05,
        "timer at 600ms should be near zero: {opacity_hidden}",
    );

    let input2 = blink_input(cell, opacity_hidden);
    let target2 = gpu.create_render_target(w, h);
    // content_changed = false: this is the fast path.
    renderer.prepare(&input2, &gpu, &pipelines, (0.0, 0.0), 1.0, false);
    renderer.render_frame(&gpu, &pipelines, target2.view());
    let pixels2 = gpu.read_render_target(&target2).expect("readback");

    // BLINK cell must be bright in frame 1, dark in frame 2.
    let br1 = cell_pixel(&pixels1, w, BLINK_COL, cell.width, cell.height);
    let br2 = cell_pixel(&pixels2, w, BLINK_COL, cell.width, cell.height);
    let bright1: u32 = br1[0] as u32 + br1[1] as u32 + br1[2] as u32;
    let bright2: u32 = br2[0] as u32 + br2[1] as u32 + br2[2] as u32;

    assert!(
        bright1 > bright2 + 50,
        "BLINK cell must dim through fast path: visible={bright1} hidden={bright2}",
    );

    // Non-BLINK cell must be constant.
    let n1 = cell_pixel(&pixels1, w, NORMAL_COL, cell.width, cell.height);
    let n2 = cell_pixel(&pixels2, w, NORMAL_COL, cell.width, cell.height);
    let diff: i32 = (0..3).map(|i| (n1[i] as i32 - n2[i] as i32).abs()).sum();
    assert!(
        diff < 5,
        "non-BLINK cell must be constant: {n1:?} vs {n2:?}",
    );
}
