//! Built-in color scheme definitions (50+ schemes).
//!
//! Pure const data — no logic. Each scheme defines 16 ANSI colors plus
//! foreground, background, and cursor colors.

// Hex color literals (0xRRGGBB) intentionally match CSS/HTML color codes.
// Adding underscores (0x00RR_GGBB) would obscure the R/G/B byte boundaries.
#![allow(clippy::unreadable_literal)]

mod catppuccin;
mod material;
mod modern;
mod nature;
mod popular;
mod retro;
mod tokyo;

use oriterm_core::Rgb;

use super::BuiltinScheme;

/// Helper to construct `Rgb` from a 24-bit hex value at compile time.
pub(super) const fn rgb(hex: u32) -> Rgb {
    Rgb {
        r: ((hex >> 16) & 0xFF) as u8,
        g: ((hex >> 8) & 0xFF) as u8,
        b: (hex & 0xFF) as u8,
    }
}

/// Helper to construct a 16-entry ANSI palette from hex values.
pub(super) const fn ansi16(c: [u32; 16]) -> [Rgb; 16] {
    [
        rgb(c[0]),
        rgb(c[1]),
        rgb(c[2]),
        rgb(c[3]),
        rgb(c[4]),
        rgb(c[5]),
        rgb(c[6]),
        rgb(c[7]),
        rgb(c[8]),
        rgb(c[9]),
        rgb(c[10]),
        rgb(c[11]),
        rgb(c[12]),
        rgb(c[13]),
        rgb(c[14]),
        rgb(c[15]),
    ]
}

/// All built-in color schemes.
pub(super) const BUILTIN_SCHEMES: &[&BuiltinScheme] = &[
    &catppuccin::CATPPUCCIN_MOCHA,
    &catppuccin::CATPPUCCIN_LATTE,
    &catppuccin::CATPPUCCIN_FRAPPE,
    &catppuccin::CATPPUCCIN_MACCHIATO,
    &popular::ONE_DARK,
    &popular::ONE_LIGHT,
    &popular::SOLARIZED_DARK,
    &popular::SOLARIZED_LIGHT,
    &popular::DRACULA,
    &tokyo::TOKYO_NIGHT,
    &tokyo::TOKYO_NIGHT_STORM,
    &tokyo::TOKYO_NIGHT_LIGHT,
    &tokyo::WEZTERM_DEFAULT,
    &popular::GRUVBOX_DARK,
    &popular::GRUVBOX_LIGHT,
    &popular::NORD,
    &nature::ROSE_PINE,
    &nature::ROSE_PINE_MOON,
    &nature::ROSE_PINE_DAWN,
    &nature::EVERFOREST_DARK,
    &nature::EVERFOREST_LIGHT,
    &nature::KANAGAWA,
    &nature::KANAGAWA_LIGHT,
    &nature::AYU_DARK,
    &nature::AYU_MIRAGE,
    &nature::AYU_LIGHT,
    &material::MATERIAL_DARK,
    &material::MATERIAL_LIGHT,
    &popular::MONOKAI,
    &material::NIGHTFOX,
    &material::DAWNFOX,
    &material::CARBONFOX,
    &material::GITHUB_DARK,
    &material::GITHUB_LIGHT,
    &material::GITHUB_DARK_DIMMED,
    &retro::SNAZZY,
    &retro::TOMORROW_NIGHT,
    &retro::TOMORROW_LIGHT,
    &retro::ZENBURN,
    &retro::ICEBERG_DARK,
    &retro::ICEBERG_LIGHT,
    &modern::NIGHT_OWL,
    &modern::PALENIGHT,
    &modern::HORIZON,
    &modern::POIMANDRES,
    &modern::VESPER,
    &modern::SONOKAI,
    &modern::ONEDARK_PRO,
    &modern::MOONFLY,
    &retro::PAPERCOLOR_DARK,
    &retro::PAPERCOLOR_LIGHT,
    &modern::OXOCARBON,
    &modern::ANDROMEDA,
];
