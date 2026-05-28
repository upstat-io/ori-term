//! End-to-end DEC Locator pin tests — byte stream → vte parser →
//! Handler trait → DecLocatorState mutation.
//!
//! Catalog rows: MOUSE-DECEFR, MOUSE-DECELR, MOUSE-DECSLE, MOUSE-DECRQLP.
//! State-rung pin tests; apex-emission (MOUSE-DECLRP-REPLY +
//! MOUSE-XTHIMOUSE-INIT) tests await mux-routing PtyWriteKind work
//! tracked in bug-tracker §11.

use crate::term::Term;
use crate::term::dec_locator::{LocatorEventMask, LocatorRect, LocatorReportingMode};
use crate::theme::Theme;

use super::super::test_helpers::feed;

fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// ── DECELR (CSI Ps;Pu ' z) ──────────────────────────────────────────

#[test]
fn decelr_ps1_continuous_cells_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;0'z");
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
    assert!(!t.dec_locator().pixel_unit());
}

#[test]
fn decelr_ps2_oneshot_pixels_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[2;1'z");
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::OneShot)
    );
    assert!(t.dec_locator().pixel_unit());
}

#[test]
fn decelr_ps0_disables_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;0'z"); // enable continuous
    assert!(t.dec_locator().reporting().is_some());
    feed(&mut t, b"\x1b[0;0'z"); // disable
    assert_eq!(t.dec_locator().reporting(), None);
}

// ── DECSLE (CSI Pm ' {) ─────────────────────────────────────────────

#[test]
fn decsle_pm1_sets_button_down_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1'{");
    assert_eq!(t.dec_locator().event_mask(), LocatorEventMask::BUTTON_DOWN);
}

#[test]
fn decsle_pm_list_combines_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;3'{");
    assert_eq!(
        t.dec_locator().event_mask(),
        LocatorEventMask::BUTTON_DOWN | LocatorEventMask::BUTTON_UP
    );
}

// ── DECEFR (CSI Pt;Pl;Pb;Pr ' w) ────────────────────────────────────

#[test]
fn decefr_stores_rectangle_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;10;15;20'w");
    assert_eq!(
        t.dec_locator().filter_rect(),
        Some(LocatorRect {
            top: 5,
            left: 10,
            bottom: 15,
            right: 20,
        })
    );
}

#[test]
fn decefr_all_zeros_clears_rectangle_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;10;15;20'w");
    assert!(t.dec_locator().filter_rect().is_some());
    feed(&mut t, b"\x1b[0;0;0;0'w");
    assert_eq!(t.dec_locator().filter_rect(), None);
}

// ── DECRQLP (CSI Ps ' |) ────────────────────────────────────────────

#[test]
fn decrqlp_in_oneshot_auto_clears_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[2;0'z"); // one-shot
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::OneShot)
    );
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    assert_eq!(
        t.dec_locator().reporting(),
        None,
        "OneShot must auto-clear after DECRQLP per xterm spec"
    );
}

#[test]
fn decrqlp_in_continuous_does_not_clear_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;0'z"); // continuous
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
}

// ── Cross-state independence (independent of DECSET 1001) ───────────

#[test]
fn dec_locator_independent_of_mode_1001() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1001h"); // enable highlight tracking (mode 1001)
    // Mode 1001 must NOT enable DEC Locator reporting.
    assert_eq!(t.dec_locator().reporting(), None);

    feed(&mut t, b"\x1b[1;0'z"); // enable DEC Locator (continuous, cells)
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
    // Mode 1001 still set; DEC Locator independently active.
    assert!(t.mode().contains(crate::term::TermMode::MOUSE_HIGHLIGHT));

    feed(&mut t, b"\x1b[?1001l"); // disable highlight tracking
    // DEC Locator state unaffected.
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
}
