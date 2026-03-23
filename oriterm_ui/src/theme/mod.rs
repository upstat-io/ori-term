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
/// `dark()` uses the brutal mockup palette (`mockups/settings-brutal.html`).
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
    /// Strong border (panel edges, footer separators).
    pub border_strong: Color,
    /// Shadow color (transparent for flat/brutal design).
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
    /// Bright text (page titles, emphasized headings).
    pub fg_bright: Color,
    /// Accent tint for active/selected backgrounds (low opacity).
    pub accent_bg: Color,
    /// Stronger accent tint (selected card, active nav item).
    pub accent_bg_strong: Color,
    /// Accent hover color (lighter accent for hover states).
    pub accent_hover: Color,
    /// Subtle border (separators, dividers — lighter than `border`).
    pub border_subtle: Color,
    /// Danger color (for destructive actions).
    pub danger: Color,
    /// Danger hover color (lighter danger for hover states).
    pub danger_hover: Color,
    /// Subtle danger background tint (destructive action hover bg).
    pub danger_bg: Color,
    /// Success color.
    pub success: Color,
    /// Warning color (caution indicators, alerts).
    pub warning: Color,
    /// Subtle warning background tint (callout backgrounds).
    pub warning_bg: Color,
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
    /// Dark theme matching the brutal design mockup CSS variables.
    pub const fn dark() -> Self {
        Self {
            bg_primary: Color::hex(0x16_16_1C),   // --bg-surface
            bg_secondary: Color::hex(0x0E_0E_12), // --bg-base (sidebar)
            bg_hover: Color::hex(0x24_24_2E),     // --bg-hover
            bg_active: Color::hex(0x2A_2A_36),    // --bg-active
            fg_primary: Color::hex(0xD4_D4_DC),   // --text
            fg_secondary: Color::hex(0x94_94_A8), // --text-muted
            fg_disabled: Color::from_rgb_u8(0x80, 0x80, 0x80),
            accent: Color::hex(0x6D_9B_E0),        // --accent
            border: Color::hex(0x2A_2A_36),        // --border
            border_strong: Color::hex(0x3A_3A_48), // --border-strong
            shadow: Color::TRANSPARENT,            // --shadow: none
            close_hover_bg: Color::hex(0xC4_2B_1C),
            close_pressed_bg: Color::hex(0xA1_20_12),
            bg_input: Color::hex(0x12_12_1A),      // --bg-input
            bg_card: Color::hex(0x1C_1C_24),       // --bg-raised
            bg_card_hover: Color::hex(0x24_24_2E), // --bg-hover (cards)
            fg_faint: Color::hex(0x8C_8C_A0),      // --text-faint
            fg_bright: Color::hex(0xEE_EE_EF),     // --text-bright
            accent_bg: Color::rgba(0.427, 0.608, 0.878, 0.08), // --accent-bg
            accent_bg_strong: Color::rgba(0.427, 0.608, 0.878, 0.14), // --accent-bg-strong
            accent_hover: Color::hex(0x85_AD_E8),  // --accent-hover
            border_subtle: Color::hex(0x1E_1E_28), // --border-subtle
            danger: Color::hex(0xC8_78_78),        // --danger
            danger_hover: Color::hex(0xD8_90_90),  // --danger-hover
            danger_bg: Color::rgba(0.784, 0.471, 0.471, 0.08), // --danger-bg
            success: Color::hex(0x6B_FF_B8),       // --success
            warning: Color::hex(0xE0_C4_54),       // --warning
            warning_bg: Color::rgba(0.878, 0.769, 0.329, 0.08), // --warning-bg
            corner_radius: 0.0,                    // --radius: 0px (brutal)
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
            border_strong: Color::from_rgb_u8(0xAA, 0xAA, 0xAA),
            shadow: Color::rgba(0.0, 0.0, 0.0, 0.15),
            close_hover_bg: Color::hex(0xC4_2B_1C),
            close_pressed_bg: Color::hex(0xA1_20_12),
            bg_input: Color::from_rgb_u8(0xE8, 0xE8, 0xF0),
            bg_card: Color::from_rgb_u8(0xFF, 0xFF, 0xFF),
            bg_card_hover: Color::from_rgb_u8(0xF0, 0xF0, 0xF5),
            fg_faint: Color::from_rgb_u8(0x99, 0x99, 0xAA),
            fg_bright: Color::from_rgb_u8(0x0A, 0x0A, 0x0A),
            accent_bg: Color::rgba(0.0, 0.47, 0.83, 0.08),
            accent_bg_strong: Color::rgba(0.0, 0.47, 0.83, 0.14),
            accent_hover: Color::from_rgb_u8(0x00, 0x5A, 0x9E),
            border_subtle: Color::from_rgb_u8(0xE0, 0xE0, 0xE0),
            danger: Color::from_rgb_u8(0xD3, 0x2F, 0x2F),
            danger_hover: Color::from_rgb_u8(0xE0, 0x45, 0x45),
            danger_bg: Color::rgba(0.827, 0.184, 0.184, 0.08),
            success: Color::from_rgb_u8(0x2E, 0x7D, 0x32),
            warning: Color::from_rgb_u8(0xED, 0x6C, 0x02),
            warning_bg: Color::rgba(0.929, 0.424, 0.008, 0.08),
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
