//! Tests for font types defined in `font/mod.rs`.

use super::{GlyphFormat, HintingMode, SubpixelMode};

// ── SubpixelMode ──

#[test]
fn subpixel_mode_default_is_rgb() {
    assert_eq!(SubpixelMode::default(), SubpixelMode::Rgb);
}

#[test]
fn subpixel_mode_from_scale_factor_low_dpi() {
    assert_eq!(
        SubpixelMode::from_scale_factor(1.0),
        SubpixelMode::Rgb,
        "1x scale → RGB subpixel",
    );
    assert_eq!(
        SubpixelMode::from_scale_factor(1.5),
        SubpixelMode::Rgb,
        "1.5x scale → RGB subpixel",
    );
}

#[test]
fn subpixel_mode_from_scale_factor_high_dpi() {
    assert_eq!(
        SubpixelMode::from_scale_factor(2.0),
        SubpixelMode::None,
        "2x scale → disabled",
    );
    assert_eq!(
        SubpixelMode::from_scale_factor(3.0),
        SubpixelMode::None,
        "3x scale → disabled",
    );
}

#[test]
fn subpixel_mode_glyph_format() {
    assert_eq!(SubpixelMode::Rgb.glyph_format(), GlyphFormat::SubpixelRgb);
    assert_eq!(SubpixelMode::Bgr.glyph_format(), GlyphFormat::SubpixelBgr);
    assert_eq!(SubpixelMode::None.glyph_format(), GlyphFormat::Alpha);
}

#[test]
fn subpixel_none_forces_alpha_regardless_of_scale() {
    // Config "none" overrides scale factor — always produces Alpha (grayscale).
    assert_eq!(
        SubpixelMode::None.glyph_format(),
        GlyphFormat::Alpha,
        "None at any scale → Alpha",
    );
    // Even though 1x scale would normally give RGB, explicit None wins.
    assert_ne!(
        SubpixelMode::None.glyph_format(),
        SubpixelMode::from_scale_factor(1.0).glyph_format(),
        "explicit None differs from auto-detected 1x",
    );
}

#[test]
fn subpixel_rgb_and_bgr_are_distinct() {
    let rgb = SubpixelMode::Rgb.glyph_format();
    let bgr = SubpixelMode::Bgr.glyph_format();

    // Both are subpixel formats.
    assert!(rgb.is_subpixel());
    assert!(bgr.is_subpixel());

    // But they are not equal — channel order differs.
    assert_ne!(
        rgb, bgr,
        "RGB and BGR produce different GlyphFormat variants"
    );
}

// ── SubpixelMode::for_display (transparent background fallback) ──

#[test]
fn subpixel_for_display_opaque_uses_scale_factor() {
    // Fully opaque background delegates to scale factor logic.
    assert_eq!(
        SubpixelMode::for_display(1.0, 1.0),
        SubpixelMode::Rgb,
        "opaque + 1x → RGB",
    );
    assert_eq!(
        SubpixelMode::for_display(2.0, 1.0),
        SubpixelMode::None,
        "opaque + 2x → None (HiDPI)",
    );
}

#[test]
fn subpixel_for_display_transparent_forces_none() {
    // Transparent background disables subpixel regardless of scale factor.
    assert_eq!(
        SubpixelMode::for_display(1.0, 0.9),
        SubpixelMode::None,
        "transparent + 1x → None (fringing prevention)",
    );
    assert_eq!(
        SubpixelMode::for_display(1.0, 0.5),
        SubpixelMode::None,
        "half-transparent + 1x → None",
    );
    assert_eq!(
        SubpixelMode::for_display(1.0, 0.0),
        SubpixelMode::None,
        "fully transparent + 1x → None",
    );
}

// ── GlyphFormat ──

#[test]
fn glyph_format_bytes_per_pixel() {
    assert_eq!(GlyphFormat::Alpha.bytes_per_pixel(), 1);
    assert_eq!(GlyphFormat::SubpixelRgb.bytes_per_pixel(), 4);
    assert_eq!(GlyphFormat::SubpixelBgr.bytes_per_pixel(), 4);
    assert_eq!(GlyphFormat::Color.bytes_per_pixel(), 4);
}

#[test]
fn glyph_format_is_subpixel() {
    assert!(GlyphFormat::SubpixelRgb.is_subpixel());
    assert!(GlyphFormat::SubpixelBgr.is_subpixel());
    assert!(!GlyphFormat::Alpha.is_subpixel());
    assert!(!GlyphFormat::Color.is_subpixel());
}

// ── HintingMode ──

#[test]
fn hinting_mode_default_is_full() {
    assert_eq!(HintingMode::default(), HintingMode::Full);
}

#[test]
fn hinting_mode_hint_flag() {
    assert!(HintingMode::Full.hint_flag());
    assert!(!HintingMode::None.hint_flag());
}
