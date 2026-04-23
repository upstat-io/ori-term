//! Spec-chain tests for Mode 2031 (Color Scheme Update Notification).
//!
//! Section 09.3 scope: verifies full implementation — VTE parse, core-layer
//! DECSET/DECRST flag toggle, DECRQM, and `Term::set_theme()` notification
//! hook that emits `CSI ? 997 ; Ps n` when mode 2031 is enabled.
//!
//! # Policy pins (per kitty semantics)
//!
//! - `Theme::Unknown` emits NO notification
//! - Enabling mode 2031 does NOT backfill a stale notification
//! - No-op `set_theme()` (same theme) emits NO notification
//! - No notification on `Term::new()` construction
//!
//! # Catalog rows verified
//!
//! - `DEC-COLOR-SCHEME-UPDATE` — flag toggle + DECRQM + notification

use oriterm_core::{TermMode, Theme};
use oriterm_test_support::spec_chain::{SpecHarness, pty_writes};

// ── Flag toggle ──────────────────────────────────────────────────────

/// DECSET `?2031` sets `TermMode::COLOR_SCHEME_UPDATE`.
#[test]
fn mode_2031_decset_toggles_flag() {
    let mut harness = SpecHarness::new();

    assert!(
        !harness
            .term()
            .mode()
            .contains(TermMode::COLOR_SCHEME_UPDATE),
        "default TermMode must NOT include COLOR_SCHEME_UPDATE"
    );

    harness.feed(b"\x1b[?2031h");
    assert!(
        harness
            .term()
            .mode()
            .contains(TermMode::COLOR_SCHEME_UPDATE),
        "after DECSET ?2031, COLOR_SCHEME_UPDATE must be set"
    );
}

/// DECRST `?2031` clears `TermMode::COLOR_SCHEME_UPDATE`.
#[test]
fn mode_2031_decrst_clears_flag() {
    let mut harness = SpecHarness::new();

    harness.feed(b"\x1b[?2031h");
    assert!(
        harness
            .term()
            .mode()
            .contains(TermMode::COLOR_SCHEME_UPDATE)
    );

    harness.feed(b"\x1b[?2031l");
    assert!(
        !harness
            .term()
            .mode()
            .contains(TermMode::COLOR_SCHEME_UPDATE),
        "after DECRST ?2031, COLOR_SCHEME_UPDATE must be cleared"
    );
}

// ── DECRQM ───────────────────────────────────────────────────────────

/// DECRQM `?2031` returns `2` (reset) in the default state.
#[test]
fn mode_2031_decrqm_reports_correctly() {
    let mut harness = SpecHarness::new();

    // Query while reset.
    harness.feed(b"\x1b[?2031$p");
    let reset_response = b"\x1b[?2031;2$y";
    let found_reset = pty_writes(&harness).any(|(b, _)| b == reset_response);
    assert!(
        found_reset,
        "DECRQM ?2031 when reset should emit {:?}, got: {:?}",
        String::from_utf8_lossy(reset_response),
        harness.outcome().effects_emitted
    );

    // Set the mode and query again.
    harness.feed(b"\x1b[?2031h");
    harness.feed(b"\x1b[?2031$p");
    let set_response = b"\x1b[?2031;1$y";
    let found_set = pty_writes(&harness).any(|(b, _)| b == set_response);
    assert!(
        found_set,
        "DECRQM ?2031 when set should emit {:?}, got: {:?}",
        String::from_utf8_lossy(set_response),
        harness.outcome().effects_emitted
    );
}

// ── Notification tests ───────────────────────────────────────────────

/// With mode 2031 disabled, `set_theme()` emits NO notification.
#[test]
fn mode_2031_disabled_no_notification_on_scheme_change() {
    let mut harness = SpecHarness::new();

    // Mode 2031 is off by default.
    harness.term_mut().set_theme(Theme::Light);
    harness.drain_effects();

    let has_notification = pty_writes(&harness).any(|(b, _)| b.starts_with(b"\x1b[?997"));
    assert!(
        !has_notification,
        "with mode 2031 disabled, set_theme() must not emit ?997 notification"
    );
}

/// Mode 2031 enabled: `set_theme(Dark)` on a Light terminal emits `\x1b[?997;1n`.
#[test]
fn mode_2031_dark_scheme_emits_997_1_notification() {
    let mut harness = SpecHarness::new();

    // Set to Light first (with mode disabled — no notification).
    harness.term_mut().set_theme(Theme::Light);
    harness.drain_effects();

    // Enable mode 2031.
    harness.feed(b"\x1b[?2031h");

    // Switch to Dark — should emit notification.
    harness.term_mut().set_theme(Theme::Dark);
    harness.drain_effects();

    let expected = b"\x1b[?997;1n";
    let found = pty_writes(&harness).any(|(b, _)| b == expected);
    assert!(
        found,
        "set_theme(Dark) with mode 2031 enabled should emit {:?}, got: {:?}",
        String::from_utf8_lossy(expected),
        harness.outcome().effects_emitted
    );
}

/// Mode 2031 enabled: `set_theme(Light)` on a Dark terminal emits `\x1b[?997;2n`.
#[test]
fn mode_2031_light_scheme_emits_997_2_notification() {
    let mut harness = SpecHarness::new();

    // Default theme is Dark — enable mode 2031.
    harness.feed(b"\x1b[?2031h");

    // Switch to Light.
    harness.term_mut().set_theme(Theme::Light);
    harness.drain_effects();

    let expected = b"\x1b[?997;2n";
    let found = pty_writes(&harness).any(|(b, _)| b == expected);
    assert!(
        found,
        "set_theme(Light) with mode 2031 enabled should emit {:?}, got: {:?}",
        String::from_utf8_lossy(expected),
        harness.outcome().effects_emitted
    );
}

