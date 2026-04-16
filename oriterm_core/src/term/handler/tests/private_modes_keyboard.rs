//! DECNKM (mode 66) and DECBKM (mode 67) core-layer flag tests.
//!
//! Section 09.4 scope: verifies DECSET/DECRST flag toggle and DECRQM for
//! both keyboard-related private modes. DECNKM shares `TermMode::APP_KEYPAD`
//! with ESC =/ESC > (DECKPAM/DECKPNM); reconciliation cells verify SSOT.

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect, QueueingEffectSink};
use crate::term::{Term, TermMode};

use super::super::test_helpers::{feed, term_with_effect_sink};

/// Create a Term with `QueueingEffectSink` for DECRQM/effect verification.
fn term() -> Term<QueueingEffectSink> {
    term_with_effect_sink()
}

/// Drain effects and check for a specific DECRQM response.
fn has_decrqm_response(t: &impl EffectSink, expected: &[u8]) -> bool {
    let mut effects = Vec::new();
    t.drain_into(&mut effects);
    effects
        .iter()
        .any(|e| matches!(e, Effect::Pty(PtyEffect::Write { bytes, .. }) if bytes == expected))
}

// ── DECNKM (mode 66) — flag toggle ─────────────────────────────────

#[test]
fn decnkm_decset_sets_app_keypad() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::APP_KEYPAD));
    feed(&mut t, b"\x1b[?66h");
    assert!(
        t.mode().contains(TermMode::APP_KEYPAD),
        "DECSET ?66 must set APP_KEYPAD"
    );
}

#[test]
fn decnkm_decrst_clears_app_keypad() {
    let mut t = term();
    feed(&mut t, b"\x1b[?66h");
    assert!(t.mode().contains(TermMode::APP_KEYPAD));
    feed(&mut t, b"\x1b[?66l");
    assert!(
        !t.mode().contains(TermMode::APP_KEYPAD),
        "DECRST ?66 must clear APP_KEYPAD"
    );
}

// ── DECNKM reconciliation with ESC =/ESC > (4-cell matrix) ─────────

/// Cell 1: ESC = (DECKPAM) → DECRQM ?66 reports set.
/// Proves DECRQM queries the shared state bit, not the entry-point.
#[test]
fn deckpam_then_decrqm_66_reports_set() {
    let mut t = term();
    feed(&mut t, b"\x1b="); // DECKPAM
    assert!(t.mode().contains(TermMode::APP_KEYPAD));

    // Drain any prior effects.
    has_decrqm_response(t.effect_sink(), b"");

    feed(&mut t, b"\x1b[?66$p");
    assert!(
        has_decrqm_response(t.effect_sink(), b"\x1b[?66;1$y"),
        "DECRQM ?66 must report 1 (set) after ESC ="
    );
}

/// Cell 2: DECSET ?66 → flag is set (same as ESC =).
#[test]
fn decset_66_sets_same_flag_as_deckpam() {
    let mut t = term();
    feed(&mut t, b"\x1b[?66h");
    assert!(
        t.mode().contains(TermMode::APP_KEYPAD),
        "DECSET ?66 must set the same APP_KEYPAD flag that ESC = sets"
    );

    // Drain DECSET effects.
    has_decrqm_response(t.effect_sink(), b"");

    feed(&mut t, b"\x1b[?66$p");
    assert!(
        has_decrqm_response(t.effect_sink(), b"\x1b[?66;1$y"),
        "DECRQM ?66 must report 1 after DECSET ?66"
    );
}

/// Cell 3: ESC = → DECRST ?66 clears APP_KEYPAD.
#[test]
fn deckpam_then_decrst_66_clears_app_keypad() {
    let mut t = term();
    feed(&mut t, b"\x1b="); // DECKPAM
    assert!(t.mode().contains(TermMode::APP_KEYPAD));

    feed(&mut t, b"\x1b[?66l"); // DECRST ?66
    assert!(
        !t.mode().contains(TermMode::APP_KEYPAD),
        "DECRST ?66 must clear APP_KEYPAD even when set by ESC ="
    );

    // Drain prior effects.
    has_decrqm_response(t.effect_sink(), b"");

    feed(&mut t, b"\x1b[?66$p");
    assert!(
        has_decrqm_response(t.effect_sink(), b"\x1b[?66;2$y"),
        "DECRQM ?66 must report 2 after DECRST ?66"
    );
}

