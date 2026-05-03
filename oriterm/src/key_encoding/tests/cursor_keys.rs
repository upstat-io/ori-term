//! Unit tests for the DECCKM cursor-key SSOT helper.
//!
//! Pinned by BUG-08-033 (xterm `ctlseqs.txt:2465-2473`).

use crate::key_encoding::cursor_keys::{CursorKey, cursor_key_bytes};

// --- 12-cell cursor_key_bytes matrix (6 keys × 2 DECCKM modes) ---

#[test]
fn cursor_key_bytes_up_app_cursor_returns_ss3_a() {
    assert_eq!(cursor_key_bytes(CursorKey::Up, true), b"\x1bOA");
}

#[test]
fn cursor_key_bytes_up_normal_returns_csi_a() {
    assert_eq!(cursor_key_bytes(CursorKey::Up, false), b"\x1b[A");
}

#[test]
fn cursor_key_bytes_down_app_cursor_returns_ss3_b() {
    assert_eq!(cursor_key_bytes(CursorKey::Down, true), b"\x1bOB");
}

#[test]
fn cursor_key_bytes_down_normal_returns_csi_b() {
    assert_eq!(cursor_key_bytes(CursorKey::Down, false), b"\x1b[B");
}

#[test]
fn cursor_key_bytes_right_app_cursor_returns_ss3_c() {
    assert_eq!(cursor_key_bytes(CursorKey::Right, true), b"\x1bOC");
}

#[test]
fn cursor_key_bytes_right_normal_returns_csi_c() {
    assert_eq!(cursor_key_bytes(CursorKey::Right, false), b"\x1b[C");
}

#[test]
fn cursor_key_bytes_left_app_cursor_returns_ss3_d() {
    assert_eq!(cursor_key_bytes(CursorKey::Left, true), b"\x1bOD");
}

#[test]
fn cursor_key_bytes_left_normal_returns_csi_d() {
    assert_eq!(cursor_key_bytes(CursorKey::Left, false), b"\x1b[D");
}

#[test]
fn cursor_key_bytes_home_app_cursor_returns_ss3_h() {
    assert_eq!(cursor_key_bytes(CursorKey::Home, true), b"\x1bOH");
}

#[test]
fn cursor_key_bytes_home_normal_returns_csi_h() {
    assert_eq!(cursor_key_bytes(CursorKey::Home, false), b"\x1b[H");
}

#[test]
fn cursor_key_bytes_end_app_cursor_returns_ss3_f() {
    assert_eq!(cursor_key_bytes(CursorKey::End, true), b"\x1bOF");
}

#[test]
fn cursor_key_bytes_end_normal_returns_csi_f() {
    assert_eq!(cursor_key_bytes(CursorKey::End, false), b"\x1b[F");
}

// --- 6-cell terminator() unit tests (modifier-CSI form pin) ---

#[test]
fn cursor_key_terminator_up_returns_a() {
    assert_eq!(CursorKey::Up.terminator(), b'A');
}

#[test]
fn cursor_key_terminator_down_returns_b() {
    assert_eq!(CursorKey::Down.terminator(), b'B');
}

#[test]
fn cursor_key_terminator_right_returns_c() {
    assert_eq!(CursorKey::Right.terminator(), b'C');
}

#[test]
fn cursor_key_terminator_left_returns_d() {
    assert_eq!(CursorKey::Left.terminator(), b'D');
}

#[test]
fn cursor_key_terminator_home_returns_h() {
    assert_eq!(CursorKey::Home.terminator(), b'H');
}

#[test]
fn cursor_key_terminator_end_returns_f() {
    assert_eq!(CursorKey::End.terminator(), b'F');
}
