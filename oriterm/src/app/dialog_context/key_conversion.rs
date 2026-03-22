//! Winit → UI key event conversion for dialog content dispatch.

use oriterm_ui::input::{InputEvent, Key as UiKey, Modifiers as UiModifiers};
use winit::keyboard::{Key, NamedKey};

/// Converts a winit key event to a UI `InputEvent`.
///
/// Returns `None` for keys that the UI widget system doesn't handle
/// (e.g., function keys, media keys). Maps both Pressed → `KeyDown`
/// and Released → `KeyUp` so controllers can consume matching releases.
pub(in crate::app) fn winit_key_to_input_event(
    event: &winit::event::KeyEvent,
    winit_mods: winit::keyboard::ModifiersState,
) -> Option<InputEvent> {
    let key = match &event.logical_key {
        Key::Named(named) => match named {
            NamedKey::Tab => UiKey::Tab,
            NamedKey::Enter => UiKey::Enter,
            NamedKey::Space => UiKey::Space,
            NamedKey::Backspace => UiKey::Backspace,
            NamedKey::Delete => UiKey::Delete,
            NamedKey::Home => UiKey::Home,
            NamedKey::End => UiKey::End,
            NamedKey::ArrowUp => UiKey::ArrowUp,
            NamedKey::ArrowDown => UiKey::ArrowDown,
            NamedKey::ArrowLeft => UiKey::ArrowLeft,
            NamedKey::ArrowRight => UiKey::ArrowRight,
            NamedKey::PageUp => UiKey::PageUp,
            NamedKey::PageDown => UiKey::PageDown,
            _ => return None,
        },
        Key::Character(ch) => {
            let c = ch.chars().next()?;
            UiKey::Character(c)
        }
        _ => return None,
    };

    let modifiers = winit_mods_to_ui(winit_mods);

    Some(match event.state {
        winit::event::ElementState::Pressed => InputEvent::KeyDown { key, modifiers },
        winit::event::ElementState::Released => InputEvent::KeyUp { key, modifiers },
    })
}

/// Converts a winit logical key to a UI [`UiKey`].
///
/// Returns `None` for keys that the overlay system doesn't handle
/// (e.g., function keys, media keys).
pub(super) fn winit_key_to_ui_key(key: &Key) -> Option<UiKey> {
    match key {
        Key::Named(named) => match named {
            NamedKey::Tab => Some(UiKey::Tab),
            NamedKey::Enter => Some(UiKey::Enter),
            NamedKey::Space => Some(UiKey::Space),
            NamedKey::Escape => Some(UiKey::Escape),
            NamedKey::Backspace => Some(UiKey::Backspace),
            NamedKey::Delete => Some(UiKey::Delete),
            NamedKey::Home => Some(UiKey::Home),
            NamedKey::End => Some(UiKey::End),
            NamedKey::ArrowUp => Some(UiKey::ArrowUp),
            NamedKey::ArrowDown => Some(UiKey::ArrowDown),
            NamedKey::ArrowLeft => Some(UiKey::ArrowLeft),
            NamedKey::ArrowRight => Some(UiKey::ArrowRight),
            NamedKey::PageUp => Some(UiKey::PageUp),
            NamedKey::PageDown => Some(UiKey::PageDown),
            _ => None,
        },
        Key::Character(ch) => ch.chars().next().map(UiKey::Character),
        _ => None,
    }
}

/// Converts winit modifier state to UI modifier flags.
pub(super) fn winit_mods_to_ui(m: winit::keyboard::ModifiersState) -> UiModifiers {
    let mut mods = UiModifiers::NONE;
    if m.shift_key() {
        mods = mods.union(UiModifiers::SHIFT_ONLY);
    }
    if m.control_key() {
        mods = mods.union(UiModifiers::CTRL_ONLY);
    }
    if m.alt_key() {
        mods = mods.union(UiModifiers::ALT_ONLY);
    }
    if m.super_key() {
        mods = mods.union(UiModifiers::LOGO_ONLY);
    }
    mods
}
