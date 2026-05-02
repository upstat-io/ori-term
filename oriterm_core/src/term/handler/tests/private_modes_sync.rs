//! Catalog rows: DEC-DECSCNM, DEC-DECNRCM, DEC-SIXEL-SCROLLING, DEC-BRACKETED-PASTE,
//! DEC-SIXEL-CURSOR-RIGHT

use crate::effect::sink::EffectSink;
use crate::effect::{Effect, HostEffect};
use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_effect_sink};

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

/// Count how many of the given `HostEffect` variants appear in the drained
/// effect set. Used by mode-1042 / urgency-hint tests to assert that BEL
/// emits `Bell` always and `UrgencyHint` only when mode 1042 is set.
fn count_host_effects(
    effects: &[Effect],
    matcher: impl Fn(&HostEffect) -> bool,
) -> usize {
    effects
        .iter()
        .filter(|e| matches!(e, Effect::Host(h) if matcher(h)))
        .count()
}

// --- BSU/ESU (Synchronized Update, mode 2026) ---

#[test]
fn bsu_esu_sync_update_via_vte() {
    let mut t = term();

    // Mode 2026 should start off.
    assert!(
        !t.mode().contains(TermMode::SYNC_UPDATE),
        "SYNC_UPDATE should be off by default"
    );

    // BSU: Begin Synchronized Update (DECSET ?2026).
    feed(&mut t, b"\x1b[?2026h");
    assert!(
        t.mode().contains(TermMode::SYNC_UPDATE),
        "SYNC_UPDATE should be on after \\x1b[?2026h"
    );

    // ESU: End Synchronized Update (DECRST ?2026).
    feed(&mut t, b"\x1b[?2026l");
    assert!(
        !t.mode().contains(TermMode::SYNC_UPDATE),
        "SYNC_UPDATE should be off after \\x1b[?2026l"
    );
}

// --- Focus in/out (mode 1004) ---

#[test]
fn focus_in_out_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::FOCUS_IN_OUT));

    feed(&mut t, b"\x1b[?1004h");
    assert!(t.mode().contains(TermMode::FOCUS_IN_OUT));
}

#[test]
fn focus_in_out_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1004h");
    feed(&mut t, b"\x1b[?1004l");
    assert!(!t.mode().contains(TermMode::FOCUS_IN_OUT));
}

// --- Alternate scroll (mode 1007) — default ON ---

#[test]
fn alternate_scroll_is_on_by_default() {
    let t = term();
    assert!(
        t.mode().contains(TermMode::ALTERNATE_SCROLL),
        "ALTERNATE_SCROLL should be on by default"
    );
}

#[test]
fn alternate_scroll_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1007l");
    assert!(!t.mode().contains(TermMode::ALTERNATE_SCROLL));
}

#[test]
fn alternate_scroll_decset_restores_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1007l");
    assert!(!t.mode().contains(TermMode::ALTERNATE_SCROLL));

    feed(&mut t, b"\x1b[?1007h");
    assert!(t.mode().contains(TermMode::ALTERNATE_SCROLL));
}

// --- Urgency hints (mode 1042) ---

#[test]
fn urgency_hints_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::URGENCY_HINTS));

    feed(&mut t, b"\x1b[?1042h");
    assert!(t.mode().contains(TermMode::URGENCY_HINTS));
}

#[test]
fn urgency_hints_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1042h");
    feed(&mut t, b"\x1b[?1042l");
    assert!(!t.mode().contains(TermMode::URGENCY_HINTS));
}

/// BUG-08-014 — semantic pin: BEL with mode 1042 set emits `HostEffect::UrgencyHint`
/// in addition to the unconditional `HostEffect::Bell`.
#[test]
fn bell_with_mode_1042_active_emits_urgency_hint() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?1042h");
    feed(&mut t, b"\x07");

    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::Bell)),
        1,
        "BEL must always emit HostEffect::Bell, got {effects:?}"
    );
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::UrgencyHint)),
        1,
        "BEL with mode 1042 set must emit HostEffect::UrgencyHint, got {effects:?}"
    );
}

