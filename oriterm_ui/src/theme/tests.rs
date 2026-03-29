use crate::color::Color;

use super::UiTheme;

#[test]
fn default_is_dark() {
    assert_eq!(UiTheme::default(), UiTheme::dark());
}

#[test]
fn dark_bg_primary_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_primary, Color::hex(0x16_16_1C)); // --bg-surface
}

#[test]
fn dark_bg_secondary_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_secondary, Color::hex(0x0E_0E_12)); // --bg-base
}

#[test]
fn dark_bg_hover_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_hover, Color::hex(0x24_24_2E)); // --bg-hover
}

#[test]
fn dark_bg_active_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.bg_active, Color::hex(0x2A_2A_36)); // --bg-active
}

#[test]
fn dark_fg_primary_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.fg_primary, Color::hex(0xD4_D4_DC)); // --text
}

#[test]
fn dark_fg_secondary_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.fg_secondary, Color::hex(0x94_94_A8)); // --text-muted
}

#[test]
fn dark_fg_faint_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.fg_faint, Color::hex(0x8C_8C_A0)); // --text-faint
}

#[test]
fn dark_fg_bright_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.fg_bright, Color::hex(0xEE_EE_EF)); // --text-bright
}

#[test]
fn dark_accent_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.accent, Color::hex(0x6D_9B_E0)); // --accent
}

#[test]
fn dark_accent_hover_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.accent_hover, Color::hex(0x85_AD_E8)); // --accent-hover
}

#[test]
fn dark_border_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.border, Color::hex(0x2A_2A_36)); // --border
}

#[test]
fn dark_border_strong_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.border_strong, Color::hex(0x3A_3A_48)); // --border-strong
}

#[test]
fn dark_border_subtle_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.border_subtle, Color::hex(0x1E_1E_28)); // --border-subtle
}

#[test]
fn dark_danger_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.danger, Color::hex(0xC8_78_78)); // --danger
}

#[test]
fn dark_danger_hover_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.danger_hover, Color::hex(0xD8_90_90)); // --danger-hover
}

#[test]
fn dark_warning_matches_mockup() {
    let dark = UiTheme::dark();
    assert_eq!(dark.warning, Color::hex(0xE0_C4_54)); // --warning
}

#[test]
fn dark_shadow_is_transparent() {
    let dark = UiTheme::dark();
    assert_eq!(dark.shadow, Color::TRANSPARENT); // --shadow: none
}

#[test]
fn dark_disabled_fg_unchanged() {
    let dark = UiTheme::dark();
    assert_eq!(dark.fg_disabled, Color::from_rgb_u8(0x80, 0x80, 0x80));
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
    assert_ne!(dark.border_strong, light.border_strong);
    assert_ne!(dark.shadow, light.shadow);
    assert_ne!(dark.bg_input, light.bg_input);
    assert_ne!(dark.bg_card, light.bg_card);
    assert_ne!(dark.bg_card_hover, light.bg_card_hover);
    assert_ne!(dark.fg_faint, light.fg_faint);
    assert_ne!(dark.fg_bright, light.fg_bright);
    assert_ne!(dark.accent_bg, light.accent_bg);
    assert_ne!(dark.accent_bg_strong, light.accent_bg_strong);
    assert_ne!(dark.accent_hover, light.accent_hover);
    assert_ne!(dark.border_subtle, light.border_subtle);
    assert_ne!(dark.danger, light.danger);
    assert_ne!(dark.danger_hover, light.danger_hover);
    assert_ne!(dark.danger_bg, light.danger_bg);
    assert_ne!(dark.success, light.success);
    assert_ne!(dark.warning, light.warning);
    assert_ne!(dark.warning_bg, light.warning_bg);
}

#[test]
fn light_sizing_matches_dark() {
    let dark = UiTheme::dark();
    let light = UiTheme::light();
    // corner_radius diverges: dark=0.0 (brutal), light=4.0 (soft).
    assert_eq!(dark.corner_radius, 0.0);
    assert_eq!(light.corner_radius, 4.0);
    assert_eq!(dark.spacing, light.spacing);
    assert_eq!(dark.font_size, light.font_size);
    assert_eq!(dark.font_size_small, light.font_size_small);
    assert_eq!(dark.font_size_large, light.font_size_large);
}

#[test]
fn dark_extended_tokens_are_non_default() {
    let dark = UiTheme::dark();
    let zero = Color::TRANSPARENT;
    assert_ne!(dark.bg_input, zero);
    assert_ne!(dark.bg_card, zero);
    assert_ne!(dark.bg_card_hover, zero);
    assert_ne!(dark.fg_faint, zero);
    assert_ne!(dark.fg_bright, zero);
    assert_ne!(dark.accent_bg, zero);
    assert_ne!(dark.accent_bg_strong, zero);
    assert_ne!(dark.accent_hover, zero);
    assert_ne!(dark.border_subtle, zero);
    assert_ne!(dark.danger, zero);
    assert_ne!(dark.danger_hover, zero);
    assert_ne!(dark.danger_bg, zero);
    assert_ne!(dark.success, zero);
    assert_ne!(dark.warning, zero);
    assert_ne!(dark.warning_bg, zero);
    assert_ne!(dark.border_strong, zero);
}

#[test]
fn light_extended_tokens_are_non_default() {
    let light = UiTheme::light();
    let zero = Color::TRANSPARENT;
    assert_ne!(light.bg_input, zero);
    assert_ne!(light.bg_card, zero);
    assert_ne!(light.bg_card_hover, zero);
    assert_ne!(light.fg_faint, zero);
    assert_ne!(light.fg_bright, zero);
    assert_ne!(light.accent_bg, zero);
    assert_ne!(light.accent_bg_strong, zero);
    assert_ne!(light.accent_hover, zero);
    assert_ne!(light.border_subtle, zero);
    assert_ne!(light.danger, zero);
    assert_ne!(light.danger_hover, zero);
    assert_ne!(light.danger_bg, zero);
    assert_ne!(light.success, zero);
    assert_ne!(light.warning, zero);
    assert_ne!(light.warning_bg, zero);
    assert_ne!(light.border_strong, zero);
}
