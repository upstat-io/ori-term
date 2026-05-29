//! Tests for mouse event encoding.

use oriterm_core::TermMode;
use oriterm_core::encode::mouse::{
    MouseButton, MouseEvent, MouseEventKind, MouseModifiers, apply_modifiers, button_code,
    encode_mouse_event, encode_normal, encode_sgr, encode_utf8,
};

// -- button_code tests --

#[test]
fn button_code_left_press() {
    assert_eq!(button_code(MouseButton::Left, MouseEventKind::Press), 0);
}

#[test]
fn button_code_middle_press() {
    assert_eq!(button_code(MouseButton::Middle, MouseEventKind::Press), 1);
}

#[test]
fn button_code_right_press() {
    assert_eq!(button_code(MouseButton::Right, MouseEventKind::Press), 2);
}

#[test]
fn button_code_none_press() {
    assert_eq!(button_code(MouseButton::None, MouseEventKind::Press), 3);
}

#[test]
fn button_code_scroll_up() {
    assert_eq!(
        button_code(MouseButton::ScrollUp, MouseEventKind::Press),
        64
    );
}

#[test]
fn button_code_scroll_down() {
    assert_eq!(
        button_code(MouseButton::ScrollDown, MouseEventKind::Press),
        65
    );
}

#[test]
fn button_code_motion_adds_32() {
    assert_eq!(button_code(MouseButton::Left, MouseEventKind::Motion), 32);
    assert_eq!(button_code(MouseButton::Middle, MouseEventKind::Motion), 33);
    assert_eq!(button_code(MouseButton::Right, MouseEventKind::Motion), 34);
}

#[test]
fn button_code_none_motion_is_35() {
    // Mode 1003 no-button motion: base 3 + 32 = 35.
    assert_eq!(button_code(MouseButton::None, MouseEventKind::Motion), 35);
}

// -- apply_modifiers tests --

#[test]
fn modifiers_none() {
    let mods = MouseModifiers::default();
    assert_eq!(apply_modifiers(0, mods), 0);
}

#[test]
fn modifiers_shift() {
    let mods = MouseModifiers {
        shift: true,
        ..Default::default()
    };
    assert_eq!(apply_modifiers(0, mods), 4);
}

#[test]
fn modifiers_alt() {
    let mods = MouseModifiers {
        alt: true,
        ..Default::default()
    };
    assert_eq!(apply_modifiers(0, mods), 8);
}

#[test]
fn modifiers_ctrl() {
    let mods = MouseModifiers {
        ctrl: true,
        ..Default::default()
    };
    assert_eq!(apply_modifiers(0, mods), 16);
}

#[test]
fn modifiers_combined() {
    let mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    assert_eq!(apply_modifiers(0, mods), 28);
}

// -- SGR encoding tests --

#[test]
fn sgr_left_click_origin() {
    let mut buf = [0u8; 32];
    let len = encode_sgr(&mut buf, 0, 0, 0, true);
    assert_eq!(&buf[..len], b"\x1b[<0;1;1M");
}

#[test]
fn sgr_right_click() {
    let mut buf = [0u8; 32];
    let len = encode_sgr(&mut buf, 2, 5, 10, true);
    assert_eq!(&buf[..len], b"\x1b[<2;6;11M");
}

#[test]
fn sgr_release_uses_lowercase_m() {
    let mut buf = [0u8; 32];
    let len = encode_sgr(&mut buf, 0, 0, 0, false);
    assert_eq!(&buf[..len], b"\x1b[<0;1;1m");
}

#[test]
fn sgr_coordinates_are_1_indexed() {
    let mut buf = [0u8; 32];
    let len = encode_sgr(&mut buf, 0, 9, 19, true);
    let s = std::str::from_utf8(&buf[..len]).unwrap();
    assert!(s.contains(";10;20M"));
}

#[test]
fn sgr_with_shift_modifier() {
    let mut buf = [0u8; 32];
    let code = apply_modifiers(
        0,
        MouseModifiers {
            shift: true,
            ..Default::default()
        },
    );
    let len = encode_sgr(&mut buf, code, 0, 0, true);
    assert_eq!(&buf[..len], b"\x1b[<4;1;1M");
}

#[test]
fn sgr_scroll_up() {
    let mut buf = [0u8; 32];
    let len = encode_sgr(&mut buf, 64, 5, 5, true);
    assert_eq!(&buf[..len], b"\x1b[<64;6;6M");
}

#[test]
fn sgr_motion() {
    let mut buf = [0u8; 32];
    // Left drag = 0 + 32 = 32.
    let len = encode_sgr(&mut buf, 32, 3, 7, true);
    assert_eq!(&buf[..len], b"\x1b[<32;4;8M");
}

#[test]
fn sgr_large_coordinates() {
    let mut buf = [0u8; 32];
    let len = encode_sgr(&mut buf, 0, 999, 499, true);
    let s = std::str::from_utf8(&buf[..len]).unwrap();
    assert!(s.contains(";1000;500M"));
}

#[test]
fn sgr_middle_release_preserves_button_code() {
    // SGR release encodes the real button code (not generic 3 like Normal).
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event_with_mods(
        MouseButton::Middle,
        MouseEventKind::Release,
        5,
        10,
        MouseModifiers::default(),
    );
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // Middle button = code 1, coords (6,11), release = 'm'.
    assert_eq!(s, "\x1b[<1;6;11m");
}

#[test]
fn sgr_right_release_preserves_button_code() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event(MouseButton::Right, MouseEventKind::Release, 3, 7);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // Right button = code 2, coords (4,8), release = 'm'.
    assert_eq!(s, "\x1b[<2;4;8m");
}

#[test]
fn sgr_all_modifiers_full_round_trip() {
    // Shift+Alt+Ctrl right-click through full SGR encoding.
    let mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event_with_mods(MouseButton::Right, MouseEventKind::Press, 10, 20, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // Right(2) + shift(4) + alt(8) + ctrl(16) = 30, coords (11,21).
    assert_eq!(s, "\x1b[<30;11;21M");
}

// -- Normal (X10) encoding tests --

#[test]
fn normal_left_click() {
    let mut buf = [0u8; 32];
    let len = encode_normal(&mut buf, 0, 0, 0);
    assert_eq!(len, 6);
    assert_eq!(&buf[..3], b"\x1b[M");
    assert_eq!(buf[3], 32); // 32 + 0
    assert_eq!(buf[4], 33); // 32 + 1 + 0
    assert_eq!(buf[5], 33); // 32 + 1 + 0
}

#[test]
fn normal_out_of_range_drops_event() {
    let mut buf = [0u8; 32];
    // Coords > 222 are unencodable — event is silently dropped.
    assert_eq!(encode_normal(&mut buf, 0, 500, 0), 0);
    assert_eq!(encode_normal(&mut buf, 0, 0, 500), 0);
    assert_eq!(encode_normal(&mut buf, 0, 500, 500), 0);
}

#[test]
fn normal_at_max_encodable_coord() {
    let mut buf = [0u8; 32];
    // 222 is the max encodable coordinate (32 + 1 + 222 = 255).
    let len = encode_normal(&mut buf, 0, 222, 222);
    assert_eq!(len, 6);
    assert_eq!(buf[4], 255);
    assert_eq!(buf[5], 255);
}

#[test]
fn normal_just_past_max_drops() {
    let mut buf = [0u8; 32];
    // 223 is one past the limit — should drop.
    assert_eq!(encode_normal(&mut buf, 0, 223, 0), 0);
    assert_eq!(encode_normal(&mut buf, 0, 0, 223), 0);
}

#[test]
fn normal_release_code_is_3() {
    let mut buf = [0u8; 32];
    // Release in Normal mode uses code 3 (not the button code).
    let len = encode_normal(&mut buf, 3, 5, 5);
    assert_eq!(len, 6);
    assert_eq!(buf[3], 35); // 32 + 3
}

#[test]
fn normal_release_with_shift_modifier() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event_with_mods(
        MouseButton::Left,
        MouseEventKind::Release,
        0,
        0,
        MouseModifiers {
            shift: true,
            ..Default::default()
        },
    );
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Release code 3 + shift 4 = 7; button byte = 32 + 7 = 39.
    assert_eq!(bytes[3], 39);
}

#[test]
fn normal_release_with_ctrl_modifier() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event_with_mods(
        MouseButton::Left,
        MouseEventKind::Release,
        0,
        0,
        MouseModifiers {
            ctrl: true,
            ..Default::default()
        },
    );
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Release code 3 + ctrl 16 = 19; button byte = 32 + 19 = 51.
    assert_eq!(bytes[3], 51);
}

#[test]
fn normal_release_with_all_modifiers() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event_with_mods(
        MouseButton::Left,
        MouseEventKind::Release,
        0,
        0,
        MouseModifiers {
            shift: true,
            alt: true,
            ctrl: true,
        },
    );
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Release code 3 + shift(4) + alt(8) + ctrl(16) = 31; byte = 32 + 31 = 63.
    assert_eq!(bytes[3], 63);
}

// -- UTF-8 encoding tests --

#[test]
fn utf8_small_coords_single_byte() {
    let mut buf = [0u8; 32];
    let len = encode_utf8(&mut buf, 0, 5, 10);
    assert!(len > 0);
    assert_eq!(&buf[..3], b"\x1b[M");
    assert_eq!(buf[3], 32); // 32 + 0 (button)
    assert_eq!(buf[4], 38); // 32 + 1 + 5
    assert_eq!(buf[5], 43); // 32 + 1 + 10
}

#[test]
fn utf8_boundary_pos_94_single_byte() {
    // pos=94: val = 32 + 1 + 94 = 127 (still fits in single byte).
    let mut buf = [0u8; 32];
    let len = encode_utf8(&mut buf, 0, 94, 94);
    assert_eq!(len, 3 + 1 + 1 + 1); // header + button + 2 single-byte coords
    assert_eq!(buf[4], 127);
    assert_eq!(buf[5], 127);
}

#[test]
fn utf8_boundary_pos_95_two_bytes() {
    // pos=95: val = 32 + 1 + 95 = 128 (needs 2-byte encoding).
    let mut buf = [0u8; 32];
    let len = encode_utf8(&mut buf, 0, 95, 95);
    assert_eq!(len, 3 + 1 + 2 + 2); // header + button + 2 two-byte coords
}

#[test]
fn utf8_large_coords_multi_byte() {
    let mut buf = [0u8; 32];
    // Position 200: val = 32 + 1 + 200 = 233 (> 127, needs 2-byte encoding).
    let len = encode_utf8(&mut buf, 0, 200, 200);
    assert!(len > 0);
    // Each coordinate should be 2 bytes.
    assert_eq!(len, 3 + 1 + 2 + 2); // header + button + 2 coords * 2 bytes
}

#[test]
fn utf8_out_of_range_returns_zero() {
    let mut buf = [0u8; 32];
    // Position 2016+: val = 32 + 1 + 2016 = 2049 > 0x7FF.
    let len = encode_utf8(&mut buf, 0, 2016, 0);
    assert_eq!(len, 0);
}

#[test]
fn utf8_max_button_code_with_all_modifiers_in_range() {
    // Highest realistic code: ScrollDown(65) + shift(4) + alt(8) + ctrl(16) = 93.
    // Button byte: 32 + 93 = 125 (< 128, single byte — safe).
    let mut buf = [0u8; 32];
    let len = encode_utf8(&mut buf, 93, 0, 0);
    assert!(len > 0);
    assert_eq!(buf[3], 125);
}

// -- encode_mouse_event dispatch tests --

fn event(button: MouseButton, kind: MouseEventKind, col: usize, line: usize) -> MouseEvent {
    MouseEvent::cell(button, kind, col, line, MouseModifiers::default())
}

fn event_with_mods(
    button: MouseButton,
    kind: MouseEventKind,
    col: usize,
    line: usize,
    mods: MouseModifiers,
) -> MouseEvent {
    MouseEvent::cell(button, kind, col, line, mods)
}

#[test]
fn dispatch_sgr_when_sgr_mode() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.starts_with(b"\x1b[<"));
    assert_eq!(*bytes.last().unwrap(), b'M');
}

#[test]
fn dispatch_utf8_when_utf8_mode() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.starts_with(b"\x1b[M"));
}

#[test]
fn dispatch_normal_when_no_encoding_flags() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(bytes.len(), 6);
    assert!(bytes.starts_with(b"\x1b[M"));
}

