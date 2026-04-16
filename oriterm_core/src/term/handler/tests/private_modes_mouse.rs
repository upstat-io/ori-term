use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::feed;

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- Mouse mutual exclusion ---

#[test]
fn mouse_mode_1003_clears_1000_and_1002() {
    let mut t = term();
    // Set mode 1000 (report clicks).
    feed(&mut t, b"\x1b[?1000h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));

    // Set mode 1003 (all motion) — should clear 1000.
    feed(&mut t, b"\x1b[?1003h");
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));
    assert!(!t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(!t.mode().contains(TermMode::MOUSE_DRAG));
}

#[test]
fn mouse_mode_1002_clears_1000_and_1003() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1003h");
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));

    feed(&mut t, b"\x1b[?1002h");
    assert!(t.mode().contains(TermMode::MOUSE_DRAG));
    assert!(!t.mode().contains(TermMode::MOUSE_MOTION));
    assert!(!t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
}

#[test]
fn mouse_encoding_1006_clears_1005_and_1015() {
    let mut t = term();
    // Set UTF-8 mouse.
    feed(&mut t, b"\x1b[?1005h");
    assert!(t.mode().contains(TermMode::MOUSE_UTF8));

    // Set SGR mouse — should clear UTF-8.
    feed(&mut t, b"\x1b[?1006h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR));
    assert!(!t.mode().contains(TermMode::MOUSE_UTF8));
    assert!(!t.mode().contains(TermMode::MOUSE_URXVT));
}

#[test]
fn mouse_encoding_1015_clears_1005_and_1006() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1006h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR));

    feed(&mut t, b"\x1b[?1015h");
    assert!(t.mode().contains(TermMode::MOUSE_URXVT));
    assert!(!t.mode().contains(TermMode::MOUSE_SGR));
    assert!(!t.mode().contains(TermMode::MOUSE_UTF8));
}

// --- DECRST mouse tracking does not reactivate previous mode ---

#[test]
fn decrst_mouse_tracking_does_not_reactivate_previous() {
    let mut t = term();
    // Set mode 1000 (clicks).
    feed(&mut t, b"\x1b[?1000h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));

    // Set mode 1003 (all motion) — clears 1000.
    feed(&mut t, b"\x1b[?1003h");
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));
    assert!(!t.mode().contains(TermMode::MOUSE_REPORT_CLICK));

    // Unset mode 1003.
    feed(&mut t, b"\x1b[?1003l");
    // 1000 should NOT auto-reactivate. No mouse mode should be active.
    assert!(!t.mode().contains(TermMode::MOUSE_MOTION));
    assert!(!t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(!t.mode().contains(TermMode::MOUSE_DRAG));
    assert!(!t.mode().intersects(TermMode::ANY_MOUSE));
}

// --- RIS clears all mouse tracking and encoding modes ---

#[test]
fn ris_clears_all_mouse_modes() {
    let mut t = term();
    // Set mouse tracking and encoding.
    feed(&mut t, b"\x1b[?1003h"); // all motion
    feed(&mut t, b"\x1b[?1006h"); // SGR encoding
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));
    assert!(t.mode().contains(TermMode::MOUSE_SGR));

    // Full reset.
    feed(&mut t, b"\x1bc");

    assert!(!t.mode().intersects(TermMode::ANY_MOUSE));
    assert!(!t.mode().intersects(TermMode::ANY_MOUSE_ENCODING));
}

// --- Encoding mode 1005 clears when setting 1015 (reverse direction) ---

#[test]
fn mouse_encoding_1005_clears_when_setting_1015() {
    let mut t = term();
    // Set UTF-8 mouse (1005).
    feed(&mut t, b"\x1b[?1005h");
    assert!(t.mode().contains(TermMode::MOUSE_UTF8));

    // Set URXVT mouse (1015) — should clear 1005.
    feed(&mut t, b"\x1b[?1015h");
    assert!(t.mode().contains(TermMode::MOUSE_URXVT));
    assert!(!t.mode().contains(TermMode::MOUSE_UTF8));
    assert!(!t.mode().contains(TermMode::MOUSE_SGR));
}

// --- DECRST encoding mode reverts to Normal format ---

#[test]
fn decrst_encoding_reverts_to_no_encoding() {
    let mut t = term();
    // Set SGR encoding (1006).
    feed(&mut t, b"\x1b[?1006h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR));

    // Disable SGR encoding.
    feed(&mut t, b"\x1b[?1006l");

    // No encoding flags should remain — events fall through to Normal format.
    assert!(!t.mode().intersects(TermMode::ANY_MOUSE_ENCODING));
}

#[test]
fn decrst_utf8_encoding_reverts_to_no_encoding() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1005h");
    assert!(t.mode().contains(TermMode::MOUSE_UTF8));

    feed(&mut t, b"\x1b[?1005l");
    assert!(!t.mode().intersects(TermMode::ANY_MOUSE_ENCODING));
}

#[test]
fn decrst_urxvt_encoding_reverts_to_no_encoding() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1015h");
    assert!(t.mode().contains(TermMode::MOUSE_URXVT));

    feed(&mut t, b"\x1b[?1015l");
    assert!(!t.mode().intersects(TermMode::ANY_MOUSE_ENCODING));
}

// --- DECRST targeted clear: only clears the specified mode ---

#[test]
fn decrst_1000_preserves_active_1003() {
    let mut t = term();
    // Set mode 1003 (all motion).
    feed(&mut t, b"\x1b[?1003h");
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));

    // DECRST 1000 (clicks) — mode 1003 should remain active.
    feed(&mut t, b"\x1b[?1000l");
    assert!(
        t.mode().contains(TermMode::MOUSE_MOTION),
        "DECRST 1000 should not clear 1003"
    );
    assert!(t.mode().intersects(TermMode::ANY_MOUSE));
}

#[test]
fn decrst_1002_preserves_active_1003() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1003h");
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));

    // DECRST 1002 (drag) — mode 1003 should remain active.
    feed(&mut t, b"\x1b[?1002l");
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));
}

#[test]
fn decrst_9_preserves_active_1000() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1000h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));

    // DECRST 9 (X10) — mode 1000 should remain active.
    feed(&mut t, b"\x1b[?9l");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
}