/// BUG-08-014 — negative pin: BEL without mode 1042 must NOT emit `UrgencyHint`.
#[test]
fn bell_without_mode_1042_must_not_emit_urgency_hint() {
    let mut t = term_with_effect_sink();
    // Mode 1042 is OFF by default — verify before BEL.
    assert!(!t.mode().contains(TermMode::URGENCY_HINTS));

    feed(&mut t, b"\x07");

    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::Bell)),
        1,
        "BEL must always emit HostEffect::Bell, got {effects:?}"
    );
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::UrgencyHint)),
        0,
        "BEL without mode 1042 must NOT emit HostEffect::UrgencyHint, got {effects:?}"
    );
}

/// BUG-08-014 — toggle interaction: SET → RESET → SET, then BEL emits urgency.
#[test]
fn bell_after_mode_1042_decset_decrst_decset_emits_urgency_hint() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?1042h");
    feed(&mut t, b"\x1b[?1042l");
    feed(&mut t, b"\x1b[?1042h");
    feed(&mut t, b"\x07");

    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::UrgencyHint)),
        1,
        "BEL after SET-RESET-SET must emit one UrgencyHint, got {effects:?}"
    );
}

/// BUG-08-014 — toggle interaction: SET → RESET, then BEL emits no urgency.
#[test]
fn bell_after_mode_1042_decset_decrst_must_not_emit_urgency_hint() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?1042h");
    feed(&mut t, b"\x1b[?1042l");
    feed(&mut t, b"\x07");

    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::UrgencyHint)),
        0,
        "BEL after SET-RESET must emit zero UrgencyHint, got {effects:?}"
    );
}

/// BUG-08-014 — repeated emission: each BEL with mode 1042 set emits one urgency hint.
#[test]
fn three_bells_with_mode_1042_active_emit_three_urgency_hints() {
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?1042h");
    feed(&mut t, b"\x07\x07\x07");

    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::Bell)),
        3,
        "Three BELs must emit three HostEffect::Bell, got {effects:?}"
    );
    assert_eq!(
        count_host_effects(&effects, |h| matches!(h, HostEffect::UrgencyHint)),
        3,
        "Three BELs with mode 1042 must emit three UrgencyHint, got {effects:?}"
    );
}

// --- Sixel scrolling (mode 80) — default ON ---

#[test]
fn sixel_scrolling_is_on_by_default() {
    let t = term();
    assert!(
        t.mode().contains(TermMode::SIXEL_SCROLLING),
        "SIXEL_SCROLLING should be on by default"
    );
}

#[test]
fn sixel_scrolling_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?80l");
    assert!(!t.mode().contains(TermMode::SIXEL_SCROLLING));
}

#[test]
fn sixel_scrolling_decset_restores_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?80l");
    feed(&mut t, b"\x1b[?80h");
    assert!(t.mode().contains(TermMode::SIXEL_SCROLLING));
}

// --- Sixel cursor right (mode 8452) ---

#[test]
fn sixel_cursor_right_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::SIXEL_CURSOR_RIGHT));

    feed(&mut t, b"\x1b[?8452h");
    assert!(t.mode().contains(TermMode::SIXEL_CURSOR_RIGHT));
}

#[test]
fn sixel_cursor_right_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?8452h");
    feed(&mut t, b"\x1b[?8452l");
    assert!(!t.mode().contains(TermMode::SIXEL_CURSOR_RIGHT));
}

// --- Win32 input (mode 9001) ---

#[test]
fn win32_input_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::WIN32_INPUT));

    feed(&mut t, b"\x1b[?9001h");
    assert!(t.mode().contains(TermMode::WIN32_INPUT));
}