/// Cell 4: DECSET ?66 → ESC > (DECKPNM) clears APP_KEYPAD.
#[test]
fn decset_66_then_deckpnm_clears_app_keypad() {
    let mut t = term();
    feed(&mut t, b"\x1b[?66h"); // DECSET ?66
    assert!(t.mode().contains(TermMode::APP_KEYPAD));

    feed(&mut t, b"\x1b>"); // DECKPNM
    assert!(
        !t.mode().contains(TermMode::APP_KEYPAD),
        "ESC > must clear APP_KEYPAD even when set by DECSET ?66"
    );

    // Drain prior effects.
    has_decrqm_response(t.effect_sink(), b"");

    feed(&mut t, b"\x1b[?66$p");
    assert!(
        has_decrqm_response(t.effect_sink(), b"\x1b[?66;2$y"),
        "DECRQM ?66 must report 2 after ESC >"
    );
}

// ── DECBKM (mode 67) — flag toggle ─────────────────────────────────

#[test]
fn decbkm_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::DECBKM));
    feed(&mut t, b"\x1b[?67h");
    assert!(
        t.mode().contains(TermMode::DECBKM),
        "DECSET ?67 must set DECBKM"
    );
}

#[test]
fn decbkm_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?67h");
    assert!(t.mode().contains(TermMode::DECBKM));
    feed(&mut t, b"\x1b[?67l");
    assert!(
        !t.mode().contains(TermMode::DECBKM),
        "DECRST ?67 must clear DECBKM"
    );
}

#[test]
fn decbkm_decrqm_default_reset() {
    let mut t = term();
    feed(&mut t, b"\x1b[?67$p");
    assert!(
        has_decrqm_response(t.effect_sink(), b"\x1b[?67;2$y"),
        "DECRQM ?67 must report 2 (reset) by default"
    );
}

#[test]
fn decbkm_decrqm_after_set() {
    let mut t = term();
    feed(&mut t, b"\x1b[?67h");
    // Drain DECSET effects.
    has_decrqm_response(t.effect_sink(), b"");

    feed(&mut t, b"\x1b[?67$p");
    assert!(
        has_decrqm_response(t.effect_sink(), b"\x1b[?67;1$y"),
        "DECRQM ?67 must report 1 (set) after DECSET"
    );
}

// ── DECNKM DECRQM default state ────────────────────────────────────

#[test]
fn decnkm_decrqm_default_reset() {
    let mut t = term();
    feed(&mut t, b"\x1b[?66$p");
    assert!(
        has_decrqm_response(t.effect_sink(), b"\x1b[?66;2$y"),
        "DECRQM ?66 must report 2 (reset) by default"
    );
}

// ── Orthogonality: DECNKM and DECBKM are independent ───────────────

#[test]
fn decnkm_and_decbkm_orthogonal() {
    let mut t = term();

    // Set both modes.
    feed(&mut t, b"\x1b[?66h\x1b[?67h");
    assert!(t.mode().contains(TermMode::APP_KEYPAD));
    assert!(t.mode().contains(TermMode::DECBKM));

    // Reset DECNKM — DECBKM must remain.
    feed(&mut t, b"\x1b[?66l");
    assert!(!t.mode().contains(TermMode::APP_KEYPAD));
    assert!(
        t.mode().contains(TermMode::DECBKM),
        "DECRST ?66 must not affect DECBKM"
    );

    // Re-set DECNKM, reset DECBKM — DECNKM must remain.
    feed(&mut t, b"\x1b[?66h\x1b[?67l");
    assert!(
        t.mode().contains(TermMode::APP_KEYPAD),
        "DECRST ?67 must not affect APP_KEYPAD"
    );
    assert!(!t.mode().contains(TermMode::DECBKM));
}
