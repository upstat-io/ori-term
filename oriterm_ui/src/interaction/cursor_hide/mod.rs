//! Mouse cursor hiding while typing.
//!
//! Hides the OS mouse cursor when the user types in the terminal grid,
//! restoring it on any mouse movement. The decision logic is a pure
//! function for testability — the actual `set_cursor_visible()` calls
//! happen at the integration points in the app layer.

use winit::keyboard::{Key, NamedKey};

/// Inputs for the cursor-hide decision.
#[allow(
    clippy::struct_excessive_bools,
    reason = "state flags are naturally boolean"
)]
pub struct HideContext<'a> {
    /// Whether `hide_mouse_when_typing` is enabled in config.
    pub config_enabled: bool,
    /// Whether the mouse cursor is already hidden.
    pub already_hidden: bool,
    /// The key that was pressed.
    pub key: &'a Key,
    /// Whether the terminal has mouse reporting active.
    pub mouse_reporting: bool,
    /// Whether IME composition is in progress.
    pub ime_active: bool,
}

/// Whether a keypress should hide the mouse cursor.
///
/// Returns `true` when all conditions for hiding are met:
/// - The feature is enabled in config.
/// - The cursor is not already hidden.
/// - The key is not a modifier-only press (Shift, Ctrl, Alt, Super).
/// - The terminal does not have mouse reporting active.
/// - IME composition is not in progress.
pub fn should_hide_cursor(ctx: &HideContext<'_>) -> bool {
    if !ctx.config_enabled || ctx.already_hidden || ctx.mouse_reporting || ctx.ime_active {
        return false;
    }
    !is_modifier_only(ctx.key)
}

/// Whether the key is a modifier-only press that should not trigger hiding.
fn is_modifier_only(key: &Key) -> bool {
    matches!(
        key,
        Key::Named(
            NamedKey::Shift
                | NamedKey::Control
                | NamedKey::Alt
                | NamedKey::Super
                | NamedKey::Hyper
                | NamedKey::Meta
        )
    )
}

#[cfg(test)]
mod tests;
