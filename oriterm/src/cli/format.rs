//! Keybinding / Action human-readable formatters for CLI `show-keys` output.
//!
//! Extracted from `cli/mod.rs` to keep that file under the 500-line budget.

use crate::keybindings::{Action, BindingKey, KeyBinding};

/// Format a single keybinding as `Mods+Key -> Action`.
pub(super) fn format_binding(b: &KeyBinding) -> String {
    let mut parts = Vec::new();

    if b.mods.contains(crate::key_encoding::Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if b.mods.contains(crate::key_encoding::Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if b.mods.contains(crate::key_encoding::Modifiers::ALT) {
        parts.push("Alt");
    }
    if b.mods.contains(crate::key_encoding::Modifiers::SUPER) {
        parts.push("Super");
    }

    let key_name = format_binding_key(&b.key);
    parts.push(&key_name);
    let combo = parts.join("+");

    let action = format_action(&b.action);
    format!("{combo} -> {action}")
}

/// Format a `BindingKey` as a human-readable string.
pub(super) fn format_binding_key(key: &BindingKey) -> String {
    match key {
        BindingKey::Named(n) => format!("{n:?}"),
        BindingKey::Character(s) => s.to_uppercase(),
    }
}

/// Format an `Action` as a human-readable string.
pub(super) fn format_action(action: &Action) -> String {
    match action {
        Action::SendText(t) => format!("SendText:{t:?}"),
        other => other.as_str().to_owned(),
    }
}
