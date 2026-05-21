//! §13.6.1 placeholder + cache-coordinate coexistence pilot.
//!
//! Catalog row: `KG-CROSS-STACK-SIXEL-PLACEHOLDER-COEXIST`
//! Apex: `GoldenImage`
//!
//! Drives a two-protocol composition:
//! 1. Kitty U=1 placeholder spanning row=5, col=10..20 — 11-cell
//!    horizontal strip carrying a `c=11,r=1` red 11×1 RGBA image.
//!    Multi-cell UV slicing per §13.6.1 ensures each cell renders its
//!    1/11 vertical slice of the source image.
//! 2. Sixel cache-coordinate placement at row=10, col=0 — 8-cell-wide
//!    red sixel strip from the production sixel handler (`dcs_n_cols_wide(8)`).
//!
//! Both protocols render together at the GPU apex. Tests both the
//! cross-protocol cache coexistence (sixel + kitty U=1 hold their
//! anchors / placements concurrently) and the GPU emit path's ability
//! to fold image_quads_above (kitty placeholder, default z=0) and
//! image_quads_below/above (sixel, default z=0) into one frame.
//!
//! ## Cell-dimension sync — required for visual harness
//!
//! `Term::cell_pixel_width / cell_pixel_height` default to `(8, 16)`,
//! diverging from the GPU renderer's font-derived metrics. Production
//! GUI calls `set_cell_dimensions` on every font/resize event; the
//! visual harness skips that by default. The pilot syncs explicitly so
//! image placements and text positions agree on pixel coords.

use oriterm_test_support::spec_chain::{
    FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, ScenarioExpectations,
    TextureExpectation,
};

use super::super::visual_harness::VisualSpecHarness;

/// CUP `row=5, col=10` (0-indexed) → `ESC [ 6 ; 11 H` (1-indexed). Park
/// cursor where the 11 placeholder cells will be written.
const CUP_ROW5_COL10: &[u8] = b"\x1b[6;11H";

/// CUP `row=10, col=0` (0-indexed) → `ESC [ 11 ; 1 H` (1-indexed).
const CUP_ROW10_COL0: &[u8] = b"\x1b[11;1H";

/// Transmit `a=T,U=1,i=1,c=11,r=1` with a 44-byte (11 pixels × 4 RGBA)
/// red strip. Base64-encoded payload computed at author-time.
const KITTY_TRANSMIT_U1_11X1: &[u8] = b"\x1b_Ga=T,U=1,i=1,f=32,s=11,v=1,c=11,r=1,q=2;/wAA//8AAP//AAD//wAA//8AAP//AAD//wAA//8AAP//AAD//wAA//8AAP8=\x1b\\";

/// Set foreground to palette index 1 (carries image_id_low=1 for U=1
/// placeholder cells) then write 11 placeholder cells with row=0 and
/// col=0..10 diacritic-encoded.
///
/// Bytes per cell: `U+10EEEE` (4 bytes UTF-8) + row diacritic (U+0305,
/// 2 bytes) + col diacritic (varies, 2 bytes). 11 cells × 8 bytes =
/// 88 bytes total. Cells written contiguously at the parked cursor.
const PLACEHOLDER_CELLS_11: &[u8] = b"\x1b[38;5;1m\xf4\x8e\xbb\xae\xcc\x85\xcc\x85\xf4\x8e\xbb\xae\xcc\x85\xcc\x8d\xf4\x8e\xbb\xae\xcc\x85\xcc\x8e\xf4\x8e\xbb\xae\xcc\x85\xcc\x90\xf4\x8e\xbb\xae\xcc\x85\xcc\x92\xf4\x8e\xbb\xae\xcc\x85\xcc\xbd\xf4\x8e\xbb\xae\xcc\x85\xcc\xbe\xf4\x8e\xbb\xae\xcc\x85\xcc\xbf\xf4\x8e\xbb\xae\xcc\x85\xcd\x86\xf4\x8e\xbb\xae\xcc\x85\xcd\x8a\xf4\x8e\xbb\xae\xcc\x85\xcd\x8b\x1b[39m";

/// Sixel DCS: 8-pixel-wide red strip. Built inline from the production
/// sixel-fixtures helper's prefix + N `~` (sixel pixel column) +
/// terminator. The handler stores + places the sixel at the cursor.
const SIXEL_RED_8COL: &[u8] = b"\x1bP0;0;0q\"1;1;8;6#0;2;100;0;0#0!8~\x1b\\";

#[test]
fn kitty_placeholder_and_sixel_coexist_at_gpu_apex() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Sync Term cell metrics with the GPU renderer per §"Cell-dimension sync".
    let cell = harness.renderer().cell_metrics();
    harness
        .core_mut()
        .term_mut()
        .set_cell_dimensions(cell.width as u16, cell.height as u16);

    // 1. Park cursor + transmit kitty U=1 image (no placement created —
    //    cells will be written next).
    harness.core_mut().feed(KITTY_TRANSMIT_U1_11X1);

    // 2. Park cursor at row=5,col=10 and write 11 placeholder cells.
    //    Each cell carries diacritic-encoded (image_row=0, image_col=i)
    //    that resolves back to image_id=1 (fg palette index 1).
    harness.core_mut().feed(CUP_ROW5_COL10);
    harness.core_mut().feed(PLACEHOLDER_CELLS_11);

    // 3. Park cursor at row=10,col=0 and feed sixel — production handler
    //    stores + places the sixel placement at the cursor cell.
    harness.core_mut().feed(CUP_ROW10_COL0);
    harness.core_mut().feed(SIXEL_RED_8COL);

    // 4. Sanity: cache holds the kitty image (anchored, not placed) AND
    //    the sixel image+placement. The kitty image is reachable via the
    //    placeholder anchor — its `placement_count()` contribution is 0.
    let placements = harness
        .core_mut()
        .term_mut()
        .image_cache()
        .placement_count();
    assert_eq!(
        placements, 1,
        "expected exactly 1 placement (sixel only); kitty U=1 uses anchors not placements. \
         Got placements={placements} — drift in cross-protocol cache state."
    );
    assert_eq!(
        harness.core_mut().term_mut().image_cache().image_count(),
        2,
        "expected 2 images (kitty + sixel)"
    );

    // 5. Render through the visual rung chain. Expect 12 image quads:
    //    11 placeholder cells (above text) + 1 sixel (z=0 default,
    //    above-with-text).
    let expectations = ScenarioExpectations {
        frame_input: Some(FrameInputExpectation::default_grid()),
        gpu_instance: Some(GpuInstanceExpectation::at_least(0, 0).with_images(12)),
        texture: Some(TextureExpectation {
            min_non_zero_pixels: Some(1),
            width: None,
            height: None,
        }),
        golden: Some(GoldenExpectation {
            golden_name: Some("kitty_placeholder_sixel_coexist"),
        }),
        ..ScenarioExpectations::default()
    };
    let results =
        harness.render_visual_rungs("KG-CROSS-STACK-SIXEL-PLACEHOLDER-COEXIST", &expectations);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }
}
