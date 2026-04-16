use crate::index::Column;
use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::feed;

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- Legacy alt screen ---

#[test]
fn mode_47_swaps_without_cursor_save() {
    let mut t = term();
    // Move cursor to (5, 10).
    feed(&mut t, b"\x1b[6;11H");
    assert_eq!(t.grid().cursor().line(), 5);
    assert_eq!(t.grid().cursor().col(), Column(10));

    // Enter alt screen (mode 47).
    feed(&mut t, b"\x1b[?47h");
    assert!(t.mode().contains(TermMode::ALT_SCREEN));
    // Cursor is NOT saved; alt screen cursor starts at origin.
    // (The alt grid's cursor was never explicitly set, so it's at 0,0.)

    // Move cursor in alt screen.
    feed(&mut t, b"\x1b[3;5H");
    assert_eq!(t.grid().cursor().line(), 2);
    assert_eq!(t.grid().cursor().col(), Column(4));

    // Leave alt screen (mode 47).
    feed(&mut t, b"\x1b[?47l");
    assert!(!t.mode().contains(TermMode::ALT_SCREEN));
    // Cursor is NOT restored — whatever was on primary grid stays.
    // We verify alt screen was exited.
}

#[test]
fn mode_1047_clears_alt_on_enter() {
    let mut t = term();
    // Enter alt screen with mode 1049 first (which saves/restores).
    feed(&mut t, b"\x1b[?1049h");
    // Write content.
    feed(&mut t, b"ALTTEXT");
    // Leave alt screen.
    feed(&mut t, b"\x1b[?1049l");

    // Now enter alt screen with mode 1047 — alt should be cleared.
    feed(&mut t, b"\x1b[?1047h");
    assert!(t.mode().contains(TermMode::ALT_SCREEN));

    // Verify alt grid is clean (first cell is blank).
    let ch = t.grid()[crate::index::Line(0)][Column(0)].ch;
    assert!(
        ch == ' ' || ch == '\0',
        "alt grid should be cleared on mode 1047 enter, got {ch:?}"
    );

    feed(&mut t, b"\x1b[?1047l");
    assert!(!t.mode().contains(TermMode::ALT_SCREEN));
}

#[test]
fn mode_1048_saves_and_restores_cursor() {
    let mut t = term();
    // Move cursor to (3, 7).
    feed(&mut t, b"\x1b[4;8H");
    assert_eq!(t.grid().cursor().line(), 3);
    assert_eq!(t.grid().cursor().col(), Column(7));

    // Save cursor (mode 1048 DECSET).
    feed(&mut t, b"\x1b[?1048h");

    // Move cursor elsewhere.
    feed(&mut t, b"\x1b[1;1H");
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));

    // Restore cursor (mode 1048 DECRST).
    feed(&mut t, b"\x1b[?1048l");
    assert_eq!(t.grid().cursor().line(), 3);
    assert_eq!(t.grid().cursor().col(), Column(7));
}

// --- Reverse wraparound (mode 45) ---

#[test]
fn reverse_wrap_at_col0_wraps_to_previous_wrapped_line() {
    let mut t = Term::new(24, 10, 0, Theme::default(), crate::effect::VoidEffectSink);
    // Enable reverse wraparound.
    feed(&mut t, b"\x1b[?45h");
    assert!(t.mode().contains(TermMode::REVERSE_WRAP));

    // Fill first line and force wrap with one more char.
    feed(&mut t, b"1234567890X");
    // "1234567890" fills line 0, WRAP flag set, "X" goes to line 1 col 0.
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(1));

    // Move to col 0.
    feed(&mut t, b"\r");
    assert_eq!(t.grid().cursor().col(), Column(0));

    // BS should wrap back to line 0, col 9 (last col of wrapped line).
    feed(&mut t, b"\x08");
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(9));
}

