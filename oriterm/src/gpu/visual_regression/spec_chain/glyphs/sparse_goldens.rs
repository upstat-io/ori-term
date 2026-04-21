//! Sparse golden pins — one representative codepoint per subcell-glyph family.
//!
//! Each test drives a single non-trivial codepoint through the full 8-rung
//! verification chain and commits an exact-pixel golden at
//! `oriterm/tests/references/subcell_<family>.png`. The companion exhaustive
//! semantic-raster sweep in `semantic_raster.rs` proves per-codepoint correctness;
//! these sparse pins are the visual golden anchors that also catch regressions
//! in the GPU pipeline (cell metrics, glyph quad placement, alpha blending).
//!
//! Catalog rows: USC-BLOCKS, USC-BOX, USC-BRAILLE, USC-LEGACY-SEXTANT

use super::super::visual_harness::VisualSpecHarness;
use super::run_glyph_scenario;

/// Box drawing: U+256C (BOX DRAWINGS DOUBLE VERTICAL AND HORIZONTAL, ╬).
///
/// Non-trivial: four double-stroke arms meeting at a center. Catches any
/// regression in the `box_drawing.rs` weight-segment table dispatch.
#[test]
fn subcell_box_drawings_double_cross_sparse_golden() {
    const BYTES: &[u8] = "\u{256C}".as_bytes();
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    let _ = run_glyph_scenario(
        &mut harness,
        '\u{256C}',
        "subcell_box_drawings_double_cross",
        "USC-BOX",
        BYTES,
    );
}

/// Block elements: U+2588 (FULL BLOCK, █).
///
/// All 4 quadrants filled. Proves `blocks::draw()` handles the max-fill case
/// and that the Canvas → GlyphAtlas path writes every pixel in the cell.
#[test]
fn subcell_full_block_sparse_golden() {
    const BYTES: &[u8] = "\u{2588}".as_bytes();
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    let _ = run_glyph_scenario(
        &mut harness,
        '\u{2588}',
        "subcell_full_block",
        "USC-BLOCKS",
        BYTES,
    );
}

/// Quadrants: U+259F (QUADRANT UPPER RIGHT AND LOWER LEFT AND LOWER RIGHT, ▟).
///
/// Three of four bits set — proves the quadrant dispatch cannot silently
/// alias with U+2588 (all-4-bits) or an empty cell (zero bits).
#[test]
fn subcell_quadrant_ur_bl_br_sparse_golden() {
    const BYTES: &[u8] = "\u{259F}".as_bytes();
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    let _ = run_glyph_scenario(
        &mut harness,
        '\u{259F}',
        "subcell_quadrant_ur_bl_br",
        "USC-BLOCKS",
        BYTES,
    );
}

/// Sextants: U+1FB3B — last codepoint in the sextant range. Its derived bit
/// mask `(0x3B + 0x3B / 0x14 + 1) = 62 = 0b111110` sets 5 of 6 subcells,
/// exercising a non-trivial 2×3 pattern.
#[test]
fn subcell_sextant_near_full_sparse_golden() {
    const BYTES: &[u8] = "\u{1FB3B}".as_bytes();
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    let _ = run_glyph_scenario(
        &mut harness,
        '\u{1FB3B}',
        "subcell_sextant_near_full",
        "USC-LEGACY-SEXTANT",
        BYTES,
    );
}

/// Octants: U+1CDE5 — last codepoint in the octant range. Anchors the
/// octant dispatch AND the `is_builtin` + `rasterize` match-arm extensions
/// from §11.1. A regression in either wiring fails this test.
#[test]
fn subcell_octant_end_of_range_sparse_golden() {
    const BYTES: &[u8] = "\u{1CDE5}".as_bytes();
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    let _ = run_glyph_scenario(
        &mut harness,
        '\u{1CDE5}',
        "subcell_octant_end_of_range",
        "USC-LEGACY-OCTANT",
        BYTES,
    );
}

/// Braille: U+28FF (BRAILLE PATTERN DOTS-12345678, ⣿).
///
/// All 8 dots set — proves the braille dispatch in `builtin_glyphs/braille.rs`
/// renders the full 2×4 dot grid. The companion adjacency test proves there
/// is no interference with U+1CDE5 (also a 2×4 glyph with a different bit
/// ordering).
#[test]
fn subcell_braille_all_dots_sparse_golden() {
    const BYTES: &[u8] = "\u{28FF}".as_bytes();
    let Some(mut harness) = VisualSpecHarness::new() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };
    let _ = run_glyph_scenario(
        &mut harness,
        '\u{28FF}',
        "subcell_braille_all_dots",
        "USC-BRAILLE",
        BYTES,
    );
}
