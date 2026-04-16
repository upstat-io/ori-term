//! Win32 input mode encoding edge cases.

use winit::keyboard::Key;

use super::{Modifiers, enc, enc_release, win32_input_mode};

#[test]
fn ctrl_c_win32_input_mode() {
    // Win32 input mode is parsed but NOT used for encoding — Ctrl+C goes
    // through the legacy path as raw 0x03 for reliable ConPTY delivery.
    let r = enc(
        Key::Character("c".into()),
        Modifiers::CONTROL,
        win32_input_mode(),
    );
    assert_eq!(r, vec![0x03]);
}

#[test]
fn ctrl_c_release_win32_input_mode() {
    // Key releases produce empty output in legacy mode (no encoding).
    let r = enc_release(
        Key::Character("c".into()),
        Modifiers::CONTROL,
        win32_input_mode(),
    );
    assert_eq!(r, Vec::<u8>::new());
}
