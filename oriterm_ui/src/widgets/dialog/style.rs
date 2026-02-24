//! Visual style for the dialog widget.

use crate::animation::Lerp;
use crate::color::Color;
use crate::draw::Shadow;
use crate::geometry::Insets;
use crate::theme::UiTheme;
use crate::widgets::button::ButtonStyle;

/// Visual style for a [`super::DialogWidget`].
#[derive(Debug, Clone, PartialEq)]
pub struct DialogStyle {
    /// Panel background color.
    pub bg: Color,
    /// Border color.
    pub border_color: Color,
    /// Border width.
    pub border_width: f32,
    /// Corner radius.
    pub corner_radius: f32,
    /// Drop shadow.
    pub shadow: Option<Shadow>,
    /// Content zone padding (top/left/right surround title+message+preview).
    pub padding: Insets,
    /// Title text color.
    pub title_fg: Color,
    /// Title font size.
    pub title_font_size: f32,
    /// Message text color.
    pub message_fg: Color,
    /// Message font size.
    pub message_font_size: f32,
    /// Spacing between content items (title, message, preview).
    pub content_spacing: f32,
    /// Gap between buttons in the footer.
    pub button_gap: f32,
    /// Style for the default (primary) button.
    pub primary_button: ButtonStyle,
    /// Style for the non-default (secondary) button.
    pub secondary_button: ButtonStyle,
    /// Background for the button footer zone.
    pub footer_bg: Color,
    /// Padding inside the button footer zone.
    pub footer_padding: Insets,
    /// 1px separator line between content and footer.
    pub separator_color: Color,
    /// Background for the optional preview block.
    pub preview_bg: Color,
    /// Padding inside the preview block.
    pub preview_padding: Insets,
    /// Corner radius for the preview block.
    pub preview_radius: f32,
    /// Maximum height for the preview block before clipping.
    pub preview_max_height: f32,
}

impl DialogStyle {
    /// Derives a dialog style from the given theme.
    pub fn from_theme(theme: &UiTheme) -> Self {
        let mut primary_button = ButtonStyle::from_theme(theme);
        primary_button.bg = theme.accent;
        primary_button.hover_bg = Color::lerp(theme.accent, Color::WHITE, 0.15);
        primary_button.pressed_bg = Color::lerp(theme.accent, Color::BLACK, 0.15);
        primary_button.fg = Color::WHITE;
        primary_button.border_color = theme.accent;

        Self {
            bg: theme.bg_primary,
            border_color: theme.border,
            border_width: 1.0,
            corner_radius: theme.corner_radius * 2.0,
            shadow: Some(Shadow {
                offset_x: 0.0,
                offset_y: 4.0,
                blur_radius: 16.0,
                spread: 0.0,
                color: theme.shadow,
            }),
            padding: Insets::all(24.0),
            title_fg: theme.fg_primary,
            title_font_size: theme.font_size_large,
            message_fg: theme.fg_secondary,
            message_font_size: theme.font_size,
            content_spacing: 12.0,
            button_gap: theme.spacing,
            primary_button,
            secondary_button: ButtonStyle::from_theme(theme),
            footer_bg: theme.bg_secondary,
            footer_padding: Insets::vh(12.0, 24.0),
            separator_color: theme.border,
            preview_bg: theme.bg_active,
            preview_padding: Insets::all(12.0),
            preview_radius: theme.corner_radius,
            preview_max_height: 100.0,
        }
    }
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self::from_theme(&UiTheme::dark())
    }
}
