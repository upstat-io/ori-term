//! Centralized theming for the UI framework.
//!
//! [`UiTheme`] provides a single source of truth for colors and sizing tokens
//! used across all widget styles. Dark and light factories ensure consistency;
//! widget `*Style` structs derive their defaults from the theme via
//! `from_theme()`.

use crate::color::Color;

/// Centralized color and sizing tokens for the UI framework.
///
/// All widget `*Style::from_theme()` constructors read from these fields.
/// `dark()` matches the legacy `DEFAULT_*` constants — zero visual regression.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiTheme {
    /// Primary background (widget surfaces).
    pub bg_primary: Color,
    /// Secondary background (disabled surfaces, panels).
    pub bg_secondary: Color,
    /// Background on hover.
    pub bg_hover: Color,
    /// Background on press/active.
    pub bg_active: Color,
    /// Primary foreground (text, icons).
    pub fg_primary: Color,
    /// Secondary foreground (captions, metadata).
    pub fg_secondary: Color,
    /// Disabled foreground.
    pub fg_disabled: Color,
    /// Accent color (toggles, focus rings, checked states).
    pub accent: Color,
    /// Border color.
    pub border: Color,
    /// Shadow color (typically semi-transparent black).
    pub shadow: Color,
    /// Close button hover background (platform standard red).
    pub close_hover_bg: Color,
    /// Close button pressed background (darker red).
    pub close_pressed_bg: Color,
    /// Input field background (darker than surface).
    pub bg_input: Color,
    /// Card/raised surface background.
    pub bg_card: Color,
    /// Card hover background.
    pub bg_card_hover: Color,
    /// Very muted text (descriptions, version labels, section rules).
    pub fg_faint: Color,
    /// Accent tint for active/selected backgrounds (low opacity).
    pub accent_bg: Color,
    /// Stronger accent tint (selected card, active nav item).
    pub accent_bg_strong: Color,
    /// Accent hover color (lighter accent for hover states).
    pub accent_hover: Color,
    /// Danger color (for destructive actions).
    pub danger: Color,
    /// Success color.
    pub success: Color,
    /// Default corner radius in logical pixels.
    pub corner_radius: f32,
    /// Default spacing/gap in logical pixels.
    pub spacing: f32,
    /// Default font size in points.
    pub font_size: f32,
    /// Small font size in points.
    pub font_size_small: f32,
    /// Large font size in points.
    pub font_size_large: f32,
}

impl UiTheme {
    /// Dark theme matching the legacy `DEFAULT_*` constants.
    pub const fn dark() -> Self {
        Self {
            bg_primary: Color::from_rgb_u8(0x2D, 0x2D, 0x2D),
            bg_secondary: Color::from_rgb_u8(0x25, 0x25, 0x25),
            bg_hover: Color::from_rgb_u8(0x3D, 0x3D, 0x3D),
            bg_active: Color::from_rgb_u8(0x1D, 0x1D, 0x1D),
            fg_primary: Color::from_rgb_u8(0xE0, 0xE0, 0xE0),
            fg_secondary: Color::from_rgb_u8(0xA0, 0xA0, 0xA0),
            fg_disabled: Color::from_rgb_u8(0x80, 0x80, 0x80),
            accent: Color::from_rgb_u8(0x4A, 0x9E, 0xFF),
            border: Color::from_rgb_u8(0x55, 0x55, 0x55),
            shadow: Color::rgba(0.0, 0.0, 0.0, 0.5),
            close_hover_bg: Color::hex(0xC4_2B_1C),
            close_pressed_bg: Color::hex(0xA1_20_12),
            bg_input: Color::hex(0x12_12_1A),
            bg_card: Color::hex(0x1C_1C_24),
            bg_card_hover: Color::hex(0x24_24_2E),
            fg_faint: Color::hex(0x4E_4E_5E),
            accent_bg: Color::rgba(0.42, 0.55, 1.0, 0.08),
            accent_bg_strong: Color::rgba(0.42, 0.55, 1.0, 0.14),
            accent_hover: Color::hex(0x8A_A4_FF),
            danger: Color::hex(0xFF_6B_6B),
            success: Color::hex(0x6B_FF_B8),
            corner_radius: 4.0,
            spacing: 8.0,
            font_size: 13.0,
            font_size_small: 11.0,
            font_size_large: 16.0,
        }
    }

    /// Light theme for bright environments.
    pub const fn light() -> Self {
        Self {
            bg_primary: Color::from_rgb_u8(0xF5, 0xF5, 0xF5),
            bg_secondary: Color::from_rgb_u8(0xFF, 0xFF, 0xFF),
            bg_hover: Color::from_rgb_u8(0xE8, 0xE8, 0xE8),
            bg_active: Color::from_rgb_u8(0xD0, 0xD0, 0xD0),
            fg_primary: Color::from_rgb_u8(0x1A, 0x1A, 0x1A),
            fg_secondary: Color::from_rgb_u8(0x60, 0x60, 0x60),
            fg_disabled: Color::from_rgb_u8(0xA0, 0xA0, 0xA0),
            accent: Color::from_rgb_u8(0x00, 0x78, 0xD4),
            border: Color::from_rgb_u8(0xCC, 0xCC, 0xCC),
            shadow: Color::rgba(0.0, 0.0, 0.0, 0.15),
            close_hover_bg: Color::hex(0xC4_2B_1C),
            close_pressed_bg: Color::hex(0xA1_20_12),
            bg_input: Color::from_rgb_u8(0xE8, 0xE8, 0xF0),
            bg_card: Color::from_rgb_u8(0xFF, 0xFF, 0xFF),
            bg_card_hover: Color::from_rgb_u8(0xF0, 0xF0, 0xF5),
            fg_faint: Color::from_rgb_u8(0x99, 0x99, 0xAA),
            accent_bg: Color::rgba(0.0, 0.47, 0.83, 0.08),
            accent_bg_strong: Color::rgba(0.0, 0.47, 0.83, 0.14),
            accent_hover: Color::from_rgb_u8(0x00, 0x5A, 0x9E),
            danger: Color::from_rgb_u8(0xD3, 0x2F, 0x2F),
            success: Color::from_rgb_u8(0x2E, 0x7D, 0x32),
            corner_radius: 4.0,
            spacing: 8.0,
            font_size: 13.0,
            font_size_small: 11.0,
            font_size_large: 16.0,
        }
    }
}

impl Default for UiTheme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests;
