//! Tests for GoldenLaneConfig.

use super::GoldenLaneConfig;
use crate::font::{GlyphFormat, HintingMode};

#[test]
fn spec_default_uses_grayscale_alpha_hinting() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    assert_eq!(config.hinting_mode, HintingMode::None);
    assert_eq!(config.glyph_format, GlyphFormat::Alpha);
}

#[test]
fn spec_default_disables_subpixel_positioning() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    assert!(!config.subpixel_positioning);
}

#[test]
fn spec_default_uses_exact_tolerance() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    assert_eq!(config.pixel_tolerance, 0);
    assert_eq!(config.max_diff_percent, 0.0);
}

#[test]
fn spec_default_viewport_is_80x24() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    assert_eq!(config.viewport_cols, 80);
    assert_eq!(config.viewport_rows, 24);
}

#[test]
fn spec_default_font_size_matches_test_font() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    assert_eq!(config.font_size_pt, 12.0);
    assert_eq!(config.dpi, 96.0);
}

#[test]
fn config_clone_is_independent() {
    let config = GoldenLaneConfig::SPEC_DEFAULT;
    let mut cloned = config.clone();
    cloned.pixel_tolerance = 1;
    // Original unchanged.
    assert_eq!(GoldenLaneConfig::SPEC_DEFAULT.pixel_tolerance, 0);
    assert_eq!(cloned.pixel_tolerance, 1);
}
