//! Color scheme resolution and built-in definitions.
//!
//! Resolves `ColorConfig.scheme` to a [`ColorScheme`] via built-in lookup or
//! TOML theme file loading. Supports conditional `"dark:X, light:Y"` syntax
//! for automatic light/dark switching.

mod builtin;
mod loader;

use oriterm_core::{Palette, Rgb, Theme};

use builtin::BUILTIN_SCHEMES;

/// Whether a color scheme is dark or light.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemeBrightness {
    /// Dark background, light text.
    Dark,
    /// Light background, dark text.
    Light,
}

/// A resolved color scheme with 16 ANSI colors and semantic colors.
#[derive(Debug, Clone)]
pub(crate) struct ColorScheme {
    /// Display name.
    pub name: String,
    /// ANSI colors 0–15.
    pub ansi: [Rgb; 16],
    /// Default foreground.
    pub fg: Rgb,
    /// Default background.
    pub bg: Rgb,
    /// Cursor color.
    pub cursor: Rgb,
    /// Explicit selection foreground (scheme-provided, may be `None`).
    pub selection_fg: Option<Rgb>,
    /// Explicit selection background (scheme-provided, may be `None`).
    pub selection_bg: Option<Rgb>,
}

/// A built-in scheme definition (compile-time constant).
struct BuiltinScheme {
    name: &'static str,
    ansi: [Rgb; 16],
    fg: Rgb,
    bg: Rgb,
    cursor: Rgb,
}

impl BuiltinScheme {
    /// Convert to a full [`ColorScheme`] (no selection colors).
    fn to_scheme(&self) -> ColorScheme {
        ColorScheme {
            name: self.name.to_owned(),
            ansi: self.ansi,
            fg: self.fg,
            bg: self.bg,
            cursor: self.cursor,
            selection_fg: None,
            selection_bg: None,
        }
    }
}

/// Luminance threshold separating dark from light backgrounds.
///
/// Uses the ITU-R BT.601 luma formula: `Y = 0.299R + 0.587G + 0.114B`.
/// Values below this threshold are classified as dark.
const LUMINANCE_THRESHOLD: f32 = 128.0;

impl ColorScheme {
    /// Classify this scheme as dark or light based on background luminance.
    pub(crate) fn brightness(&self) -> SchemeBrightness {
        brightness_of(self.bg)
    }

    /// Label suffix for display: `" (dark)"` or `" (light)"`.
    pub(crate) fn brightness_label(&self) -> &'static str {
        match self.brightness() {
            SchemeBrightness::Dark => " (dark)",
            SchemeBrightness::Light => " (light)",
        }
    }
}

/// Classify a background color as dark or light.
fn brightness_of(bg: Rgb) -> SchemeBrightness {
    let luma = 0.299 * bg.r as f32 + 0.587 * bg.g as f32 + 0.114 * bg.b as f32;
    if luma < LUMINANCE_THRESHOLD {
        SchemeBrightness::Dark
    } else {
        SchemeBrightness::Light
    }
}

/// Find a built-in scheme by name (case-insensitive).
pub(crate) fn find_builtin(name: &str) -> Option<ColorScheme> {
    BUILTIN_SCHEMES
        .iter()
        .find(|s| s.name.eq_ignore_ascii_case(name))
        .map(|s| s.to_scheme())
}

/// All built-in scheme names, in definition order.
pub(crate) fn builtin_names() -> Vec<&'static str> {
    BUILTIN_SCHEMES.iter().map(|s| s.name).collect()
}

/// Resolve a scheme name to a [`ColorScheme`].
///
/// If `name` is an absolute file path, loads directly from that path.
/// Otherwise checks built-in schemes first, then attempts to load from
/// the themes directory. Returns `None` if the scheme cannot be found.
pub(crate) fn resolve_scheme(name: &str) -> Option<ColorScheme> {
    let path = std::path::Path::new(name);
    if path.is_absolute() {
        if let Some(scheme) = loader::load_from_path(path) {
            return Some(scheme);
        }
    }

    find_builtin(name).or_else(|| loader::load_from_themes_dir(name))
}

/// Discover all available schemes (built-in + user themes).
///
/// Returns built-in schemes merged with user themes from `config_dir/themes/`.
/// User themes override built-in schemes with the same name (case-insensitive).
#[cfg(test)]
fn discover_all() -> Vec<ColorScheme> {
    let mut schemes: Vec<ColorScheme> = BUILTIN_SCHEMES.iter().map(|s| s.to_scheme()).collect();

    let user_themes = match crate::config::config_path().parent() {
        Some(dir) => loader::discover_themes(&dir.join("themes")),
        None => Vec::new(),
    };

    for user in user_themes {
        let lower = user.name.to_ascii_lowercase();
        if let Some(existing) = schemes
            .iter_mut()
            .find(|s| s.name.to_ascii_lowercase() == lower)
        {
            *existing = user;
        } else {
            schemes.push(user);
        }
    }

    schemes
}

/// Count available schemes without allocating full objects.
///
/// Returns `(builtin_count, user_count)`. User count is the number of
/// `.toml` files in the themes directory (not validated).
pub(crate) fn discover_count() -> (usize, usize) {
    let builtin = BUILTIN_SCHEMES.len();
    let user = match crate::config::config_path().parent() {
        Some(dir) => loader::count_themes(&dir.join("themes")),
        None => 0,
    };
    (builtin, user)
}

/// Parse a conditional scheme string: `"dark:X, light:Y"`.
///
/// Returns `Some((dark_name, light_name))` if the input contains both
/// `dark:` and `light:` prefixed names. Returns `None` for plain names.
pub(crate) fn parse_conditional(spec: &str) -> Option<(&str, &str)> {
    let mut dark = None;
    let mut light = None;

    for part in spec.split(',') {
        let part = part.trim();
        if let Some(name) = part.strip_prefix("dark:") {
            dark = Some(name.trim());
        } else if let Some(name) = part.strip_prefix("light:") {
            light = Some(name.trim());
        } else {
            // Unrecognized prefix — ignored.
        }
    }

    match (dark, light) {
        (Some(d), Some(l)) => Some((d, l)),
        _ => None,
    }
}

/// Resolve a scheme name that may be conditional, given the current theme.
///
/// If the name is `"dark:X, light:Y"`, picks `X` or `Y` based on `theme`.
/// Otherwise returns the name as-is.
pub(crate) fn resolve_scheme_name(spec: &str, theme: Theme) -> &str {
    match parse_conditional(spec) {
        Some((dark, light)) => {
            if theme.is_dark() {
                dark
            } else {
                light
            }
        }
        None => spec.trim(),
    }
}

/// Build a [`Palette`] from a [`ColorScheme`].
pub(crate) fn palette_from_scheme(scheme: &ColorScheme) -> Palette {
    let mut palette =
        Palette::from_scheme_colors(&scheme.ansi, scheme.fg, scheme.bg, scheme.cursor);
    palette.set_selection_fg(scheme.selection_fg);
    palette.set_selection_bg(scheme.selection_bg);
    palette
}

#[cfg(test)]
mod tests;
