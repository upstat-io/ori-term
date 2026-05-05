//! Backspace encoding under DECBKM (mode 67).
//!
//! Section 09.4b scope: verifies backspace polarity inversion when
//! DECBKM is set. Default (mode reset): plain=DEL, Ctrl=BS. DECBKM set:
//! plain=BS, Ctrl=DEL. Alt prefix applied after byte choice in both modes.

use winit::keyboard::{Key, NamedKey};

use super::{Modifiers, decbkm_mode, enc, no_mode};

// ── Default (DECBKM reset) ──────────────────────────────────────────

/// Property: plain Backspace → DEL (0x7f) when DECBKM is reset.
#[test]
fn backspace_sends_del_when_decbkm_reset() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::empty(),
        no_mode(),
    );
    assert_eq!(r, vec![0x7f]);
}

/// Regression pin: Ctrl+Backspace → BS (0x08) when DECBKM is reset.
#[test]
fn ctrl_backspace_sends_bs_when_decbkm_reset() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::CONTROL,
        no_mode(),
    );
    assert_eq!(r, vec![0x08]);
}

// ── DECBKM set ──────────────────────────────────────────────────────

/// Property: plain Backspace → BS (0x08) when DECBKM is set.
#[test]
fn backspace_sends_bs_when_decbkm_set() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::empty(),
        decbkm_mode(),
    );
    assert_eq!(r, vec![0x08]);
}

/// Property: Ctrl+Backspace → DEL (0x7f) when DECBKM is set (inverted).
#[test]
fn ctrl_backspace_sends_del_when_decbkm_set() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::CONTROL,
        decbkm_mode(),
    );
    assert_eq!(r, vec![0x7f]);
}

// ── Alt prefix preserved under both modes ───────────────────────────

/// Regression pin: Alt+Backspace ESC prefix preserved when DECBKM is reset.
#[test]
fn alt_backspace_esc_prefix_decbkm_reset() {
    let r = enc(Key::Named(NamedKey::Backspace), Modifiers::ALT, no_mode());
    assert_eq!(r, vec![0x1b, 0x7f]);
}

/// Regression pin: Alt+Backspace ESC prefix preserved when DECBKM is set.
#[test]
fn alt_backspace_esc_prefix_decbkm_set() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::ALT,
        decbkm_mode(),
    );
    assert_eq!(r, vec![0x1b, 0x08]);
}

// ── Alt+Ctrl under both modes ───────────────────────────────────────

/// Alt+Ctrl+Backspace under DECBKM reset: ESC + BS (0x08).
#[test]
fn alt_ctrl_backspace_decbkm_reset() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::ALT | Modifiers::CONTROL,
        no_mode(),
    );
    assert_eq!(r, vec![0x1b, 0x08]);
}

/// Alt+Ctrl+Backspace under DECBKM set: ESC + DEL (0x7f).
#[test]
fn alt_ctrl_backspace_decbkm_set() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::ALT | Modifiers::CONTROL,
        decbkm_mode(),
    );
    assert_eq!(r, vec![0x1b, 0x7f]);
}

// ── Shift does not participate in DECBKM polarity ───────────────────

/// Regression guard: Shift+Backspace unchanged by DECBKM (still DEL/BS per Ctrl).
#[test]
fn shift_backspace_unchanged_by_decbkm_reset() {
    let r = enc(Key::Named(NamedKey::Backspace), Modifiers::SHIFT, no_mode());
    // Shift alone does not change backspace — still DEL.
    assert_eq!(r, vec![0x7f]);
}

#[test]
fn shift_backspace_unchanged_by_decbkm_set() {
    let r = enc(
        Key::Named(NamedKey::Backspace),
        Modifiers::SHIFT,
        decbkm_mode(),
    );
    // Shift alone → DECBKM inverts plain → BS.
    assert_eq!(r, vec![0x08]);
}

// ── Matrix completeness assertion ───────────────────────────────────

/// Self-verifying matrix: all modifier × DECBKM combinations covered.
#[test]
fn backspace_matrix_completeness() {
    let modes = [no_mode(), decbkm_mode()];
    let mod_combos = [
        Modifiers::empty(),
        Modifiers::ALT,
        Modifiers::CONTROL,
        Modifiers::ALT | Modifiers::CONTROL,
        Modifiers::SHIFT,
    ];

    let mut count = 0;
    for mode in &modes {
        for mods in &mod_combos {
            let result = enc(Key::Named(NamedKey::Backspace), *mods, *mode);
            assert!(
                !result.is_empty(),
                "backspace must produce output for mods={mods:?}, decbkm={}",
                mode.contains(oriterm_core::TermMode::DECBKM)
            );
            count += 1;
        }
    }
    assert_eq!(
        count, 10,
        "matrix must cover 2 modes × 5 mod combos = 10 cells"
    );
}
