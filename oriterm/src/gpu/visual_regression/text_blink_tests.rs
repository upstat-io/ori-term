//! Frame-by-frame text blink golden image tests.
//!
//! Captures golden PNGs at key points in the blink cycle, then reads
//! them back and verifies brightness follows the expected pattern.
//! Each frame is rendered through the full GPU pipeline using opacity
//! values from the real `CursorBlink` timer.

use std::path::PathBuf;
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

/// Number of frames in one full blink cycle.
const CYCLE_FRAMES: usize = 62;

/// Frame interval (~60fps).
const FRAME_MS: u32 = 16;

/// Blink interval for tests.
const BLINK_INTERVAL: Duration = Duration::from_millis(500);

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

    input.content.cells[BLINK_COL].ch = 'A';
    input.content.cells[BLINK_COL].flags = CellFlags::BLINK;
    input.content.cells[NORMAL_COL].ch = 'A';

    input
}

/// Directory for blink frame golden images.
fn blink_golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/references/blink_frames")
}

/// Compute average RGB brightness across all pixels in a grid cell.
fn cell_brightness(pixels: &[u8], width: u32, col: usize, cw: f32, ch: f32) -> u32 {
    let x0 = (col as f32 * cw) as u32;
    let y0 = 0_u32; // Row 0 always.
    let x1 = ((col + 1) as f32 * cw).ceil() as u32;
    let y1 = ch.ceil() as u32;

    let mut total: u64 = 0;
    let mut count: u64 = 0;
    for py in y0..y1 {
        for px in x0..x1 {
            let idx = ((py * width + px) * 4) as usize;
            total += pixels[idx] as u64 + pixels[idx + 1] as u64 + pixels[idx + 2] as u64;
            count += 1;
        }
    }
    if count == 0 {
        0
    } else {
        (total / count) as u32
    }
}

/// Capture golden PNGs for every frame in one blink cycle.
///
/// Saves 62 PNGs to `tests/references/blink_frames/`. Each frame uses the
/// real `CursorBlink` timer at the corresponding elapsed time. Run with
/// `ORITERM_UPDATE_GOLDEN=1` to regenerate.
#[test]
fn generate_blink_cycle_golden_frames() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let dir = blink_golden_dir();
    std::fs::create_dir_all(&dir).expect("create blink_frames dir");

    let cell = renderer.cell_metrics();
    let update = std::env::var("ORITERM_UPDATE_GOLDEN").as_deref() == Ok("1");

    for frame_idx in 0..CYCLE_FRAMES {
        let elapsed = Duration::from_millis((frame_idx as u32 * FRAME_MS) as u64);
        let mut timer = CursorBlink::new(BLINK_INTERVAL);
        timer.backdate(elapsed);
        let opacity = timer.intensity();

        let input = blink_input(cell, opacity);
        let w = input.viewport.width;
        let h = input.viewport.height;
        let pixels = render_to_pixels(&gpu, &pipelines, &mut renderer, &input);

        let path = dir.join(format!("frame_{frame_idx:03}.png"));

        if update || !path.exists() {
            let img =
                image::RgbaImage::from_raw(w, h, pixels).expect("pixel buffer matches image dims");
            img.save(&path).expect("save golden frame PNG");
        }
    }
}

