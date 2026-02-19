//! Multi-size and multi-DPI visual regression tests.
//!
//! Validates that the rendering pipeline handles different font sizes and
//! display densities correctly. Catches size-dependent hinting, rounding,
//! and metric calculation issues.

use crate::gpu::frame_input::{FrameInput, ViewportSize};

use super::{compare_with_reference, headless_env_with_config, render_to_pixels};

#[test]
fn ascii_multi_size() {
    let text = "The quick brown fox";
    let cols = text.len();
    let rows = 1;

    for size_pt in [10.0f32, 14.0, 20.0] {
        let Some((gpu, mut renderer)) = headless_env_with_config(size_pt, 96.0) else {
            eprintln!("skipped: no GPU adapter available");
            return;
        };

        let cell = renderer.cell_metrics();
        let w = (cell.width * cols as f32).ceil() as u32;
        let h = (cell.height * rows as f32).ceil() as u32;

        let mut input = FrameInput::test_grid(cols, rows, text);
        input.viewport = ViewportSize::new(w, h);
        input.cell_size = cell;
        input.content.cursor.visible = false;

        let name = format!("ascii_{size_pt:.0}pt_96dpi");
        let pixels = render_to_pixels(&gpu, &mut renderer, &input);
        if let Err(msg) = compare_with_reference(&name, &pixels, w, h) {
            panic!("visual regression ({name}): {msg}");
        }
    }
}

#[test]
fn ascii_multi_dpi() {
    let text = "The quick brown fox";
    let cols = text.len();
    let rows = 1;

    for dpi in [96.0f32, 192.0] {
        let Some((gpu, mut renderer)) = headless_env_with_config(14.0, dpi) else {
            eprintln!("skipped: no GPU adapter available");
            return;
        };

        let cell = renderer.cell_metrics();
        let w = (cell.width * cols as f32).ceil() as u32;
        let h = (cell.height * rows as f32).ceil() as u32;

        let mut input = FrameInput::test_grid(cols, rows, text);
        input.viewport = ViewportSize::new(w, h);
        input.cell_size = cell;
        input.content.cursor.visible = false;

        let name = format!("ascii_14pt_{dpi:.0}dpi");
        let pixels = render_to_pixels(&gpu, &mut renderer, &input);
        if let Err(msg) = compare_with_reference(&name, &pixels, w, h) {
            panic!("visual regression ({name}): {msg}");
        }
    }
}