#[test]
fn win32_input_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?9001h");
    feed(&mut t, b"\x1b[?9001l");
    assert!(!t.mode().contains(TermMode::WIN32_INPUT));
}

// --- Enable mode 3 / DECNRCM (mode 40) ---

#[test]
fn enable_mode_3_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::ENABLE_MODE_3));

    feed(&mut t, b"\x1b[?40h");
    assert!(t.mode().contains(TermMode::ENABLE_MODE_3));
}

#[test]
fn enable_mode_3_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?40h");
    feed(&mut t, b"\x1b[?40l");
    assert!(!t.mode().contains(TermMode::ENABLE_MODE_3));
}

// --- Reverse video / DECSCNM (mode 5) ---

#[test]
fn reverse_video_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::REVERSE_VIDEO));

    feed(&mut t, b"\x1b[?5h");
    assert!(t.mode().contains(TermMode::REVERSE_VIDEO));
}

#[test]
fn reverse_video_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?5h");
    feed(&mut t, b"\x1b[?5l");
    assert!(!t.mode().contains(TermMode::REVERSE_VIDEO));
}

// --- Bracketed paste (mode 2004) ---

#[test]
fn bracketed_paste_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::BRACKETED_PASTE));

    feed(&mut t, b"\x1b[?2004h");
    assert!(t.mode().contains(TermMode::BRACKETED_PASTE));
}

#[test]
fn bracketed_paste_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?2004h");
    feed(&mut t, b"\x1b[?2004l");
    assert!(!t.mode().contains(TermMode::BRACKETED_PASTE));
}

// --- Left-right margin / DECLRMM (mode 69) ---

#[test]
fn left_right_margin_decset_sets_flag() {
    let mut t = term();
    assert!(!t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));

    feed(&mut t, b"\x1b[?69h");
    assert!(t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));
}

#[test]
fn left_right_margin_decrst_clears_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[?69l");
    assert!(!t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));
}

// --- RIS clears all miscellaneous modes ---

#[test]
fn ris_restores_default_mode_flags() {
    let mut t = term();
    // Set a variety of non-default modes.
    feed(&mut t, b"\x1b[?1004h"); // FOCUS_IN_OUT
    feed(&mut t, b"\x1b[?2004h"); // BRACKETED_PASTE
    feed(&mut t, b"\x1b[?2026h"); // SYNC_UPDATE
    feed(&mut t, b"\x1b[?9001h"); // WIN32_INPUT
    feed(&mut t, b"\x1b[?5h"); // REVERSE_VIDEO
    // Turn off a default-on mode.
    feed(&mut t, b"\x1b[?1007l"); // ALTERNATE_SCROLL off

    // Verify non-default state.
    assert!(t.mode().contains(TermMode::FOCUS_IN_OUT));
    assert!(t.mode().contains(TermMode::BRACKETED_PASTE));
    assert!(t.mode().contains(TermMode::SYNC_UPDATE));
    assert!(t.mode().contains(TermMode::WIN32_INPUT));
    assert!(t.mode().contains(TermMode::REVERSE_VIDEO));
    assert!(!t.mode().contains(TermMode::ALTERNATE_SCROLL));

    // Full reset.
    feed(&mut t, b"\x1bc");

    // All should return to default.
    assert!(!t.mode().contains(TermMode::FOCUS_IN_OUT));
    assert!(!t.mode().contains(TermMode::BRACKETED_PASTE));
    assert!(!t.mode().contains(TermMode::SYNC_UPDATE));
    assert!(!t.mode().contains(TermMode::WIN32_INPUT));
    assert!(!t.mode().contains(TermMode::REVERSE_VIDEO));
    assert!(
        t.mode().contains(TermMode::ALTERNATE_SCROLL),
        "ALTERNATE_SCROLL should be restored by RIS"
    );
    assert!(
        t.mode().contains(TermMode::SIXEL_SCROLLING),
        "SIXEL_SCROLLING should be restored by RIS"
    );
}
