//! Tests for the `Modifiers` SSOT type.

use super::Modifiers;

#[test]
fn xterm_param_empty_returns_zero() {
    assert_eq!(Modifiers::empty().xterm_param(), 0);
}

#[test]
fn xterm_param_shift_only_returns_2() {
    // Shift bit = 1; 1 + 1 = 2.
    assert_eq!(Modifiers::SHIFT.xterm_param(), 2);
}

#[test]
fn xterm_param_ctrl_shift_returns_6() {
    // Shift + Ctrl = 1 + 4 = 5; xterm_param = 5 + 1 = 6.
    assert_eq!((Modifiers::SHIFT | Modifiers::CONTROL).xterm_param(), 6);
}

#[test]
fn xterm_param_all_modifiers_returns_16() {
    // Shift+Alt+Ctrl+Super = 1+2+4+8 = 15; +1 = 16.
    let all = Modifiers::SHIFT | Modifiers::ALT | Modifiers::CONTROL | Modifiers::SUPER;
    assert_eq!(all.xterm_param(), 16);
}
