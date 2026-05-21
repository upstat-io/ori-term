//! §13.6.1 z-order text-interleaving pilot — sixel below, text middle, kitty above.
//!
//! Catalog row: `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER` (shared with the
//! sibling pilot `kitty_sixel_mixed_z_order`; this pilot pins the
//! text-interleaving leg of the same row).
//! Apex: `GoldenImage` at
//! `oriterm/tests/references/kitty_sixel_mixed_with_text.png`.
//!
//! ## Scenario
//!
//! Sixel `z = -1` paints a solid-red 30×12-px region spanning cells
//! `(10..13, 5)` (3 cells horizontally at 10-px cell width). Text and
//! kitty are then layered on top to exercise the 3-layer composite:
//!
//! | Cell        | Sixel z=-1 | Text glyph | Kitty z=1 | Renders as       |
//! |-------------|------------|------------|-----------|------------------|
//! | `(10, 5)`   | red        | —          | —         | sixel red        |
//! | `(11, 5)`   | red        | 'X'        | —         | red + text-light |
//! | `(12, 5)`   | red        | —          | blue      | kitty blue       |
//! | `(13, 5)`   | —          | 'Y'        | —         | text on bg       |
//!
//! Each cell isolates one composite contract:
//! - Cell `(10, 5)` — sixel `image_quads_below` survives without
//!   anything on top: pins the below-pass producer.
//! - Cell `(11, 5)` — text glyph renders BETWEEN image layers: pins
//!   that the text pass runs after `image_quads_below` and before
//!   `image_quads_above`.
//! - Cell `(12, 5)` — kitty `image_quads_above` occludes sixel even
//!   when both target the same cell: pins the above-pass producer
//!   and the renderer order.
//! - Cell `(13, 5)` — text alone on background: control for the text
//!   pass independent of any image layer.
//!
//! ## Relationship to `kitty_sixel_mixed_z_order`
//!
//! The sibling pilot demonstrates same-cell occlusion (kitty fully
//! covering one cell, sixel surviving at adjacent cells) plus a single
//! text glyph at a separate cell. This pilot focuses on text
//! *interleaving* — text drawn ON sixel and kitty ON sixel — and pins
//! the contract that the text pass sits between the two image passes
//! in render order. The two pilots share `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER`
//! but produce distinct goldens, narrowing the failure mode if one
//! regresses.
//!
//! The companion unit tests `mixed_protocol_z_split_routes_by_z_index_not_image_id`,
//! `mixed_protocol_z_split_inverts_when_z_indices_swap`, and
//! `mixed_protocol_z_split_unaffected_by_text_content` at
//! `oriterm/src/gpu/prepare/tests.rs` pin the `emit_image_quads` split
//! deterministically, independent of GPU correctness — together the unit
//! tests and this golden bracket the bug surface.
//!
//! ## Sixel z-injection and cell-dimension sync
//!
//! See `kitty_sixel_mixed_z_order` rustdoc — both apply identically
//! here. The sixel DCS protocol has no `z=` field; the production
//! handler hard-codes `z_index: 0` (sixel `mod.rs`), so the pilot uses
//! `ImageCache::set_placement_z_index_for_test` to flip the
//! auto-assigned sixel placement to `z=-1`. `Term::set_cell_dimensions`
//! sync immediately after harness creation aligns image-placement pixel
//! math with the GPU renderer's font-derived cell metrics.

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::{
    FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Matches `oriterm_core::image::cache::AUTO_ID_START` (private). The
/// constant is duplicated here only for the mutator call — the live
/// guard is the bool return value of the mutator.
const AUTO_ID_START: u32 = 2_147_483_647;

/// CUP `row=5, col=10` (0-based) → `ESC [ 6 ; 11 H` (1-based).
const CUP_ROW5_COL10: &[u8] = b"\x1b[6;11H";

/// CUP `row=5, col=11` (0-based) → `ESC [ 6 ; 12 H` (1-based).
const CUP_ROW5_COL11: &[u8] = b"\x1b[6;12H";

/// CUP `row=5, col=12` (0-based) → `ESC [ 6 ; 13 H` (1-based).
const CUP_ROW5_COL12: &[u8] = b"\x1b[6;13H";

/// CUP `row=5, col=13` (0-based) → `ESC [ 6 ; 14 H` (1-based).
const CUP_ROW5_COL13: &[u8] = b"\x1b[6;14H";

/// Sixel DCS sequence painting a solid red 30×12-pixel region at the
/// cursor (3 cells × 1 cell tall at 10×22 cell metrics). Same payload
/// as `kitty_sixel_mixed_z_order::SIXEL_RED_WIDE` — keeping the byte
/// stream identical so both pilots exercise the same sixel parsing
/// path; only the layering above the sixel differs.
const SIXEL_RED_WIDE: &[u8] = b"\x1bPq#0;2;100;0;0#0!30~-#0!30~\x1b\\";

/// Kitty APC: 1×1 opaque blue RGBA pixel, transmit+place, `i=20`
/// (distinct from the sibling pilot's `i=10` so the two pilots never
/// collide on cached id state if they ever share a harness), `z=1`,
/// `q=2` (suppress replies).
///
/// Payload `AAD//w==` decodes to `0x00 0x00 0xFF 0xFF` — opaque blue.
const KITTY_BLUE_Z1: &[u8] = b"\x1b_Gf=32,s=1,v=1,a=T,i=20,z=1,q=2;AAD//w==\x1b\\";

#[test]
fn kitty_sixel_mixed_with_text_renders_three_layer_composition() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Sync Term cell metrics with the GPU renderer so image
    // placements and text positions use the same pixel grid.
    let cell = harness.renderer().cell_metrics();
    harness
        .core_mut()
        .term_mut()
        .set_cell_dimensions(cell.width as u16, cell.height as u16);

    // 1. Sixel red across cells (10..13, 5).
    harness.core_mut().feed(CUP_ROW5_COL10);
    harness.core_mut().feed(SIXEL_RED_WIDE);

    // 2. Text 'X' inside the sixel region (cell 11).
    harness.core_mut().feed(CUP_ROW5_COL11);
    harness.core_mut().feed(b"X");

    // 3. Kitty blue at cell 12 — same cell as sixel red so the
    //    above-pass occludes the below-pass at this cell.
    harness.core_mut().feed(CUP_ROW5_COL12);
    harness.core_mut().feed(KITTY_BLUE_Z1);

    // 4. Text 'Y' at cell 13 — outside the sixel region; pure text
    //    on background as a control.
    harness.core_mut().feed(CUP_ROW5_COL13);
    harness.core_mut().feed(b"Y");

    // 5. Sanity: cache holds exactly two placements.
    assert_eq!(
        harness
            .core_mut()
            .term_mut()
            .image_cache()
            .placement_count(),
        2,
        "expected 2 placements (sixel + kitty); cache state drift"
    );

    // 6. Flip sixel z to -1 (sixel handler hard-codes z=0).
    let mutated = harness
        .core_mut()
        .term_mut()
        .image_cache_mut()
        .set_placement_z_index_for_test(ImageId::from_raw(AUTO_ID_START), None, -1);
    assert!(
        mutated,
        "sixel placement (image_id=AUTO_ID_START, placement_id=None) \
         not found; sixel handler may have changed its id-assignment \
         path"
    );

    // 7. Render through the visual rung chain to GoldenImage apex.
    let expectations = ScenarioExpectations {
        frame_input: Some(FrameInputExpectation::default_grid()),
        gpu_instance: Some(GpuInstanceExpectation::at_least(0, 0).with_images(2)),
        texture: Some(TextureExpectation {
            min_non_zero_pixels: Some(1),
            width: None,
            height: None,
        }),
        golden: Some(GoldenExpectation {
            golden_name: Some("kitty_sixel_mixed_with_text"),
        }),
        ..ScenarioExpectations::default()
    };
    let results = harness.render_visual_rungs("KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER", &expectations);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    // 8. Pixel-level layering assertions, one per cell.
    let (pixels, w, _h) = harness
        .last_rendered_pixels()
        .map(|(p, w, h)| (p.to_vec(), w, h))
        .expect("render_visual_rungs must populate last_rendered_pixels");

    let cell = harness.renderer().cell_metrics();
    let cell_w = cell.width as usize;
    let cell_h = cell.height as usize;

    let count_at_cell = |col: usize, row: usize| -> (usize, usize, usize) {
        let px_x = (cell.width * col as f32) as usize;
        let px_y = (cell.height * row as f32) as usize;
        let mut blue = 0usize;
        let mut red = 0usize;
        let mut text_like = 0usize;
        for y in px_y..(px_y + cell_h) {
            for x in px_x..(px_x + cell_w) {
                let off = (y * w as usize + x) * 4;
                if off + 4 > pixels.len() {
                    continue;
                }
                let (r, g, b) = (pixels[off], pixels[off + 1], pixels[off + 2]);
                if b > 200 && r < 80 && g < 80 {
                    blue += 1;
                } else if r > 200 && g < 80 && b < 80 {
                    red += 1;
                } else if r > 150 && g > 150 && b > 150 {
                    text_like += 1;
                }
            }
        }
        (blue, red, text_like)
    };

    let (_, sixel_only_red, _) = count_at_cell(10, 5);
    let (_, sixel_under_text_red, text_over_sixel) = count_at_cell(11, 5);
    let (kitty_over_sixel_blue, _, _) = count_at_cell(12, 5);
    let (_, _, text_alone) = count_at_cell(13, 5);

    assert!(
        sixel_only_red >= 4,
        "cell (10, 5) — sixel z=-1 layer not visible: expected ≥4 red \
         pixels, found {sixel_only_red}. The sixel quad is not landing \
         in `image_quads_below`, or the below-pass is being clobbered."
    );
    assert!(
        sixel_under_text_red >= 4,
        "cell (11, 5) — sixel layer under text disappeared: expected ≥4 \
         red pixels surrounding the text glyph, found \
         {sixel_under_text_red}. The text pass may be opaque-clearing \
         past the glyph stroke (regression: text background should be \
         transparent over `image_quads_below`)."
    );
    assert!(
        text_over_sixel >= 1,
        "cell (11, 5) — text glyph 'X' not visible over sixel: expected \
         ≥1 light-gray pixel, found {text_over_sixel}. The text pass \
         is firing BEFORE `image_quads_below` (wrong renderer order) \
         or the glyph is being occluded by the below layer."
    );
    assert!(
        kitty_over_sixel_blue >= 4,
        "cell (12, 5) — kitty z=1 layer did not occlude sixel: expected \
         ≥4 blue pixels, found {kitty_over_sixel_blue}. The kitty quad \
         is not landing in `image_quads_above`, the renderer order is \
         drawing below AFTER above, or the z-index split misrouted the \
         kitty placement."
    );
    assert!(
        text_alone >= 1,
        "cell (13, 5) — text glyph 'Y' on background not visible: \
         expected ≥1 light-gray pixel, found {text_alone}. The text \
         pass is broken independently of the image layers."
    );
}
