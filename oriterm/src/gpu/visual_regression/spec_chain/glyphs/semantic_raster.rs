//! Exhaustive semantic raster sweep for every subcell-glyph codepoint.
//!
//! Per Testing Rule` and the §11.2 plan,
//! sparse golden pins establish the visual anchor for each family while
//! this sweep proves per-codepoint dispatch wiring across all 706
//! codepoints in the five covered ranges.
//!
//! For every codepoint in every range, this module asserts:
//! 1. `font::is_builtin(ch) == true` — the font shaper bypasses this codepoint.
//! 2. `builtin_glyphs::rasterize(ch, cell_w, cell_h)` returns `Some(glyph)` —
//!    the built-in dispatch fires and produces a bitmap.
//! 3. The rendered bitmap has at least one non-zero alpha pixel — the glyph
//!    actually writes ink. Codepoints like `SPACE` that render nothing are
//!    not in scope (the five ranges here are all geometric glyphs).
//!
//! Correctness of the specific bitmask values is anchored by:
//! - **Octants**: `legacy_computing::tests::octants_table_matches_canonical_artifact`
//!   pins every entry in `OCTANT_MASKS` against the §11.0 canonical artifact.
//! - **Sextants**: `sextants_bit_decomposition_matches_canonical_formula` below
//!   checks every codepoint's 2×3 subcell pattern against the Ghostty formula
//!   `bits = idx + idx/0x14 + 1`.
//! - **Block elements (non-quadrant)**: `block_elements_non_quadrant_fill_regions_match_unicode_spec`
//!   below checks fill regions for half-blocks, eighths, and shades against
//!   Unicode's canonical geometry.
//! - **Quadrants**: `quadrant_subcells_match_codepoint_semantics` below.
//! - **Braille**: `braille_dot_bits_match_codepoint_low_byte` below.
//! - **Box drawing**: pixel-exact pinned by the `subcell_box_drawings_double_cross`
//!   sparse golden (U+256C) plus the `box_drawing_builtin_wins_over_configured_font`
//!   precedence test. Box-drawing glyphs encode stroke weights (light / heavy /
//!   double) over a 12-cell grid rather than a simple bit-per-subcell mask, so a
//!   per-codepoint bit decomposition would just re-encode `box_drawing.rs`'s
//!   weight table (tautological). Coverage is `is_builtin + rasterize + ink`
//!   across all 128 codepoints here + 1 sparse golden + 1 precedence pin.

use crate::font::is_builtin;
use crate::gpu::builtin_glyphs::rasterize;

/// Canonical test cell dimensions. Divisible by 2 and 4 so 2×2, 2×3, and 2×4
/// subcell grids all partition cleanly.
const TEST_CELL_W: u32 = 20;
const TEST_CELL_H: u32 = 40;

/// Assert dispatch and ink for every codepoint in the given inclusive range.
fn assert_every_codepoint_rasterizes(start: u32, end: u32, family: &'static str, expect_ink: bool) {
    let mut visited = 0usize;
    for cp in start..=end {
        let ch = char::from_u32(cp).expect("valid scalar codepoint");
        assert!(
            is_builtin(ch),
            "U+{cp:04X} ({family}): font::is_builtin must return true",
        );
        let glyph = rasterize(ch, TEST_CELL_W, TEST_CELL_H);
        assert!(
            glyph.is_some(),
            "U+{cp:04X} ({family}): built-in dispatch must return Some(glyph)",
        );
        if expect_ink {
            let bitmap = glyph.expect("rasterize returned Some above").bitmap;
            assert!(
                bitmap.iter().any(|&a| a > 0),
                "U+{cp:04X} ({family}): rendered bitmap must have at least one non-zero alpha pixel",
            );
        }
        visited += 1;
    }
    let expected = (end - start + 1) as usize;
    assert_eq!(
        visited, expected,
        "{family}: expected to walk {expected} codepoints, visited {visited}",
    );
}

/// Box drawing: U+2500..=U+257F (128 codepoints).
///
/// Every box-drawing glyph draws strokes and therefore inks at least one
/// pixel — none of them are "blank" like the ASCII space.
#[test]
fn box_drawing_exhaustive_raster() {
    assert_every_codepoint_rasterizes(0x2500, 0x257F, "BOX DRAWING", true);
}

