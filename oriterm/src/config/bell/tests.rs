use oriterm_ui::animation::Easing;

use super::{BellAnimation, BellConfig, bell_animation_to_easing, parse_bell_color, parse_bell_color_as_ui};

/// Regression: BUG-11-008 — `BellConfig::is_enabled()` is the gate that
/// `mux_pump` uses to decide whether to ring the visual flash overlay.
/// Default config (animation EaseOut, 150ms) is enabled; a 0ms or None
/// animation disables.
#[test]
fn bell_config_default_is_enabled() {
    assert!(BellConfig::default().is_enabled());
}

/// Regression: BUG-11-008 — duration_ms == 0 disables visual flash.
#[test]
fn bell_config_zero_duration_is_disabled() {
    let cfg = BellConfig {
        duration_ms: 0,
        ..BellConfig::default()
    };
    assert!(!cfg.is_enabled());
}

/// Regression: BUG-11-008 — animation None disables visual flash.
#[test]
fn bell_config_animation_none_is_disabled() {
    let cfg = BellConfig {
        animation: BellAnimation::None,
        ..BellConfig::default()
    };
    assert!(!cfg.is_enabled());
}

/// Regression: BUG-11-008 — parse `"#RRGGBB"` to `Rgb`.
#[test]
fn parse_bell_color_accepts_well_formed_hex() {
    let c = parse_bell_color(Some("#ff8000")).expect("valid hex");
    assert_eq!(c.r, 0xff);
    assert_eq!(c.g, 0x80);
    assert_eq!(c.b, 0x00);
}

/// Regression: BUG-11-008 — None / malformed input returns None so
/// `parse_bell_color_as_ui` can fall back to default white.
#[test]
fn parse_bell_color_rejects_malformed() {
    assert!(parse_bell_color(None).is_none());
    assert!(parse_bell_color(Some("ff8000")).is_none()); // missing #
    assert!(parse_bell_color(Some("#ff80")).is_none()); // too short
    assert!(parse_bell_color(Some("#gghhii")).is_none()); // non-hex
}

/// Regression: BUG-11-008 — `parse_bell_color_as_ui` produces white as
/// the default when the config is absent or malformed; mux_pump relies
/// on this to never pass a "None" color into `ring_visual_bell`.
#[test]
fn parse_bell_color_as_ui_defaults_to_white_on_missing() {
    let c = parse_bell_color_as_ui(None);
    let white = oriterm_ui::color::Color::from_rgb_u8(255, 255, 255);
    assert_eq!(c, white);
}

/// Regression: BUG-11-008 — `bell_animation_to_easing` maps the config
/// enum to the UI-side `Easing` curve enum so `WindowRoot::ring_visual_bell`
/// receives the right curve. Verifies the mapping is correct.
#[test]
fn bell_animation_to_easing_maps_curves_correctly() {
    assert_eq!(bell_animation_to_easing(BellAnimation::EaseOut), Easing::EaseOut);
    assert_eq!(bell_animation_to_easing(BellAnimation::Linear), Easing::Linear);
    // None defensively falls through to Linear; mux_pump's is_enabled() gate
    // means this branch never fires in production.
    assert_eq!(bell_animation_to_easing(BellAnimation::None), Easing::Linear);
}
