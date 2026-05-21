//! §13.6.1 items 14, 15, 17 — cure-confirmation gates.
//!
//! These tests are NOT `#[ignore]`-gated. They run in the normal
//! `./test-all.sh` path (which enables `oriterm/gpu-tests`). Their role
//! is to clamp the cure for the kitty quad → texture pipeline gap:
//!
//! - **Item 14**: `kitty_image_renders_visible_red_pixel_at_cell_zero_zero`
//! — regression guard (cell-pinned, not region-scan). Passes ONLY when the
//! kitty image actually paints red pixels at cell (0,0). Permanent
//! regression guard against future re-breakage AND against placement-
//! drift regressions (the region-anchored alternative passes when a
//! stray red pixel sits ANYWHERE in the search rectangle).
//!
//! - **Item 15**: `kitty_image_and_text_glyph_both_visible_in_same_cell_under_premul_alpha_blend`
//! — clamps the `PREMUL_ALPHA_BLEND` contract. The blend-bypass
//! diagnostic feature flag (`gpu-debug-image-blend-off`) provides a
//! `blend: None` pipeline variant that would falsely appear to "fix"
//! the bug while breaking text-over-image compositing. This pairing
//! test ensures the cure preserves the production blend contract.
//!
//! - **Item 17**: `kitty_image_pipeline_does_not_silently_drop_quads`
//! — negative-case guard. Catches the failure mode where the cure regresses
//! to "draw fires but does nothing" by asserting the diagnostic
//! surfaces (prepared quads + texture upload) stay populated.
//!
//! Plan body cross-reference:
//! §13.6.1` items
//! 14, 15, 17 + success criterion lines 549-554.

use base64::Engine;
use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::{
 FrameInputExpectation, GpuInstanceExpectation, ScenarioExpectations,
};

use super::super::visual_harness::VisualSpecHarness;

/// Encode the kitty bytes for a single-cell solid-red placement at the
/// current cursor position via TransmitAndPlace (`a=T`) with the given
/// alpha channel. `q=2` suppresses replies (this test path pins
/// rendering, not reply emission).
fn kitty_solid_color_1x1(alpha: u8, image_id: u32) -> Vec<u8> {
 let payload = [0xFFu8, 0x00, 0x00, alpha];
 let b64 = base64::engine::general_purpose::STANDARD.encode(payload);
 format!("\x1b_Ga=T,i={image_id},f=32,s=1,v=1,q=2;{b64}\x1b\\").into_bytes()
}

/// Render the harness's current state through the visual rung chain.
/// Returns `(pixels, width, height)` from the rendered output.
fn drive_render(harness: &mut VisualSpecHarness, catalog_row: &str) -> (Vec<u8>, u32, u32) {
 let expectations = ScenarioExpectations {
 frame_input: Some(FrameInputExpectation::default_grid()),
 gpu_instance: Some(GpuInstanceExpectation::at_least(0, 0).with_images(1)),
 ..ScenarioExpectations::default()
 };
 let _results = harness.render_visual_rungs(catalog_row, &expectations);
 let (slice, w, h) = harness
 .last_rendered_pixels()
 .expect("render_visual_rungs must populate last_rendered_pixels");
 (slice.to_vec(), w, h)
}

/// Predicate: pixel is "red-ish" — high red, low green, low blue.
/// Tolerance matches the §13.6.1 cure-confirmation contract per plan
/// body line 537 (`r > 200 && g < 50 && b < 50`).
fn is_red_ish(pixels: &[u8], off: usize) -> bool {
 if off + 4 > pixels.len() {
 return false;
 }
 pixels[off] > 200 && pixels[off + 1] < 50 && pixels[off + 2] < 50
}

/// Predicate: pixel is "white-ish" — high red, green, and blue. Used
/// for the text-glyph half of the pairing assertion.
fn is_white_ish(pixels: &[u8], off: usize) -> bool {
 if off + 4 > pixels.len() {
 return false;
 }
 pixels[off] > 200 && pixels[off + 1] > 200 && pixels[off + 2] > 200
}

