//! Tests for `legacy_computing` — octants canonical-mapping guard,
//! braille-vs-octant rendering-model check, and representative
//! codepoint render assertions.

use std::fs;
use std::path::PathBuf;

use super::Canvas;
use super::octants::{OCTANT_END, OCTANT_MASKS, OCTANT_START};

/// Path to the canonical octant bitmask artifact.
///
/// The file lives in the wrapper repo at
/// `plans/spec-conformance/specs/octant-bitmask-mapping.md`. Path discovery
/// goes through the SSOT helper introduced in BUG-08-028 — never reintroduce
/// ad-hoc `crate_root.join("..").join("plans")` arithmetic, which silently
/// breaks under the wrapper/subrepo split.
///
/// Returns `None` when the wrapper repo is not discoverable (standalone
/// term_repo checkout); consumers MUST graceful-skip per
/// `.claude/rules/tests.md §Graceful Skip Protocol`.
fn octant_mapping_artifact_path() -> Option<PathBuf> {
    oriterm_test_support::paths::specs_dir().map(|d| d.join("octant-bitmask-mapping.md"))
}

/// Parse the canonical artifact's table into a `Vec<(u32, u8)>`.
///
/// Each row has the shape `| U+1CDxx | 0xNN | 0b... | ... |`; the parser
/// extracts the codepoint (hex after `U+`) and the mask (hex after `0x`).
fn parse_canonical_mapping(path: &std::path::Path) -> Vec<(u32, u8)> {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read canonical artifact at {}: {e}", path.display()));

    let mut rows = Vec::new();
    for line in contents.lines() {
        let line = line.trim();
        if !line.starts_with("| U+") {
            continue;
        }
        let mut cols = line.split('|').map(str::trim);
        let _leading = cols.next();
        let codepoint_col = cols.next().expect("codepoint column present");
        let mask_col = cols.next().expect("mask hex column present");

        let cp_hex = codepoint_col
            .strip_prefix("U+")
            .expect("codepoint column begins with U+");
        let mask_hex = mask_col
            .strip_prefix("0x")
            .expect("mask column begins with 0x");

        let cp = u32::from_str_radix(cp_hex, 16).expect("codepoint is hex");
        let mask = u8::from_str_radix(mask_hex, 16).expect("mask is hex");
        rows.push((cp, mask));
    }
    rows
}

/// **Canonical-mapping guard test** (primary regression guard).
///
/// Asserts that `OCTANT_MASKS[cp - OCTANT_START]` is byte-identical to
/// the mask recorded in `plans/spec-conformance/specs/octant-bitmask-mapping.md`
/// for every codepoint in `U+1CD00..=U+1CDE5`. Any divergence between
/// the renderer's in-source table and the canonical artifact fails the
/// build. This is the regression guard §11.1 documents.
#[test]
fn octants_table_matches_canonical_artifact() {
    let Some(path) = octant_mapping_artifact_path() else {
        eprintln!(
            "SKIP: canonical octant artifact — wrapper repo not discoverable \
             (standalone term_repo checkout)"
        );
        return;
    };
    let rows = parse_canonical_mapping(&path);
    assert_eq!(
        rows.len(),
        230,
        "canonical artifact must define exactly 230 octant rows"
    );

    for (cp, expected_mask) in rows {
        let ch = char::from_u32(cp).expect("artifact codepoint is valid");
        assert!(
            (OCTANT_START..=OCTANT_END).contains(&ch),
            "codepoint U+{cp:04X} in artifact is outside octant range"
        );
        let idx = (cp - OCTANT_START as u32) as usize;
        let actual = OCTANT_MASKS[idx];
        assert_eq!(
            actual, expected_mask,
            "octant mask mismatch at U+{cp:04X}: table has 0x{actual:02X}, artifact has 0x{expected_mask:02X}",
        );
    }
}

/// Coverage completeness: the 230-entry table covers the full
/// `OCTANT_START..=OCTANT_END` range and every entry is unique.
#[test]
fn octants_table_covers_full_range_and_is_unique() {
    let span = (OCTANT_END as u32 - OCTANT_START as u32 + 1) as usize;
    assert_eq!(span, OCTANT_MASKS.len(), "table length matches range span");

    let mut seen = [false; 256];
    for mask in OCTANT_MASKS {
        assert!(
            !seen[mask as usize],
            "duplicate mask 0x{mask:02X} — every codepoint should encode a unique subset",
        );
        seen[mask as usize] = true;
    }
    // 0x00 and 0xFF MUST be excluded (space + full block cover them).
    assert!(!seen[0x00], "empty mask 0x00 must not appear");
    assert!(!seen[0xFF], "full mask 0xFF must not appear");
}

