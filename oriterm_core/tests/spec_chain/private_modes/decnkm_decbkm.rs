//! Spec-chain tests for DECNKM (mode 66) and DECBKM (mode 67).
//!
//! Section 09.4 scope: verifies DECSET/DECRST flag toggle and DECRQM
//! for both keyboard-related DEC private modes through the full
//! VTE → Term → Effect pipeline.
//!
//! - Mode 66 (DECNKM) shares `TermMode::APP_KEYPAD` with ESC =/ESC >.
//! - Mode 67 (DECBKM) has its own `TermMode::DECBKM` flag.

use oriterm_core::TermMode;
use oriterm_test_support::spec_chain::{SpecHarness, pty_writes};

// ── DECNKM (mode 66) — flag toggle ─────────────────────────────────

#[test]
fn decnkm_decset_toggles_flag() {
    let mut h = SpecHarness::new();
    assert!(
        !h.term().mode().contains(TermMode::APP_KEYPAD),
        "default must NOT include APP_KEYPAD"
    );

    h.feed(b"\x1b[?66h");
    assert!(
        h.term().mode().contains(TermMode::APP_KEYPAD),
        "DECSET ?66 must set APP_KEYPAD"
    );
}

#[test]
fn decnkm_decrst_clears_flag() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?66h");
    assert!(h.term().mode().contains(TermMode::APP_KEYPAD));

    h.feed(b"\x1b[?66l");
    assert!(
        !h.term().mode().contains(TermMode::APP_KEYPAD),
        "DECRST ?66 must clear APP_KEYPAD"
    );
}

#[test]
fn decnkm_round_trip_toggle() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?66h");
    assert!(h.term().mode().contains(TermMode::APP_KEYPAD));
    h.feed(b"\x1b[?66l");
    assert!(!h.term().mode().contains(TermMode::APP_KEYPAD));
    h.feed(b"\x1b[?66h");
    assert!(h.term().mode().contains(TermMode::APP_KEYPAD));
}

#[test]
fn decnkm_does_not_touch_unrelated_flags() {
    let mut h = SpecHarness::new();
    let before = h.term().mode();
    h.feed(b"\x1b[?66h");
    let after = h.term().mode();
    // Only APP_KEYPAD should have changed.
    let diff = before.symmetric_difference(after);
    assert_eq!(
        diff,
        TermMode::APP_KEYPAD,
        "DECSET ?66 must only change APP_KEYPAD"
    );
}

// ── DECNKM DECRQM ──────────────────────────────────────────────────

#[test]
fn decnkm_decrqm_default_reset() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?66$p");
    let expected = b"\x1b[?66;2$y";
    let found = pty_writes(&h).any(|(b, _)| b == expected);
    assert!(found, "DECRQM ?66 must report 2 (reset) by default");
}

#[test]
fn decnkm_decrqm_after_set() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?66h");
    h.feed(b"\x1b[?66$p");
    let expected = b"\x1b[?66;1$y";
    let found = pty_writes(&h).any(|(b, _)| b == expected);
    assert!(found, "DECRQM ?66 must report 1 (set) after DECSET");
}

// ── DECNKM reconciliation with ESC =/ESC > ──────────────────────────

/// ESC = (DECKPAM) sets the shared APP_KEYPAD flag; DECRQM ?66 sees it.
#[test]
fn deckpam_propagates_to_decrqm_66() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b="); // DECKPAM
    assert!(h.term().mode().contains(TermMode::APP_KEYPAD));

    h.feed(b"\x1b[?66$p");
    let expected = b"\x1b[?66;1$y";
    let found = pty_writes(&h).any(|(b, _)| b == expected);
    assert!(
        found,
        "DECRQM ?66 must report 1 when APP_KEYPAD was set by ESC ="
    );
}

/// DECRST ?66 clears the flag even when it was set by ESC =.
#[test]
fn decrst_66_clears_deckpam_flag() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b="); // DECKPAM
    h.feed(b"\x1b[?66l"); // DECRST
    assert!(
        !h.term().mode().contains(TermMode::APP_KEYPAD),
        "DECRST ?66 must clear APP_KEYPAD set by ESC ="
    );
}

/// ESC > (DECKPNM) clears the flag even when it was set by DECSET ?66.
#[test]
fn deckpnm_clears_decset_66_flag() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?66h"); // DECSET
    h.feed(b"\x1b>"); // DECKPNM
    assert!(
        !h.term().mode().contains(TermMode::APP_KEYPAD),
        "ESC > must clear APP_KEYPAD set by DECSET ?66"
    );
}

// ── DECBKM (mode 67) — flag toggle ─────────────────────────────────

#[test]
fn decbkm_decset_toggles_flag() {
    let mut h = SpecHarness::new();
    assert!(
        !h.term().mode().contains(TermMode::DECBKM),
        "default must NOT include DECBKM"
    );

    h.feed(b"\x1b[?67h");
    assert!(
        h.term().mode().contains(TermMode::DECBKM),
        "DECSET ?67 must set DECBKM"
    );
}

#[test]
fn decbkm_decrst_clears_flag() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?67h");
    assert!(h.term().mode().contains(TermMode::DECBKM));

    h.feed(b"\x1b[?67l");
    assert!(
        !h.term().mode().contains(TermMode::DECBKM),
        "DECRST ?67 must clear DECBKM"
    );
}

#[test]
fn decbkm_round_trip_toggle() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?67h");
    assert!(h.term().mode().contains(TermMode::DECBKM));
    h.feed(b"\x1b[?67l");
    assert!(!h.term().mode().contains(TermMode::DECBKM));
    h.feed(b"\x1b[?67h");
    assert!(h.term().mode().contains(TermMode::DECBKM));
}

#[test]
fn decbkm_does_not_touch_unrelated_flags() {
    let mut h = SpecHarness::new();
    let before = h.term().mode();
    h.feed(b"\x1b[?67h");
    let after = h.term().mode();
    let diff = before.symmetric_difference(after);
    assert_eq!(diff, TermMode::DECBKM, "DECSET ?67 must only change DECBKM");
}

// ── DECBKM DECRQM ──────────────────────────────────────────────────

#[test]
fn decbkm_decrqm_default_reset() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?67$p");
    let expected = b"\x1b[?67;2$y";
    let found = pty_writes(&h).any(|(b, _)| b == expected);
    assert!(found, "DECRQM ?67 must report 2 (reset) by default");
}

#[test]
fn decbkm_decrqm_after_set() {
    let mut h = SpecHarness::new();
    h.feed(b"\x1b[?67h");
    h.feed(b"\x1b[?67$p");
    let expected = b"\x1b[?67;1$y";
    let found = pty_writes(&h).any(|(b, _)| b == expected);
    assert!(found, "DECRQM ?67 must report 1 (set) after DECSET");
}

// ── Orthogonality ───────────────────────────────────────────────────

/// DECNKM and DECBKM operate on different flags; neither interferes.
#[test]
fn decnkm_and_decbkm_orthogonal() {
    let mut h = SpecHarness::new();

    h.feed(b"\x1b[?66h\x1b[?67h");
    assert!(h.term().mode().contains(TermMode::APP_KEYPAD));
    assert!(h.term().mode().contains(TermMode::DECBKM));

    // Reset DECNKM only.
    h.feed(b"\x1b[?66l");
    assert!(!h.term().mode().contains(TermMode::APP_KEYPAD));
    assert!(
        h.term().mode().contains(TermMode::DECBKM),
        "DECRST ?66 must not affect DECBKM"
    );
}