/// Block elements: U+2580..=U+259F (32 codepoints).
///
/// Includes half blocks, eighths, shades, full block, and quadrants. Every
/// codepoint in the range produces ink — there are no blank block elements
/// between U+2580 and U+259F.
#[test]
fn block_elements_exhaustive_raster() {
    assert_every_codepoint_rasterizes(0x2580, 0x259F, "BLOCK ELEMENTS", true);
}

/// Sextants: U+1FB00..=U+1FB3B (60 codepoints).
///
/// The sextant range covers 60 consecutive codepoints and excludes the
/// "all-bits-zero" and "all-bits-one" corner cases (which would collide
/// with SPACE and FULL BLOCK respectively). Every entry inks.
#[test]
fn sextants_exhaustive_raster() {
    assert_every_codepoint_rasterizes(0x1FB00, 0x1FB3B, "SEXTANTS", true);
}

/// Octants: U+1CD00..=U+1CDE5 (230 codepoints).
///
/// The octant range is the §11.1 deliverable. Every entry inks — the
/// canonical artifact's bitmask values are all non-zero. Per-codepoint
/// bitmask correctness is anchored by
/// `legacy_computing::tests::octants_table_matches_canonical_artifact`.
#[test]
fn octants_exhaustive_raster() {
    assert_every_codepoint_rasterizes(0x1CD00, 0x1CDE5, "OCTANTS", true);
}

/// Braille: U+2800..=U+28FF (256 codepoints).
///
/// U+2800 (BRAILLE PATTERN BLANK) has no dots set and therefore inks
/// nothing — it is specifically the empty-dot case. The rest of the range
/// (U+2801..=U+28FF) always inks at least one dot.
#[test]
fn braille_exhaustive_raster() {
    // U+2800 (blank) — must dispatch but produces no ink.
    let blank = '\u{2800}';
    assert!(is_builtin(blank), "U+2800 blank must be built-in");
    let glyph = rasterize(blank, TEST_CELL_W, TEST_CELL_H)
        .expect("U+2800 must dispatch to built-in renderer");
    assert!(
        glyph.bitmap.iter().all(|&a| a == 0),
        "U+2800 BRAILLE PATTERN BLANK must produce an all-zero bitmap",
    );

    // U+2801..=U+28FF — every codepoint has at least one dot and therefore inks.
    assert_every_codepoint_rasterizes(0x2801, 0x28FF, "BRAILLE", true);
}

