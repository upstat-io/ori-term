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
//! 2. Sixel cache-coordinate placement at row=10, col=0 — 8-cell-wide ×
//!    1-cell-tall red sixel strip from the production sixel handler.
//!    Bytes computed dynamically from `harness.renderer().cell_metrics()`
//!    via `sixel_fixtures::dcs_red_pixel_block(cell_w_px * 8, cell_h_px)`
//!    so the rendered sixel actually fills 8 cells horizontally and ≥1
//!    cell vertically. PIXEL-vs-CELL parameter naming on the helper
//!    blocks the prior cell-vs-pixel confusion that left a sub-cell red
//!    speck on the operator-visual gate.
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

use oriterm_core::image::ImageId;
/// The auto-namespace base imported from its single canonical owner — on a
/// fresh harness the first auto-assigned image ID is exactly this value.
use oriterm_core::image::AUTO_ID_START_FOR_TEST as AUTO_ID_START;
use oriterm_test_support::spec_chain::sixel_fixtures;
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

/// Pins cross-protocol coexistence at the GPU apex: kitty U=1
/// placeholder (11 cells) + sixel cache-coordinate placement (8 cells)
/// hold their cache entries concurrently and emit 12 image quads in one
/// frame. Catalog row: `KG-CROSS-STACK-SIXEL-PLACEHOLDER-COEXIST`.
/// Image-dimension pins assert the sixel fills ≥ 8 cells × ≥ 1 cell
/// after cell-scaled byte construction via `dcs_red_pixel_block` —
/// rejects the prior sub-cell-pixel regression class.
#[test]
fn kitty_placeholder_and_sixel_coexist_at_gpu_apex() {
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    // Sync Term cell metrics with the GPU renderer per §"Cell-dimension sync".
    // Capture the integer metrics (cell_w_px / cell_h_px) used for sixel
    // sizing below — `CellMetrics` is fractional but `set_cell_dimensions`
    // stores u16, so cast once and reuse.
    let cell = harness.renderer().cell_metrics();
    let cell_w_px = cell.width as u16;
    let cell_h_px = cell.height as u16;
    harness
        .core_mut()
        .term_mut()
        .set_cell_dimensions(cell_w_px, cell_h_px);

    // 1. Park cursor + transmit kitty U=1 image (no placement created —
    //    cells will be written next).
    harness.core_mut().feed(KITTY_TRANSMIT_U1_11X1);

    // 2. Park cursor at row=5,col=10 and write 11 placeholder cells.
    //    Each cell carries diacritic-encoded (image_row=0, image_col=i)
    //    that resolves back to image_id=1 (fg palette index 1).
    harness.core_mut().feed(CUP_ROW5_COL10);
    harness.core_mut().feed(PLACEHOLDER_CELLS_11);

    // 3. Park cursor at row=10,col=0 and feed cell-scaled sixel — bytes
    //    computed from the live cell metrics so the rendered sixel fills
    //    8 cells horizontally × 1 cell vertically (rounded up to the
    //    next sixel band of 6 px).
    harness.core_mut().feed(CUP_ROW10_COL0);
    let sixel_bytes =
        sixel_fixtures::dcs_red_pixel_block(cell_w_px as usize * 8, cell_h_px as usize);
    harness.core_mut().feed(&sixel_bytes);

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

    // 4b. Pin the sixel image cache dimensions. The sixel handler
    //     auto-assigns IDs starting from AUTO_ID_START
    //     (oriterm_core/src/image/cache/mod.rs:24); on a fresh harness
    //     the first auto-ID is exactly AUTO_ID_START. The kitty U=1
    //     image uses i=1 (explicit), so the sixel placement's image_id
    //     is AUTO_ID_START.
    let sixel_img = harness
        .core_mut()
        .term_mut()
        .image_cache()
        .get_no_touch(ImageId::from_raw(AUTO_ID_START))
        .expect("sixel image must be in cache at AUTO_ID_START");
    // Image must fill ≥8 cells horizontally AND ≥1 cell tall.
    assert!(
        sixel_img.width >= 8 * cell_w_px as u32,
        "sixel must fill ≥8 cells horizontally; \
         got width={} px, cell_w_px={} (need ≥{})",
        sixel_img.width,
        cell_w_px,
        8 * cell_w_px as u32,
    );
    assert!(
        sixel_img.height >= cell_h_px as u32,
        "sixel must fill ≥1 cell vertically; \
         got height={} px, cell_h_px={} (need ≥{})",
        sixel_img.height,
        cell_h_px,
        cell_h_px,
    );
    // Reject the broken sub-cell-width regression class.
    assert_ne!(
        sixel_img.width, 8,
        "sixel regressed to literal 8-PIXEL-wide image. \
         The pilot must use cell-scaled bytes via dcs_red_pixel_block, \
         NOT a pixel-scaled !8~ inline constant."
    );
    assert!(
        sixel_img.width > 8,
        "sixel narrower than 8 px is the sub-cell regression class — \
         verify dcs_red_pixel_block is called with cell_w_px * 8, \
         not a literal `8`."
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