/// Toggling mode 2031 on/off does NOT emit a notification by itself.
#[test]
fn mode_2031_mode_toggle_does_not_emit_notification_by_itself() {
    let mut harness = SpecHarness::new();

    // Toggle on and off.
    harness.feed(b"\x1b[?2031h");
    harness.feed(b"\x1b[?2031l");

    let has_notification = pty_writes(&harness).any(|(b, _)| b.starts_with(b"\x1b[?997"));
    assert!(
        !has_notification,
        "toggling mode 2031 on/off must not emit a ?997 notification"
    );
}

/// No-op `set_theme()`: calling `set_theme(Dark)` when already Dark emits nothing.
#[test]
fn mode_2031_same_theme_no_notification() {
    let mut harness = SpecHarness::new();

    // Enable mode 2031 (default theme is Dark).
    harness.feed(b"\x1b[?2031h");

    // Call set_theme with the same theme — should be a no-op.
    harness.term_mut().set_theme(Theme::Dark);
    harness.drain_effects();

    let has_notification = pty_writes(&harness).any(|(b, _)| b.starts_with(b"\x1b[?997"));
    assert!(
        !has_notification,
        "set_theme(Dark) when already Dark must not emit notification, even with mode 2031 on"
    );
}

// ── Policy pins (C.1) ────────────────────────────────────────────────

/// Theme::Unknown policy pin (positive): no notification emitted.
#[test]
fn mode_2031_unknown_theme_no_notification() {
    let mut harness = SpecHarness::new();

    // Enable mode 2031.
    harness.feed(b"\x1b[?2031h");

    // Set theme to Unknown.
    harness.term_mut().set_theme(Theme::Unknown);
    harness.drain_effects();

    let has_any_pty = pty_writes(&harness).next().is_some();
    assert!(
        !has_any_pty,
        "Theme::Unknown must produce zero PTY effects, not just zero ?997 bytes"
    );
}

/// Theme::Unknown policy pin (negative): effect sink was NOT pushed to at all.
#[test]
fn mode_2031_unknown_theme_zero_pty_effects() {
    let mut harness = SpecHarness::new();

    // Set to Light first, then enable mode 2031.
    harness.term_mut().set_theme(Theme::Light);
    harness.drain_effects();
    harness.feed(b"\x1b[?2031h");

    // Now transition to Unknown — must produce zero effects.
    harness.term_mut().set_theme(Theme::Unknown);
    harness.drain_effects();

    // Filter effects to only those after the DECSET (the DECSET doesn't emit PTY writes).
    let pty_effects_after_unknown: Vec<_> = pty_writes(&harness).collect();
    assert!(
        pty_effects_after_unknown.is_empty(),
        "Theme::Unknown transition must not push any PTY write effect: {:?}",
        pty_effects_after_unknown
    );
}

/// No-backfill on enable pin: enabling mode 2031 does NOT replay prior theme.
#[test]
fn mode_2031_no_backfill_on_enable() {
    let mut harness = SpecHarness::new();

    // Set theme to Light with mode disabled.
    harness.term_mut().set_theme(Theme::Light);
    harness.drain_effects();

    // Enable mode 2031 — must NOT synthesize a stale notification.
    harness.feed(b"\x1b[?2031h");

    let has_notification = pty_writes(&harness).any(|(b, _)| b.starts_with(b"\x1b[?997"));
    assert!(
        !has_notification,
        "enabling mode 2031 must NOT back-fill a stale ?997 notification"
    );
}

/// No-notification-on-no-op pin: 100 consecutive same-theme calls emit nothing.
#[test]
fn mode_2031_no_notification_on_repeated_same_theme() {
    let mut harness = SpecHarness::new();

    // Enable mode 2031.
    harness.feed(b"\x1b[?2031h");

    // 100 consecutive Dark → Dark calls.
    for _ in 0..100 {
        harness.term_mut().set_theme(Theme::Dark);
        harness.drain_effects();
    }

    let notification_count = pty_writes(&harness)
        .filter(|(b, _)| b.starts_with(b"\x1b[?997"))
        .count();
    assert_eq!(
        notification_count, 0,
        "repeated same-theme calls must emit zero notifications, got {notification_count}"
    );
}

/// No-notification-on-construction pin: `Term::new()` does not emit notification.
#[test]
fn mode_2031_no_notification_on_construction() {
    // SpecHarness::new() creates a Term — verify no stale effects.
    let harness = SpecHarness::new();

    let has_notification = pty_writes(&harness).any(|(b, _)| b.starts_with(b"\x1b[?997"));
    assert!(
        !has_notification,
        "Term::new() must not emit a ?997 notification"
    );
}

/// DECSET ?2031 must NOT touch unrelated flags.
#[test]
fn mode_2031_does_not_touch_unrelated_flags() {
    let mut harness = SpecHarness::new();
    let mode_before = harness.term().mode().bits();

    harness.feed(b"\x1b[?2031h");

    let mode_after = harness.term().mode().bits();
    let changed_bits = mode_before ^ mode_after;

    assert_eq!(
        changed_bits,
        TermMode::COLOR_SCHEME_UPDATE.bits(),
        "DECSET ?2031 must only toggle COLOR_SCHEME_UPDATE; unexpected bits: {changed_bits:#b}"
    );
}
