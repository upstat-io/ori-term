//! §13.6 cross-stack rendering pilot — kitty + sixel mixed z-order.
//!
//! Catalog row: `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER`
//! Apex: `GoldenImage`
//!
//! Drives a 3-layer composite anchored at row=5, col=10:
//!   1. Sixel placement at `z = -1` — drawn BELOW text. Paints 30×12
//!      pixels (sub-cell vertically — only `12 / cell_h_px` ≈ 0.55 cell
//!      at the typical 22 px metric, NOT a full cell tall as the prior
//!      wording claimed). The sub-cell vertical extent is INTENTIONAL —
//!      assertions at `count_at_cell()` (lines 212-263) check per-cell
//!      pixel color counts with `≥4` thresholds, not full-cell coverage.
//!      Horizontal extent of 30 px is ≈3 cells wide at 10-px cell width,
//!      placing red into cells `(10..13, 5)` on the test box.
//!   2. Text glyph `T` — drawn between image layers. Written outside
//!      the kitty-covered cell (col=13) so it remains visible in the
//!      rendered output.
//!   3. Kitty placement at `z =  1` — drawn ABOVE text. Cell-snaps to
//!      a single cell `(10, 5)` (1×1 source → 1 cell display per
//!      `kitty/place.rs:59-64`), opaque blue.
//!
//! Cross-cell layout is required: a 1×1 kitty placement cell-snaps to
//! a full opaque cell, fully occluding whatever sixel painted at the
//! SAME cell. Spreading sixel across adjacent cells gives the test a
//! window into the `image_quads_below` layer that survives without
//! kitty on top. The golden image captures the full 3-layer composite
//! for `oriterm/tests/references/kitty_sixel_mixed_z_order.png`.
//!
//! ## Sixel z-injection — protocol-gap workaround
//!
//! The sixel DCS protocol has NO `z=` field; the production handler
//! at `oriterm_core/src/term/handler/image/sixel.rs:122` hard-codes
//! every sixel placement to `z_index: 0`. To exercise the
//! cross-protocol z-ordering pipeline the pilot:
//!
//! 1. Feeds real sixel DCS bytes through the production handler
//!    (preserving §13.6.1 line 542's architectural pin that
//!    cross-stack tests MUST drive real protocol bytes and NOT
//!    synthesize cache entries).
//! 2. Calls `ImageCache::set_placement_z_index_for_test` to mutate
//!    the resulting placement's `z_index` to `-1`.
//!
//! The mutator is `pub fn ..._for_test` — naming signals test-only
//! intent; production code never invokes it. See its rustdoc on
//! `oriterm_core::image::cache::ImageCache::set_placement_z_index_for_test`.
//!
//! ## Cell-dimension sync — required for visual harness
//!
//! `Term::cell_pixel_width / cell_pixel_height` default to `(8, 16)`
//! at construction (`oriterm_core/src/term/mod.rs:335-336`) and are
//! updated by the GUI via `Term::set_cell_dimensions` once font
//! metrics are known. The visual harness defaults skip this step, so
//! image placements compute `viewport_x = cell_col * 8` while the GPU
//! renderer maps cell positions through `renderer.cell_metrics()`
//! (font-derived). Without the sync, an image placed at cell `(10, 5)`
//! lands at pixel `(80, 80)` instead of `(100, 110)`, diverging from
//! where text at the same cell renders. The pilot calls
//! `set_cell_dimensions` immediately after harness creation so both
//! metrics agree — production GUI does the same path on every font /
//! resize event.
//!
//! ## Auto-assigned sixel image id
//!
//! `ImageCache::next_image_id` starts at `AUTO_ID_START =
//! 2_147_483_647` (`oriterm_core/src/image/cache/mod.rs:24`). On a
//! fresh harness, the first auto-assigned ID is exactly
//! `AUTO_ID_START` — the pilot relies on that determinism when
//! mutating the sixel placement.

