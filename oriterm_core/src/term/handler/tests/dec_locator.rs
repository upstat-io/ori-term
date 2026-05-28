//! End-to-end DEC Locator pin tests — byte stream → vte parser →
//! Handler trait → DecLocatorState mutation + DECLRP apex emission.
//!
//! Catalog rows: MOUSE-DECEFR, MOUSE-DECELR, MOUSE-DECSLE, MOUSE-DECRQLP,
//! MOUSE-DECLRP-REPLY. State-rung + apex-emission pin tests.

use crate::term::Term;
use crate::term::dec_locator::{LocatorEventMask, LocatorRect, LocatorReportingMode};
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_recorder};

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

// ── DECLRP apex-emission pin tests (§16.1.C item 1) ─────────────────

/// DECRQLP when locator is disabled (default) emits DECLRP with Pe=0
/// ("locator unavailable") + placeholder coords via Effect::Pty with
/// kind = PtyWriteKind::MouseEvent.
#[test]
fn decrqlp_disabled_emits_declrp_pe0_via_effect_pty_mouse_event() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1'|"); // DECRQLP Ps=1
    let events = listener.events();
    let expected = "PtyWrite(\x1b[0;0;1;1;1&w)".to_string();
    assert!(
        events.iter().any(|e| *e == expected),
        "expected DECLRP Pe=0 reply, got events: {events:?}"
    );
}

/// DECRQLP when locator is Continuous-enabled emits DECLRP with Pe=1
/// ("request response") + placeholder coords. Locator stays Continuous
/// (no auto-clear).
#[test]
fn decrqlp_continuous_emits_declrp_pe1_no_auto_clear() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // DECELR Ps=1 (continuous)
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    let events = listener.events();
    let expected = "PtyWrite(\x1b[1;0;1;1;1&w)".to_string();
    assert!(
        events.iter().any(|e| *e == expected),
        "expected DECLRP Pe=1 reply, got events: {events:?}"
    );
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous),
        "Continuous reporting persists across DECRQLP per xterm spec"
    );
}

/// DECRQLP when locator is OneShot-enabled emits DECLRP with Pe=1 AND
/// auto-clears reporting → None per xterm spec.
#[test]
fn decrqlp_oneshot_emits_declrp_pe1_and_auto_clears() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[2;0'z"); // DECELR Ps=2 (one-shot)
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    let events = listener.events();
    let expected = "PtyWrite(\x1b[1;0;1;1;1&w)".to_string();
    assert!(
        events.iter().any(|e| *e == expected),
        "expected DECLRP Pe=1 reply, got events: {events:?}"
    );
    assert_eq!(
        t.dec_locator().reporting(),
        None,
        "OneShot reporting must auto-clear to None after DECRQLP reply"
    );
}

/// Second DECRQLP after OneShot auto-clear emits Pe=0 (locator now
/// disabled by the auto-clear) — pins the OneShot semantic end-to-end.
#[test]
fn second_decrqlp_after_oneshot_clear_emits_pe0() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[2;0'z"); // OneShot
    feed(&mut t, b"\x1b[1'|"); // DECRQLP (auto-clears)
    feed(&mut t, b"\x1b[1'|"); // DECRQLP again
    let events = listener.events();
    let pe1 = "PtyWrite(\x1b[1;0;1;1;1&w)".to_string();
    let pe0 = "PtyWrite(\x1b[0;0;1;1;1&w)".to_string();
    let pe1_count = events.iter().filter(|e| **e == pe1).count();
    let pe0_count = events.iter().filter(|e| **e == pe0).count();
    assert_eq!(pe1_count, 1, "exactly one Pe=1 reply (the OneShot)");
    assert_eq!(pe0_count, 1, "second DECRQLP emits Pe=0 (locator cleared)");
}

/// DECRQLP with Ps other than 0/1 is silently dropped per xterm spec.
#[test]
fn decrqlp_invalid_ps_emits_nothing() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // enable continuous
    feed(&mut t, b"\x1b[99'|"); // invalid Ps=99
    let events = listener.events();
    let declrp_emitted = events.iter().any(|e| e.contains("&w"));
    assert!(
        !declrp_emitted,
        "DECRQLP with Ps != 0|1 must emit no reply, got: {events:?}"
    );
}

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