#[test]
fn dispatch_sgr_takes_priority_over_utf8() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.starts_with(b"\x1b[<"));
}

#[test]
fn dispatch_normal_release_uses_code_3() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event(MouseButton::Left, MouseEventKind::Release, 0, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Normal release: button byte is 32 + 3 = 35.
    assert_eq!(bytes[3], 35);
}

#[test]
fn dispatch_sgr_release_uses_lowercase_m() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event(MouseButton::Left, MouseEventKind::Release, 0, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(*bytes.last().unwrap(), b'm');
}

// -- Motion tests (drag and buttonless) --

#[test]
fn no_button_motion_sgr_uses_code_35() {
    // Mode 1003 buttonless motion: None(3) + motion(32) = 35.
    let mode = TermMode::MOUSE_MOTION | TermMode::MOUSE_SGR;
    let e = event(MouseButton::None, MouseEventKind::Motion, 10, 20);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(s, "\x1b[<35;11;21M");
}

#[test]
fn no_button_motion_normal_uses_code_35() {
    let mode = TermMode::MOUSE_MOTION;
    let e = event(MouseButton::None, MouseEventKind::Motion, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Button byte: 32 + 35 = 67.
    assert_eq!(bytes[3], 67);
}

#[test]
fn middle_drag_motion_sgr_uses_code_33() {
    // Middle button drag: Middle(1) + motion(32) = 33.
    let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR;
    let e = event(MouseButton::Middle, MouseEventKind::Motion, 8, 12);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(s, "\x1b[<33;9;13M");
}

#[test]
fn right_drag_motion_sgr_uses_code_34() {
    // Right button drag: Right(2) + motion(32) = 34.
    let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR;
    let e = event(MouseButton::Right, MouseEventKind::Motion, 3, 7);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(s, "\x1b[<34;4;8M");
}

#[test]
fn middle_drag_motion_normal_uses_code_33() {
    let mode = TermMode::MOUSE_DRAG;
    let e = event(MouseButton::Middle, MouseEventKind::Motion, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Middle(1) + motion(32) = 33; button byte: 32 + 33 = 65.
    assert_eq!(bytes[3], 65);
}

#[test]
fn right_drag_motion_normal_uses_code_34() {
    let mode = TermMode::MOUSE_DRAG;
    let e = event(MouseButton::Right, MouseEventKind::Motion, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Right(2) + motion(32) = 34; button byte: 32 + 34 = 66.
    assert_eq!(bytes[3], 66);
}

#[test]
fn left_drag_motion_sgr_uses_code_32() {
    // Left button drag: Left(0) + motion(32) = 32.
    let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR;
    let e = event(MouseButton::Left, MouseEventKind::Motion, 3, 7);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    assert_eq!(s, "\x1b[<32;4;8M");
}

// -- Modifier combinations on scroll --

#[test]
fn scroll_up_with_shift_sgr() {
    let mods = MouseModifiers {
        shift: true,
        ..Default::default()
    };
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event_with_mods(MouseButton::ScrollUp, MouseEventKind::Press, 5, 5, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // ScrollUp(64) + shift(4) = 68.
    assert_eq!(s, "\x1b[<68;6;6M");
}

#[test]
fn scroll_down_with_ctrl_sgr() {
    let mods = MouseModifiers {
        ctrl: true,
        ..Default::default()
    };
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event_with_mods(MouseButton::ScrollDown, MouseEventKind::Press, 5, 5, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // ScrollDown(65) + ctrl(16) = 81.
    assert_eq!(s, "\x1b[<81;6;6M");
}

#[test]
fn scroll_up_with_alt_normal() {
    let mods = MouseModifiers {
        alt: true,
        ..Default::default()
    };
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event_with_mods(MouseButton::ScrollUp, MouseEventKind::Press, 0, 0, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // ScrollUp(64) + alt(8) = 72; button byte = 32 + 72 = 104.
    assert_eq!(bytes[3], 104);
}

// -- Motion always reports as "pressed" (M, not m) --

#[test]
fn sgr_motion_always_uppercase_m() {
    // Motion events are neither press nor release — they use 'M' (pressed
    // flag) because there's no "motion release". Verify for all button types.
    let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR;
    for button in [
        MouseButton::Left,
        MouseButton::Middle,
        MouseButton::Right,
        MouseButton::None,
    ] {
        let e = event(button, MouseEventKind::Motion, 5, 5);
        let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(
            s.ends_with('M'),
            "motion for {button:?} should use 'M', got {s:?}"
        );
    }
}

// -- Normal encoding: scroll events --

#[test]
fn normal_scroll_up_encoding() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event(MouseButton::ScrollUp, MouseEventKind::Press, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // ScrollUp(64); button byte = 32 + 64 = 96.
    assert_eq!(bytes[3], 96);
}

#[test]
fn normal_scroll_down_encoding() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event(MouseButton::ScrollDown, MouseEventKind::Press, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // ScrollDown(65); button byte = 32 + 65 = 97.
    assert_eq!(bytes[3], 97);
}

#[test]
fn normal_scroll_down_with_ctrl() {
    let mods = MouseModifiers {
        ctrl: true,
        ..Default::default()
    };
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event_with_mods(MouseButton::ScrollDown, MouseEventKind::Press, 5, 5, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // ScrollDown(65) + ctrl(16) = 81; button byte = 32 + 81 = 113.
    assert_eq!(bytes[3], 113);
}

// -- UTF-8 encoding: motion --

#[test]
fn utf8_motion_encoding() {
    let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::Left, MouseEventKind::Motion, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.starts_with(b"\x1b[M"));
    // Left(0) + motion(32) = 32; button byte = 32 + 32 = 64.
    assert_eq!(bytes[3], 64);
}

#[test]
fn utf8_no_button_motion_encoding() {
    let mode = TermMode::MOUSE_MOTION | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::None, MouseEventKind::Motion, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.starts_with(b"\x1b[M"));
    // None(3) + motion(32) = 35; button byte = 32 + 35 = 67.
    assert_eq!(bytes[3], 67);
}

// -- Full dispatch: out-of-range returns empty --

#[test]
fn dispatch_normal_out_of_range_returns_empty() {
    // Normal encoding drops events when coords > 222.
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event(MouseButton::Left, MouseEventKind::Press, 300, 300);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(
        bytes.is_empty(),
        "out-of-range Normal event should be empty"
    );
}

#[test]
fn dispatch_utf8_out_of_range_returns_empty() {
    // UTF-8 encoding drops events when coords > 2015.
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::Left, MouseEventKind::Press, 3000, 3000);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.is_empty(), "out-of-range UTF-8 event should be empty");
}

// -- Buffer overflow safety --

#[test]
fn sgr_extreme_coordinates_fit_in_buffer() {
    // Max realistic SGR: "\x1b[<125;65536;65536M" = ~24 chars, fits in 32.
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event(MouseButton::Left, MouseEventKind::Press, 65535, 65535);
    let report = encode_mouse_event(&e, mode);
    let bytes = report.as_bytes();
    assert!(!bytes.is_empty());
    assert!(bytes.len() <= 32);
    let s = std::str::from_utf8(bytes).unwrap();
    assert!(s.ends_with('M'));
}

// -- Scroll events always use Press kind --

#[test]
fn scroll_release_still_encodes_as_press() {
    // Scroll events don't have a "release" — even if Release kind is passed,
    // the scroll button code (64/65) carries through. In SGR, this means
    // the event still uses 'M' (not 'm') because scroll has no release state.
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event(MouseButton::ScrollUp, MouseEventKind::Release, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // Release uses 'm' in SGR, even for scroll. This tests the actual behavior.
    assert!(
        s.ends_with('m'),
        "SGR encodes Release as lowercase m: {s:?}"
    );
    // But the button code is still ScrollUp (64), not generic release (3).
    assert!(s.contains("<64;"));
}

// -- Multi-button state machine --
// Verifies that encode_mouse_event produces the correct button codes when
// multiple buttons transition through press/motion/release sequences.

#[test]
fn multi_button_press_release_sequence_sgr() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;

    // Press Left → button code 0.
    let e1 = event(MouseButton::Left, MouseEventKind::Press, 5, 5);
    let s1 = std::str::from_utf8(encode_mouse_event(&e1, mode).as_bytes())
        .unwrap()
        .to_string();
    assert!(s1.contains("<0;"), "left press should be code 0: {s1:?}");
    assert!(s1.ends_with('M'));

    // Press Right (while Left still held by app) → button code 2.
    let e2 = event(MouseButton::Right, MouseEventKind::Press, 5, 5);
    let s2 = std::str::from_utf8(encode_mouse_event(&e2, mode).as_bytes())
        .unwrap()
        .to_string();
    assert!(s2.contains("<2;"), "right press should be code 2: {s2:?}");

    // Release Left → SGR preserves button code 0 in release.
    let e3 = event(MouseButton::Left, MouseEventKind::Release, 5, 5);
    let s3 = std::str::from_utf8(encode_mouse_event(&e3, mode).as_bytes())
        .unwrap()
        .to_string();
    assert!(
        s3.contains("<0;"),
        "left release should preserve code 0: {s3:?}"
    );
    assert!(s3.ends_with('m'), "release should use lowercase m: {s3:?}");

    // Release Right → SGR preserves button code 2 in release.
    let e4 = event(MouseButton::Right, MouseEventKind::Release, 5, 5);
    let s4 = std::str::from_utf8(encode_mouse_event(&e4, mode).as_bytes())
        .unwrap()
        .to_string();
    assert!(
        s4.contains("<2;"),
        "right release should preserve code 2: {s4:?}"
    );
    assert!(s4.ends_with('m'));
}

#[test]
fn multi_button_press_release_sequence_normal() {
    let mode = TermMode::MOUSE_REPORT_CLICK;

    // Press Left → button byte = 32 + 0 = 32.
    let e1 = event(MouseButton::Left, MouseEventKind::Press, 5, 5);
    let b1 = encode_mouse_event(&e1, mode).as_bytes().to_vec();
    assert_eq!(b1[3], 32, "left press button byte");

    // Press Right → button byte = 32 + 2 = 34.
    let e2 = event(MouseButton::Right, MouseEventKind::Press, 5, 5);
    let b2 = encode_mouse_event(&e2, mode).as_bytes().to_vec();
    assert_eq!(b2[3], 34, "right press button byte");

    // Release Left → Normal always uses code 3: button byte = 32 + 3 = 35.
    let e3 = event(MouseButton::Left, MouseEventKind::Release, 5, 5);
    let b3 = encode_mouse_event(&e3, mode).as_bytes().to_vec();
    assert_eq!(b3[3], 35, "normal release always uses code 3");

    // Release Right → same code 3 regardless of which button was released.
    let e4 = event(MouseButton::Right, MouseEventKind::Release, 5, 5);
    let b4 = encode_mouse_event(&e4, mode).as_bytes().to_vec();
    assert_eq!(b4[3], 35);
}

// -- Release after motion --

#[test]
fn sgr_release_after_motion_uses_lowercase_m() {
    let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR;

    // Motion event: Left drag = code 32, uppercase M.
    let motion = event(MouseButton::Left, MouseEventKind::Motion, 10, 10);
    let ms = std::str::from_utf8(encode_mouse_event(&motion, mode).as_bytes())
        .unwrap()
        .to_string();
    assert!(ms.ends_with('M'), "motion uses uppercase M: {ms:?}");
    assert!(ms.contains("<32;"), "left drag motion is code 32: {ms:?}");

    // Release event: Left = code 0, lowercase m.
    let release = event(MouseButton::Left, MouseEventKind::Release, 10, 10);
    let rs = std::str::from_utf8(encode_mouse_event(&release, mode).as_bytes())
        .unwrap()
        .to_string();
    assert!(
        rs.ends_with('m'),
        "release after motion uses lowercase m: {rs:?}"
    );
    assert!(
        rs.contains("<0;"),
        "release uses base button code (no +32): {rs:?}"
    );
}

#[test]
fn normal_release_after_motion_uses_code_3() {
    let mode = TermMode::MOUSE_DRAG;

    // Motion: Left drag = code 32, button byte = 32 + 32 = 64.
    let motion = event(MouseButton::Left, MouseEventKind::Motion, 5, 5);
    let mb = encode_mouse_event(&motion, mode).as_bytes().to_vec();
    assert_eq!(mb[3], 64, "left drag motion button byte");

    // Release: code 3, button byte = 32 + 3 = 35.
    let release = event(MouseButton::Left, MouseEventKind::Release, 5, 5);
    let rb = encode_mouse_event(&release, mode).as_bytes().to_vec();
    assert_eq!(rb[3], 35, "normal release uses code 3 after motion");
}

// -- Scroll with combined modifiers --

#[test]
fn scroll_up_shift_ctrl_sgr() {
    let mods = MouseModifiers {
        shift: true,
        alt: false,
        ctrl: true,
    };
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event_with_mods(MouseButton::ScrollUp, MouseEventKind::Press, 5, 5, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // ScrollUp(64) + shift(4) + ctrl(16) = 84.
    assert_eq!(s, "\x1b[<84;6;6M");
}

#[test]
fn scroll_down_shift_alt_sgr() {
    let mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: false,
    };
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let e = event_with_mods(MouseButton::ScrollDown, MouseEventKind::Press, 5, 5, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // ScrollDown(65) + shift(4) + alt(8) = 77.
    assert_eq!(s, "\x1b[<77;6;6M");
}

#[test]
fn scroll_up_all_modifiers_normal() {
    let mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let e = event_with_mods(MouseButton::ScrollUp, MouseEventKind::Press, 0, 0, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // ScrollUp(64) + shift(4) + alt(8) + ctrl(16) = 92; button byte = 32 + 92 = 124.
    assert_eq!(bytes[3], 124);
}

// -- UTF-8 encoding boundary at pos=2015 --

#[test]
fn utf8_exact_max_coord_2015() {
    let mut buf = [0u8; 32];
    // pos=2015: val = 32 + 1 + 2015 = 2048 = 0x800 — OUT of range (> 0x7FF).
    let len = encode_utf8(&mut buf, 0, 2015, 0);
    assert_eq!(len, 0, "pos 2015 should be out of range for UTF-8");
}

#[test]
fn utf8_one_below_max_coord_2014() {
    let mut buf = [0u8; 32];
    // pos=2014: val = 32 + 1 + 2014 = 2047 = 0x7FF — exactly at the limit.
    let len = encode_utf8(&mut buf, 0, 2014, 0);
    assert!(len > 0, "pos 2014 should encode successfully");
}

#[test]
fn utf8_boundary_symmetry() {
    let mut buf = [0u8; 32];
    // Both coords at max limit.
    let len = encode_utf8(&mut buf, 0, 2014, 2014);
    assert!(len > 0, "both coords at max should encode");

    // One coord over limit.
    let len = encode_utf8(&mut buf, 0, 2014, 2015);
    assert_eq!(len, 0, "one coord over limit should fail");
}

// -- Double-press same button without release --

#[test]
fn double_press_same_button_encodes_independently() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;

    // Two left presses without a release — each produces a valid, identical report.
    let e = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let b1 = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let b2 = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(b1, b2, "duplicate press should produce identical encoding");
    assert!(!b1.is_empty());
}

#[test]
fn double_press_normal_encodes_independently() {
    let mode = TermMode::MOUSE_REPORT_CLICK;

    let e = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let b1 = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let b2 = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(b1, b2, "duplicate press should produce identical encoding");
    assert_eq!(b1.len(), 6);
}

// -- Exhaustive modifier × button matrix --

#[test]
fn all_modifier_combinations_all_buttons() {
    // 8 modifier combinations × 4 base buttons = 32 entries.
    let buttons = [
        (MouseButton::Left, 0u8),
        (MouseButton::Middle, 1),
        (MouseButton::Right, 2),
        (MouseButton::None, 3),
    ];
    let modifiers = [
        (false, false, false, 0u8),
        (true, false, false, 4),
        (false, true, false, 8),
        (false, false, true, 16),
        (true, true, false, 12),
        (true, false, true, 20),
        (false, true, true, 24),
        (true, true, true, 28),
    ];

    for (button, base_code) in &buttons {
        for (shift, alt, ctrl, mod_bits) in &modifiers {
            let expected_code = base_code + mod_bits;
            let mods = MouseModifiers {
                shift: *shift,
                alt: *alt,
                ctrl: *ctrl,
            };
            let code = apply_modifiers(button_code(*button, MouseEventKind::Press), mods);
            assert_eq!(
                code, expected_code,
                "{button:?} + shift={shift} alt={alt} ctrl={ctrl}: expected {expected_code}, got {code}",
            );

            // Verify SGR encoding round-trips correctly.
            let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
            let e = event_with_mods(*button, MouseEventKind::Press, 0, 0, mods);
            let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
            let s = std::str::from_utf8(&bytes).unwrap();
            assert!(
                s.starts_with(&format!("\x1b[<{expected_code};")),
                "{button:?} + mods({mod_bits}): SGR should start with code {expected_code}, got {s:?}",
            );
        }
    }
}

// --- URXVT encoding ---

#[test]
fn urxvt_left_click_at_origin() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT;
    let e = event(MouseButton::Left, MouseEventKind::Press, 0, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Button code = 32 + 0 = 32, col = 1, line = 1.
    assert_eq!(bytes, b"\x1b[32;1;1M");
}

#[test]
fn urxvt_right_click_at_large_coords() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT;
    let e = event(MouseButton::Right, MouseEventKind::Press, 500, 300);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Button code = 32 + 2 = 34, col = 501, line = 301.
    assert_eq!(bytes, b"\x1b[34;501;301M");
}

#[test]
fn urxvt_scroll_up() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT;
    let e = event(MouseButton::ScrollUp, MouseEventKind::Press, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Button code = 32 + 64 = 96, col = 6, line = 11.
    assert_eq!(bytes, b"\x1b[96;6;11M");
}

#[test]
fn urxvt_has_higher_priority_than_utf8() {
    // When both URXVT and UTF-8 are set, URXVT wins.
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::Left, MouseEventKind::Press, 0, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Should be URXVT format, not UTF-8.
    assert_eq!(bytes, b"\x1b[32;1;1M");
}

#[test]
fn sgr_has_higher_priority_than_urxvt() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR | TermMode::MOUSE_URXVT;
    let e = event(MouseButton::Left, MouseEventKind::Press, 0, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Should be SGR format.
    let s = std::str::from_utf8(&bytes).unwrap();
    assert!(
        s.starts_with("\x1b[<"),
        "SGR should take priority over URXVT"
    );
}

#[test]
fn urxvt_with_shift_modifier() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT;
    let e = MouseEvent {
        button: MouseButton::Left,
        kind: MouseEventKind::Press,
        col: 5,
        line: 3,
        mods: MouseModifiers {
            shift: true,
            alt: false,
            ctrl: false,
        },
        px: None,
        py: None,
        physical_px: None,
        in_grid: true,
    };
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Button code = 32 + 0 + 4 (shift) = 36, col = 6, line = 4.
    assert_eq!(bytes, b"\x1b[36;6;4M");
}

#[test]
fn urxvt_with_ctrl_modifier() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT;
    let e = MouseEvent {
        button: MouseButton::Left,
        kind: MouseEventKind::Press,
        col: 0,
        line: 0,
        mods: MouseModifiers {
            shift: false,
            alt: false,
            ctrl: true,
        },
        px: None,
        py: None,
        physical_px: None,
        in_grid: true,
    };
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // Button code = 32 + 0 + 16 (ctrl) = 48.
    assert_eq!(bytes, b"\x1b[48;1;1M");
}

#[test]
fn urxvt_release_uses_m_suffix() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT;
    let e = event(MouseButton::Left, MouseEventKind::Release, 5, 3);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    let s = std::str::from_utf8(&bytes).unwrap();
    // URXVT always uses 'M' suffix — no press/release distinction.
    assert!(
        s.ends_with('M'),
        "URXVT release should use M suffix, got: {s}"
    );
}