/// Resolve cell (col, row) to pixel coordinates via the harness's
/// renderer cell metrics — the SSOT from §05.
fn cell_to_pixel(harness: &VisualSpecHarness, col: usize, row: usize) -> (usize, usize) {
 let cell = harness.renderer().cell_metrics();
 let px_x = (cell.width * col as f32) as usize;
 let px_y = (cell.height * row as f32) as usize;
 (px_x, px_y)
}

/// §13.6.1 item 14 — cure-confirmation gate, cell-pinned regression guard.
/// Feeds an opaque-red 1×1 kitty image at cell (0,0) via TransmitAndPlace.
/// Resolves cell (0,0) coordinates via `FontCollection::cell_metrics()`
/// (the SSOT from §05) and asserts that the rendered output contains
/// red pixels at the exact cell-(0,0) sub-region.
/// **Cell-pinned vs region-scan**: a region-anchored assertion over a
/// 16×8 rectangle passes when a stray red pixel exists ANYWHERE in
/// that area, even if the image rendered at the wrong cell. Cell-pinned
/// clamps both presence AND placement. This is the load-bearing guard for
/// the cure: it ONLY passes when the kitty image actually paints red
/// AT THE CORRECT CELL.
#[test]
fn kitty_image_renders_visible_red_pixel_at_cell_zero_zero() {
 let Some(mut harness) = VisualSpecHarness::new() else {
 eprintln!("SKIP: software rasterizer unavailable");
 return;
 };

 let bytes = kitty_solid_color_1x1(0xFF, 1);
 harness.core_mut().feed(&bytes);

 let (pixels, w, h) = drive_render(&mut harness, "KG-CURE-CELL-PINNED");

 let (px_x, px_y) = cell_to_pixel(&harness, 0, 0);
 let cell = harness.renderer().cell_metrics();
 let cell_w = cell.width as usize;
 let cell_h = cell.height as usize;

 // Count red pixels inside cell (0,0) ⊂ [px_x..px_x+cell_w, px_y..px_y+cell_h].
 let mut red_in_cell = 0usize;
 let mut sample_colors = Vec::new();
 for y in px_y..(px_y + cell_h).min(h as usize) {
 for x in px_x..(px_x + cell_w).min(w as usize) {
 let off = (y * w as usize + x) * 4;
 if is_red_ish(&pixels, off) {
 red_in_cell += 1;
 } else if sample_colors.len() < 4 && off + 4 <= pixels.len() {
 sample_colors.push((
 x,
 y,
 pixels[off],
 pixels[off + 1],
 pixels[off + 2],
 pixels[off + 3],
 ));
 }
 }
 }

 assert!(
 red_in_cell >= 4,
 "§13.6.1 item 14 (cure-confirmation gate): cell (0,0) contains \
 {red_in_cell} red-ish pixels (r>200,g<50,b<50); expected ≥4 \
 inside the {cell_w}×{cell_h} cell region at pixel (px_x={px_x}, \
 px_y={py}). The kitty image is not painting visible pixels at \
 the correct cell. Non-red sample colors from the cell: {samples:?}. \
 Cure status: image bytes flow through Parser→cache→snapshot→prepare→\
 record_image_draws AND reach the content_cache_view (per probe 4 — \
 128 red pixels in cache view), but the final readback is dark or \
 clobbered. Cure surface candidates: cursor pass overdraws the image \
 cell, cell BG draws AFTER image, copy_cache_to_output stage truncates \
 to a clip rect that excludes (0,0), or the harness's read_render_target \
 reads from the wrong attachment.",
 py = px_y,
 samples = sample_colors,
 );
}

