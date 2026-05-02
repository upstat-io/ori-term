//! Modifier-encoding matrix: Ctrl C0 codes, Alt prefix, modifier parameters,
//! F-key dispatch, bare modifier keys, and winit conversion.

use winit::keyboard::{Key, ModifiersState, NamedKey};

use super::{Modifiers, enc, enc_text, no_mode};

// --- Ctrl+letter C0 codes ---

#[test]
fn ctrl_a() {
    let r = enc(Key::Character("a".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x01]);
}

#[test]
fn ctrl_c() {
    let r = enc(Key::Character("c".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x03]);
}

#[test]
fn ctrl_d() {
    let r = enc(Key::Character("d".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x04]);
}

#[test]
fn ctrl_z() {
    let r = enc(Key::Character("z".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x1a]);
}

#[test]
fn ctrl_a_uppercase() {
    let r = enc(Key::Character("A".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x01]);
}

#[test]
fn ctrl_space() {
    let r = enc(Key::Named(NamedKey::Space), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x00]);
}

#[test]
fn ctrl_bracket_esc() {
    let r = enc(Key::Character("[".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x1b]);
}

#[test]
fn ctrl_backslash() {
    let r = enc(Key::Character("\\".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x1c]);
}

#[test]
fn ctrl_close_bracket() {
    let r = enc(Key::Character("]".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x1d]);
}

// --- Alt prefix ---

#[test]
fn alt_a() {
    let r = enc_text(Key::Character("a".into()), Modifiers::ALT, no_mode(), "a");
    assert_eq!(r, vec![0x1b, b'a']);
}

#[test]
fn alt_ctrl_a() {
    let r = enc(
        Key::Character("a".into()),
        Modifiers::ALT | Modifiers::CONTROL,
        no_mode(),
    );
    assert_eq!(r, vec![0x1b, 0x01]);
}

#[test]
fn alt_space() {
    let r = enc(Key::Named(NamedKey::Space), Modifiers::ALT, no_mode());
    assert_eq!(r, vec![0x1b, b' ']);
}

#[test]
fn alt_ctrl_space() {
    let r = enc(
        Key::Named(NamedKey::Space),
        Modifiers::ALT | Modifiers::CONTROL,
        no_mode(),
    );
    assert_eq!(r, vec![0x1b, 0x00]);
}

// --- Modifier-encoded named keys ---

#[test]
fn ctrl_up() {
    let r = enc(Key::Named(NamedKey::ArrowUp), Modifiers::CONTROL, no_mode());
    assert_eq!(r, b"\x1b[1;5A");
}

#[test]
fn shift_right() {
    let r = enc(
        Key::Named(NamedKey::ArrowRight),
        Modifiers::SHIFT,
        no_mode(),
    );
    assert_eq!(r, b"\x1b[1;2C");
}

#[test]
fn ctrl_shift_left() {
    let r = enc(
        Key::Named(NamedKey::ArrowLeft),
        Modifiers::CONTROL | Modifiers::SHIFT,
        no_mode(),
    );
    assert_eq!(r, b"\x1b[1;6D");
}

/// Regression: BUG-08-033 — modifier-path coverage gap for ArrowDown.
/// Pinned by Plan-TPR Round 0 codex F1 (modifier_matrix.rs covered Up/Right/Left
/// pre-fix but not Down). The terminator byte flows through `CursorKey::Down.terminator()`.
#[test]
fn modified_arrow_down_ctrl_emits_csi_1_5_b() {
    let r = enc(
        Key::Named(NamedKey::ArrowDown),
        Modifiers::CONTROL,
        no_mode(),
    );
    assert_eq!(r, b"\x1b[1;5B");
}

/// Regression: BUG-08-033 — modifier-path coverage gap for Home.
/// Terminator byte flows through `CursorKey::Home.terminator()`.
#[test]
fn modified_home_shift_emits_csi_1_2_h() {
    let r = enc(Key::Named(NamedKey::Home), Modifiers::SHIFT, no_mode());
    assert_eq!(r, b"\x1b[1;2H");
}