/// **Braille-vs-octant rendering-model check.**
///
/// Both renderers operate on a 2×4 grid but use different bit orderings
/// and visual shapes. This test asserts the two renderers are NOT
/// interchangeable by rendering the same bitmask value through each
/// and confirming the output pixel buffers differ. If they ever become
/// byte-identical, either (a) the renderers have accidentally converged
/// (a hygiene regression), or (b) someone has shared a Canvas helper
/// across the two modules without preserving their distinct semantics.
///
/// Per `.claude/rules/impl-hygiene.md §Algorithmic DRY` — shared code
/// requires shared semantics. This guard prevents inadvertent DRY
/// extraction between the two 2×4 renderers.
#[test]
fn braille_and_octant_rendering_models_are_distinct() {
    use crate::gpu::builtin_glyphs::rasterize;

    // Octant U+1CD00 has mask 0x04 (upper-mid-left filled only).
    // Braille U+2804 has bit 2 set (left column, row 2 — lower-left per
    // Unicode-dot-order, NOT upper-mid-left). The two renderers must
    // produce visibly different canvases for the same bit-2 input.
    let octant_glyph =
        rasterize('\u{1CD00}', 20, 40).expect("octant U+1CD00 rasterizes via built-in");
    let braille_glyph =
        rasterize('\u{2804}', 20, 40).expect("braille U+2804 rasterizes via built-in");

    assert_ne!(
        octant_glyph.bitmap, braille_glyph.bitmap,
        "braille and octant renderers share geometry semantics — this is a regression: \
         octants use row-major bit numbering, braille uses Unicode-dot column-major order",
    );
}

/// Representative render check: U+1CD00 (mask 0x04, upper-mid-left only)
/// produces a canvas where the upper-mid-left quadrant is filled and
/// every other cell is empty.
#[test]
fn octant_u1cd00_renders_upper_mid_left_cell() {
    let mut canvas = Canvas::new(20, 40);
    super::octants::draw(&mut canvas, '\u{1CD00}');

    let glyph = canvas.into_rasterized_glyph();
    // In a 20×40 canvas with row-quarter = 10, the upper-mid row is
    // y ∈ [10, 20). The left column is x ∈ [0, 10). A pixel at (5, 15)
    // is inside that region and MUST be filled (255).
    let idx = (15_u32 * 20 + 5) as usize;
    assert_eq!(
        glyph.bitmap[idx], 255,
        "U+1CD00 must fill the upper-mid-left cell (mask 0x04)"
    );

    // A pixel at (15, 5) is in the top-right cell, which mask 0x04 does
    // NOT include. It must be empty (0).
    let idx = (5_u32 * 20 + 15) as usize;
    assert_eq!(
        glyph.bitmap[idx], 0,
        "U+1CD00 must NOT fill the top-right cell (mask 0x04 excludes bit 1)"
    );
}

/// Representative render check: U+1CDE5 (mask 0xFE — all bits set
/// except bit 0, i.e. every cell except top-left).
#[test]
fn octant_u1cde5_renders_all_but_top_left() {
    let mut canvas = Canvas::new(20, 40);
    super::octants::draw(&mut canvas, '\u{1CDE5}');

    let glyph = canvas.into_rasterized_glyph();
    // Top-left cell (x < 10, y < 10) must be empty.
    let top_left = (5_u32 * 20 + 5) as usize;
    assert_eq!(
        glyph.bitmap[top_left], 0,
        "U+1CDE5 mask 0xFE excludes bit 0 (top-left)"
    );
    // Bottom-right cell (x >= 10, y >= 30) must be filled.
    let bottom_right = (35_u32 * 20 + 15) as usize;
    assert_eq!(
        glyph.bitmap[bottom_right], 255,
        "U+1CDE5 mask 0xFE includes bit 7 (bottom-right)"
    );
}

/// Dispatch wiring: `is_builtin(ch)` returns `true` for every octant
/// codepoint; `rasterize(ch, ..)` returns `Some(..)` for every octant
/// codepoint — the font shaper must never see these.
#[test]
fn every_octant_codepoint_is_builtin_and_rasterizes() {
    use crate::font::is_builtin;
    use crate::gpu::builtin_glyphs::rasterize;

    for cp in (OCTANT_START as u32)..=(OCTANT_END as u32) {
        let ch = char::from_u32(cp).expect("octant codepoint valid");
        assert!(
            is_builtin(ch),
            "U+{cp:04X} must be a built-in glyph (font shaper would otherwise bypass the renderer)",
        );
        let glyph = rasterize(ch, 20, 40);
        assert!(
            glyph.is_some(),
            "U+{cp:04X} must rasterize via the built-in renderer",
        );
    }
}