/// Regression: UTF-8 (mode 1005) reports a button RELEASE with X10 button
/// code 3 (low-bits), NOT the real button code — 1005 changes only the
/// coordinate transport, the Cb encoding is X10 (xterm ctlseqs). Left
/// release → 32 + 3 = byte 35.
#[test]
fn utf8_release_encodes_button_code_3() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::Left, MouseEventKind::Release, 5, 3);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // \x1b[M {32+code} {32+col+1} {32+line+1}; byte[3] is Cb.
    assert_eq!(
        bytes[3],
        32 + 3,
        "UTF-8 release Cb must be X10 code 3 (byte 35), not the real button"
    );
}

/// URXVT (mode 1015) "uses the same button encoding as X10" (xterm ctlseqs)
/// — a release reports decimal Cb = 35 (32 + 3), not the real button.
#[test]
fn urxvt_release_encodes_button_code_3() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT;
    let e = event(MouseButton::Left, MouseEventKind::Release, 5, 3);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(
        bytes, b"\x1b[35;6;4M",
        "URXVT release decimal Cb must be X10 code 3 (35)"
    );
}

/// Release code 3 still carries modifier bits (xterm: +4 shift / +8 alt /
/// +16 ctrl). Left+shift release under UTF-8 → 3 + 4 = byte 39.
#[test]
fn utf8_release_code_3_carries_modifier_bits() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8;
    let mut e = event(MouseButton::Left, MouseEventKind::Release, 5, 3);
    e.mods = MouseModifiers {
        shift: true,
        alt: false,
        ctrl: false,
    };
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(
        bytes[3],
        32 + 3 + 4,
        "UTF-8 release Cb = code 3 + shift(4) = byte 39"
    );
}

/// Companion rejecting the pre-fix behavior: UTF-8 release must NOT carry the
/// real Left button code (0 → byte 32).
#[test]
fn utf8_release_rejects_real_button_code() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8;
    let e = event(MouseButton::Left, MouseEventKind::Release, 5, 3);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_ne!(
        bytes[3], 32,
        "release must not encode the real Left button code (byte 32)"
    );
}

// --- X10 mode (mode 9) tests ---

#[test]
fn x10_mode_press_encodes_normally() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(bytes.len(), 6);
    assert!(bytes.starts_with(b"\x1b[M"));
    // Left(0), no modifiers; button byte = 32 + 0 = 32.
    assert_eq!(bytes[3], 32);
    // col=5: 32 + 1 + 5 = 38.
    assert_eq!(bytes[4], 38);
    // line=10: 32 + 1 + 10 = 43.
    assert_eq!(bytes[5], 43);
}

#[test]
fn x10_mode_release_is_suppressed() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::Left, MouseEventKind::Release, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.is_empty(), "X10 mode should not report releases");
}

#[test]
fn x10_mode_motion_is_suppressed() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::Left, MouseEventKind::Motion, 5, 10);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // X10 (mode 9) is press-only: it reports neither releases nor motion
    // (xterm ctlseqs). The encoder enforces this at the protocol boundary
    // (defense in depth), independent of the App-layer motion gate — so a
    // Motion event under bare mode 9 produces NO report rather than a
    // spurious `CSI M`.
    assert!(
        bytes.is_empty(),
        "X10 mode-9 is press-only — motion must be suppressed at the encoder"
    );
}

#[test]
fn x10_mode_strips_modifiers() {
    let mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    let mode = TermMode::MOUSE_X10;
    let e = event_with_mods(MouseButton::Left, MouseEventKind::Press, 0, 0, mods);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    // X10 mode should NOT add modifier bits (shift=4, alt=8, ctrl=16).
    // Button byte should be 32 + 0 = 32 (bare left click).
    assert_eq!(bytes[3], 32, "X10 mode should strip all modifiers");
}

#[test]
fn x10_mode_right_click_press() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::Right, MouseEventKind::Press, 3, 7);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(bytes.len(), 6);
    // Right(2), no modifiers; button byte = 32 + 2 = 34.
    assert_eq!(bytes[3], 34);
}

#[test]
fn x10_mode_middle_click_press() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::Middle, MouseEventKind::Press, 0, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(bytes.len(), 6);
    // Middle(1); button byte = 32 + 1 = 33.
    assert_eq!(bytes[3], 33);
}

#[test]
fn x10_mode_scroll_up_press() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::ScrollUp, MouseEventKind::Press, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(bytes.len(), 6);
    // ScrollUp(64), no modifiers; button byte = 32 + 64 = 96.
    assert_eq!(bytes[3], 96);
}

#[test]
fn x10_mode_scroll_down_press() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::ScrollDown, MouseEventKind::Press, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(bytes.len(), 6);
    // ScrollDown(65), no modifiers; button byte = 32 + 65 = 97.
    assert_eq!(bytes[3], 97);
}

#[test]
fn x10_mode_out_of_range_drops_event() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::Left, MouseEventKind::Press, 300, 300);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.is_empty(), "X10 mode should drop out-of-range events");
}