/// Additional property: for braille, the 8 low bits of the codepoint
/// drive which dots are set, per Unicode. Dot-1 = bit 0, dot-2 = bit 1,
/// ..., dot-8 = bit 7. Proves the dispatch is reading codepoint bits in
/// the canonical order — not scrambled.
///
/// Methodology: for each braille codepoint, sample four corner positions
/// that fall inside the expected dot region for each bit, and verify that
/// the pixel alpha is non-zero iff the corresponding bit is set in the
/// codepoint's low byte.
#[test]
fn braille_dot_bits_match_codepoint_low_byte() {
    // The braille implementation (oriterm/src/gpu/builtin_glyphs/braille.rs)
    // places each dot at a column × row derived from its Unicode bit index:
    //   bit 0..=2 → left column,  rows 0, 1, 2
    //   bit 3..=5 → right column, rows 0, 1, 2
    //   bit 6     → left column,  row 3
    //   bit 7     → right column, row 3
    //
    // We sample a 2-column × 4-row grid of representative positions inside
    // each dot's expected cell region and confirm the rendered bitmap's
    // filled-cell pattern matches the codepoint's low byte.
    let cell_w = TEST_CELL_W;
    let cell_h = TEST_CELL_H;

    // Sample-point offsets chosen to land safely inside the subcell region
    // where a braille dot would be drawn (well away from the subcell edge
    // so we don't alias with a neighbor's anti-aliased skirt).
    let col_x = [cell_w / 4, 3 * cell_w / 4];
    let row_y = [cell_h / 8, 3 * cell_h / 8, 5 * cell_h / 8, 7 * cell_h / 8];

    // Bit → (col_index, row_index) per Unicode braille dot mapping.
    let bit_positions = [
        (0, (0, 0)), // dot 1: L col, row 0
        (1, (0, 1)), // dot 2: L col, row 1
        (2, (0, 2)), // dot 3: L col, row 2
        (3, (1, 0)), // dot 4: R col, row 0
        (4, (1, 1)), // dot 5: R col, row 1
        (5, (1, 2)), // dot 6: R col, row 2
        (6, (0, 3)), // dot 7: L col, row 3
        (7, (1, 3)), // dot 8: R col, row 3
    ];

    for cp in 0x2800u32..=0x28FFu32 {
        let ch = char::from_u32(cp).expect("braille codepoint valid");
        let glyph =
            rasterize(ch, cell_w, cell_h).expect("braille codepoint must rasterize via built-in");
        let bitmap = &glyph.bitmap;
        let mask = (cp - 0x2800) as u8;

        for (bit, (c, r)) in bit_positions {
            let px = col_x[c] as usize;
            let py = row_y[r] as usize;
            let idx = py * cell_w as usize + px;
            let alpha = bitmap[idx];
            let expected_set = (mask >> bit) & 1 == 1;
            if expected_set {
                assert!(
                    alpha > 0,
                    "U+{cp:04X} mask=0x{mask:02X} bit {bit} (col={c}, row={r}): \
                     dot must be set, but sample pixel ({px},{py}) has alpha=0",
                );
            } else {
                assert_eq!(
                    alpha, 0,
                    "U+{cp:04X} mask=0x{mask:02X} bit {bit} (col={c}, row={r}): \
                     dot must be UNSET, but sample pixel ({px},{py}) has alpha={alpha}",
                );
            }
        }
    }
}

/// Additional property: for sextants U+1FB00..=U+1FB3B, the 2×3 subcell
/// grid's 6-bit mask is derived from the codepoint via the Ghostty arithmetic
/// formula `bits = idx + idx/0x14 + 1` (where `idx = cp - 0x1FB00`), with bit
/// ordering `tl, tr, ml, mr, bl, br`. Sample the center of each of the six
/// subcells and verify the filled pattern matches the formula exactly.
///
/// Proves that the sextant dispatch at `legacy_computing/mod.rs::draw_sextant`
/// encodes the canonical mask for every codepoint in the range — not just
/// the sparse-pin codepoint U+1FB3B anchored by `subcell_sextant_near_full`.
#[test]
fn sextants_bit_decomposition_matches_canonical_formula() {
    let cell_w = TEST_CELL_W;
    let cell_h = TEST_CELL_H;
    // Subcell sample centers match the `draw_sextant` geometry:
    //   hw = round(cell_w / 2) = 10
    //   th = round(cell_h / 3) = 13
    //   th2 = round(cell_h * 2 / 3) = 27
    // Bits: bit 0 = tl, bit 1 = tr, bit 2 = ml, bit 3 = mr, bit 4 = bl, bit 5 = br.
    let subcell_centers = [
        (cell_w / 4, cell_h / 6),         // tl (bit 0)
        (3 * cell_w / 4, cell_h / 6),     // tr (bit 1)
        (cell_w / 4, cell_h / 2),         // ml (bit 2)
        (3 * cell_w / 4, cell_h / 2),     // mr (bit 3)
        (cell_w / 4, 5 * cell_h / 6),     // bl (bit 4)
        (3 * cell_w / 4, 5 * cell_h / 6), // br (bit 5)
    ];

    for cp in 0x1FB00u32..=0x1FB3Bu32 {
        let ch = char::from_u32(cp).expect("sextant codepoint valid");
        let glyph = rasterize(ch, cell_w, cell_h).expect("sextant must rasterize via built-in");
        let bitmap = &glyph.bitmap;

        let idx = cp - 0x1FB00;
        let expected_mask = (idx + idx / 0x14 + 1) as u8;

        for (bit, (x, y)) in subcell_centers.iter().enumerate() {
            let idx_px = (*y as usize) * (cell_w as usize) + (*x as usize);
            let alpha = bitmap[idx_px];
            let expected_set = (expected_mask >> bit) & 1 == 1;
            if expected_set {
                assert!(
                    alpha > 0,
                    "U+{cp:04X} mask=0b{expected_mask:06b} bit {bit}: \
                     subcell must be filled but center pixel has alpha=0",
                );
            } else {
                assert_eq!(
                    alpha, 0,
                    "U+{cp:04X} mask=0b{expected_mask:06b} bit {bit}: \
                     subcell must be unfilled but center pixel has alpha={alpha}",
                );
            }
        }
    }
}