/// Regression: BUG-08-033 — modifier-path coverage gap for End.
/// Terminator byte flows through `CursorKey::End.terminator()`.
#[test]
fn modified_end_alt_emits_csi_1_3_f() {
    let r = enc(Key::Named(NamedKey::End), Modifiers::ALT, no_mode());
    assert_eq!(r, b"\x1b[1;3F");
}

#[test]
fn ctrl_f5() {
    let r = enc(Key::Named(NamedKey::F5), Modifiers::CONTROL, no_mode());
    assert_eq!(r, b"\x1b[15;5~");
}

#[test]
fn shift_f1() {
    let r = enc(Key::Named(NamedKey::F1), Modifiers::SHIFT, no_mode());
    assert_eq!(r, b"\x1b[1;2P");
}

#[test]
fn ctrl_delete() {
    let r = enc(Key::Named(NamedKey::Delete), Modifiers::CONTROL, no_mode());
    assert_eq!(r, b"\x1b[3;5~");
}

#[test]
fn ctrl_page_up() {
    let r = enc(Key::Named(NamedKey::PageUp), Modifiers::CONTROL, no_mode());
    assert_eq!(r, b"\x1b[5;5~");
}

#[test]
fn shift_f5() {
    let r = enc(Key::Named(NamedKey::F5), Modifiers::SHIFT, no_mode());
    assert_eq!(r, b"\x1b[15;2~");
}

// --- Modifier parameter encoding ---

#[test]
fn modifier_param_shift() {
    assert_eq!(Modifiers::SHIFT.xterm_param(), 2);
}

#[test]
fn modifier_param_alt() {
    assert_eq!(Modifiers::ALT.xterm_param(), 3);
}

#[test]
fn modifier_param_ctrl() {
    assert_eq!(Modifiers::CONTROL.xterm_param(), 5);
}

#[test]
fn modifier_param_ctrl_shift() {
    assert_eq!((Modifiers::CONTROL | Modifiers::SHIFT).xterm_param(), 6);
}

#[test]
fn modifier_param_ctrl_alt() {
    assert_eq!((Modifiers::CONTROL | Modifiers::ALT).xterm_param(), 7);
}

#[test]
fn modifier_param_ctrl_alt_shift() {
    assert_eq!(
        (Modifiers::CONTROL | Modifiers::ALT | Modifiers::SHIFT).xterm_param(),
        8
    );
}

#[test]
fn modifier_param_none() {
    assert_eq!(Modifiers::empty().xterm_param(), 0);
}

// --- F1-F4 use SS3, F5+ use tilde ---

#[test]
fn f1_ss3() {
    assert_eq!(
        enc(Key::Named(NamedKey::F1), Modifiers::empty(), no_mode()),
        b"\x1bOP"
    );
}

#[test]
fn f2_ss3() {
    assert_eq!(
        enc(Key::Named(NamedKey::F2), Modifiers::empty(), no_mode()),
        b"\x1bOQ"
    );
}

#[test]
fn f3_ss3() {
    assert_eq!(
        enc(Key::Named(NamedKey::F3), Modifiers::empty(), no_mode()),
        b"\x1bOR"
    );
}

#[test]
fn f4_ss3() {
    assert_eq!(
        enc(Key::Named(NamedKey::F4), Modifiers::empty(), no_mode()),
        b"\x1bOS"
    );
}

#[test]
fn f6_tilde() {
    assert_eq!(
        enc(Key::Named(NamedKey::F6), Modifiers::empty(), no_mode()),
        b"\x1b[17~"
    );
}

#[test]
fn f11_tilde() {
    assert_eq!(
        enc(Key::Named(NamedKey::F11), Modifiers::empty(), no_mode()),
        b"\x1b[23~"
    );
}

// --- F1-F4 with modifiers use CSI, not SS3 ---

#[test]
fn f1_with_ctrl() {
    assert_eq!(
        enc(Key::Named(NamedKey::F1), Modifiers::CONTROL, no_mode()),
        b"\x1b[1;5P"
    );
}

