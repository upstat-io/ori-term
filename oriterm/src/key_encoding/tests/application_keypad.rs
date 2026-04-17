//! Application cursor mode (DECCKM) and application keypad mode (DECKPAM/DECKPNM) encoding.

use winit::keyboard::{Key, NamedKey};

use super::{Modifiers, app_cursor_mode, app_keypad_mode, enc, enc_numpad, enc_text};

// --- Application cursor mode ---

#[test]
fn app_cursor_up_no_mods() {
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOA");
}

#[test]
fn app_cursor_down_no_mods() {
    let r = enc(
        Key::Named(NamedKey::ArrowDown),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOB");
}

#[test]
fn app_cursor_home_no_mods() {
    let r = enc(
        Key::Named(NamedKey::Home),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOH");
}

#[test]
fn app_cursor_end_no_mods() {
    let r = enc(
        Key::Named(NamedKey::End),
        Modifiers::empty(),
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1bOF");
}

#[test]
fn app_cursor_up_with_ctrl() {
    // Modifiers override SS3 — use CSI format.
    let r = enc(
        Key::Named(NamedKey::ArrowUp),
        Modifiers::CONTROL,
        app_cursor_mode(),
    );
    assert_eq!(r, b"\x1b[1;5A");
}

// --- APP_KEYPAD numpad ---

#[test]
fn numpad_5_app_keypad() {
    let r = enc_numpad(
        Key::Character("5".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOu");
}

#[test]
fn numpad_0_app_keypad() {
    let r = enc_numpad(
        Key::Character("0".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOp");
}

#[test]
fn numpad_5_no_app_keypad() {
    let r = enc_numpad(
        Key::Character("5".into()),
        Modifiers::empty(),
        super::no_mode(),
    );
    // Without `APP_KEYPAD`, numpad falls through to legacy text. No text → empty.
    assert!(r.is_empty());
}

#[test]
fn numpad_enter_app_keypad() {
    let r = enc_numpad(
        Key::Named(NamedKey::Enter),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOM");
}

#[test]
fn numpad_plus_app_keypad() {
    let r = enc_numpad(
        Key::Character("+".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOk");
}

#[test]
fn numpad_minus_app_keypad() {
    let r = enc_numpad(
        Key::Character("-".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOm");
}

#[test]
fn numpad_star_app_keypad() {
    let r = enc_numpad(
        Key::Character("*".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOj");
}

#[test]
fn numpad_dot_app_keypad() {
    let r = enc_numpad(
        Key::Character(".".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOn");
}

#[test]
fn non_numpad_5_app_keypad() {
    // Standard location — `APP_KEYPAD` should not affect it.
    let r = enc_text(
        Key::Character("5".into()),
        Modifiers::empty(),
        app_keypad_mode(),
        "5",
    );
    assert_eq!(r, b"5");
}

// --- Numpad divide in APP_KEYPAD ---

#[test]
fn numpad_divide_app_keypad() {
    let r = enc_numpad(
        Key::Character("/".into()),
        Modifiers::empty(),
        app_keypad_mode(),
    );
    assert_eq!(r, b"\x1bOo");
}