/// Additional property: for non-quadrant block elements U+2580..=U+2595,
/// each codepoint has a canonical fill region per Unicode (upper half, lower
/// N/8, full block, left N/8, right half, 25%/50%/75% shade, upper 1/8, right
/// 1/8). Sample points inside-vs-outside the expected fill region and verify
/// the observed alpha matches Unicode's canonical geometry. Quadrants
/// U+2596..=U+259F are anchored by `quadrant_subcells_match_codepoint_semantics`.
///
/// Methodology: for each codepoint, the canonical fill is a rectangle in cell
/// coordinates. A pixel inside the rectangle must have alpha > 0; a pixel
/// outside must have alpha == 0. Shade glyphs (U+2591/2/3) fill the entire
/// cell uniformly at alphas 64/128/191 — those are checked as "every pixel
/// has alpha > 0" since the exact alpha is pinned by the sparse goldens.
#[test]
fn block_elements_non_quadrant_fill_regions_match_unicode_spec() {
    let cell_w = TEST_CELL_W;
    let cell_h = TEST_CELL_H;

    // (codepoint, list of (x, y, should_be_filled) samples) per
    // `oriterm/src/gpu/builtin_glyphs/blocks.rs` canonical geometry.
    let samples: &[(u32, &[(u32, u32, bool)])] = &[
        // U+2580 UPPER HALF BLOCK — top cell_h/2 filled, bottom empty.
        (
            0x2580,
            &[
                (cell_w / 2, cell_h / 4, true),
                (cell_w / 2, 3 * cell_h / 4, false),
            ],
        ),
        // U+2581..=U+2587 LOWER N/8 BLOCKS: bottom N/8 fraction filled.
        (
            0x2581,
            &[
                (cell_w / 2, cell_h - 2, true),
                (cell_w / 2, cell_h / 4, false),
            ],
        ),
        (
            0x2584,
            &[
                (cell_w / 2, 3 * cell_h / 4, true),
                (cell_w / 2, cell_h / 4, false),
            ],
        ),
        (
            0x2587,
            &[(cell_w / 2, cell_h / 2, true), (cell_w / 2, 0, false)],
        ),
        // U+2588 FULL BLOCK — every corner filled.
        (
            0x2588,
            &[
                (0, 0, true),
                (cell_w - 1, 0, true),
                (0, cell_h - 1, true),
                (cell_w - 1, cell_h - 1, true),
            ],
        ),
        // U+2589..=U+258F LEFT N/8 BLOCKS: left N/8 fraction filled (7/8 down to 1/8).
        (
            0x2589,
            &[(1, cell_h / 2, true), (cell_w - 1, cell_h / 2, false)],
        ),
        (
            0x258C,
            &[
                (cell_w / 4, cell_h / 2, true),
                (3 * cell_w / 4, cell_h / 2, false),
            ],
        ),
        (
            0x258F,
            &[(0, cell_h / 2, true), (cell_w / 2, cell_h / 2, false)],
        ),
        // U+2590 RIGHT HALF BLOCK — right half filled, left empty.
        (
            0x2590,
            &[
                (3 * cell_w / 4, cell_h / 2, true),
                (cell_w / 4, cell_h / 2, false),
            ],
        ),
        // U+2594 UPPER 1/8 BLOCK — top 1/8 filled only.
        (
            0x2594,
            &[(cell_w / 2, 0, true), (cell_w / 2, cell_h / 2, false)],
        ),
        // U+2595 RIGHT 1/8 BLOCK — right 1/8 filled only.
        (
            0x2595,
            &[
                (cell_w - 1, cell_h / 2, true),
                (cell_w / 2, cell_h / 2, false),
            ],
        ),
    ];

    for (cp, points) in samples {
        let ch = char::from_u32(*cp).expect("block codepoint valid");
        let glyph = rasterize(ch, cell_w, cell_h).expect("block codepoint must rasterize");
        let bitmap = &glyph.bitmap;
        for (x, y, should_be_filled) in *points {
            let idx = (*y as usize) * (cell_w as usize) + (*x as usize);
            let alpha = bitmap[idx];
            if *should_be_filled {
                assert!(
                    alpha > 0,
                    "U+{cp:04X} at pixel ({x},{y}): must be filled but alpha=0",
                );
            } else {
                assert_eq!(
                    alpha, 0,
                    "U+{cp:04X} at pixel ({x},{y}): must be empty but alpha={alpha}",
                );
            }
        }
    }

    // Shade blocks: uniform fill across the cell at a fractional alpha.
    // Exact alpha values pinned by the sparse goldens; here we only pin the
    // "whole-cell uniform fill" invariant distinct from U+2580's half-block.
    for cp in [0x2591u32, 0x2592, 0x2593] {
        let ch = char::from_u32(cp).expect("shade codepoint valid");
        let glyph = rasterize(ch, cell_w, cell_h).expect("shade must rasterize");
        let bitmap = &glyph.bitmap;
        assert!(
            bitmap.iter().all(|&a| a > 0),
            "U+{cp:04X} shade block must fill every pixel with non-zero alpha",
        );
    }
}

