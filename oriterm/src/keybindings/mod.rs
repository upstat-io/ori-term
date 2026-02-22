//! Keybinding system — map key+modifiers to actions.

mod defaults;
mod parse;

use winit::keyboard::{Key, NamedKey};

use crate::key_encoding::Modifiers;

pub(crate) use defaults::default_bindings;
pub(crate) use parse::merge_bindings;
#[allow(unused_imports, reason = "re-exported for future CLI use")]
pub(crate) use parse::parse_mods;
pub(crate) use parse::{parse_action, parse_key};

/// Identifies a key independent of modifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum BindingKey {
    Named(NamedKey),
    /// Always stored lowercase.
    Character(String),
}

/// Action to execute when a keybinding matches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Action {
    Copy,
    Paste,
    /// Copy if selection exists, else fall through to PTY.
    SmartCopy,
    /// Paste from clipboard (Ctrl+V without Shift).
    SmartPaste,
    NewTab,
    CloseTab,
    NextTab,
    PrevTab,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ScrollPageUp,
    ScrollPageDown,
    ScrollToTop,
    ScrollToBottom,
    OpenSearch,
    ReloadConfig,
    /// Navigate to previous prompt mark (OSC 133;A).
    PreviousPrompt,
    /// Navigate to next prompt mark (OSC 133;A).
    NextPrompt,
    /// Duplicate the current tab (spawn new tab with same CWD).
    DuplicateTab,
    /// Move the current tab into a new window.
    MoveTabToNewWindow,
    /// Toggle fullscreen mode.
    ToggleFullscreen,
    /// Enter vi-style mark/selection mode.
    EnterMarkMode,
    /// Send literal bytes to the PTY.
    SendText(String),
    /// Explicitly unbinds a default binding.
    None,
}

/// A resolved keybinding: key + modifiers -> action.
#[derive(Debug, Clone)]
pub(crate) struct KeyBinding {
    pub key: BindingKey,
    pub mods: Modifiers,
    pub action: Action,
}

/// Convert a winit `Key` to a `BindingKey`, normalizing characters to lowercase.
pub(crate) fn key_to_binding_key(key: &Key) -> Option<BindingKey> {
    match key {
        Key::Named(n) => Some(BindingKey::Named(*n)),
        Key::Character(s) => {
            let lower = s.as_str().to_lowercase();
            if lower.is_empty() {
                None
            } else {
                Some(BindingKey::Character(lower))
            }
        }
        _ => None,
    }
}

/// Find the first binding matching the given key and modifiers.
pub(crate) fn find_binding<'a>(
    bindings: &'a [KeyBinding],
    key: &BindingKey,
    mods: Modifiers,
) -> Option<&'a Action> {
    bindings.iter().find_map(|b| {
        if b.key == *key && b.mods == mods {
            Some(&b.action)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests;