#[test]
fn reverse_wrap_at_col0_noop_if_not_wrapped() {
    let mut t = Term::new(24, 10, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, b"\x1b[?45h");

    // Write a short line (no wrap) and move to start of next line.
    feed(&mut t, b"hello\r\n");
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(0));

    // BS at col 0: previous line was NOT soft-wrapped, so no-op.
    feed(&mut t, b"\x08");
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn reverse_wrap_disabled_does_not_wrap() {
    let mut t = Term::new(24, 10, 0, Theme::default(), crate::effect::VoidEffectSink);
    // Do NOT enable mode 45.

    // Fill first line and force wrap.
    feed(&mut t, b"1234567890X");
    assert_eq!(t.grid().cursor().line(), 1);

    // Move to col 0.
    feed(&mut t, b"\r");
    assert_eq!(t.grid().cursor().col(), Column(0));

    // BS should stay at col 0 (normal behavior, no reverse wrap).
    feed(&mut t, b"\x08");
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- XTSAVE/XTRESTORE ---

#[test]
fn xtsave_xtrestore_saves_and_restores_mode() {
    let mut t = term();
    // Verify cursor is visible (default).
    assert!(t.mode().contains(TermMode::SHOW_CURSOR));

    // Save mode 25 (show cursor).
    feed(&mut t, b"\x1b[?25s");

    // Clear mode 25.
    feed(&mut t, b"\x1b[?25l");
    assert!(!t.mode().contains(TermMode::SHOW_CURSOR));

    // Restore mode 25 — should re-enable.
    feed(&mut t, b"\x1b[?25r");
    assert!(t.mode().contains(TermMode::SHOW_CURSOR));
}

#[test]
fn xtrestore_without_save_is_noop() {
    let mut t = term();
    let before = t.mode();
    // Restore mode 25 without saving — should be no-op.
    feed(&mut t, b"\x1b[?25r");
    assert_eq!(t.mode(), before);
}

#[test]
fn xtsave_xtrestore_multiple_modes_independently() {
    let mut t = term();
    // Enable bracketed paste.
    feed(&mut t, b"\x1b[?2004h");
    assert!(t.mode().contains(TermMode::BRACKETED_PASTE));

    // Save modes 25 and 2004.
    feed(&mut t, b"\x1b[?25;2004s");

    // Disable both.
    feed(&mut t, b"\x1b[?25l\x1b[?2004l");
    assert!(!t.mode().contains(TermMode::SHOW_CURSOR));
    assert!(!t.mode().contains(TermMode::BRACKETED_PASTE));

    // Restore both.
    feed(&mut t, b"\x1b[?25;2004r");
    assert!(t.mode().contains(TermMode::SHOW_CURSOR));
    assert!(t.mode().contains(TermMode::BRACKETED_PASTE));
}

#[test]
fn ris_clears_saved_private_modes() {
    let mut t = term();
    // Save mode 25.
    feed(&mut t, b"\x1b[?25s");
    // Disable mode 25.
    feed(&mut t, b"\x1b[?25l");

    // Full reset.
    feed(&mut t, b"\x1bc");

    // Restore should be no-op (saved modes cleared by RIS).
    // Mode 25 is set by default after RIS.
    assert!(t.mode().contains(TermMode::SHOW_CURSOR));

    // Disable it again.
    feed(&mut t, b"\x1b[?25l");
    assert!(!t.mode().contains(TermMode::SHOW_CURSOR));

    // Restore — should still be no-op.
    feed(&mut t, b"\x1b[?25r");
    assert!(!t.mode().contains(TermMode::SHOW_CURSOR));
}

// --- Alt screen + scroll region interaction ---

#[test]
fn alt_screen_with_scroll_region() {
    let mut t = term();
    // Write something on primary.
    feed(&mut t, b"PRIMARY");
    // Enter alt screen (1049).
    feed(&mut t, b"\x1b[?1049h");
    assert!(t.mode().contains(TermMode::ALT_SCREEN));

    // Set scroll region (lines 2-5).
    feed(&mut t, b"\x1b[2;5r");
    // Move into the scroll region and scroll.
    feed(&mut t, b"\x1b[3;1H");
    feed(&mut t, b"SCROLL");

    // Exit alt screen.
    feed(&mut t, b"\x1b[?1049l");
    assert!(!t.mode().contains(TermMode::ALT_SCREEN));

    // Primary screen content should be restored.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'P');
}

// --- Reverse wraparound at line 0 (boundary) ---

#[test]
fn reverse_wrap_at_line_0_is_noop() {
    let mut t = Term::new(24, 10, 0, Theme::default(), crate::effect::VoidEffectSink);
    // Enable reverse wraparound.
    feed(&mut t, b"\x1b[?45h");

    // Cursor starts at (0, 0). BS should be no-op — can't wrap above top.
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));

    feed(&mut t, b"\x08");
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- XTSAVE overwrite (last-write-wins) ---