/// Additional property: for quadrants U+2596..=U+259F, the four 2×2
/// subcells correspond to explicit bit positions per Unicode (TL, TR, BL,
/// BR). Sample the center of each subcell and verify the filled pattern
/// matches the codepoint's canonical quadrant semantics.
#[test]
fn quadrant_subcells_match_codepoint_semantics() {
    // Quadrant mask per codepoint (TL=bit 0, TR=bit 1, BL=bit 2, BR=bit 3).
    //
    // Source: Unicode 2580 Block Elements chart — rows U+2596..=U+259F.
    let expected = [
        (0x2596u32, 0b0100), // ▖ lower left
        (0x2597, 0b1000),    // ▗ lower right
        (0x2598, 0b0001),    // ▘ upper left
        (0x2599, 0b1101),    // ▙ UL + LL + LR
        (0x259A, 0b1001),    // ▚ UL + LR
        (0x259B, 0b0111),    // ▛ UL + UR + LL
        (0x259C, 0b1011),    // ▜ UL + UR + LR
        (0x259D, 0b0010),    // ▝ UR
        (0x259E, 0b0110),    // ▞ UR + LL
        (0x259F, 0b1110),    // ▟ UR + LL + LR
    ];

    let cell_w = TEST_CELL_W;
    let cell_h = TEST_CELL_H;
    // Sample the geometric center of each 2×2 subcell.
    let centers = [
        (cell_w / 4, cell_h / 4),         // TL (bit 0)
        (3 * cell_w / 4, cell_h / 4),     // TR (bit 1)
        (cell_w / 4, 3 * cell_h / 4),     // BL (bit 2)
        (3 * cell_w / 4, 3 * cell_h / 4), // BR (bit 3)
    ];

    for (cp, mask) in expected {
        let ch = char::from_u32(cp).expect("valid quadrant codepoint");
        let glyph = rasterize(ch, cell_w, cell_h).expect("quadrant must rasterize");
        let bitmap = &glyph.bitmap;
        for (bit, (x, y)) in centers.iter().enumerate() {
            let idx = (*y as usize) * (cell_w as usize) + (*x as usize);
            let alpha = bitmap[idx];
            let expected_set = (mask >> bit) & 1 == 1;
            if expected_set {
                assert!(
                    alpha > 0,
                    "U+{cp:04X} mask=0b{mask:04b} bit {bit}: \
                     subcell must be filled but center pixel has alpha=0",
                );
            } else {
                assert_eq!(
                    alpha, 0,
                    "U+{cp:04X} mask=0b{mask:04b} bit {bit}: \
                     subcell must be unfilled but center pixel has alpha={alpha}",
                );
            }
        }
    }
}
