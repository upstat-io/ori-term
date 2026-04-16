//! Tests for key event encoding (legacy xterm + Kitty protocol dispatcher).

use winit::keyboard::{Key, KeyLocation};

use oriterm_core::TermMode;

pub(super) use super::{KeyEventType, KeyInput, Modifiers, encode_key};

mod application_keypad;
mod kitty_precedence;
mod legacy_backspace;
mod legacy_core;
mod modifier_matrix;
mod win32;

pub(super) fn no_mode() -> TermMode {
    TermMode::default()
}

pub(super) fn app_cursor_mode() -> TermMode {
    TermMode::default() | TermMode::APP_CURSOR
}

pub(super) fn app_keypad_mode() -> TermMode {
    TermMode::default() | TermMode::APP_KEYPAD
}

pub(super) fn decbkm_mode() -> TermMode {
    TermMode::default() | TermMode::DECBKM
}

pub(super) fn win32_input_mode() -> TermMode {
    TermMode::default() | TermMode::WIN32_INPUT
}

/// Encode a key press at standard location with no text.
pub(super) fn enc(key: Key, mods: Modifiers, mode: TermMode) -> Vec<u8> {
    encode_key(&KeyInput {
        key: &key,
        mods,
        mode,
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    })
}

/// Encode a key press at standard location with text.
pub(super) fn enc_text(key: Key, mods: Modifiers, mode: TermMode, text: &str) -> Vec<u8> {
    encode_key(&KeyInput {
        key: &key,
        mods,
        mode,
        text: Some(text),
        location: KeyLocation::Standard,
        event_type: KeyEventType::Press,
        alternate_key: None,
    })
}

/// Encode a key press at numpad location.
pub(super) fn enc_numpad(key: Key, mods: Modifiers, mode: TermMode) -> Vec<u8> {
    encode_key(&KeyInput {
        key: &key,
        mods,
        mode,
        text: None,
        location: KeyLocation::Numpad,
        event_type: KeyEventType::Press,
        alternate_key: None,
    })
}

/// Encode a key release at standard location.
pub(super) fn enc_release(key: Key, mods: Modifiers, mode: TermMode) -> Vec<u8> {
    encode_key(&KeyInput {
        key: &key,
        mods,
        mode,
        text: None,
        location: KeyLocation::Standard,
        event_type: KeyEventType::Release,
        alternate_key: None,
    })
}
