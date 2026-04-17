//! Handler-level tests for Mode 2031 (Color Scheme Update Notification).
//!
//! Tests the `Term::set_theme()` notification hook and mode flag directly,
//! complementing the spec_chain tests in `oriterm_core/tests/spec_chain/
//! private_modes/mode_2031.rs`.

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, PtyEffect};
use crate::term::TermMode;
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_effect_sink};

/// DECSET `?2031` sets `COLOR_SCHEME_UPDATE` flag.
#[test]
fn mode_2031_decset_sets_flag() {
    let mut t = term_with_effect_sink();
    assert!(!t.mode().contains(TermMode::COLOR_SCHEME_UPDATE));
    feed(&mut t, b"\x1b[?2031h");
    assert!(t.mode().contains(TermMode::COLOR_SCHEME_UPDATE));
}

/// DECRST `?2031` clears `COLOR_SCHEME_UPDATE` flag.
#[test]
fn mode_2031_decrst_clears_flag() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?2031h");
    feed(&mut t, b"\x1b[?2031l");
    assert!(!t.mode().contains(TermMode::COLOR_SCHEME_UPDATE));
}

/// Helper: drain all PTY write effects from the sink.
fn drain_pty_writes(t: &impl EffectSink) -> Vec<Vec<u8>> {
    let mut effects = Vec::new();
    t.drain_into(&mut effects);
    effects
        .into_iter()
        .filter_map(|e| match e {
            Effect::Pty(PtyEffect::Write { bytes, .. }) => Some(bytes),
            _ => None,
        })
        .collect()
}

/// Mode 2031 enabled + Dark→Light: emits `\x1b[?997;2n`.
#[test]
fn mode_2031_dark_to_light_notification() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?2031h");
    // Drain DECSET effects.
    drain_pty_writes(t.effect_sink());

    t.set_theme(Theme::Light);
    let writes = drain_pty_writes(t.effect_sink());

    assert_eq!(writes.len(), 1, "expected exactly one PTY write");
    assert_eq!(writes[0], b"\x1b[?997;2n");
}

/// Mode 2031 enabled + Light→Dark: emits `\x1b[?997;1n`.
#[test]
fn mode_2031_light_to_dark_notification() {
    let mut t = term_with_effect_sink();
    // Start with Light, mode disabled — no notification.
    t.set_theme(Theme::Light);
    drain_pty_writes(t.effect_sink());

    // Enable mode 2031.
    feed(&mut t, b"\x1b[?2031h");
    drain_pty_writes(t.effect_sink());

    // Dark transition.
    t.set_theme(Theme::Dark);
    let writes = drain_pty_writes(t.effect_sink());

    assert_eq!(writes.len(), 1, "expected exactly one PTY write");
    assert_eq!(writes[0], b"\x1b[?997;1n");
}

/// Mode 2031 disabled: no notification on theme change.
#[test]
fn mode_2031_disabled_no_notification() {
    let mut t = term_with_effect_sink();
    t.set_theme(Theme::Light);
    let writes = drain_pty_writes(t.effect_sink());
    assert!(
        writes.is_empty(),
        "no notification expected when mode 2031 is disabled"
    );
}

/// Theme::Unknown emits no notification (policy pin).
#[test]
fn mode_2031_unknown_theme_no_notification() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?2031h");
    drain_pty_writes(t.effect_sink());

    t.set_theme(Theme::Unknown);
    let writes = drain_pty_writes(t.effect_sink());
    assert!(
        writes.is_empty(),
        "Theme::Unknown must produce zero PTY effects"
    );
}

/// Same theme twice: no duplicate notification.
#[test]
fn mode_2031_same_theme_no_duplicate() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?2031h");
    drain_pty_writes(t.effect_sink());

    // Dark is the default — calling set_theme(Dark) is a no-op.
    t.set_theme(Theme::Dark);
    let writes = drain_pty_writes(t.effect_sink());
    assert!(writes.is_empty(), "no-op set_theme must emit zero effects");
}

/// No backfill: enabling mode 2031 after a prior theme change does NOT emit.
#[test]
fn mode_2031_no_backfill_on_enable() {
    let mut t = term_with_effect_sink();
    t.set_theme(Theme::Light);
    drain_pty_writes(t.effect_sink());

    feed(&mut t, b"\x1b[?2031h");
    let writes = drain_pty_writes(t.effect_sink());
    assert!(
        writes.is_empty(),
        "enabling mode 2031 must NOT back-fill a stale notification"
    );
}

/// Alt screen does NOT suppress mode 2031 notification.
#[test]
fn mode_2031_fires_in_alt_screen() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?2031h");
    drain_pty_writes(t.effect_sink());

    // Switch to alt screen.
    feed(&mut t, b"\x1b[?1049h");
    drain_pty_writes(t.effect_sink());

    // Theme change while in alt screen — must still fire.
    t.set_theme(Theme::Light);
    let writes = drain_pty_writes(t.effect_sink());
    assert_eq!(writes.len(), 1, "notification must fire in alt screen");
    assert_eq!(writes[0], b"\x1b[?997;2n");
}

/// DECRQM reports correctly for mode 2031.
#[test]
fn mode_2031_decrqm() {
    let mut t = term_with_effect_sink();

    // Default: reset (2).
    feed(&mut t, b"\x1b[?2031$p");
    let writes = drain_pty_writes(t.effect_sink());
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0], b"\x1b[?2031;2$y");

    // Set, then query.
    feed(&mut t, b"\x1b[?2031h\x1b[?2031$p");
    let writes = drain_pty_writes(t.effect_sink());
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0], b"\x1b[?2031;1$y");
}