/// Read back the golden frame PNGs and verify the blink brightness sequence.
///
/// Asserts:
/// 1. All 62 golden PNGs exist (generate first with `ORITERM_UPDATE_GOLDEN=1`).
/// 2. BLINK cell reaches full brightness during visible plateau.
/// 3. BLINK cell reaches background brightness during hidden plateau.
/// 4. Fade-out is monotonically non-increasing.
/// 5. Fade-in is monotonically non-decreasing.
/// 6. Intermediate brightness values exist (smooth fade, not binary).
/// 7. Non-BLINK cell brightness is constant across all frames.
#[test]
fn verify_blink_cycle_golden_frames() {
    let dir = blink_golden_dir();

    // Derive cell dimensions from the first golden frame's image size.
    // Image was rendered as COLS x ROWS cells.
    let first = dir.join("frame_000.png");
    if !first.exists() {
        panic!(
            "golden frames missing at {}. Run generate_blink_cycle_golden_frames \
             with ORITERM_UPDATE_GOLDEN=1 first.",
            first.display()
        );
    }
    let first_img = image::open(&first).expect("read first frame").to_rgba8();
    let (img_w, img_h) = (first_img.width(), first_img.height());
    let cw = img_w as f32 / COLS as f32;
    let ch = img_h as f32 / ROWS as f32;

    let mut blink_br: Vec<u32> = Vec::new();
    let mut normal_br: Vec<u32> = Vec::new();
    let mut opacities: Vec<f32> = Vec::new();

    for frame_idx in 0..CYCLE_FRAMES {
        let path = dir.join(format!("frame_{frame_idx:03}.png"));
        assert!(
            path.exists(),
            "golden frame {frame_idx:03} missing at {}. Run with ORITERM_UPDATE_GOLDEN=1 first.",
            path.display()
        );

        let img =
            image::open(&path).unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let rgba = img.to_rgba8();
        let (w, _h) = rgba.dimensions();
        let pixels = rgba.as_raw();

        let b = cell_brightness(pixels, w, BLINK_COL, cw, ch);
        let n = cell_brightness(pixels, w, NORMAL_COL, cw, ch);
        blink_br.push(b);
        normal_br.push(n);

        // Compute expected opacity for logging.
        let elapsed = Duration::from_millis((frame_idx as u32 * FRAME_MS) as u64);
        let mut timer = CursorBlink::new(BLINK_INTERVAL);
        timer.backdate(elapsed);
        opacities.push(timer.intensity());
    }

    // --- Assertions ---

    // 1. Non-BLINK cell constant across all frames.
    let normal_ref = normal_br[0];
    assert!(
        normal_ref > 100,
        "normal cell should be visible: {normal_ref}"
    );
    for (i, &br) in normal_br.iter().enumerate() {
        let diff = (br as i32 - normal_ref as i32).abs();
        assert!(
            diff < 5,
            "non-BLINK cell must be constant: frame {i} br={br} vs frame 0 br={normal_ref}",
        );
    }

    // 2. BLINK cell reaches full brightness (visible plateau matches normal cell).
    let max_br = *blink_br.iter().max().unwrap();
    assert!(
        (max_br as i32 - normal_ref as i32).abs() < 10,
        "BLINK cell at full opacity should match normal cell: max={max_br} normal={normal_ref}",
    );

    // 3. BLINK cell reaches background brightness during hidden plateau.
    // Cell bg = RGB(30,30,46) → brightness 106.
    let min_br = *blink_br.iter().min().unwrap();
    let bg_brightness = 30 + 30 + 46;
    assert!(
        (min_br as i32 - bg_brightness).abs() < 15,
        "BLINK cell at opacity 0 should match cell bg ({bg_brightness}): got {min_br}",
    );

    // 4. Dynamic range: glyph brightness must drop by at least 40% of max.
    //    (PNG sRGB round-trip compresses the range vs raw GPU readback.)
    let range = max_br as i32 - min_br as i32;
    let min_range = (max_br as i32 * 40) / 100;
    assert!(
        range > min_range,
        "BLINK brightness range too small: {max_br} → {min_br} (range={range}, need >{min_range})",
    );

    // 5. Intermediate opacity values exist (proving fade, not binary on/off).
    let has_intermediate = opacities.iter().any(|&o| o > 0.1 && o < 0.9);
    assert!(
        has_intermediate,
        "no intermediate opacity values — blink is binary, not fading",
    );

    // 6. Intermediate brightness values exist in rendered output.
    let mid_threshold_low = min_br + (range as u32 / 4);
    let mid_threshold_high = max_br - (range as u32 / 4);
    let has_mid_brightness = blink_br
        .iter()
        .any(|&b| b > mid_threshold_low && b < mid_threshold_high);
    assert!(
        has_mid_brightness,
        "no intermediate brightness values in rendered frames — fade not reaching GPU. \
         range=[{min_br}..{max_br}], looking for values in [{mid_threshold_low}..{mid_threshold_high}]",
    );

    // 7. Fade-out monotonic: find frames where opacity drops from >0.95 to <0.05.
    let fade_out_start = opacities.iter().position(|&o| o < 0.95);
    let fade_out_end = opacities.iter().position(|&o| o < 0.05);
    if let (Some(start), Some(end)) = (fade_out_start, fade_out_end) {
        if end > start + 2 {
            for i in start..end - 1 {
                assert!(
                    blink_br[i] >= blink_br[i + 1],
                    "fade-out not monotonic: frame {i} br={} > frame {} br={}",
                    blink_br[i],
                    i + 1,
                    blink_br[i + 1],
                );
            }
        }
    }

    // 8. Fade-in monotonic: find frames where opacity rises from <0.05 to >0.95.
    let fade_in_start = opacities.iter().rposition(|&o| o < 0.05);
    let fade_in_end = opacities.iter().rposition(|&o| o > 0.95);
    if let (Some(start), Some(end)) = (fade_in_start, fade_in_end) {
        if end > start + 2 {
            for i in start..end - 1 {
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
}

/// Fast-path regression test: renders two frames through
/// `WindowRenderer::prepare()` with `content_changed = false` on frame 2.
/// Uses timer-derived opacities (visible plateau vs hidden plateau).
/// The BLINK cell must dim — proving `has_visual_change()` detects the
/// opacity delta and triggers a full instance rebuild.
#[test]
fn text_blink_fast_path_with_timer() {
    let Some((gpu, pipelines, mut renderer)) = headless_env() else {
        eprintln!("skipped: no GPU adapter available");
        return;
    };

    let cell = renderer.cell_metrics();

    // Frame 1: visible plateau (t=0, opacity=1.0), content_changed=true.
    let timer_vis = CursorBlink::new(BLINK_INTERVAL);
    let input1 = blink_input(cell, timer_vis.intensity());
    let w = input1.viewport.width;
    let h = input1.viewport.height;
    let t1 = gpu.create_render_target(w, h);
    renderer.prepare(&input1, &gpu, &pipelines, (0.0, 0.0), 1.0, true);
    renderer.render_frame(&gpu, &pipelines, t1.view());
    let px1 = gpu.read_render_target(&t1).expect("readback");

    // Frame 2: hidden plateau (t=600ms, opacity≈0.0), content_changed=false.
    let mut timer_hid = CursorBlink::new(BLINK_INTERVAL);
    timer_hid.backdate(Duration::from_millis(600));
    let input2 = blink_input(cell, timer_hid.intensity());
    let t2 = gpu.create_render_target(w, h);
    renderer.prepare(&input2, &gpu, &pipelines, (0.0, 0.0), 1.0, false);
    renderer.render_frame(&gpu, &pipelines, t2.view());
    let px2 = gpu.read_render_target(&t2).expect("readback");

    let br1 = cell_brightness(&px1, w, BLINK_COL, cell.width, cell.height);
    let br2 = cell_brightness(&px2, w, BLINK_COL, cell.width, cell.height);

    assert!(
        br1 > br2 + 50,
        "BLINK cell must dim through fast path: visible={br1} hidden={br2}. \
         If equal, has_visual_change() missed the opacity delta.",
    );

    let n1 = cell_brightness(&px1, w, NORMAL_COL, cell.width, cell.height);
    let n2 = cell_brightness(&px2, w, NORMAL_COL, cell.width, cell.height);
    let diff = (n1 as i32 - n2 as i32).abs();
    assert!(diff < 5, "non-BLINK cell must be constant: {n1} vs {n2}");
}
