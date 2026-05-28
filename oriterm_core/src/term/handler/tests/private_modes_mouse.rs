//! Catalog rows: DEC-X10-MOUSE, DEC-MOUSE-CLICKS, DEC-MOUSE-DRAG, DEC-MOUSE-MOTION,
//! DEC-UTF8-MOUSE, DEC-SGR-MOUSE, DEC-URXVT-MOUSE, DEC-VT200-HIGHLIGHT-MOUSE,
//! DEC-SGR-PIXEL-MOUSE

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

// --- Mode 1001 (VT200 highlight mouse tracking) ---

/// DECSET ?1001h sets MOUSE_HIGHLIGHT; DECRST ?1001l clears it.
///
/// Mode 1001 is highlight tracking (SET_VT200_HIGHLIGHT_MOUSE). NOT the DEC
/// Locator subsystem — DEC Locator is independently activated by DECELR
/// (`CSI Ps;Pu ' z`), no DECSET dependency. NOT a base tracking mode;
/// supplements an active 1000/1002/1003 base mode.
#[test]
fn mouse_mode_1001_highlight_tracking_toggle() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::MOUSE_HIGHLIGHT));

    feed(&mut t, b"\x1b[?1001h");
    assert!(t.mode().contains(TermMode::MOUSE_HIGHLIGHT));

    feed(&mut t, b"\x1b[?1001l");
    assert!(!t.mode().contains(TermMode::MOUSE_HIGHLIGHT));
}

/// Mode 1001 does NOT participate in ANY_MOUSE mutual exclusion — it is a
/// supplement to base tracking, not a base tracking mode itself.
#[test]
fn mouse_mode_1001_does_not_clear_base_tracking() {
    let mut t = term();
    // Set base tracking mode 1000.
    feed(&mut t, b"\x1b[?1000h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));

    // Set highlight tracking 1001 — should NOT clear base tracking.
    feed(&mut t, b"\x1b[?1001h");
    assert!(t.mode().contains(TermMode::MOUSE_HIGHLIGHT));
    assert!(
        t.mode().contains(TermMode::MOUSE_REPORT_CLICK),
        "mode 1001 must not clear ANY_MOUSE base tracking — it supplements it"
    );
}

/// Mode 1001 is NOT a member of ANY_MOUSE_ENCODING — it is independent of
/// the encoding-format mutual-exclusion group (1005/1006/1015/1016).
#[test]
fn mouse_mode_1001_not_in_any_mouse_encoding_union() {
    assert!(
        !TermMode::ANY_MOUSE_ENCODING.contains(TermMode::MOUSE_HIGHLIGHT),
        "MOUSE_HIGHLIGHT is highlight tracking, not an encoding format"
    );
    assert!(
        !TermMode::ANY_MOUSE.contains(TermMode::MOUSE_HIGHLIGHT),
        "MOUSE_HIGHLIGHT supplements base tracking, not a base tracking mode"
    );
}

// --- Mode 1016 (SGR-Pixel mouse encoding) ---

/// DECSET ?1016h sets MOUSE_SGR_PIXEL; DECRST ?1016l clears it.
#[test]
fn mouse_encoding_1016_sgr_pixel_toggle() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::MOUSE_SGR_PIXEL));

    feed(&mut t, b"\x1b[?1016h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR_PIXEL));

    feed(&mut t, b"\x1b[?1016l");
    assert!(!t.mode().contains(TermMode::MOUSE_SGR_PIXEL));
}

/// Mode 1016 participates in ANY_MOUSE_ENCODING mutual exclusion — setting
/// it clears 1005/1006/1015 per the encoding-format precedence rule.
#[test]
fn mouse_encoding_1016_clears_1005_and_1006_and_1015() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1005h");
    assert!(t.mode().contains(TermMode::MOUSE_UTF8));

    feed(&mut t, b"\x1b[?1006h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR));
    assert!(!t.mode().contains(TermMode::MOUSE_UTF8));

    feed(&mut t, b"\x1b[?1015h");
    assert!(t.mode().contains(TermMode::MOUSE_URXVT));
    assert!(!t.mode().contains(TermMode::MOUSE_SGR));

    // Now SGR-Pixel — clears URXVT.
    feed(&mut t, b"\x1b[?1016h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR_PIXEL));
    assert!(!t.mode().contains(TermMode::MOUSE_URXVT));
    assert!(!t.mode().contains(TermMode::MOUSE_SGR));
    assert!(!t.mode().contains(TermMode::MOUSE_UTF8));
}

/// Setting 1006 after 1016 clears 1016 — the mutual exclusion is symmetric.
#[test]
fn mouse_encoding_1006_clears_1016() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1016h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR_PIXEL));

    feed(&mut t, b"\x1b[?1006h");
    assert!(t.mode().contains(TermMode::MOUSE_SGR));
    assert!(!t.mode().contains(TermMode::MOUSE_SGR_PIXEL));
}