/// §13.6.1 item 15 — blend-bypass success-trap pairing.
/// Feeds a semi-transparent (alpha=128) red 1×1 kitty image AT cell
/// (0,0), then writes a white 'X' text glyph also AT cell (0,0). The
/// rendered output must show BOTH the image's red contribution AND
/// the text glyph's white pixels — the `PREMUL_ALPHA_BLEND` contract
/// produces a blended composite.
/// **Why this matters**: the
/// `gpu-debug-image-blend-off` diagnostic feature provides a
/// `blend: None` pipeline variant where direct writes succeed but
/// text-over-image compositing breaks. The shortest path to making
/// item 14 green would be to ship the no-blend variant in production,
/// which would FALSELY appear to fix the bug while breaking
/// compositing. This pairing test CLAMPS the cure to the
/// PREMUL_ALPHA_BLEND contract.
#[test]
fn kitty_image_and_text_glyph_both_visible_in_same_cell_under_premul_alpha_blend() {
 let Some(mut harness) = VisualSpecHarness::new() else {
 eprintln!("SKIP: software rasterizer unavailable");
 return;
 };

 // Write 'X' at cursor (which starts at cell (0,0)).
 // After writing, cursor advances to (0,1).
 harness.core_mut().feed(b"X");
 // Move cursor back to home (0,0) via CSI H so the kitty image
 // places at the SAME cell as the 'X'.
 harness.core_mut().feed(b"\x1b[H");
 // Place a semi-transparent red 1×1 image at cell (0,0).
 let bytes = kitty_solid_color_1x1(0x80, 2);
 harness.core_mut().feed(&bytes);

 let (pixels, w, h) = drive_render(&mut harness, "KG-CURE-BLEND-PAIRING");

 let (px_x, px_y) = cell_to_pixel(&harness, 0, 0);
 let cell = harness.renderer().cell_metrics();
 let cell_w = cell.width as usize;
 let cell_h = cell.height as usize;

 let mut red_count = 0usize;
 let mut white_count = 0usize;
 for y in px_y..(px_y + cell_h).min(h as usize) {
 for x in px_x..(px_x + cell_w).min(w as usize) {
 let off = (y * w as usize + x) * 4;
 // For semi-transparent red over white background, the blended
 // pixel has reduced red and reduced green/blue — accept a
 // looser red predicate here.
 if off + 4 <= pixels.len() {
 let r = pixels[off];
 let g = pixels[off + 1];
 let b = pixels[off + 2];
 if r > 100 && g < 100 && b < 100 {
 red_count += 1;
 }
 if is_white_ish(&pixels, off) {
 white_count += 1;
 }
 }
 }
 }

 assert!(
 red_count >= 1 && white_count >= 1,
 "§13.6.1 item 15 (blend-pairing): cell (0,0) must contain BOTH \
 the semi-transparent image tint AND the text glyph after \
 PREMUL_ALPHA_BLEND compositing — saw {red_count} red-tinted \
 pixels + {white_count} white pixels. If white_count == 0, the \
 image is overdrawing the text opaquely (production switched \
 to blend: None — banned). If red_count == 0, the image is not \
 rendering at all (item 14's bug). Both must be > 0 to confirm \
 the cure preserves the production blend contract.",
 );
}

/// §13.6.1 item 17 — silent-drop negative-case guard.
/// Catches the failure mode where the cure regresses to "draw fires but
/// does nothing" by ensuring the diagnostic surfaces (prepared quads +
/// texture upload) stay populated even if pixels later regress.
#[test]
fn kitty_image_pipeline_does_not_silently_drop_quads() {
 let Some(mut harness) = VisualSpecHarness::new() else {
 eprintln!("SKIP: software rasterizer unavailable");
 return;
 };

 let bytes = kitty_solid_color_1x1(0xFF, 3);
 harness.core_mut().feed(&bytes);

 let _ = drive_render(&mut harness, "KG-CURE-NO-SILENT-DROP");

 let (below, above) = harness.prepared_image_quad_counts();
 assert!(
 above >= 1,
 "§13.6.1 item 17 (silent-drop guard): prepared.image_quads_above \
 must contain at least 1 quad after a successful kitty placement; \
 got below={below}, above={above}. Zero quads = the placement was \
 silently dropped between snapshot and prepare; downstream cure \
 work is pointless because there is no image to render."
 );

 // Texture upload verification — the cached texture for ImageId(3)
 // must exist after prepare runs.
 let device = harness.gpu().device_for_test();
 let queue = harness.gpu().queue_for_test();
 let tex = harness
 .renderer()
 .image_texture_cache_for_test()
 .read_texture_for_test(ImageId::from_raw(3), device, queue);
 assert!(
 tex.is_some(),
 "§13.6.1 item 17 (silent-drop guard): ImageId(3)'s texture must \
 be uploaded to the GPU after prepare runs. None = the image's \
 RGBA bytes never reached GPU memory; downstream cure work is \
 pointless."
 );
}