#[test]
fn xtsave_overwrite_uses_latest_value() {
    let mut t = term();
    // Mode 25 (show cursor) is set by default.
    assert!(t.mode().contains(TermMode::SHOW_CURSOR));

    // Save mode 25 (currently set).
    feed(&mut t, b"\x1b[?25s");

    // Disable mode 25 and save again (now reset).
    feed(&mut t, b"\x1b[?25l");
    feed(&mut t, b"\x1b[?25s");

    // Re-enable mode 25.
    feed(&mut t, b"\x1b[?25h");
    assert!(t.mode().contains(TermMode::SHOW_CURSOR));

    // Restore — should restore to "reset" (the LATEST save), not "set".
    feed(&mut t, b"\x1b[?25r");
    assert!(
        !t.mode().contains(TermMode::SHOW_CURSOR),
        "XTSAVE should use last-write-wins, not first save"
    );
}

// --- Alt screen mode 47 double-enter is no-op ---

#[test]
fn mode_47_double_enter_is_noop() {
    let mut t = term();
    // Write on primary.
    feed(&mut t, b"hello");

    // Enter alt screen (mode 47).
    feed(&mut t, b"\x1b[?47h");
    assert!(t.mode().contains(TermMode::ALT_SCREEN));

    // Write on alt.
    feed(&mut t, b"ALT");

    // Enter again — should be no-op (already in alt screen).
    feed(&mut t, b"\x1b[?47h");
    assert!(t.mode().contains(TermMode::ALT_SCREEN));

    // Exit once should return to primary.
    feed(&mut t, b"\x1b[?47l");
    assert!(!t.mode().contains(TermMode::ALT_SCREEN));
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'h');
}

// --- Mode 1049 enter then 47 exit ---

#[test]
fn mode_1049_enter_then_47_exit() {
    let mut t = term();
    // Move cursor to (3, 5).
    feed(&mut t, b"\x1b[4;6H");
    assert_eq!(t.grid().cursor().line(), 3);
    assert_eq!(t.grid().cursor().col(), Column(5));

    // Enter alt screen via mode 1049 (saves cursor).
    feed(&mut t, b"\x1b[?1049h");
    assert!(t.mode().contains(TermMode::ALT_SCREEN));

    // Move cursor in alt screen.
    feed(&mut t, b"\x1b[1;1H");

    // Exit alt screen via mode 47 (no cursor restore — uses swap_alt_no_cursor).
    feed(&mut t, b"\x1b[?47l");
    assert!(!t.mode().contains(TermMode::ALT_SCREEN));
    // We simply verify the swap happened cleanly; cursor state depends on
    // which grid's cursor was active.
}

// --- XTSAVE/XTRESTORE with unknown mode number ---

#[test]
fn xtsave_xtrestore_unknown_mode_is_noop() {
    let mut t = term();
    let before = t.mode();

    // Save and restore an unknown mode number.
    feed(&mut t, b"\x1b[?99999s");
    feed(&mut t, b"\x1b[?99999r");

    // Mode should be unchanged.
    assert_eq!(t.mode(), before);
}

// --- DECSCNM (Reverse Video, mode 5) ---

#[test]
fn decscnm_set_enables_reverse_video() {
    use crate::term::TermMode;
    let mut t = term();
    assert!(!t.mode().contains(TermMode::REVERSE_VIDEO));
    feed(&mut t, b"\x1b[?5h");
    assert!(t.mode().contains(TermMode::REVERSE_VIDEO));
}

#[test]
fn decscnm_reset_disables_reverse_video() {
    use crate::term::TermMode;
    let mut t = term();
    feed(&mut t, b"\x1b[?5h");
    assert!(t.mode().contains(TermMode::REVERSE_VIDEO));
    feed(&mut t, b"\x1b[?5l");
    assert!(!t.mode().contains(TermMode::REVERSE_VIDEO));
}
