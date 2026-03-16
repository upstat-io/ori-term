use crate::color::Color;

use super::UiTheme;

#[test]
fn default_is_dark() {
    assert_eq!(UiTheme::default(), UiTheme::dark());
}

#[test]
fn dark_matches_legacy_default_bg() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_primary, Color::from_rgb_u8(0x2D, 0x2D, 0x2D));
}

#[test]
fn dark_matches_legacy_default_hover_bg() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_hover, Color::from_rgb_u8(0x3D, 0x3D, 0x3D));
}

#[test]
fn dark_matches_legacy_default_pressed_bg() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_active, Color::from_rgb_u8(0x1D, 0x1D, 0x1D));
}

#[test]
fn dark_matches_legacy_default_fg() {
    let dark = UiTheme::dark();
    assert_eq!(dark.fg_primary, Color::from_rgb_u8(0xE0, 0xE0, 0xE0));
}

#[test]
fn dark_matches_legacy_default_border() {
    let dark = UiTheme::dark();
    assert_eq!(dark.border, Color::from_rgb_u8(0x55, 0x55, 0x55));
}

#[test]
fn dark_matches_legacy_default_accent() {
    let dark = UiTheme::dark();
    assert_eq!(dark.accent, Color::from_rgb_u8(0x4A, 0x9E, 0xFF));
}

#[test]
fn dark_matches_legacy_default_disabled_fg() {
    let dark = UiTheme::dark();
    assert_eq!(dark.fg_disabled, Color::from_rgb_u8(0x80, 0x80, 0x80));
}

#[test]
fn dark_matches_legacy_default_disabled_bg() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_secondary, Color::from_rgb_u8(0x25, 0x25, 0x25));
}

#[test]
fn dark_matches_legacy_default_focus_ring() {
    let dark = UiTheme::dark();
    // Legacy DEFAULT_FOCUS_RING was the same as DEFAULT_ACCENT.
    assert_eq!(dark.accent, Color::from_rgb_u8(0x4A, 0x9E, 0xFF));
}

#[test]
fn light_differs_from_dark_on_all_colors() {
    let dark = UiTheme::dark();
    let light = UiTheme::light();
    assert_ne!(dark.bg_primary, light.bg_primary);
    assert_ne!(dark.bg_secondary, light.bg_secondary);
    assert_ne!(dark.bg_hover, light.bg_hover);
    assert_ne!(dark.bg_active, light.bg_active);
    assert_ne!(dark.fg_primary, light.fg_primary);
    assert_ne!(dark.fg_secondary, light.fg_secondary);
    assert_ne!(dark.fg_disabled, light.fg_disabled);
    assert_ne!(dark.accent, light.accent);
    assert_ne!(dark.border, light.border);
    assert_ne!(dark.shadow, light.shadow);
    assert_ne!(dark.bg_input, light.bg_input);
    assert_ne!(dark.bg_card, light.bg_card);
    assert_ne!(dark.bg_card_hover, light.bg_card_hover);
    assert_ne!(dark.fg_faint, light.fg_faint);
    assert_ne!(dark.accent_bg, light.accent_bg);
    assert_ne!(dark.accent_bg_strong, light.accent_bg_strong);
    assert_ne!(dark.accent_hover, light.accent_hover);
    assert_ne!(dark.danger, light.danger);
    assert_ne!(dark.success, light.success);
}

#[test]
fn light_sizing_matches_dark() {
    let dark = UiTheme::dark();
    let light = UiTheme::light();
    assert_eq!(dark.corner_radius, light.corner_radius);
    assert_eq!(dark.spacing, light.spacing);
    assert_eq!(dark.font_size, light.font_size);
    assert_eq!(dark.font_size_small, light.font_size_small);
    assert_eq!(dark.font_size_large, light.font_size_large);
}

#[test]
fn dark_shadow_is_semi_transparent() {
    let dark = UiTheme::dark();
    assert_eq!(dark.shadow.r, 0.0);
    assert_eq!(dark.shadow.g, 0.0);
    assert_eq!(dark.shadow.b, 0.0);
    assert!((dark.shadow.a - 0.5).abs() < f32::EPSILON);
}

#[test]
fn light_shadow_is_less_opaque() {
    let light = UiTheme::light();
    let dark = UiTheme::dark();
    assert!(light.shadow.a < dark.shadow.a);
}

#[test]
fn dark_extended_tokens_are_non_default() {
    let dark = UiTheme::dark();
    let zero = Color::TRANSPARENT;
    assert_ne!(dark.bg_input, zero);
    assert_ne!(dark.bg_card, zero);
    assert_ne!(dark.bg_card_hover, zero);
    assert_ne!(dark.fg_faint, zero);
    assert_ne!(dark.accent_bg, zero);
    assert_ne!(dark.accent_bg_strong, zero);
    assert_ne!(dark.accent_hover, zero);
    assert_ne!(dark.danger, zero);
    assert_ne!(dark.success, zero);
}

#[test]
fn light_extended_tokens_are_non_default() {
    let light = UiTheme::light();
    let zero = Color::TRANSPARENT;
    assert_ne!(light.bg_input, zero);
    assert_ne!(light.bg_card, zero);
    assert_ne!(light.bg_card_hover, zero);
    assert_ne!(light.fg_faint, zero);
    assert_ne!(light.accent_bg, zero);
    assert_ne!(light.accent_bg_strong, zero);
    assert_ne!(light.accent_hover, zero);
    assert_ne!(light.danger, zero);
    assert_ne!(light.success, zero);
}