#[test]
fn x10_mode_scroll_release_is_suppressed() {
    let mode = TermMode::MOUSE_X10;
    let e = event(MouseButton::ScrollUp, MouseEventKind::Release, 5, 5);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.is_empty(), "X10 mode should suppress scroll releases");
}

// -- Dispatch-level coordinate boundary tests --
// These test the full encode_mouse_event dispatch path at Normal mode's
// 222/223 boundary (fencepost) to verify no off-by-one in dispatch.

#[test]
fn dispatch_normal_boundary_at_max_coord_succeeds() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    // col=222 is the max encodable coordinate in Normal mode.
    let e = event(MouseButton::Left, MouseEventKind::Press, 222, 222);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert_eq!(bytes.len(), 6, "col=222 should encode successfully");
    assert_eq!(bytes[4], 255); // 32 + 1 + 222 = 255
    assert_eq!(bytes[5], 255);
}

#[test]
fn dispatch_normal_boundary_one_past_max_drops() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    // col=223 exceeds Normal's limit — full dispatch should drop.
    let e = event(MouseButton::Left, MouseEventKind::Press, 223, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.is_empty(), "col=223 should be dropped via dispatch");
}

#[test]
fn dispatch_normal_boundary_line_past_max_drops() {
    let mode = TermMode::MOUSE_REPORT_CLICK;
    // line=223 also exceeds the limit.
    let e = event(MouseButton::Left, MouseEventKind::Press, 0, 223);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(bytes.is_empty(), "line=223 should be dropped via dispatch");
}

#[test]
fn dispatch_utf8_boundary_at_max_coord_succeeds() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8;
    // pos=2014: val = 32 + 1 + 2014 = 2047 = 0x7FF — max for 2-byte UTF-8.
    let e = event(MouseButton::Left, MouseEventKind::Press, 2014, 2014);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(!bytes.is_empty(), "pos=2014 should encode via dispatch");
}

#[test]
fn dispatch_utf8_boundary_one_past_max_drops() {
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8;
    // pos=2015: val = 32 + 1 + 2015 = 2048 > 0x7FF — out of range.
    let e = event(MouseButton::Left, MouseEventKind::Press, 2015, 0);
    let bytes = encode_mouse_event(&e, mode).as_bytes().to_vec();
    assert!(
        bytes.is_empty(),
        "pos=2015 should be dropped via UTF-8 dispatch"
    );
}

// -- App-level scenarios (documented, not unit-testable) --
// The following scenarios require the full App context and cannot be tested
// at the encoding layer:
// - Same-cell motion deduplication: App::report_mouse_motion skips reporting
// when mouse.last_reported_cell() matches the current cell. The dedup
// happens before encode_mouse_event is ever called.
// - Focus-loss button release synthesis: When the window loses focus with
// buttons held, the app should synthesize release events for all held
// buttons. This prevents apps (vim, tmux) from thinking buttons are still
// held after focus returns.

// -- parse_wheel_delta tests --

use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;

use super::parse_wheel_delta;

#[test]
fn wheel_line_delta_scroll_up() {
    // Positive y = scroll up. 1 notch × os_lines = expected lines.
    let os_lines = crate::platform::scroll::wheel_scroll_lines() as usize;
    let result = parse_wheel_delta(MouseScrollDelta::LineDelta(0.0, 1.0), 16.0);
    assert_eq!(result, Some((os_lines, true)));
}

#[test]
fn wheel_line_delta_scroll_down() {
    // Negative y = scroll down.
    let os_lines = crate::platform::scroll::wheel_scroll_lines() as usize;
    let result = parse_wheel_delta(MouseScrollDelta::LineDelta(0.0, -1.0), 16.0);
    assert_eq!(result, Some((os_lines, false)));
}

#[test]
fn wheel_line_delta_zero_returns_none() {
    let result = parse_wheel_delta(MouseScrollDelta::LineDelta(0.0, 0.0), 16.0);
    assert_eq!(result, None);
}

#[test]
fn wheel_line_delta_fractional_rounds_up() {
    // 0.5 notch * os_lines → ceil.
    let os_lines = crate::platform::scroll::wheel_scroll_lines() as f32;
    let expected = (0.5 * os_lines).ceil() as usize;
    let result = parse_wheel_delta(MouseScrollDelta::LineDelta(0.0, 0.5), 16.0);
    assert_eq!(result, Some((expected, true)));
}

#[test]
fn wheel_line_delta_tiny_produces_at_least_one() {
    // Very small delta * os_lines = small → ceil.max(1) = at least 1.
    let result = parse_wheel_delta(MouseScrollDelta::LineDelta(0.0, 0.01), 16.0);
    assert!(result.is_some());
    assert!(result.unwrap().0 >= 1);
}

#[test]
fn wheel_pixel_delta_scroll_up() {
    // 32px up with 16px cell height → ceil(32/16) = 2 lines.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 32.0));
    let result = parse_wheel_delta(delta, 16.0);
    assert_eq!(result, Some((2, true)));
}

#[test]
fn wheel_pixel_delta_scroll_down() {
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -48.0));
    let result = parse_wheel_delta(delta, 16.0);
    assert_eq!(result, Some((3, false)));
}

#[test]
fn wheel_pixel_delta_below_half_cell_returns_none() {
    // 7px with 16px cell height → 7 < 8 (half cell) → None.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 7.0));
    let result = parse_wheel_delta(delta, 16.0);
    assert_eq!(result, None);
}

#[test]
fn wheel_pixel_delta_exactly_half_cell_passes() {
    // Exactly at threshold: |y| < cell_height/2 uses strict less-than,
    // so 8.0 == 8.0 is NOT filtered. ceil(8/16) = 1.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 8.0));
    let result = parse_wheel_delta(delta, 16.0);
    assert_eq!(result, Some((1, true)));
}

#[test]
fn wheel_pixel_delta_just_above_half_cell() {
    // 8.01px with 16px cell → above threshold → ceil(8.01/16) = 1.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 8.01));
    let result = parse_wheel_delta(delta, 16.0);
    assert_eq!(result, Some((1, true)));
}

#[test]
fn wheel_pixel_delta_fractional_cell_rounds_up() {
    // 25px with 16px cell → ceil(25/16) = ceil(1.5625) = 2.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 25.0));
    let result = parse_wheel_delta(delta, 16.0);
    assert_eq!(result, Some((2, true)));
}

#[test]
fn wheel_pixel_delta_large_scroll() {
    // 200px with 16px cell → ceil(200/16) = ceil(12.5) = 13.
    let delta = MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, 200.0));
    let result = parse_wheel_delta(delta, 16.0);
    assert_eq!(result, Some((13, true)));
}

// --- Alternate scroll decision (should_translate_wheel_to_arrows) ---

use super::should_translate_wheel_to_arrows;

#[test]
fn alt_scroll_active_when_alt_screen_and_alternate_scroll_set() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    assert!(should_translate_wheel_to_arrows(mode, false));
}

#[test]
fn alt_scroll_inactive_without_alt_screen() {
    let mode = TermMode::ALTERNATE_SCROLL;
    assert!(!should_translate_wheel_to_arrows(mode, false));
}

#[test]
fn alt_scroll_inactive_without_alternate_scroll() {
    let mode = TermMode::ALT_SCREEN;
    assert!(!should_translate_wheel_to_arrows(mode, false));
}

#[test]
fn alt_scroll_inactive_when_shift_held() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    assert!(!should_translate_wheel_to_arrows(mode, true));
}

#[test]
fn alt_scroll_inactive_with_empty_mode() {
    assert!(!should_translate_wheel_to_arrows(TermMode::empty(), false));
}

/// Bridge cell: CSI ? 1049 h + default ALTERNATE_SCROLL parsed through
/// a real Term produces both ALT_SCREEN and ALTERNATE_SCROLL flags,
/// which `should_translate_wheel_to_arrows` then correctly enables.
#[test]
fn bridge_parser_to_alt_scroll_decision() {
    let mut term = oriterm_core::term::Term::new(
        24,
        80,
        0,
        oriterm_core::Theme::default(),
        oriterm_core::effect::VoidEffectSink,
    );

    /// Feed raw bytes through VTE processor.
    fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
        let mut processor = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
        processor.advance(term, bytes);
    }

    // Default: ALTERNATE_SCROLL is on but ALT_SCREEN is off → no arrows.
    let mode = term.mode();
    assert!(mode.contains(TermMode::ALTERNATE_SCROLL));
    assert!(!mode.contains(TermMode::ALT_SCREEN));
    assert!(!should_translate_wheel_to_arrows(mode, false));

    // Enter alt screen via mode 1049 → arrows enabled.
    feed(&mut term, b"\x1b[?1049h");
    let mode = term.mode();
    assert!(mode.contains(TermMode::ALT_SCREEN));
    assert!(mode.contains(TermMode::ALTERNATE_SCROLL));
    assert!(should_translate_wheel_to_arrows(mode, false));

    // Shift-bypass → arrows disabled even with both flags.
    assert!(!should_translate_wheel_to_arrows(mode, true));

    // DECRST alternate scroll → arrows disabled.
    feed(&mut term, b"\x1b[?1007l");
    let mode = term.mode();
    assert!(!should_translate_wheel_to_arrows(mode, false));

    // Leave alt screen → arrows disabled.
    feed(&mut term, b"\x1b[?1049l");
    let mode = term.mode();
    assert!(!should_translate_wheel_to_arrows(mode, false));
}

// : tier2_alt_scroll_payload + classify_wheel_event matrix.
// Verifies DECCKM-aware Tier-2 alt-scroll byte selection, the WheelTier
// dispatcher, and ANY_MOUSE_ENCODING isolation per xterm spec
// (ctlseqs.txt:2465-2473 + scrollbar.c:711-727).

use super::{
    AltScrollPayload, ScrollDirection, WheelTier, classify_wheel_event, should_report_mouse,
    tier2_alt_scroll_payload,
};

// --- Exact failing case (from repro) ---

/// Catalog row: DEC-ALT-SCROLL
/// Repro: `printf '\x1b[?1049h\x1b[?1007h\x1b[?1h'` then wheel up.
/// Pinned by (Phase 1 root cause §1A).
#[test]
fn tier2_alt_scroll_payload_alt_screen_plus_alternate_scroll_app_cursor_emits_ss3_arrow_up() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    let result = tier2_alt_scroll_payload(mode, false, 3, ScrollDirection::Up);
    assert_eq!(
        result,
        Some(AltScrollPayload {
            bytes: b"\x1bOA",
            repeat: 3,
        })
    );
}

// --- DECCKM × direction matrix (semantic apex) ---

/// Catalog row: DEC-ALT-SCROLL
/// xterm spec: ctlseqs.txt:2465-2473 (`Cursor Up | CSI A | SS3 A` table).
/// xterm impl: scrollbar.c:711-727 (`MODE_DECCKM ? ANSI_SS3 : ANSI_CSI`).
/// Ghostty impl: Surface.zig:3492-3506 (parity with xterm DECCKM split).
/// Pinned by (Phase 1.75 consensus).
#[test]
fn tier2_alt_scroll_payload_app_cursor_set_scroll_up_emits_ss3_a() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    let payload = tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Up).unwrap();
    assert_eq!(payload.bytes, b"\x1bOA");
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_app_cursor_set_scroll_down_emits_ss3_b() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    let payload = tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Down).unwrap();
    assert_eq!(payload.bytes, b"\x1bOB");
}

/// Catalog row: DEC-ALT-SCROLL
/// DECCKM clear → CSI normal-cursor sequence per xterm spec.
#[test]
fn tier2_alt_scroll_payload_app_cursor_clear_scroll_up_emits_csi_a() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    let payload = tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Up).unwrap();
    assert_eq!(payload.bytes, b"\x1b[A");
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_app_cursor_clear_scroll_down_emits_csi_b() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    let payload = tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Down).unwrap();
    assert_eq!(payload.bytes, b"\x1b[B");
}

// --- Repeat count pass-through ---

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_repeat_passes_through_lines_count() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    let payload = tier2_alt_scroll_payload(mode, false, 5, ScrollDirection::Up).unwrap();
    assert_eq!(payload.repeat, 5);
}