#[test]
fn f4_with_shift() {
    assert_eq!(
        enc(Key::Named(NamedKey::F4), Modifiers::SHIFT, no_mode()),
        b"\x1b[1;2S"
    );
}

// --- Bare modifier keys produce nothing ---

#[test]
fn bare_shift_produces_nothing() {
    let r = enc(Key::Named(NamedKey::Shift), Modifiers::SHIFT, no_mode());
    assert!(r.is_empty());
}

#[test]
fn bare_control_produces_nothing() {
    let r = enc(Key::Named(NamedKey::Control), Modifiers::CONTROL, no_mode());
    assert!(r.is_empty());
}

#[test]
fn bare_alt_produces_nothing() {
    let r = enc(Key::Named(NamedKey::Alt), Modifiers::ALT, no_mode());
    assert!(r.is_empty());
}

#[test]
fn bare_super_produces_nothing() {
    let r = enc(Key::Named(NamedKey::Super), Modifiers::SUPER, no_mode());
    assert!(r.is_empty());
}

// --- Ctrl+/ and Ctrl+@ edge cases ---

#[test]
fn ctrl_slash() {
    // Ctrl+/ traditionally maps to 0x1f (US) via Ctrl+_ alias.
    // Our implementation handles this through the '_' → 0x1f mapping.
    // On most keyboards, Ctrl+/ sends Key::Character("_") or is handled
    // by the OS. If it arrives as "/", it won't produce a control code
    // (correct — "/" is not in the C0 mapping table).
    let r = enc(Key::Character("_".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x1f]);
}

#[test]
fn ctrl_at() {
    // Ctrl+@ = NUL (0x00), via the backtick/2 alias.
    let r = enc(Key::Character("`".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x00]);
}

// --- Ctrl+2, Ctrl+6, Ctrl+8 ---

#[test]
fn ctrl_2() {
    // Ctrl+2 = NUL (0x00), xterm-compatible alias.
    let r = enc(Key::Character("2".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x00]);
}

#[test]
fn ctrl_6() {
    // Ctrl+6 = RS (0x1e), xterm-compatible alias for Ctrl+^.
    let r = enc(Key::Character("6".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x1e]);
}

#[test]
fn ctrl_8() {
    // Ctrl+8 = DEL (0x7f), xterm-compatible.
    let r = enc(Key::Character("8".into()), Modifiers::CONTROL, no_mode());
    assert_eq!(r, vec![0x7f]);
}

// --- From<ModifiersState> for Modifiers ---

#[test]
fn from_modifiers_state_empty() {
    let m: Modifiers = ModifiersState::empty().into();
    assert_eq!(m, Modifiers::empty());
}

#[test]
fn from_modifiers_state_shift() {
    let m: Modifiers = ModifiersState::SHIFT.into();
    assert_eq!(m, Modifiers::SHIFT);
}

#[test]
fn from_modifiers_state_alt() {
    let m: Modifiers = ModifiersState::ALT.into();
    assert_eq!(m, Modifiers::ALT);
}

#[test]
fn from_modifiers_state_control() {
    let m: Modifiers = ModifiersState::CONTROL.into();
    assert_eq!(m, Modifiers::CONTROL);
}

#[test]
fn from_modifiers_state_super() {
    let m: Modifiers = ModifiersState::SUPER.into();
    assert_eq!(m, Modifiers::SUPER);
}

#[test]
fn from_modifiers_state_ctrl_shift() {
    let m: Modifiers = (ModifiersState::CONTROL | ModifiersState::SHIFT).into();
    assert_eq!(m, Modifiers::CONTROL | Modifiers::SHIFT);
}

#[test]
fn from_modifiers_state_all() {
    let winit_all = ModifiersState::SHIFT
        | ModifiersState::ALT
        | ModifiersState::CONTROL
        | ModifiersState::SUPER;
    let m: Modifiers = winit_all.into();
    assert_eq!(
        m,
        Modifiers::SHIFT | Modifiers::ALT | Modifiers::CONTROL | Modifiers::SUPER,
    );
}