/// Mode 1016 is NOT a member of ANY_MOUSE — it is an encoding format, not a
/// base tracking mode. Setting 1016 alone does NOT enable mouse reporting.
#[test]
fn mouse_encoding_1016_not_in_any_mouse_union() {
    assert!(
        !TermMode::ANY_MOUSE.contains(TermMode::MOUSE_SGR_PIXEL),
        "MOUSE_SGR_PIXEL is an encoding format, not a base tracking mode"
    );
    assert!(
        TermMode::ANY_MOUSE_ENCODING.contains(TermMode::MOUSE_SGR_PIXEL),
        "MOUSE_SGR_PIXEL must be in ANY_MOUSE_ENCODING for mutual exclusion"
    );
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

// --- X10 mouse (mode 9) ---

#[test]
fn x10_mouse_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::MOUSE_X10));

    feed(&mut t, b"\x1b[?9h");
    assert!(t.mode().contains(TermMode::MOUSE_X10));
}

#[test]
fn x10_mouse_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?9h");
    assert!(t.mode().contains(TermMode::MOUSE_X10));

    feed(&mut t, b"\x1b[?9l");
    assert!(!t.mode().contains(TermMode::MOUSE_X10));
}

#[test]
fn x10_mouse_clears_other_tracking() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1000h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));

    // Setting X10 (mode 9) should clear mode 1000.
    feed(&mut t, b"\x1b[?9h");
    assert!(t.mode().contains(TermMode::MOUSE_X10));
    assert!(!t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
}

#[test]
fn mode_1000_clears_x10() {
    let mut t = term();
    feed(&mut t, b"\x1b[?9h");
    assert!(t.mode().contains(TermMode::MOUSE_X10));

    // Setting mode 1000 should clear X10.
    feed(&mut t, b"\x1b[?1000h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(!t.mode().contains(TermMode::MOUSE_X10));
}

#[test]
fn x10_then_1002_only_1002_remains() {
    let mut t = term();
    feed(&mut t, b"\x1b[?9h");
    assert!(t.mode().contains(TermMode::MOUSE_X10));

    feed(&mut t, b"\x1b[?1002h");
    assert!(!t.mode().contains(TermMode::MOUSE_X10));
    assert!(t.mode().contains(TermMode::MOUSE_DRAG));
}

#[test]
fn mode_1003_then_9_only_9_remains() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1003h");
    assert!(t.mode().contains(TermMode::MOUSE_MOTION));

    feed(&mut t, b"\x1b[?9h");
    assert!(!t.mode().contains(TermMode::MOUSE_MOTION));
    assert!(t.mode().contains(TermMode::MOUSE_X10));
}

// --- Tracking × encoding orthogonality ---

#[test]
fn tracking_and_encoding_coexist() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1000h");
    feed(&mut t, b"\x1b[?1006h");

    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(t.mode().contains(TermMode::MOUSE_SGR));
}

#[test]
fn tracking_replacement_preserves_encoding() {
    let mut t = term();
    // Set tracking 1000 + encoding SGR.
    feed(&mut t, b"\x1b[?1000h");
    feed(&mut t, b"\x1b[?1006h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(t.mode().contains(TermMode::MOUSE_SGR));

    // Replace tracking with 1002 — encoding (SGR) must be preserved.
    feed(&mut t, b"\x1b[?1002h");
    assert!(!t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
    assert!(t.mode().contains(TermMode::MOUSE_DRAG));
    assert!(
        t.mode().contains(TermMode::MOUSE_SGR),
        "Switching mouse tracking mode must not clear encoding mode"
    );
}

#[test]
fn encoding_replacement_preserves_tracking() {
    let mut t = term();
    // Set tracking 1002 + encoding SGR.
    feed(&mut t, b"\x1b[?1002h");
    feed(&mut t, b"\x1b[?1006h");
    assert!(t.mode().contains(TermMode::MOUSE_DRAG));
    assert!(t.mode().contains(TermMode::MOUSE_SGR));

    // Replace encoding with UTF-8 — tracking (1002) must be preserved.
    feed(&mut t, b"\x1b[?1005h");
    assert!(!t.mode().contains(TermMode::MOUSE_SGR));
    assert!(t.mode().contains(TermMode::MOUSE_UTF8));
    assert!(
        t.mode().contains(TermMode::MOUSE_DRAG),
        "Switching mouse encoding mode must not clear tracking mode"
    );
}