// --- Mode-gating cells (single-direction by design — gate short-circuits before direction) ---

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_no_alt_screen_returns_none() {
    let mode = TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    assert_eq!(
        tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Up),
        None
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_no_alternate_scroll_returns_none() {
    let mode = TermMode::ALT_SCREEN | TermMode::APP_CURSOR;
    assert_eq!(
        tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Up),
        None
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_empty_mode_returns_none() {
    assert_eq!(
        tier2_alt_scroll_payload(TermMode::empty(), false, 1, ScrollDirection::Up),
        None
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_shift_held_returns_none() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    assert_eq!(
        tier2_alt_scroll_payload(mode, true, 1, ScrollDirection::Up),
        None
    );
}

// --- Shift-bypass × DECCKM × direction (per Plan-TPR R1 [-]) ---

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_shift_held_app_cursor_set_scroll_up_returns_none() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    assert_eq!(
        tier2_alt_scroll_payload(mode, true, 1, ScrollDirection::Up),
        None
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_shift_held_app_cursor_set_scroll_down_returns_none() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    assert_eq!(
        tier2_alt_scroll_payload(mode, true, 1, ScrollDirection::Down),
        None
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_shift_held_app_cursor_clear_scroll_up_returns_none() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    assert_eq!(
        tier2_alt_scroll_payload(mode, true, 1, ScrollDirection::Up),
        None
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn tier2_alt_scroll_payload_shift_held_app_cursor_clear_scroll_down_returns_none() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    assert_eq!(
        tier2_alt_scroll_payload(mode, true, 1, ScrollDirection::Down),
        None
    );
}

// --- Cross-tier independence ---

/// Catalog row: DEC-ALT-SCROLL
/// MOUSE_REPORT_CLICK is in ANY_MOUSE; tier2_alt_scroll_payload still
/// returns Some — the Tier-1-precedes-Tier-2 ordering lives in the
/// caller (handle_mouse_wheel via classify_wheel_event), not in the
/// helper.
#[test]
fn tier2_alt_scroll_payload_with_mouse_report_click_still_returns_some() {
    let mode = TermMode::ALT_SCREEN
        | TermMode::ALTERNATE_SCROLL
        | TermMode::MOUSE_REPORT_CLICK
        | TermMode::APP_CURSOR;
    let payload = tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Up).unwrap();
    assert_eq!(payload.bytes, b"\x1bOA");
    assert_eq!(payload.repeat, 1);
}

// --- Property (DECCKM-aware byte selection per xterm spec) ---

/// Catalog row: DEC-ALT-SCROLL
/// xterm spec: ctlseqs.txt:2465-2473 (`Cursor Up | CSI A | SS3 A` table).
/// xterm impl: scrollbar.c:711-727 (`MODE_DECCKM ? ANSI_SS3 : ANSI_CSI`).
/// Ghostty impl: Surface.zig:3492-3506 (parity with xterm DECCKM split).
/// Pinned by §1B — load-bearing property: prevents
/// regression to hardcoded-SS3 implementation.
#[test]
fn tier2_alt_scroll_payload_decckm_aware_byte_selection_per_xterm_ctlseqs() {
    // DECCKM set → SS3 sequences
    let mode_app = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::APP_CURSOR;
    assert_eq!(
        tier2_alt_scroll_payload(mode_app, false, 1, ScrollDirection::Up)
            .unwrap()
            .bytes,
        b"\x1bOA"
    );
    assert_eq!(
        tier2_alt_scroll_payload(mode_app, false, 1, ScrollDirection::Down)
            .unwrap()
            .bytes,
        b"\x1bOB"
    );
    // DECCKM clear → CSI sequences
    let mode_normal = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    assert_eq!(
        tier2_alt_scroll_payload(mode_normal, false, 1, ScrollDirection::Up)
            .unwrap()
            .bytes,
        b"\x1b[A"
    );
    assert_eq!(
        tier2_alt_scroll_payload(mode_normal, false, 1, ScrollDirection::Down)
            .unwrap()
            .bytes,
        b"\x1b[B"
    );
}

// --- Regression guard (rejects pre-fix hardcoded SS3) ---

/// Catalog row: DEC-ALT-SCROLL
/// Regression guard: rejects the pre-fix hardcoded `if scroll_up { b"\x1bOA" } else { b"\x1bOB" }`
/// implementation that emitted SS3 unconditionally regardless of DECCKM.
#[test]
fn tier2_alt_scroll_payload_does_not_hardcode_ss3_when_decckm_clear() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    let payload = tier2_alt_scroll_payload(mode, false, 1, ScrollDirection::Up).unwrap();
    assert_ne!(payload.bytes, b"\x1bOA");
    assert_eq!(payload.bytes, b"\x1b[A");
}

// --- classify_wheel_event dispatcher matrix ---

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_mouse_report_click_set_no_shift_returns_mouse_report() {
    assert_eq!(
        classify_wheel_event(TermMode::MOUSE_REPORT_CLICK, false),
        WheelTier::MouseReport
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_mouse_drag_set_no_shift_returns_mouse_report() {
    assert_eq!(
        classify_wheel_event(TermMode::MOUSE_DRAG, false),
        WheelTier::MouseReport
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_mouse_motion_set_no_shift_returns_mouse_report() {
    assert_eq!(
        classify_wheel_event(TermMode::MOUSE_MOTION, false),
        WheelTier::MouseReport
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_mouse_x10_set_no_shift_returns_mouse_report() {
    assert_eq!(
        classify_wheel_event(TermMode::MOUSE_X10, false),
        WheelTier::MouseReport
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_alt_screen_plus_alternate_scroll_set_returns_alt_scroll() {
    assert_eq!(
        classify_wheel_event(TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL, false),
        WheelTier::AltScroll
    );
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_no_tier_applies_returns_viewport_scroll() {
    assert_eq!(
        classify_wheel_event(TermMode::empty(), false),
        WheelTier::ViewportScroll
    );
}

/// Catalog row: DEC-ALT-SCROLL
/// Tier-1-precedes-Tier-2 invariant: when both Tier-1 and Tier-2 conditions
/// hold, Tier-1 wins per the canonical ordering. Replaces the doc-only
/// invariant with executable verification.
#[test]
fn classify_wheel_event_mouse_report_click_with_alt_scroll_set_returns_mouse_report() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_REPORT_CLICK;
    assert_eq!(classify_wheel_event(mode, false), WheelTier::MouseReport);
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_mouse_drag_with_alt_scroll_set_returns_mouse_report() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_DRAG;
    assert_eq!(classify_wheel_event(mode, false), WheelTier::MouseReport);
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_mouse_motion_with_alt_scroll_set_returns_mouse_report() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_MOTION;
    assert_eq!(classify_wheel_event(mode, false), WheelTier::MouseReport);
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_mouse_x10_with_alt_scroll_set_returns_mouse_report() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_X10;
    assert_eq!(classify_wheel_event(mode, false), WheelTier::MouseReport);
}

// --- Shift-bypass × ANY_MOUSE (per Plan-TPR R3 [-]) ---

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_shift_held_mouse_report_click_with_alt_scroll_returns_viewport_scroll() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_REPORT_CLICK;
    assert_eq!(classify_wheel_event(mode, true), WheelTier::ViewportScroll);
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_shift_held_mouse_drag_with_alt_scroll_returns_viewport_scroll() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_DRAG;
    assert_eq!(classify_wheel_event(mode, true), WheelTier::ViewportScroll);
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_shift_held_mouse_motion_with_alt_scroll_returns_viewport_scroll() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_MOTION;
    assert_eq!(classify_wheel_event(mode, true), WheelTier::ViewportScroll);
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn classify_wheel_event_shift_held_mouse_x10_with_alt_scroll_returns_viewport_scroll() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL | TermMode::MOUSE_X10;
    assert_eq!(classify_wheel_event(mode, true), WheelTier::ViewportScroll);
}

/// Catalog row: DEC-ALT-SCROLL
/// Pure Tier-2 shift-bypass (no ANY_MOUSE flags) per Plan-TPR R5 [-].
#[test]
fn classify_wheel_event_shift_held_alt_scroll_only_returns_viewport_scroll() {
    let mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
    assert_eq!(classify_wheel_event(mode, true), WheelTier::ViewportScroll);
}

/// Catalog row: DEC-ALT-SCROLL
/// Empty-mode + shift-held edge cell — pins shift alone doesn't alter the
/// fallback dispatch.
#[test]
fn classify_wheel_event_empty_mode_with_shift_held_returns_viewport_scroll() {
    assert_eq!(
        classify_wheel_event(TermMode::empty(), true),
        WheelTier::ViewportScroll
    );
}

// --- ANY_MOUSE_ENCODING isolation rejection tests ---

/// Catalog row: DEC-ALT-SCROLL
/// Regression guard: MOUSE_SGR is in ANY_MOUSE_ENCODING (encoding format),
/// NOT in ANY_MOUSE (tracking trigger). Pins the bit-mask separation —
/// defensive against future drift that lumps encoding flags into ANY_MOUSE.
#[test]
fn should_report_mouse_mouse_sgr_alone_returns_false() {
    assert!(!should_report_mouse(TermMode::MOUSE_SGR, false));
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn should_report_mouse_mouse_utf8_alone_returns_false() {
    assert!(!should_report_mouse(TermMode::MOUSE_UTF8, false));
}

/// Catalog row: DEC-ALT-SCROLL
#[test]
fn should_report_mouse_mouse_urxvt_alone_returns_false() {
    assert!(!should_report_mouse(TermMode::MOUSE_URXVT, false));
}

// --- Self-verifying ANY_MOUSE membership completeness assertion ---

/// Catalog row: DEC-ALT-SCROLL
/// Self-verifying matrix completeness: iterates every ANY_MOUSE flag and
/// asserts dispatcher behavior + count. Any future ANY_MOUSE addition that
/// silently bypasses dispatcher coverage would fail this test.
#[test]
fn classify_wheel_event_any_mouse_member_count_assertion() {
    let any_mouse_flags = [
        TermMode::MOUSE_REPORT_CLICK,
        TermMode::MOUSE_DRAG,
        TermMode::MOUSE_MOTION,
        TermMode::MOUSE_X10,
    ];
    let mut count = 0;
    for flag in any_mouse_flags {
        assert_eq!(
            classify_wheel_event(flag, false),
            WheelTier::MouseReport,
            "flag {flag:?} must trigger MouseReport"
        );
        count += 1;
    }
    assert_eq!(count, 4, "iteration count must match flag list length");
    // Sanity-check that the union of the 4 flags equals TermMode::ANY_MOUSE
    // (would catch a future ANY_MOUSE bit addition that this test doesn't
    // exercise — proves the test list is comprehensive against the bit-set).
    let union = TermMode::MOUSE_REPORT_CLICK
        | TermMode::MOUSE_DRAG
        | TermMode::MOUSE_MOTION
        | TermMode::MOUSE_X10;
    assert_eq!(
        union,
        TermMode::ANY_MOUSE,
        "ANY_MOUSE flag set may have grown — extend the dispatcher matrix"
    );
}

// --- : dispatch_wheel wiring matrix ---
// Tests pin the wiring layer extracted from `App::handle_mouse_wheel`:
// `dispatch_wheel<S: WheelSink>(WheelDispatch, &mut S)` consumes a sink
// trait so the side-effect surface (PTY writes, viewport scroll, mark-dirty)
// can be matrix-tested without constructing the unconstructible-in-#[test]
// `App`. already pinned the pure decision functions
// (parse_wheel_delta, classify_wheel_event, tier2_alt_scroll_payload);
// these tests pin that the dispatcher's match arms call those decisions'
// implications on the sink in the right order, with the right operand
// counts, with the right bytes, and with the right post-match mark-dirty
// invariant.

use oriterm_mux::PaneId;

use super::wheel_dispatch::{WheelDispatch, WheelSink, dispatch_wheel};

/// Test sink that records every call. Lives in tests.rs (single consumer
/// Premature Abstraction`); promote to
/// `oriterm_test_support` only when a second sink type (e.g., 's
/// keyboard-wiring sink) creates a second concrete consumer.
/// `cell_for_report_value` is the value `dispatch_wheel`'s Tier-1 arm
/// receives when it asks the sink for the cell coordinate (None matches
/// pre-fix `mouse_cell_clamped() == None` semantics).
/// `cell_for_report_calls` counts how many times the dispatcher consulted
/// the sink — non-Tier-1 dispatches MUST leave this at 0 (pre-fix lazy
/// semantics; Round 1 WASTE finding fix).
#[derive(Default)]
struct RecordingSink {
    writes: Vec<(PaneId, Vec<u8>)>,
    scrolls: Vec<(PaneId, isize)>,
    mark_dirty_calls: usize,
    cell_for_report_value: Option<(usize, usize)>,
    cell_for_report_calls: usize,
    /// Set by fixtures before dispatch_wheel calls so the post-§16.2.0.B
    /// `write_pane_mouse_input` trait method can encode locally. In
    /// production this state lives on `Term` (read by `Term::handle_mouse_input`);
    /// for tests the sink stores it.
    recorded_mode: TermMode,
}

impl WheelSink for RecordingSink {
    fn write_pane_input(&mut self, pane_id: PaneId, bytes: &[u8]) {
        self.writes.push((pane_id, bytes.to_vec()));
    }
    fn write_pane_mouse_input(&mut self, pane_id: PaneId, event: &MouseEvent) {
        // Test seam: encode locally so existing `writes` assertions
        // keep working post-§16.2.0.B migration. Production path (App
        // impl) routes through mux.send_mouse_input → PaneIoCommand
        // pipe → Term::handle_mouse_input encoder.
        let report = encode_mouse_event(event, self.recorded_mode);
        let bytes = report.as_bytes();
        if !bytes.is_empty() {
            self.writes.push((pane_id, bytes.to_vec()));
        }
    }
    fn set_recorded_mode(&mut self, mode: TermMode) {
        self.recorded_mode = mode;
    }
    fn scroll_pane(&mut self, pane_id: PaneId, lines: isize) {
        self.scrolls.push((pane_id, lines));
    }
    fn mark_dirty(&mut self) {
        self.mark_dirty_calls += 1;
    }
    fn cell_for_report(&mut self) -> Option<(usize, usize)> {
        self.cell_for_report_calls += 1;
        self.cell_for_report_value
    }
}

/// Test pane id used by every dispatch_wheel cell. Constructed via
/// `from_raw` per `id/mod.rs:117` (the documented test-construction path).
const TEST_PANE_ID: u64 = 42;

fn pane() -> PaneId {
    PaneId::from_raw(TEST_PANE_ID)
}

/// `MouseScrollDelta::PixelDelta` for exactly `lines` cells in the Up
/// direction. Cell height is 16.0 — y=16 → ceil(1) = 1 line, y=48 → 3 lines.
/// Used instead of `LineDelta` because `LineDelta` consults
/// `platform::scroll::wheel_scroll_lines()` (3 on Windows, 1 on Linux),
/// which would couple test assertions to host platform configuration.
fn px_up(lines: usize) -> MouseScrollDelta {
    MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, (lines as f64) * 16.0))
}

fn px_down(lines: usize) -> MouseScrollDelta {
    MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -(lines as f64) * 16.0))
}

fn dispatch_with(
    mode: TermMode,
    shift_held: bool,
    pane_id: Option<PaneId>,
    delta: MouseScrollDelta,
) -> WheelDispatch {
    WheelDispatch {
        delta,
        cell_height: 16.0,
        mode,
        shift_held,
        pane_id,
        mods: MouseModifiers {
            shift: shift_held,
            alt: false,
            ctrl: false,
        },
        px: None,
        py: None,
        physical_px: None,
    }
}

/// Construct a `RecordingSink` pre-loaded with a Tier-1 cell value.
/// Tier-1 tests pre-load the cell here; non-Tier-1 tests use
/// `RecordingSink::default()` and assert the sink's
/// `cell_for_report_calls` stayed at 0 (lazy semantics).
fn sink_with_cell(cell: Option<(usize, usize)>) -> RecordingSink {
    RecordingSink {
        cell_for_report_value: cell,
        ..Default::default()
    }
}

// --- Tier 1 (mouse-report) wiring cells ---

/// Regression: Tier-1 single-line dispatch.
/// Pins `lines=1` produces exactly one `write_pane_input` call and one
/// `mark_dirty`. Catches a regression that drops the per-line loop
/// entirely (would produce zero writes).
#[test]
fn dispatch_wheel_mouse_report_lines_eq_1_writes_once_marks_dirty() {
    let mut sink = sink_with_cell(Some((10, 5)));
    let input = dispatch_with(TermMode::MOUSE_REPORT_CLICK, false, Some(pane()), px_up(1));
    dispatch_wheel(input, &mut sink);
    assert_eq!(sink.writes.len(), 1, "Tier-1 lines=1 must produce 1 write");
    assert!(sink.scrolls.is_empty(), "Tier-1 must not call scroll_pane");
    assert_eq!(
        sink.mark_dirty_calls, 1,
        "mark_dirty fires once per dispatch"
    );
}

/// Regression: Tier-1 fan-out property.
/// `lines=3` MUST produce exactly 3 writes (not 0, not 1, not 6). This is
/// the headline regression guard: a dispatcher that drops the loop, mishandles
/// payload.repeat, or double-writes per iteration would all fail this cell.
#[test]
fn dispatch_wheel_mouse_report_lines_eq_3_writes_thrice_marks_dirty_once() {
    let mut sink = sink_with_cell(Some((10, 5)));
    let input = dispatch_with(TermMode::MOUSE_REPORT_CLICK, false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(
        sink.writes.len(),
        3,
        "Tier-1 lines=3 must produce 3 writes (write count == lines)"
    );
    assert_eq!(sink.mark_dirty_calls, 1, "mark_dirty fires exactly once");
}

/// Regression: Tier-1 ScrollUp byte payload.
/// Recomputes the expected payload via the canonical `encode_mouse_event`
/// path and asserts the wiring writes those exact bytes — pinning that the
/// dispatcher passes the right `MouseButton::ScrollUp` to the encoder.
#[test]
fn dispatch_wheel_mouse_report_carries_scroll_up_bytes() {
    let mut sink = sink_with_cell(Some((7, 3)));
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let input = dispatch_with(mode, false, Some(pane()), px_up(1));
    dispatch_wheel(input, &mut sink);
    let event = MouseEvent {
        button: MouseButton::ScrollUp,
        kind: MouseEventKind::Press,
        col: 7,
        line: 3,
        mods: MouseModifiers::default(),
        px: None,
        py: None,
        physical_px: None,
        in_grid: true,
    };
    let expected = encode_mouse_event(&event, mode);
    assert_eq!(sink.writes.len(), 1);
    assert_eq!(sink.writes[0].0, pane());
    assert_eq!(sink.writes[0].1, expected.as_bytes().to_vec());
}

/// Regression: Tier-1 ScrollDown byte payload.
#[test]
fn dispatch_wheel_mouse_report_carries_scroll_down_bytes() {
    let mut sink = sink_with_cell(Some((7, 3)));
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let input = dispatch_with(mode, false, Some(pane()), px_down(1));
    dispatch_wheel(input, &mut sink);
    let event = MouseEvent {
        button: MouseButton::ScrollDown,
        kind: MouseEventKind::Press,
        col: 7,
        line: 3,
        mods: MouseModifiers::default(),
        px: None,
        py: None,
        physical_px: None,
        in_grid: true,
    };
    let expected = encode_mouse_event(&event, mode);
    assert_eq!(sink.writes[0].1, expected.as_bytes().to_vec());
}

/// Regression: Tier-1 cell_for_report=None early-return.
/// When `mouse_cell_clamped` returns None, Tier-1 returns BEFORE
/// `mark_dirty`. This cell pins `mark_dirty_calls == 0` so a regression
/// that hoists `sink.mark_dirty()` ahead of the cell_for_report check
/// (firing dirty unconditionally) would fail.
#[test]
fn dispatch_wheel_mouse_report_cell_for_report_none_writes_nothing_no_mark_dirty() {
    let mut sink = RecordingSink::default();
    // Sink left at default (cell_for_report_value: None) — Tier-1 arm
    // queries the sink and gets None back.
    let input = dispatch_with(TermMode::MOUSE_REPORT_CLICK, false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert!(sink.writes.is_empty());
    assert!(sink.scrolls.is_empty());
    assert_eq!(
        sink.mark_dirty_calls, 0,
        "early-return on cell_for_report=None must not fire mark_dirty"
    );
    assert_eq!(
        sink.cell_for_report_calls, 1,
        "Tier-1 arm asks the sink exactly once before the early return"
    );
}

// --- Tier 2 (alt-scroll) wiring cells ---

const ALT_SCROLL_MODE: TermMode = TermMode::ALT_SCREEN.union(TermMode::ALTERNATE_SCROLL);

/// Regression: Tier-2 lines=1 wiring.
#[test]
fn dispatch_wheel_alt_scroll_lines_eq_1_writes_once_csi_a() {
    let mut sink = RecordingSink::default();
    let input = dispatch_with(ALT_SCROLL_MODE, false, Some(pane()), px_up(1));
    dispatch_wheel(input, &mut sink);
    assert_eq!(sink.writes.len(), 1);
    assert_eq!(sink.writes[0].1, b"\x1b[A".to_vec());
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// Regression: Tier-2 fan-out property.
/// `payload.repeat == lines` MUST produce exactly that many writes. Catches
/// a regression that drops the loop or double-writes.
#[test]
fn dispatch_wheel_alt_scroll_lines_eq_3_writes_thrice_csi_a() {
    let mut sink = RecordingSink::default();
    let input = dispatch_with(ALT_SCROLL_MODE, false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(
        sink.writes.len(),
        3,
        "Tier-2 lines=3 must produce 3 writes (payload.repeat fan-out)"
    );
    for (pid, bytes) in &sink.writes {
        assert_eq!(*pid, pane());
        assert_eq!(bytes, &b"\x1b[A".to_vec());
    }
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// Regression: DECCKM × direction byte selection wiring
/// ( made the decision SS3-vs-CSI; this pins the wiring writes
/// the chosen bytes through the loop).
#[test]
fn dispatch_wheel_alt_scroll_app_cursor_set_up_emits_ss3_a() {
    let mut sink = RecordingSink::default();
    let mode = ALT_SCROLL_MODE | TermMode::APP_CURSOR;
    let input = dispatch_with(mode, false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(sink.writes.len(), 3);
    for (_, bytes) in &sink.writes {
        assert_eq!(bytes, &b"\x1bOA".to_vec());
    }
}

/// Regression: DECCKM set + ScrollDown writes SS3 B
/// (`\x1bOB`) `lines` times via the dispatcher.
#[test]
fn dispatch_wheel_alt_scroll_app_cursor_set_down_emits_ss3_b() {
    let mut sink = RecordingSink::default();
    let mode = ALT_SCROLL_MODE | TermMode::APP_CURSOR;
    let input = dispatch_with(mode, false, Some(pane()), px_down(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(sink.writes.len(), 3);
    for (_, bytes) in &sink.writes {
        assert_eq!(bytes, &b"\x1bOB".to_vec());
    }
}

/// Regression: DECCKM clear + ScrollUp writes CSI A
/// (`\x1b[A`); confirms 's DECCKM-aware byte selection
/// stays load-bearing across the wiring path.
#[test]
fn dispatch_wheel_alt_scroll_app_cursor_clear_up_emits_csi_a() {
    let mut sink = RecordingSink::default();
    let input = dispatch_with(ALT_SCROLL_MODE, false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    for (_, bytes) in &sink.writes {
        assert_eq!(bytes, &b"\x1b[A".to_vec());
    }
}

/// Regression: DECCKM clear + ScrollDown writes CSI B
/// (`\x1b[B`); negative-pin counterpart to the SS3 B / CSI A cells.
#[test]
fn dispatch_wheel_alt_scroll_app_cursor_clear_down_emits_csi_b() {
    let mut sink = RecordingSink::default();
    let input = dispatch_with(ALT_SCROLL_MODE, false, Some(pane()), px_down(3));
    dispatch_wheel(input, &mut sink);
    for (_, bytes) in &sink.writes {
        assert_eq!(bytes, &b"\x1b[B".to_vec());
    }
}

// --- Tier 3 (viewport-scroll) wiring cells ---

/// Regression: viewport scroll up positive sign.
#[test]
fn dispatch_wheel_viewport_scroll_up_calls_scroll_pane_positive_lines() {
    let mut sink = RecordingSink::default();
    let input = dispatch_with(TermMode::empty(), false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert!(sink.writes.is_empty());
    assert_eq!(sink.scrolls, vec![(pane(), 3)]);
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// Regression: viewport scroll down negative sign pin.
/// Catches a regression that flips the sign or always passes positive.
#[test]
fn dispatch_wheel_viewport_scroll_down_calls_scroll_pane_negative_lines() {
    let mut sink = RecordingSink::default();
    let input = dispatch_with(TermMode::empty(), false, Some(pane()), px_down(3));
    dispatch_wheel(input, &mut sink);
    assert!(sink.writes.is_empty());
    assert_eq!(sink.scrolls, vec![(pane(), -3)]);
    assert_eq!(sink.mark_dirty_calls, 1);
}

// --- Cross-tier precedence wiring ---

/// Regression: when MOUSE_REPORT and ALT_SCROLL flags are
/// BOTH set, `classify_wheel_event` returns Tier 1; this pins the WIRING
/// follows that verdict (writes mouse-report bytes, NOT SS3/CSI arrows).
/// Counterpart to the pure-classifier test at tests.rs ~1732.
#[test]
fn dispatch_wheel_mouse_report_takes_priority_over_alt_scroll() {
    let mode = TermMode::MOUSE_REPORT_CLICK | ALT_SCROLL_MODE;
    let mut sink = sink_with_cell(Some((4, 2)));
    let input = dispatch_with(mode, false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(
        sink.writes.len(),
        3,
        "Tier-1 wins precedence; lines=3 → 3 mouse-report writes"
    );
    // Bytes must NOT be SS3/CSI arrows (would prove Tier-2 stole the path).
    for (_, bytes) in &sink.writes {
        assert_ne!(bytes, &b"\x1b[A".to_vec(), "must NOT be CSI A");
        assert_ne!(bytes, &b"\x1bOA".to_vec(), "must NOT be SS3 A");
    }
}

// --- Shift-bypass × all three tiers ---

/// Regression: Shift bypasses Tier-1 mouse-report → Tier-3.
/// Catches a regression where the shift gate stops working for mouse-report
/// alone (only working when alt-scroll is also set).
#[test]
fn dispatch_wheel_shift_held_with_mouse_report_routes_to_viewport_scroll() {
    // Shift bypass routes Tier-1 → Tier-3; sink_with_cell's value is
    // unused because Tier-3 doesn't consult `cell_for_report`. Asserting
    // `cell_for_report_calls == 0` pins the lazy semantics.
    let mut sink = sink_with_cell(Some((4, 2)));
    let input = dispatch_with(TermMode::MOUSE_REPORT_CLICK, true, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert!(sink.writes.is_empty(), "shift bypass → no PTY writes");
    assert_eq!(sink.scrolls, vec![(pane(), 3)]);
    assert_eq!(sink.mark_dirty_calls, 1);
    assert_eq!(
        sink.cell_for_report_calls, 0,
        "shift bypass routes to Tier-3 — cell_for_report MUST stay lazy"
    );
}

/// Regression: Shift bypasses Tier-2 alt-scroll → Tier-3.
#[test]
fn dispatch_wheel_shift_held_with_alt_scroll_routes_to_viewport_scroll() {
    let mut sink = RecordingSink::default();
    let input = dispatch_with(ALT_SCROLL_MODE, true, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert!(sink.writes.is_empty(), "shift bypass → no PTY writes");
    assert_eq!(sink.scrolls, vec![(pane(), 3)]);
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// Regression: Shift bypass on plain mouse-report (no
/// ALT_SCREEN) preserves direction sign through Tier-3.
#[test]
fn dispatch_wheel_shift_held_with_mouse_report_no_alt_screen_direction_preserved() {
    let mut sink = sink_with_cell(Some((4, 2)));
    let input = dispatch_with(TermMode::MOUSE_REPORT_CLICK, true, Some(pane()), px_down(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(
        sink.scrolls,
        vec![(pane(), -3)],
        "down direction preserved through shift bypass"
    );
}

// --- Cross-tier early-return invariants ---

/// Regression: parse_wheel_delta=None → no dispatch at all.
/// Zero-magnitude line delta produces no writes / scrolls / mark_dirty.
#[test]
fn dispatch_wheel_parse_wheel_delta_none_no_dispatch() {
    let mut sink = sink_with_cell(Some((4, 2)));
    let input = dispatch_with(
        TermMode::MOUSE_REPORT_CLICK,
        false,
        Some(pane()),
        MouseScrollDelta::LineDelta(0.0, 0.0), // returns None from parse_wheel_delta
    );
    dispatch_wheel(input, &mut sink);
    assert!(sink.writes.is_empty());
    assert!(sink.scrolls.is_empty());
    assert_eq!(sink.mark_dirty_calls, 0);
    assert_eq!(
        sink.cell_for_report_calls, 0,
        "parse_wheel_delta=None must return BEFORE the Tier-1 sink query"
    );
}

/// Regression: pane_id=None → no dispatch at all.
/// The `Option<PaneId>` lift in WheelDispatch ('s catch — the
/// `let Some(pane_id) =... else { return }` gate exists in the dispatcher,
/// not just on the App side) makes this seam testable headlessly.
#[test]
fn dispatch_wheel_pane_id_none_no_dispatch() {
    let mut sink = sink_with_cell(Some((4, 2)));
    let input = dispatch_with(
        TermMode::MOUSE_REPORT_CLICK,
        false,
        None, // pane_id
        px_up(3),
    );
    dispatch_wheel(input, &mut sink);
    assert!(sink.writes.is_empty());
    assert!(sink.scrolls.is_empty());
    assert_eq!(sink.mark_dirty_calls, 0);
    assert_eq!(
        sink.cell_for_report_calls, 0,
        "pane_id=None must return BEFORE the Tier-1 sink query"
    );
}

/// Regression: Tier-1 mods (Alt/Ctrl/Shift, with Shift NOT
/// held to avoid Tier-3 bypass) flow through the dispatcher into
/// `encode_mouse_event` so the encoded byte payload reflects the modifiers.
/// Pins that `dispatch_wheel` passes `input.mods` to the encoder rather
/// than hardcoding `MouseModifiers::default()`.
#[test]
fn dispatch_wheel_mouse_report_propagates_alt_ctrl_modifiers_to_encoded_bytes() {
    let mut sink = sink_with_cell(Some((11, 7)));
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let mods = MouseModifiers {
        shift: false,
        alt: true,
        ctrl: true,
    };
    let input = WheelDispatch {
        delta: px_up(1),
        cell_height: 16.0,
        mode,
        shift_held: false,
        pane_id: Some(pane()),
        mods,
        px: None,
        py: None,
        physical_px: None,
    };
    dispatch_wheel(input, &mut sink);
    let event = MouseEvent {
        button: MouseButton::ScrollUp,
        kind: MouseEventKind::Press,
        col: 11,
        line: 7,
        mods,
        px: None,
        py: None,
        physical_px: None,
        in_grid: true,
    };
    let expected = encode_mouse_event(&event, mode);
    assert_eq!(sink.writes.len(), 1);
    assert_eq!(sink.writes[0].1, expected.as_bytes().to_vec());
    let no_mods_event = MouseEvent {
        button: MouseButton::ScrollUp,
        kind: MouseEventKind::Press,
        col: 11,
        line: 7,
        mods: MouseModifiers::default(),
        px: None,
        py: None,
        physical_px: None,
        in_grid: true,
    };
    let no_mods_expected = encode_mouse_event(&no_mods_event, mode);
    assert_ne!(
        sink.writes[0].1,
        no_mods_expected.as_bytes().to_vec(),
        "alt+ctrl payload must differ from no-mods payload — proves dispatch_wheel propagates input.mods"
    );
    assert_eq!(sink.mark_dirty_calls, 1);
}

/// Regression: empty-bytes guard preserved from
/// at the dispatcher seam. Normal (X10-style coordinate-encoded) mode +
/// `col > 222` makes `encode_mouse_event` return empty bytes; the
/// `if !bytes.is_empty()` guard must skip the write loop AND
/// `mark_dirty` must still fire (the dispatch reached past the early
/// returns; only the side-effect bytes were elided).
#[test]
fn dispatch_wheel_mouse_report_empty_bytes_skips_writes_but_marks_dirty() {
    // col > 222 → encode_normal returns empty bytes per encode.rs:199.
    let mut sink = sink_with_cell(Some((300, 0)));
    // Normal mode (no SGR / UTF-8 / URXVT flags).
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let input = WheelDispatch {
        delta: px_up(2),
        cell_height: 16.0,
        mode,
        shift_held: false,
        pane_id: Some(pane()),
        mods: MouseModifiers::default(),
        px: None,
        py: None,
        physical_px: None,
    };
    dispatch_wheel(input, &mut sink);
    assert!(
        sink.writes.is_empty(),
        "empty-bytes guard must skip writes when encode returns empty"
    );
    assert!(sink.scrolls.is_empty());
    assert_eq!(
        sink.mark_dirty_calls, 1,
        "mark_dirty must still fire — dispatch reached past early returns"
    );
}

/// Regression: TPR R1 — Tier-2 alt-scroll dispatch MUST NOT
/// query `WheelSink::cell_for_report`. Restores pre-fix lazy semantics
/// where `mouse_cell_clamped()` was scoped to the Tier-1 arm; a regression
/// that re-introduces eager hit-testing would land here.
#[test]
fn dispatch_wheel_alt_scroll_does_not_query_cell_for_report() {
    let mut sink = sink_with_cell(Some((4, 2)));
    let input = dispatch_with(ALT_SCROLL_MODE, false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(sink.writes.len(), 3, "Tier-2 still writes lines × bytes");
    assert_eq!(
        sink.cell_for_report_calls, 0,
        "Tier-2 alt-scroll MUST NOT consult sink.cell_for_report"
    );
}

/// Regression: TPR R1 — Tier-3 viewport-scroll dispatch MUST
/// NOT query `WheelSink::cell_for_report`. Counterpart to the Tier-2 lazy
/// pin; pins the cell-extraction is gated by tier classification.
#[test]
fn dispatch_wheel_viewport_scroll_does_not_query_cell_for_report() {
    let mut sink = sink_with_cell(Some((4, 2)));
    let input = dispatch_with(TermMode::empty(), false, Some(pane()), px_up(3));
    dispatch_wheel(input, &mut sink);
    assert_eq!(sink.scrolls, vec![(pane(), 3)]);
    assert_eq!(
        sink.cell_for_report_calls, 0,
        "Tier-3 viewport scroll MUST NOT consult sink.cell_for_report"
    );
}

// --- : cross-site convergence pin ---
// Pinned by Plan-review round 0 F2: keyboard arrow encoding via the public
// `encode_key` dispatcher must produce IDENTICAL bytes to mouse-wheel
// `tier2_alt_scroll_payload` for the (DECCKM × direction) cells the wheel
// path touches (Up/Down × app_cursor/normal). Any future divergence between
// the keyboard and mouse paths breaks these tests — the SSOT helper at
// `crate::key_encoding::cursor_keys::cursor_key_bytes` is the canonical home
// for the (DECCKM × direction) → bytes mapping.

mod cross_site_convergence {
    use crate::app::mouse_report::{ScrollDirection, tier2_alt_scroll_payload};
    use crate::key_encoding::{KeyEventType, KeyInput, Modifiers, encode_key};
    use oriterm_core::TermMode;
    use winit::keyboard::{Key, KeyLocation, NamedKey};

    fn keyboard_bytes(named: NamedKey, mode: TermMode) -> Vec<u8> {
        let key = Key::Named(named);
        encode_key(&KeyInput {
            key: &key,
            mods: Modifiers::empty(),
            mode,
            text: None,
            location: KeyLocation::Standard,
            event_type: KeyEventType::Press,
            alternate_key: None,
        })
    }

    fn alt_scroll_bytes(direction: ScrollDirection, app_cursor: bool) -> Vec<u8> {
        let mut mode = TermMode::ALT_SCREEN | TermMode::ALTERNATE_SCROLL;
        if app_cursor {
            mode |= TermMode::APP_CURSOR;
        }
        tier2_alt_scroll_payload(mode, false, 1, direction)
            .expect("alt-scroll path active under ALT_SCREEN | ALTERNATE_SCROLL")
            .bytes
            .to_vec()
    }

    /// Cross-site convergence: ArrowUp under DECCKM (APP_CURSOR set) — both
    /// paths emit `\x1bOA`.
    #[test]
    fn arrow_up_app_cursor_keyboard_matches_alt_scroll() {
        let mode = TermMode::APP_CURSOR;
        assert_eq!(
            keyboard_bytes(NamedKey::ArrowUp, mode),
            alt_scroll_bytes(ScrollDirection::Up, true)
        );
    }

    /// Cross-site convergence: ArrowUp without DECCKM — both paths emit `\x1b[A`.
    #[test]
    fn arrow_up_normal_keyboard_matches_alt_scroll() {
        assert_eq!(
            keyboard_bytes(NamedKey::ArrowUp, TermMode::empty()),
            alt_scroll_bytes(ScrollDirection::Up, false)
        );
    }

    /// Cross-site convergence: ArrowDown under DECCKM — both paths emit `\x1bOB`.
    #[test]
    fn arrow_down_app_cursor_keyboard_matches_alt_scroll() {
        let mode = TermMode::APP_CURSOR;
        assert_eq!(
            keyboard_bytes(NamedKey::ArrowDown, mode),
            alt_scroll_bytes(ScrollDirection::Down, true)
        );
    }

    /// Cross-site convergence: ArrowDown without DECCKM — both paths emit `\x1b[B`.
    #[test]
    fn arrow_down_normal_keyboard_matches_alt_scroll() {
        assert_eq!(
            keyboard_bytes(NamedKey::ArrowDown, TermMode::empty()),
            alt_scroll_bytes(ScrollDirection::Down, false)
        );
    }
}

/// Pure `clamp_mouse_pixel_coords` helper returns `(Some, Some)` for
/// every position when surface inputs are valid, clamping out-of-bounds
/// positions to the nearest valid PHYSICAL (device) pixel. Returns
/// `(None, None)` ONLY for the "no usable surface" cases (zero-size or
/// sub-pixel bounds, non-finite input).
///
/// See: bug-tracker/plans/BUG-08-057/
#[cfg(test)]
mod clamp_mouse_pixel_coords_matrix {
    use winit::dpi::PhysicalPosition;

    use crate::app::mouse_report::clamp_mouse_pixel_coords;

    fn pos(x: f64, y: f64) -> PhysicalPosition<f64> {
        PhysicalPosition::new(x, y)
    }

    // -- §03 t27–t31: clamping + scale --

    /// In-grid cursor → `(Some(rel_x), Some(rel_y))`. Baseline.
    #[test]
    fn clamp_in_grid_returns_some_some() {
        let result = clamp_mouse_pixel_coords(pos(50.0, 100.0), (0.0, 0.0), (200.0, 200.0));
        assert_eq!(result, (Some(50), Some(100)));
    }

    /// Cursor exactly at `bounds.right()` (off-by-one from in-bounds):
    /// clamp to `width - 1`. Pre-fix returned `(None, None)`.
    #[test]
    fn clamp_at_right_edge_clamps_to_width_minus_one() {
        let result = clamp_mouse_pixel_coords(pos(200.0, 100.0), (0.0, 0.0), (200.0, 200.0));
        assert_eq!(result, (Some(199), Some(100)));
    }

    /// Cursor at `bounds.bottom()` → clamp to `height - 1`. Mirror.
    #[test]
    fn clamp_at_bottom_edge_clamps_to_height_minus_one() {
        let result = clamp_mouse_pixel_coords(pos(50.0, 200.0), (0.0, 0.0), (200.0, 200.0));
        assert_eq!(result, (Some(50), Some(199)));
    }

    /// Cursor outside top-left → clamp to `(0, 0)`. Pre-fix returned
    /// `(None, None)`.
    #[test]
    fn clamp_outside_top_left_clamps_to_origin() {
        let result = clamp_mouse_pixel_coords(pos(-10.0, -5.0), (0.0, 0.0), (200.0, 200.0));
        assert_eq!(result, (Some(0), Some(0)));
    }

    /// Emits PHYSICAL (device) pixels — `rel` directly, NO `/scale`
    /// division. A 100px in-grid offset reports 100, not
    /// `100/scale`. This is the load-bearing Option-B change: SGR-Pixel
    /// coords now match the physical CSI 14t report so notcurses decodes
    /// the correct cell at every DPI scale.
    #[test]
    fn clamp_emits_physical_rel_no_scale_division() {
        let result = clamp_mouse_pixel_coords(pos(100.0, 200.0), (0.0, 0.0), (400.0, 400.0));
        assert_eq!(result, (Some(100), Some(200)));
    }

    // -- §03 t32–t34: no-usable-surface cases --

    /// Zero-width bounds → `(None, None)`.
    #[test]
    fn clamp_returns_none_for_zero_width_bounds() {
        let result = clamp_mouse_pixel_coords(pos(50.0, 100.0), (0.0, 0.0), (0.0, 200.0));
        assert_eq!(result, (None, None));
    }

    /// Zero-height bounds → `(None, None)`.
    #[test]
    fn clamp_returns_none_for_zero_height_bounds() {
        let result = clamp_mouse_pixel_coords(pos(50.0, 100.0), (0.0, 0.0), (200.0, 0.0));
        assert_eq!(result, (None, None));
    }

    /// Structural: the pure helper has NO App / TermWindow / wgpu
    /// dependencies. Reviewers grep this to confirm the headless
    /// contract holds.
    #[test]
    fn clamp_pure_helper_does_not_require_app_construction() {
        // If the helper signature ever takes `&App` / `&TermWindow` /
        // `&wgpu::Device`, this test will not compile.
        let _: fn(PhysicalPosition<f64>, (f64, f64), (f64, f64)) -> (Option<u32>, Option<u32>) =
            clamp_mouse_pixel_coords;
    }

    // -- §03 t36 / t37: wheel-edge alignment via offset bounds --

    /// Bounds with non-zero origin: clamp respects origin offset.
    /// Simulates the grid widget laid out at `(8, 36)` (typical chrome
    /// inset). Cursor at `(208, 100)` is past `right` by 0.5 px →
    /// clamps to `width - 1 = 199` physical (device) pixel.
    #[test]
    fn clamp_respects_non_zero_bounds_origin() {
        let result = clamp_mouse_pixel_coords(pos(208.0, 100.0), (8.0, 36.0), (200.0, 200.0));
        assert_eq!(result, (Some(199), Some(64)));
    }

    /// Cursor at the bounds origin → `(Some(0), Some(0))` after the
    /// origin subtraction.
    #[test]
    fn clamp_at_bounds_origin_returns_origin() {
        let result = clamp_mouse_pixel_coords(pos(8.0, 36.0), (8.0, 36.0), (200.0, 200.0));
        assert_eq!(result, (Some(0), Some(0)));
    }

    // -- subpixel + non-finite input guards --

    /// Sub-pixel positive bounds (width 0.5) → `(None, None)`. Without
    /// the `< 1.0` guard, `f64::clamp(0.0, -0.5)` would panic per Rust
    /// std docs ("Panics if min > max").
    #[test]
    fn clamp_subpixel_width_returns_none() {
        let result = clamp_mouse_pixel_coords(pos(0.25, 0.0), (0.0, 0.0), (0.5, 200.0));
        assert_eq!(result, (None, None));
    }

    /// Sub-pixel positive bounds (height 0.5) → `(None, None)`.
    #[test]
    fn clamp_subpixel_height_returns_none() {
        let result = clamp_mouse_pixel_coords(pos(0.0, 0.25), (0.0, 0.0), (200.0, 0.5));
        assert_eq!(result, (None, None));
    }

    /// NaN cursor position → `(None, None)`. Without the finite guard,
    /// `(NaN).clamp(0.0, 199.0)` would return NaN and `(NaN as u32)` is
    /// platform-defined, silently fabricating origin coords.
    #[test]
    fn clamp_nan_position_returns_none() {
        let result = clamp_mouse_pixel_coords(pos(f64::NAN, 100.0), (0.0, 0.0), (200.0, 200.0));
        assert_eq!(result, (None, None));
    }

    /// Infinity cursor position → `(None, None)`. Companion to t41 —
    /// non-finite inputs of any flavor reject up front.
    #[test]
    fn clamp_infinite_position_returns_none() {
        let result =
            clamp_mouse_pixel_coords(pos(f64::INFINITY, 100.0), (0.0, 0.0), (200.0, 200.0));
        assert_eq!(result, (None, None));
    }

    // -- §03 Matrix C: cross-surface notcurses decode (the unit-mismatch proof) --

    /// Model notcurses' wire→cell decode: it reads `cellpxy` from the CSI 14t
    /// report (PHYSICAL cell-pixel height), then for an SGR-Pixel wire coord
    /// `w` (1-based) computes the row as `(w - 1) / cellpxy` (xterm/notcurses
    /// decrement-then-divide). `pch` is the physical cell-pixel height.
    fn notcurses_decode_row(wire_y: u32, pch: u32) -> u32 {
        (wire_y - 1) / pch
    }

    /// Regression: BUG-06-112 — with the encoder emitting PHYSICAL pixels,
    /// notcurses recovers the clicked row EXACTLY at DPI scales > 1.0. The
    /// pre-fix logical encoder (`physical / scale`) mis-decoded because
    /// notcurses' `cellpxy` is physical. Calls PRODUCTION
    /// `clamp_mouse_pixel_coords` (per `feedback_test_real_code_paths`);
    /// only the external notcurses decode is modelled.
    #[test]
    fn cross_surface_decode_exact_physical() {
        // Physical cell height 20px, grid 24 rows → 480px tall.
        let pch = 20u32;
        let bounds = (0.0, 0.0);
        let size = (200.0, 480.0);
        // A physical cursor at y=100 is in row 100/20 = 5. (At scale 1.25
        // the pre-fix logical encoder sent 100/1.25 = 80 → notcurses decoded
        // 80/20 = 4 — the wrong row that rolled up the menu.)
        for (phys_y, expected_row) in [(0u32, 0u32), (20, 1), (100, 5), (479, 23)] {
            let (_x, y) = clamp_mouse_pixel_coords(pos(50.0, f64::from(phys_y)), bounds, size);
            let wire_y = y.expect("in-grid → Some") + 1; // encoder sends py+1
            assert_eq!(
                notcurses_decode_row(wire_y, pch),
                expected_row,
                "physical y={phys_y} must decode to row {expected_row}",
            );
        }
    }

    /// Test-cell inventory for the `clamp_mouse_pixel_coords` matrix.
    /// Removing or renaming a referenced test fires a compile error
    /// (dangling fn pointer). The `CELLS.len()` assertion documents the
    /// expected size but does NOT auto-detect a new test added to this
    /// module without being registered in `CELLS` — that case is bounded
    /// by code-review convention. When adding a new clamp test (t44, t45,
    /// ...), register it in `CELLS` below alongside `assert_eq!`.
    #[test]
    fn app_matrix_completeness() {
        const CELLS: &[fn()] = &[
            clamp_in_grid_returns_some_some,
            clamp_at_right_edge_clamps_to_width_minus_one,
            clamp_at_bottom_edge_clamps_to_height_minus_one,
            clamp_outside_top_left_clamps_to_origin,
            clamp_emits_physical_rel_no_scale_division,
            clamp_returns_none_for_zero_width_bounds,
            clamp_returns_none_for_zero_height_bounds,
            clamp_pure_helper_does_not_require_app_construction,
            clamp_respects_non_zero_bounds_origin,
            clamp_at_bounds_origin_returns_origin,
            clamp_subpixel_width_returns_none,
            clamp_subpixel_height_returns_none,
            clamp_nan_position_returns_none,
            clamp_infinite_position_returns_none,
            cross_surface_decode_exact_physical,
        ];
        assert_eq!(
            CELLS.len(),
            15,
            "expected 15 clamp_mouse_pixel_coords test cells (the scale param was dropped: \
             scale param: scale-2 test → physical-rel pin; non-positive-scale removed; \
             cross-surface notcurses-decode pin added); a missing entry here means a \
             test was added without registering it"
        );
    }
}

/// Pin: winit→semantic button mapping for DEC Locator observation
/// dispatch. Left/Middle/Right map to their semantic counterparts;
/// Back/Forward (and any other) map to None (DEC Locator reports only
/// the three primary buttons per xterm button.c:944-948).
#[test]
fn locator_semantic_button_maps_primary_buttons_only() {
    use winit::event::MouseButton as WB;

    assert_eq!(
        super::locator_semantic_button(WB::Left),
        Some(MouseButton::Left)
    );
    assert_eq!(
        super::locator_semantic_button(WB::Middle),
        Some(MouseButton::Middle)
    );
    assert_eq!(
        super::locator_semantic_button(WB::Right),
        Some(MouseButton::Right)
    );
    assert_eq!(super::locator_semantic_button(WB::Back), None);
    assert_eq!(super::locator_semantic_button(WB::Forward), None);
    assert_eq!(super::locator_semantic_button(WB::Other(7)), None);
}