use oriterm_core::image::ImageId;
use oriterm_test_support::spec_chain::{
    FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// Matches `oriterm_core::image::cache::AUTO_ID_START` (private). The
/// constant is duplicated here only for the mutator call — the live
/// guard is the bool return value of the mutator itself.
const AUTO_ID_START: u32 = 2_147_483_647;

/// CUP `row=5, col=10` (0-based) → `ESC [ 6 ; 11 H` (1-based).
const CUP_ROW5_COL10: &[u8] = b"\x1b[6;11H";

/// CUP `row=5, col=13` (0-based) → `ESC [ 6 ; 14 H` (1-based). Used
/// to position the text glyph outside the kitty-covered cell so the
/// text layer remains visible in the rendered output.
const CUP_ROW5_COL13: &[u8] = b"\x1b[6;14H";

/// Sixel DCS sequence painting a solid red 30×12-pixel region at the
/// cursor. Spans 3 cells horizontally (3 × 10-px cell width) so the
/// sixel layer survives in cells adjacent to the kitty-covered cell.
///
/// Breakdown:
/// - `\x1bP` — DCS introducer
/// - `q` — sixel graphics mode
/// - `#0;2;100;0;0` — color 0 = RGB(100%, 0%, 0%) = pure red
/// - `#0` — select color 0
/// - `!30~` — repeat sixel `~` (0b111111) 30 times → 30 cols × 6 rows
/// - `-` — graphics new line
/// - `#0!30~` — same pattern on next sixel row
/// - `\x1b\\` — ST
const SIXEL_RED_WIDE: &[u8] = b"\x1bPq#0;2;100;0;0#0!30~-#0!30~\x1b\\";

/// Kitty APC sequence: 1×1 opaque blue RGBA pixel, transmit+place,
/// `i=10` (explicit image ID so the test can target it), `z=1`
/// (above text), `q=2` (suppress replies).
///
/// Payload `AAD//w==` decodes to `0x00 0x00 0xFF 0xFF` — opaque blue.
const KITTY_BLUE_Z1: &[u8] = b"\x1b_Gf=32,s=1,v=1,a=T,i=10,z=1,q=2;AAD//w==\x1b\\";

/// Pins z-order composition: sixel z=-1 (cells 10..13) drawn BELOW
/// text, text glyph 'T' (cell 13) drawn BETWEEN image layers, kitty
/// z=+1 (cell 10) drawn ABOVE text. Catalog row:
/// `KG-CROSS-STACK-SIXEL-MIXED-Z-ORDER`. Per-cell pixel-count
/// assertions via `count_at_cell()` confirm layering correctness.
#[test]
fn kitty_and_sixel_render_with_mixed_z_order() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Sync Term cell metrics with the GPU renderer — see module
    // docs §"Cell-dimension sync".
    let cell = harness.renderer().cell_metrics();
    harness
        .core_mut()
        .term_mut()
        .set_cell_dimensions(cell.width as u16, cell.height as u16);

    // 1. Position cursor at cell (row=5, col=10), feed sixel red fill
    //    that spans 3 cells horizontally.
    harness.core_mut().feed(CUP_ROW5_COL10);
    harness.core_mut().feed(SIXEL_RED_WIDE);

    // 2. Reposition cursor to the same cell, feed kitty blue 1×1 at
    //    z=1. The 1×1 source cell-snaps to a full opaque cell —
    //    occluding sixel red at THIS cell but leaving the sixel layer
    //    visible at cells (11, 5) and (12, 5).
    harness.core_mut().feed(CUP_ROW5_COL10);
    harness.core_mut().feed(KITTY_BLUE_Z1);

    // 3. Write text glyph 'T' at cell (13, 5) — outside the kitty-
    //    covered cell so the text layer is visible in the render.
    harness.core_mut().feed(CUP_ROW5_COL13);
    harness.core_mut().feed(b"T");

    // 4. Sanity: cache holds exactly two placements (one sixel, one
    //    kitty), with kitty `image_id == ImageId(10)` (explicit) and
    //    sixel `image_id == ImageId(AUTO_ID_START)` (auto-assigned on
    //    a fresh harness).
    assert_eq!(
        harness
            .core_mut()
            .term_mut()
            .image_cache()
            .placement_count(),
        2,
        "expected 2 placements (sixel + kitty); cache state drift"
    );

    // 5. Mutate sixel placement's z_index to -1. Sixel handler at
    //    `oriterm_core/src/term/handler/image/sixel.rs:122`
    //    hard-codes z=0; the test-only mutator simulates the
    //    cross-protocol layering scenario the catalog row pins.
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

    // 6. Render through the visual rung chain to GoldenImage apex.
    let expectations = ScenarioExpectations {
        frame_input: Some(FrameInputExpectation::default_grid()),
        // Two image quads — sixel below text, kitty above text.
        gpu_instance: Some(GpuInstanceExpectation::at_least(0, 0).with_images(2)),
        texture: Some(TextureExpectation {
            min_non_zero_pixels: Some(1),
            width: None,
            height: None,
        }),
        golden: Some(GoldenExpectation {
            golden_name: Some("kitty_sixel_mixed_z_order"),
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

    // 7. Pixel-level layering assertions, one per layer:
    //    - Cell (10, 5): kitty BLUE dominates (above sixel + bg).
    //    - Cell (11, 5): sixel RED dominates (no kitty there, sixel
    //      `image_quads_below` survives the text pass).
    //    - Cell (13, 5): text glyph 'T' renders (text pass produces
    //      light-gray pixels at the glyph's strokes).
    let (pixels, w, _h) = harness
        .last_rendered_pixels()
        .map(|(p, w, h)| (p.to_vec(), w, h))
        .expect("render_visual_rungs must populate last_rendered_pixels");

    // Cell-dimension sync above guarantees `cell` matches Term's
    // cell_pixel_*, so image placements and text positions agree.
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

    let (kitty_blue, _, _) = count_at_cell(10, 5);
    let (_, sixel_red, _) = count_at_cell(11, 5);
    let (_, _, text_at_13) = count_at_cell(13, 5);

    assert!(
        kitty_blue >= 4,
        "kitty z=1 layer not visible at cell (10, 5): expected ≥4 \
         blue pixels (kitty cell-snaps to full opaque cell), found \
         {kitty_blue}. The kitty quad is not landing in \
         `image_quads_above` or is being clobbered by a later pass."
    );
    assert!(
        sixel_red >= 4,
        "sixel z=-1 layer not visible at cell (11, 5): expected ≥4 \
         red pixels (sixel spans cells 10..13, kitty only covers cell \
         10), found {sixel_red}. The sixel quad is not landing in \
         `image_quads_below`, the text pass is opaque-clearing past \
         the glyph cell, or the z_index mutator did not take effect."
    );
    assert!(
        text_at_13 >= 1,
        "text glyph 'T' not visible at cell (13, 5): expected ≥1 \
         light-gray pixel (text drawn between image layers), found \
         {text_at_13}. The text pass may be skipping the glyph cell, \
         or the kitty layer extended past its target cell."
    );
}
