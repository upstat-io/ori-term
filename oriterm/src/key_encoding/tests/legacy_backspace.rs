//! Backspace and Ctrl+Backspace encoding.

use winit::keyboard::{Key, NamedKey};

use super::{Modifiers, enc, no_mode};

#[test]
fn ctrl_backspace() {
    // Ctrl+Backspace sends 0x08 (BS), not 0x7f (DEL).
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::CONTROL,
        no_mode(),
    );
    assert_eq!(r, vec![0x08]);
}

#[test]
fn alt_ctrl_backspace() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::ALT | Modifiers::CONTROL,
        no_mode(),
    );
    assert_eq!(r, vec![0x1b, 0x08]);
}
