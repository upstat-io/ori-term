//! Settings overlay — action handler, form builder, and per-page dirty detection.

pub(in crate::app) mod action_handler;
pub(in crate::app) mod form_builder;

pub(in crate::app) use form_builder::SettingsIds;

use crate::config::Config;

/// Number of settings pages in the sidebar.
pub(in crate::app) const PAGE_COUNT: usize = 8;

/// Compares pending vs original config per settings page.
///
/// Returns a fixed-size array of booleans — one per page — indicating
/// whether that page has unsaved changes. Page indices match the sidebar
/// nav item order (Appearance=0, Colors=1, Font=2, Terminal=3,
/// Keybindings=4, Window=5, Bell=6, Rendering=7).
///
/// Float comparisons use `to_bits()` for exact equality — config values
/// come from slider/input widgets and are never computed, so bit-exact
/// comparison is correct (no rounding drift).
pub(in crate::app) fn per_page_dirty(pending: &Config, original: &Config) -> [bool; PAGE_COUNT] {
    [
        // 0: Appearance — opacity, blur, unfocused opacity, decorations, tab bar style, scheme.
        pending.window.opacity.to_bits() != original.window.opacity.to_bits()
            || pending.window.blur != original.window.blur
            || pending.window.unfocused_opacity.to_bits()
                != original.window.unfocused_opacity.to_bits()
            || pending.window.decorations != original.window.decorations
            || pending.window.tab_bar_style != original.window.tab_bar_style
            || pending.colors.scheme != original.colors.scheme,
        // 1: Colors — scheme.
        pending.colors.scheme != original.colors.scheme,
        // 2: Font — all font config.
        pending.font != original.font,
        // 3: Terminal — terminal config + paste warning.
        pending.terminal != original.terminal
            || pending.behavior.warn_on_paste != original.behavior.warn_on_paste,
        // 4: Keybindings — no settings yet, always clean.
        false,
        // 5: Window — tab bar position, grid padding, restore session, columns, rows.
        pending.window.tab_bar_position != original.window.tab_bar_position
            || pending.window.grid_padding.to_bits() != original.window.grid_padding.to_bits()
            || pending.window.restore_session != original.window.restore_session
            || pending.window.columns != original.window.columns
            || pending.window.rows != original.window.rows,
        // 6: Bell — all bell config.
        pending.bell != original.bell,
        // 7: Rendering — GPU backend, subpixel mode.
        pending.rendering.gpu_backend != original.rendering.gpu_backend
            || pending.font.subpixel_mode != original.font.subpixel_mode,
    ]
}

#[cfg(test)]
mod tests;
