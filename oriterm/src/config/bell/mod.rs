//! Visual bell configuration.

use oriterm_core::color::Rgb;
use oriterm_ui::animation::Easing;
use oriterm_ui::color::Color;
use serde::{Deserialize, Serialize};

/// Visual bell animation curve.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BellAnimation {
    #[default]
    EaseOut,
    Linear,
    None,
}

/// Visual bell configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct BellConfig {
    /// Visual bell animation curve.
    pub animation: BellAnimation,
    /// Duration in milliseconds (0 = disabled).
    pub duration_ms: u16,
    /// Flash color as "#RRGGBB" hex (default: white).
    pub color: Option<String>,
}

impl Default for BellConfig {
    fn default() -> Self {
        Self {
            animation: BellAnimation::default(),
            duration_ms: 150,
            color: None,
        }
    }
}

impl BellConfig {
    /// Returns true when the visual bell is enabled.
    #[allow(
        dead_code,
        reason = "wired in subsequent step within BUG-11-008 Commit 2 — mux_pump PaneBell arm"
    )]
    pub fn is_enabled(&self) -> bool {
        self.duration_ms > 0 && self.animation != BellAnimation::None
    }
}

/// Parses a `"#RRGGBB"` hex color string. Returns `None` for missing or
/// malformed values; consumers fall back to a sensible default (white).
pub(crate) fn parse_bell_color(color_str: Option<&str>) -> Option<Rgb> {
    let s = color_str?.strip_prefix('#')?;
    if s.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some(Rgb { r, g, b })
}

/// Parses a `"#RRGGBB"` config color string into the UI-side `Color`
/// type, defaulting to white when absent or malformed. The mux_pump
/// consumer uses this when invoking `WindowRoot::ring_visual_bell`.
#[allow(
    dead_code,
    reason = "wired in subsequent step within BUG-11-008 Commit 2 — mux_pump PaneBell arm"
)]
pub(crate) fn parse_bell_color_as_ui(color_str: Option<&str>) -> Color {
    let rgb = parse_bell_color(color_str).unwrap_or(Rgb {
        r: 255,
        g: 255,
        b: 255,
    });
    Color::from_rgb_u8(rgb.r, rgb.g, rgb.b)
}

/// Maps the config-side `BellAnimation` enum to the UI-side `Easing`
/// curve enum. `BellAnimation::None` is the "disabled" sentinel — when
/// `BellConfig::is_enabled()` returns false the caller skips the call
/// entirely; this mapping falls through to `Linear` defensively.
#[allow(
    dead_code,
    reason = "wired in subsequent step within BUG-11-008 Commit 2 — mux_pump PaneBell arm"
)]
pub(crate) fn bell_animation_to_easing(anim: BellAnimation) -> Easing {
    match anim {
        BellAnimation::EaseOut => Easing::EaseOut,
        BellAnimation::Linear => Easing::Linear,
        BellAnimation::None => Easing::Linear,
    }
}

#[cfg(test)]
mod tests;
