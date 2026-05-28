//! Tests for the mouse-event encoder.

use crate::TermMode;

use super::{
    MouseButton, MouseEvent, MouseEventKind, MouseModifiers, apply_modifiers, button_code,
    encode_mouse_event, encode_normal, encode_sgr, encode_utf8,
};

fn event(button: MouseButton, kind: MouseEventKind, col: usize, line: usize) -> MouseEvent {
    MouseEvent {
        button,
        kind,
        col,
        line,
        mods: MouseModifiers::default(),
    }
}

#[test]
fn button_code_left_press_is_zero() {
    assert_eq!(button_code(MouseButton::Left, MouseEventKind::Press), 0);
}

#[test]
fn button_code_middle_press_is_one() {
    assert_eq!(button_code(MouseButton::Middle, MouseEventKind::Press), 1);
}

#[test]
fn button_code_right_press_is_two() {
    assert_eq!(button_code(MouseButton::Right, MouseEventKind::Press), 2);
}

#[test]
fn button_code_scroll_up_is_64() {
    assert_eq!(
        button_code(MouseButton::ScrollUp, MouseEventKind::Press),
        64
    );
}

#[test]
fn button_code_motion_adds_32() {
    assert_eq!(button_code(MouseButton::Left, MouseEventKind::Motion), 32);
    assert_eq!(button_code(MouseButton::Middle, MouseEventKind::Motion), 33);
}

#[test]
fn apply_modifiers_shift_alt_ctrl() {
    let mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    assert_eq!(apply_modifiers(0, mods), 28);
}

#[test]
fn encode_sgr_press_emits_uppercase_m_suffix() {
    let mut buf = [0u8; 32];
    let n = encode_sgr(&mut buf, 0, 10, 20, true);
    assert_eq!(&buf[..n], b"\x1b[<0;11;21M");
}

#[test]
fn encode_sgr_release_emits_lowercase_m_suffix() {
    let mut buf = [0u8; 32];
    let n = encode_sgr(&mut buf, 0, 10, 20, false);
    assert_eq!(&buf[..n], b"\x1b[<0;11;21m");
}

#[test]
fn encode_normal_basic_press() {
    let mut buf = [0u8; 32];
    let n = encode_normal(&mut buf, 0, 10, 20);
    assert_eq!(&buf[..n], &[0x1b, b'[', b'M', 32, 32 + 1 + 10, 32 + 1 + 20]);
}

#[test]
fn encode_normal_overflow_returns_zero() {
    let mut buf = [0u8; 32];
    assert_eq!(encode_normal(&mut buf, 0, 223, 0), 0);
    assert_eq!(encode_normal(&mut buf, 0, 0, 223), 0);
}

#[test]
fn encode_utf8_basic_press() {
    let mut buf = [0u8; 32];
    let n = encode_utf8(&mut buf, 0, 10, 20);
    assert_eq!(&buf[..n], &[0x1b, b'[', b'M', 32, 32 + 1 + 10, 32 + 1 + 20]);
}

#[test]
fn encode_mouse_event_sgr_mode_routes_through_sgr() {
    let ev = event(MouseButton::Left, MouseEventKind::Press, 10, 20);
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let report = encode_mouse_event(&ev, mode);
    assert_eq!(report.as_bytes(), b"\x1b[<0;11;21M");
}

#[test]
fn encode_mouse_event_x10_mode_drops_release() {
    let ev = event(MouseButton::Left, MouseEventKind::Release, 5, 5);
    let mode = TermMode::MOUSE_X10;
    let report = encode_mouse_event(&ev, mode);
    assert!(report.as_bytes().is_empty());
}

#[test]
fn encode_mouse_event_x10_mode_strips_modifiers() {
    let mut ev = event(MouseButton::Left, MouseEventKind::Press, 5, 5);
    ev.mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    let mode = TermMode::MOUSE_X10;
    let report = encode_mouse_event(&ev, mode);
    let expected = &[0x1b, b'[', b'M', 32, 32 + 1 + 5, 32 + 1 + 5];
    assert_eq!(report.as_bytes(), expected);
}

#[test]
fn encode_mouse_event_normal_release_uses_code_3() {
    let ev = event(MouseButton::Left, MouseEventKind::Release, 5, 5);
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let report = encode_mouse_event(&ev, mode);
    let expected = &[0x1b, b'[', b'M', 32 + 3, 32 + 1 + 5, 32 + 1 + 5];
    assert_eq!(report.as_bytes(), expected);
}

#[test]
fn encode_mouse_event_sgr_takes_precedence_over_urxvt() {
    let ev = event(MouseButton::Left, MouseEventKind::Press, 10, 20);
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR | TermMode::MOUSE_URXVT;
    let report = encode_mouse_event(&ev, mode);
    assert!(report.as_bytes().starts_with(b"\x1b[<"));
}
