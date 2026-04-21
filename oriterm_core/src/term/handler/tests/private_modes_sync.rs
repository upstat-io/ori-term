//! Catalog rows: DEC-DECSCNM, DEC-DECNRCM, DEC-SIXEL-SCROLLING, DEC-BRACKETED-PASTE,
//! DEC-SIXEL-CURSOR-RIGHT

use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::feed;

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
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
