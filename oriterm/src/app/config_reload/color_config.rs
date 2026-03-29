//! Color config helpers — palette construction and color overrides.

use crate::config;

/// Apply color overrides from [`ColorConfig`](crate::config::ColorConfig) to a palette.
///
/// Sets both live and default values so OSC 104 resets to config values.
pub(crate) fn apply_color_overrides(
    palette: &mut oriterm_core::Palette,
    colors: &config::ColorConfig,
) {
    if let Some(rgb) = colors
        .foreground
        .as_deref()
        .and_then(config::parse_hex_color)
    {
        palette.set_foreground(rgb);
    }
    if let Some(rgb) = colors
        .background
        .as_deref()
        .and_then(config::parse_hex_color)
    {
        palette.set_background(rgb);
    }
    if let Some(rgb) = colors.cursor.as_deref().and_then(config::parse_hex_color) {
        palette.set_cursor_color(rgb);
    }

    // ANSI colors 0–7.
    for (key, hex) in &colors.ansi {
        if let (Ok(idx), Some(rgb)) = (key.parse::<usize>(), config::parse_hex_color(hex)) {
            if idx < 8 {
                palette.set_default(idx, rgb);
            } else {
                log::warn!("config: ansi color index {idx} out of range 0-7");
            }
        }
    }

    // Bright ANSI colors: keys 0–7 map to palette indices 8–15.
    for (key, hex) in &colors.bright {
        if let (Ok(idx), Some(rgb)) = (key.parse::<usize>(), config::parse_hex_color(hex)) {
            if idx < 8 {
                palette.set_default(idx + 8, rgb);
            } else {
                log::warn!("config: bright color index {idx} out of range 0-7");
            }
        }
    }

    // Selection color overrides.
    if let Some(rgb) = colors
        .selection_foreground
        .as_deref()
        .and_then(config::parse_hex_color)
    {
        palette.set_selection_fg(Some(rgb));
    }
    if let Some(rgb) = colors
        .selection_background
        .as_deref()
        .and_then(config::parse_hex_color)
    {
        palette.set_selection_bg(Some(rgb));
    }
}

/// Build a palette from the configured color scheme and theme.
///
/// Resolves the scheme name (supporting conditional `"dark:X, light:Y"` syntax),
/// looks up the scheme (built-in then TOML file), builds the palette from scheme
/// colors, and applies user color overrides on top. Falls back to the default
/// theme-based palette if the scheme cannot be found.
pub(crate) fn build_palette_from_config(
    colors: &config::ColorConfig,
    theme: oriterm_core::Theme,
) -> oriterm_core::Palette {
    use crate::scheme;

    let scheme_name = scheme::resolve_scheme_name(&colors.scheme, theme);
    let mut palette = if let Some(s) = scheme::resolve_scheme(scheme_name) {
        log::info!("scheme: resolved '{scheme_name}' -> '{}'", s.name);
        scheme::palette_from_scheme(&s)
    } else {
        if !colors.scheme.is_empty() {
            log::warn!("scheme: '{scheme_name}' not found, using defaults");
        }
        oriterm_core::Palette::for_theme(theme)
    };
    apply_color_overrides(&mut palette, colors);
    palette
}
