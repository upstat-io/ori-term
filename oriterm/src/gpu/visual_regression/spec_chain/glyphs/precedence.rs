//! Internal-renderer-takes-precedence tests.
//!
//! Proves `ori_term`'s built-in Canvas renderer wins unconditionally over a
//! configured font that advertises coverage of the subcell-glyph codepoints.
//!
//! The test injects `subcell-precedence-test.ttf` (a minimal TTF that
//! advertises coverage of one codepoint per family and would render a solid
//! filled em-square for each one if asked) into `VisualSpecHarness` via
//! `GoldenLaneConfig::with_font_override`, then renders each codepoint and
//! compares against the SAME golden PNG the `sparse_goldens.rs` tests
//! committed. A pass proves the built-in renderer is selected regardless of
//! the configured font's coverage claim — the SSOT requirement in
//! `.claude/rules/impl-hygiene.md`.
//!
//! If the font shaper ever leaks through (because a regression in
//! `font::is_builtin` or the built-in dispatch), the rendered output would
//! be a filled em-square instead of the real glyph, and the strict
//! pixel-exact comparison with the canonical golden would fail.

use oriterm_test_support::fixtures::SUBCELL_PRECEDENCE_TEST_FONT;
use oriterm_test_support::spec_chain::{
    ApexLayer, FrameInputExpectation, GoldenExpectation, GpuInstanceExpectation, RungName,
    ScenarioExpectations, SpecScenario, TextureExpectation,
};

use super::super::super::GoldenLaneConfig;
use super::super::visual_harness::VisualSpecHarness;
use crate::font::FontSet;

/// Helper: build a `VisualSpecHarness` whose deterministic-lane font is the
/// committed precedence-test fixture.
fn precedence_harness() -> Option<VisualSpecHarness> {
    let precedence_font = FontSet::from_test_bytes(
        SUBCELL_PRECEDENCE_TEST_FONT.to_vec(),
        "Subcell Precedence Test",
    );
    let config = GoldenLaneConfig::SPEC_DEFAULT.with_font_override(precedence_font);
    VisualSpecHarness::with_config(config)
}

/// Run a single-codepoint scenario through the 8-rung visual chain with a
/// specific pinned golden. Asserts pixel-exact equality against the SAME
/// golden the `sparse_goldens.rs` test committed — if the shaper wins, the
/// pixels disagree.
fn assert_builtin_wins(
    golden_name: &'static str,
    catalog_row_id: &'static str,
    bytes: &'static [u8],
    codepoint_hex: u32,
) {
    let Some(mut harness) = precedence_harness() else {
        eprintln!("SKIP: software rasterizer unavailable");
        return;
    };

    let scenario = SpecScenario {
        catalog_row_id,
        bytes,
        apex_layer: ApexLayer::GoldenImage,
        setup: b"",
        expectations: ScenarioExpectations {
            frame_input: Some(FrameInputExpectation::default_grid()),
            gpu_instance: Some(GpuInstanceExpectation::at_least(1, 1)),
            texture: Some(TextureExpectation {
                min_non_zero_pixels: Some(1),
                width: None,
                height: None,
            }),
            golden: Some(GoldenExpectation {
                golden_name: Some(golden_name),
            }),
            ..ScenarioExpectations::default()
        },
    };

    let results = harness.run_visual_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "precedence: rung {:?} failed for U+{:04X}: {}",
            r.rung_name,
            codepoint_hex,
            r.failure.as_deref().unwrap_or("(no message)"),
        );
    }
    let last_rung = results.last().map(|r| r.rung_name);
    assert_eq!(
        last_rung,
        Some(RungName::GoldenImage),
        "precedence: U+{codepoint_hex:04X} must drive through GoldenImage apex",
    );
}

/// Box drawing: the precedence font advertises U+256C with a solid filled
/// em-square glyph. The built-in renderer must win and produce the double-
/// cross pattern matching the canonical sparse golden.
#[test]
fn box_drawing_builtin_wins_over_configured_font() {
    const BYTES: &[u8] = "\u{256C}".as_bytes();
    assert_builtin_wins(
        "subcell_box_drawings_double_cross",
        "USC-BOX",
        BYTES,
        0x256C,
    );
}

/// Block elements: precedence font covers U+2588 with a solid em-square,
/// which coincidentally is what FULL BLOCK should render as — so this test
/// doesn't distinguish shaper-win from built-in-win for the full block. We
/// use U+259F (quadrant with 3 of 4 bits set) instead, which is a partial
/// fill — the shaper would produce a solid square while the built-in
/// produces the three-quadrant pattern.
#[test]
fn quadrant_builtin_wins_over_configured_font() {
    const BYTES: &[u8] = "\u{259F}".as_bytes();
    assert_builtin_wins("subcell_quadrant_ur_bl_br", "USC-BLOCKS", BYTES, 0x259F);
}

/// Sextants: precedence font covers U+1FB3B with a solid em-square. The
/// built-in renderer must win and produce the 5-of-6-subcells sextant
/// pattern matching the canonical sparse golden.
#[test]
fn sextant_builtin_wins_over_configured_font() {
    const BYTES: &[u8] = "\u{1FB3B}".as_bytes();
    assert_builtin_wins(
        "subcell_sextant_near_full",
        "USC-LEGACY-SEXTANT",
        BYTES,
        0x1FB3B,
    );
}

/// Octants: precedence font covers U+1CDE5 with a solid em-square. The
/// built-in renderer must win and produce the canonical octant pattern
/// from the §11.0 artifact, matching the sparse golden.
#[test]
fn octant_builtin_wins_over_configured_font() {
    const BYTES: &[u8] = "\u{1CDE5}".as_bytes();
    assert_builtin_wins(
        "subcell_octant_end_of_range",
        "USC-LEGACY-OCTANT",
        BYTES,
        0x1CDE5,
    );
}

/// Braille: precedence font covers U+28FF with a solid em-square. The
/// built-in renderer must win and produce the 8-dot braille pattern
/// matching the canonical sparse golden.
#[test]
fn braille_builtin_wins_over_configured_font() {
    const BYTES: &[u8] = "\u{28FF}".as_bytes();
    assert_builtin_wins("subcell_braille_all_dots", "USC-BRAILLE", BYTES, 0x28FF);
}
