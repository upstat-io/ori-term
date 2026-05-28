//! DEC Locator subsystem state — unit tests.
//!
//! State-rung pin tests per spec-conformance §16.1.B item 7:
//! DECELR Ps mapping (0/1/2) round-trip + one-shot auto-clear after
//! DECRQLP-acknowledged + DECSLE mask round-trip + DECEFR rect
//! round-trip. End-to-end emission tests (§16.1.C) wait on the
//! mux-routing PtyWriteKind preservation work tracked in bug-tracker §11.

use super::{DecLocatorState, LocatorEventMask, LocatorRect, LocatorReportingMode};

// ── DECELR (Ps;Pu ' z) ──────────────────────────────────────────────

#[test]
fn decelr_ps0_disables_reporting() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 0); // first turn on
    assert_eq!(s.reporting, Some(LocatorReportingMode::Continuous));
    s.apply_decelr(0, 0); // then disable
    assert_eq!(s.reporting, None);
}

#[test]
fn decelr_ps1_enables_continuous() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 0);
    assert_eq!(s.reporting, Some(LocatorReportingMode::Continuous));
}

#[test]
fn decelr_ps2_enables_oneshot() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(2, 0);
    assert_eq!(s.reporting, Some(LocatorReportingMode::OneShot));
}

#[test]
fn decelr_pu0_means_character_cells() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 0);
    assert!(!s.pixel_unit, "Pu=0 means character cells (default)");
}

#[test]
fn decelr_pu1_means_pixels() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 1);
    assert!(s.pixel_unit, "Pu=1 means pixel coordinates");
}

#[test]
fn decelr_pu2_means_character_cells() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 2);
    assert!(!s.pixel_unit, "Pu=2 also means character cells");
}

#[test]
fn decelr_unknown_ps_defaults_to_disabled() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 0); // first turn on
    s.apply_decelr(99, 0); // unknown Ps
    assert_eq!(s.reporting, None, "unknown Ps disables per spec safety");
}

// ── DECRQLP-acknowledged auto-clear semantics ───────────────────────

#[test]
fn oneshot_auto_clears_after_decrqlp_acknowledged() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(2, 0); // one-shot mode
    assert_eq!(s.reporting, Some(LocatorReportingMode::OneShot));
    s.on_decrqlp_acknowledged();
    assert_eq!(
        s.reporting, None,
        "OneShot must auto-clear to None after DECRQLP reply"
    );
}

#[test]
fn continuous_does_not_auto_clear_after_decrqlp_acknowledged() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 0); // continuous mode
    s.on_decrqlp_acknowledged();
    assert_eq!(
        s.reporting,
        Some(LocatorReportingMode::Continuous),
        "Continuous persists across DECRQLP replies"
    );
}

#[test]
fn disabled_state_unaffected_by_decrqlp_acknowledged() {
    let mut s = DecLocatorState::new();
    assert_eq!(s.reporting, None);
    s.on_decrqlp_acknowledged();
    assert_eq!(s.reporting, None);
}

// ── DECSLE (Pm ' {) ─────────────────────────────────────────────────

#[test]
fn decsle_empty_defaults_to_explicit_only() {
    let mut s = DecLocatorState::new();
    s.apply_decsle(&[]);
    assert_eq!(s.event_mask, LocatorEventMask::EXPLICIT_ONLY);
}

#[test]
fn decsle_pm0_sets_explicit_only() {
    let mut s = DecLocatorState::new();
    s.apply_decsle(&[0]);
    assert_eq!(s.event_mask, LocatorEventMask::EXPLICIT_ONLY);
}

#[test]
fn decsle_pm1_sets_button_down() {
    let mut s = DecLocatorState::new();
    s.apply_decsle(&[1]);
    assert_eq!(s.event_mask, LocatorEventMask::BUTTON_DOWN);
}

#[test]
fn decsle_pm_list_combines_bits() {
    let mut s = DecLocatorState::new();
    s.apply_decsle(&[1, 3]);
    assert_eq!(
        s.event_mask,
        LocatorEventMask::BUTTON_DOWN | LocatorEventMask::BUTTON_UP
    );
}

#[test]
fn decsle_replaces_previous_mask() {
    let mut s = DecLocatorState::new();
    s.apply_decsle(&[1, 3]);
    s.apply_decsle(&[2]);
    assert_eq!(s.event_mask, LocatorEventMask::BUTTON_DOWN_OFF);
}

#[test]
fn decsle_unknown_ps_silently_ignored() {
    let mut s = DecLocatorState::new();
    s.apply_decsle(&[1, 99, 3]);
    assert_eq!(
        s.event_mask,
        LocatorEventMask::BUTTON_DOWN | LocatorEventMask::BUTTON_UP,
        "unknown Ps values silently skipped per spec leniency"
    );
}

// ── DECEFR (Pt;Pl;Pb;Pr ' w) ────────────────────────────────────────

#[test]
fn decefr_stores_rectangle() {
    let mut s = DecLocatorState::new();
    s.apply_decefr(10, 20, 30, 40);
    assert_eq!(
        s.filter_rect,
        Some(LocatorRect {
            top: 10,
            left: 20,
            bottom: 30,
            right: 40
        })
    );
}

#[test]
fn decefr_all_zeros_clears_rectangle() {
    let mut s = DecLocatorState::new();
    s.apply_decefr(10, 20, 30, 40);
    assert!(s.filter_rect.is_some());
    s.apply_decefr(0, 0, 0, 0);
    assert_eq!(s.filter_rect, None);
}

#[test]
fn decefr_replaces_previous_rectangle() {
    let mut s = DecLocatorState::new();
    s.apply_decefr(1, 2, 3, 4);
    s.apply_decefr(10, 20, 30, 40);
    assert_eq!(
        s.filter_rect,
        Some(LocatorRect {
            top: 10,
            left: 20,
            bottom: 30,
            right: 40
        })
    );
}

// ── Cross-state independence ────────────────────────────────────────

#[test]
fn decsle_does_not_affect_reporting() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(1, 0);
    s.apply_decsle(&[1, 3]);
    assert_eq!(s.reporting, Some(LocatorReportingMode::Continuous));
}

#[test]
fn decefr_does_not_affect_reporting_or_mask() {
    let mut s = DecLocatorState::new();
    s.apply_decelr(2, 1); // one-shot, pixels
    s.apply_decsle(&[1]);
    s.apply_decefr(10, 20, 30, 40);
    assert_eq!(s.reporting, Some(LocatorReportingMode::OneShot));
    assert!(s.pixel_unit);
    assert_eq!(s.event_mask, LocatorEventMask::BUTTON_DOWN);
}
