use crate::index::Column;
use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_recorder};

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- Keypad mode tests ---

#[test]
fn deckpam_sets_application_keypad() {
    let mut t = term();
    // ESC = — DECKPAM.
    feed(&mut t, b"\x1b=");

    assert!(t.mode().contains(TermMode::APP_KEYPAD));
}

#[test]
fn deckpnm_resets_application_keypad() {
    let mut t = term();
    feed(&mut t, b"\x1b="); // Enable
    // ESC > — DECKPNM.
    feed(&mut t, b"\x1b>");

    assert!(!t.mode().contains(TermMode::APP_KEYPAD));
}

// --- ESC sequence tests ---

#[test]
fn esc7_esc8_save_restore_cursor() {
    let mut t = term();
    // Move to (5, 10), save, move elsewhere, restore.
    feed(&mut t, b"\x1b[6;11H"); // CUP to line 5, col 10 (1-based)
    feed(&mut t, b"\x1b7"); // DECSC: save cursor
    feed(&mut t, b"\x1b[1;1H"); // CUP to (0, 0)
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col().0, 0);
    feed(&mut t, b"\x1b8"); // DECRC: restore cursor
    assert_eq!(t.grid().cursor().line(), 5);
    assert_eq!(t.grid().cursor().col().0, 10);
}

#[test]
fn esc_d_index_at_bottom_scrolls() {
    let mut t = term();
    // Fill first line so we can check scroll.
    feed(&mut t, b"TOP");
    // Move to the last line.
    feed(&mut t, b"\x1b[24;1H"); // CUP to last line (line 23, 0-based)
    feed(&mut t, b"BOTTOM");
    // ESC D (IND) at bottom should scroll up.
    feed(&mut t, b"\x1bD");
    // Cursor should still be on the last line.
    assert_eq!(t.grid().cursor().line(), 23);
    // The old last line ("BOTTOM") should now be on line 22.
    let row22 = &t.grid()[crate::index::Line(22)];
    let text: String = (0..6).map(|c| row22[Column(c)].ch).collect();
    assert_eq!(text, "BOTTOM");
}

#[test]
fn esc_m_reverse_index_at_top_scrolls_down() {
    let mut t = term();
    // Write on first line and move to top.
    feed(&mut t, b"LINE0");
    feed(&mut t, b"\x1b[1;1H"); // CUP to (0, 0)
    // ESC M (RI) at top should scroll content down.
    feed(&mut t, b"\x1bM");
    // Cursor stays at line 0.
    assert_eq!(t.grid().cursor().line(), 0);
    // "LINE0" should have moved to line 1.
    let row1 = &t.grid()[crate::index::Line(1)];
    let text: String = (0..5).map(|c| row1[Column(c)].ch).collect();
    assert_eq!(text, "LINE0");
}

#[test]
fn esc_c_ris_resets_all_state() {
    let (mut t, listener) = term_with_recorder();
    // Set up some state.
    feed(&mut t, b"\x1b]2;Custom Title\x07"); // Set title
    feed(&mut t, b"\x1b[5;10H"); // Move cursor
    feed(&mut t, b"\x1b[1m"); // Bold
    feed(&mut t, b"\x1b(0"); // DEC special graphics
    // Now reset everything.
    feed(&mut t, b"\x1bc");
    // Cursor should be at origin.
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col().0, 0);
    // Title should be cleared.
    assert!(t.title().is_empty());
    // Mode should be default.
    assert_eq!(t.mode(), TermMode::default());
    // Charset should be back to ASCII (not DEC special graphics).
    let mut charset = t.charset().clone();
    assert_eq!(charset.translate('q'), 'q');
    // ResetTitle event should have been sent.
    let events = listener.events();
    assert!(events.iter().any(|e| e.contains("ResetTitle")));
}

#[test]
fn esc_paren_0_activates_dec_special_graphics() {
    let mut t = term();
    // ESC (0: designate G0 as DEC Special Graphics.
    feed(&mut t, b"\x1b(0");
    // 'q' in DEC Special Graphics → '─' (U+2500, horizontal line).
    feed(&mut t, b"q");
    let ch = t.grid()[crate::index::Line(0)][Column(0)].ch;
    assert_eq!(ch, '─', "DEC special graphics 'q' should map to '─'");
}

#[test]
fn esc_paren_b_restores_ascii() {
    let mut t = term();
    // Switch to DEC special graphics, then back to ASCII.
    feed(&mut t, b"\x1b(0");
    feed(&mut t, b"\x1b(B");
    // 'q' should now be plain 'q'.
    feed(&mut t, b"q");
    let ch = t.grid()[crate::index::Line(0)][Column(0)].ch;
    assert_eq!(ch, 'q');
}

// --- ESC edge cases (from reference repos: Ghostty, WezTerm) ---

/// Ghostty: saveCursor saves/restores SGR attributes, not just position.
#[test]
fn esc7_esc8_preserves_sgr_attributes() {
    let mut t = term();
    // Set bold, then save cursor.
    feed(&mut t, b"\x1b[1m"); // Bold on
    feed(&mut t, b"\x1b7"); // DECSC
    // Clear bold.
    feed(&mut t, b"\x1b[0m"); // SGR reset
    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD)
    );
    // Restore — should bring back bold.
    feed(&mut t, b"\x1b8"); // DECRC
    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD),
        "DECRC should restore bold attribute",
    );
}

/// Ghostty: saveCursor pending wrap state — cursor at end of line, restore wraps.
#[test]
fn esc7_esc8_preserves_wrap_pending() {
    // 5-column terminal.
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    // Fill the line to trigger wrap-pending (col == cols).
    feed(&mut t, b"ABCDE");
    // Cursor col should be past last column (wrap-pending).
    assert_eq!(t.grid().cursor().col().0, 5);
    // Save, move cursor, restore.
    feed(&mut t, b"\x1b7"); // Save
    feed(&mut t, b"\x1b[1;1H"); // CUP to origin
    assert_eq!(t.grid().cursor().col().0, 0);
    feed(&mut t, b"\x1b8"); // Restore
    assert_eq!(
        t.grid().cursor().col().0,
        5,
        "DECRC should restore wrap-pending state (col == cols)",
    );
    // Next character should wrap to line 1.
    feed(&mut t, b"X");
    assert_eq!(t.grid().cursor().line(), 1);
    let ch = t.grid()[crate::index::Line(1)][Column(0)].ch;
    assert_eq!(ch, 'X');
}

/// Ghostty: configuring G1 without activating has no effect on G0 output.
#[test]
fn configure_g1_does_not_affect_g0() {
    let mut t = term();
    // Configure G1 as DEC special graphics (G0 stays ASCII).
    feed(&mut t, b"\x1b)0");
    // Print 'q' — should be plain 'q' since G0 is still active.
    feed(&mut t, b"q");
    let ch = t.grid()[crate::index::Line(0)][Column(0)].ch;
    assert_eq!(ch, 'q', "Configuring G1 should not affect G0 output");
}

/// WezTerm: SO/SI (Shift Out/Shift In) switching between G0 and G1.
#[test]
fn so_si_charset_switching() {
    let mut t = term();
    // Configure G1 as DEC special graphics.
    feed(&mut t, b"\x1b)0");
    // Print in G0 (ASCII).
    feed(&mut t, b"A");
    // SO (0x0E): switch to G1.
    feed(&mut t, b"\x0e");
    feed(&mut t, b"q"); // Should be '─' in DEC special.
    // SI (0x0F): switch back to G0.
    feed(&mut t, b"\x0f");
    feed(&mut t, b"B"); // Should be plain 'B'.

    let ch0 = t.grid()[crate::index::Line(0)][Column(0)].ch;
    let ch1 = t.grid()[crate::index::Line(0)][Column(1)].ch;
    let ch2 = t.grid()[crate::index::Line(0)][Column(2)].ch;
    assert_eq!(ch0, 'A');
    assert_eq!(ch1, '─', "After SO, G1 (DEC special) should be active");
    assert_eq!(ch2, 'B', "After SI, G0 (ASCII) should be active again");
}

/// Ghostty: single shift (SS2/SS3) applies to exactly one character.
#[test]
fn single_shift_applies_to_one_character() {
    let mut t = term();
    // Configure G2 as DEC special graphics.
    feed(&mut t, b"\x1b*0");
    // SS2 (ESC N): single shift G2 for next character only.
    feed(&mut t, b"\x1bN");
    feed(&mut t, b"q"); // Should be '─' (DEC special via SS2).
    feed(&mut t, b"q"); // Should be plain 'q' (back to G0/ASCII).

    let ch0 = t.grid()[crate::index::Line(0)][Column(0)].ch;
    let ch1 = t.grid()[crate::index::Line(0)][Column(1)].ch;
    assert_eq!(ch0, '─', "SS2 should apply DEC special for one character");
    assert_eq!(ch1, 'q', "After SS2, should revert to G0 (ASCII)");
}

/// WezTerm: DEC special graphics full alphabet mapping.
#[test]
fn dec_special_graphics_full_mapping() {
    let mut t = term();
    feed(&mut t, b"\x1b(0");
    // Characters 'a' through 'z' should all map to DEC special graphics.
    feed(&mut t, b"abcdefghijklmnopqrstuvwxyz");
    let expected = "▒␉␌␍␊°±␤␋┘┐┌└┼⎺⎻─⎼⎽├┤┴┬│≤≥";
    let chars: Vec<char> = expected.chars().collect();
    for (i, &expected_ch) in chars.iter().enumerate() {
        let actual = t.grid()[crate::index::Line(0)][Column(i)].ch;
        assert_eq!(
            actual, expected_ch,
            "DEC special graphics mapping mismatch at index {i}: expected '{expected_ch}' got '{actual}'",
        );
    }
}

/// Ghostty: fullReset resets pen/attributes to defaults.
#[test]
fn esc_c_ris_resets_pen_attributes() {
    let mut t = term();
    // Set bold + foreground color.
    feed(&mut t, b"\x1b[1;31m"); // Bold + red FG
    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD)
    );
    // RIS.
    feed(&mut t, b"\x1bc");
    // Attributes should be default.
    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD),
        "RIS should reset bold",
    );
    assert_eq!(
        t.grid().cursor().template.flags,
        crate::cell::CellFlags::empty(),
        "RIS should clear all cell flags",
    );
}

/// Ghostty: fullReset clears saved cursor.
#[test]
fn esc_c_ris_clears_saved_cursor() {
    let mut t = term();
    // Move cursor and save.
    feed(&mut t, b"\x1b[5;10H"); // CUP to (4, 9)
    feed(&mut t, b"\x1b7"); // DECSC
    // RIS.
    feed(&mut t, b"\x1bc");
    // Restore should NOT go to (4, 9) — saved cursor was cleared.
    feed(&mut t, b"\x1b8"); // DECRC
    assert_eq!(t.grid().cursor().line(), 0, "RIS should clear saved cursor");
    assert_eq!(
        t.grid().cursor().col().0,
        0,
        "RIS should clear saved cursor"
    );
}

/// Ghostty: fullReset from alt screen returns to primary.
#[test]
fn esc_c_ris_exits_alt_screen() {
    let mut t = term();
    // Write on primary, switch to alt.
    feed(&mut t, b"PRIMARY");
    feed(&mut t, b"\x1b[?1049h"); // DECSET 1049: enter alt screen
    assert!(t.mode().contains(TermMode::ALT_SCREEN));
    // RIS should exit alt screen.
    feed(&mut t, b"\x1bc");
    assert!(
        !t.mode().contains(TermMode::ALT_SCREEN),
        "RIS should exit alt screen",
    );
}

/// Ghostty: fullReset resets palette to defaults.
#[test]
fn esc_c_ris_resets_palette() {
    use vte::ansi::Color;

    let mut t = term();
    let default_color_1 = t.palette().resolve(Color::Indexed(1));
    // Modify palette color 1.
    feed(&mut t, b"\x1b]4;1;rgb:00/ff/00\x07"); // Set color 1 to green
    let modified = t.palette().resolve(Color::Indexed(1));
    assert_ne!(modified, default_color_1, "Color 1 should have changed");
    // RIS.
    feed(&mut t, b"\x1bc");
    let reset = t.palette().resolve(Color::Indexed(1));
    assert_eq!(
        reset, default_color_1,
        "RIS should reset palette (was {modified:?}, now {reset:?})",
    );
}

/// Ghostty: reverseIndex from middle just moves cursor up, no scroll.
#[test]
fn reverse_index_from_middle_moves_up() {
    let mut t = term();
    feed(&mut t, b"A\r\n"); // Line 0: "A"
    feed(&mut t, b"B\r\n"); // Line 1: "B"
    feed(&mut t, b"C"); // Line 2: "C"
    // RI from line 2 should move to line 1 (no scroll).
    feed(&mut t, b"\x1bM");
    assert_eq!(t.grid().cursor().line(), 1);
    feed(&mut t, b"D");
    // Line 1 should now have "D" at col 1 (after "B").
    let ch = t.grid()[crate::index::Line(1)][Column(1)].ch;
    assert_eq!(ch, 'D');
    // Line 0 should still have "A".
    let ch0 = t.grid()[crate::index::Line(0)][Column(0)].ch;
    assert_eq!(ch0, 'A');
}

/// Ghostty: reverseIndex at top of scroll region scrolls within region.
#[test]
fn reverse_index_top_of_scroll_region() {
    let mut t = Term::new(10, 2, 0, Theme::default(), crate::effect::VoidEffectSink);
    // Set up content.
    feed(&mut t, b"\x1b[2;1H"); // Move to line 1 (0-based)
    feed(&mut t, b"A\r\n");
    feed(&mut t, b"B\r\n");
    feed(&mut t, b"C\r\n");
    feed(&mut t, b"D\r\n");
    // Set scroll region to lines 2-5 (1-based).
    feed(&mut t, b"\x1b[2;5r");
    // Move to line 2 (top of scroll region, 1-based).
    feed(&mut t, b"\x1b[2;1H");
    // RI at top of scroll region should scroll content down within region.
    feed(&mut t, b"\x1bM");
    feed(&mut t, b"X");
    // Line 1 (0-based) should now have "X" (new blank line with our char).
    let ch = t.grid()[crate::index::Line(1)][Column(0)].ch;
    assert_eq!(
        ch, 'X',
        "RI at top of scroll region should scroll within region"
    );
    // "A" should have shifted to line 2.
    let ch2 = t.grid()[crate::index::Line(2)][Column(0)].ch;
    assert_eq!(ch2, 'A', "Content should shift down within scroll region");
}

/// Ghostty: reverseIndex outside scroll region just moves cursor up.
#[test]
fn reverse_index_outside_scroll_region() {
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, b"A\r\n");
    feed(&mut t, b"B\r\n");
    feed(&mut t, b"C");
    // Set scroll region to lines 2-3 (1-based).
    feed(&mut t, b"\x1b[2;3r");
    // Move to line 1 (1-based, outside/above the scroll region).
    feed(&mut t, b"\x1b[1;1H");
    // RI should just move cursor up (no scroll because we're outside region).
    // At line 0, RI can't move up further — cursor stays at line 0.
    feed(&mut t, b"\x1bM");
    assert_eq!(t.grid().cursor().line(), 0);
    // Content should be unchanged.
    let ch = t.grid()[crate::index::Line(0)][Column(0)].ch;
    assert_eq!(ch, 'A', "Content above scroll region should not scroll");
    let ch1 = t.grid()[crate::index::Line(1)][Column(0)].ch;
    assert_eq!(ch1, 'B');
}

/// Ghostty: fullReset clears hyperlink on cursor template.
#[test]
fn esc_c_ris_clears_hyperlink() {
    let mut t = term();
    // Set a hyperlink.
    feed(&mut t, b"\x1b]8;;https://example.com\x07");
    let has_link = t
        .grid()
        .cursor()
        .template
        .extra
        .as_ref()
        .and_then(|e| e.hyperlink.as_ref())
        .is_some();
    assert!(has_link, "Hyperlink should be set before RIS");
    // RIS.
    feed(&mut t, b"\x1bc");
    let has_link = t
        .grid()
        .cursor()
        .template
        .extra
        .as_ref()
        .and_then(|e| e.hyperlink.as_ref())
        .is_some();
    assert!(!has_link, "RIS should clear hyperlink on cursor template");
}

/// Ghostty: fullReset resets origin mode.
#[test]
fn esc_c_ris_resets_origin_mode() {
    let mut t = term();
    // Set origin mode.
    feed(&mut t, b"\x1b[?6h"); // DECSET 6: origin mode
    assert!(t.mode().contains(TermMode::ORIGIN));
    // Move cursor.
    feed(&mut t, b"\x1b[3;5H");
    // RIS.
    feed(&mut t, b"\x1bc");
    assert!(
        !t.mode().contains(TermMode::ORIGIN),
        "RIS should reset origin mode",
    );
    assert_eq!(
        t.grid().cursor().line(),
        0,
        "RIS should reset cursor to origin"
    );
    assert_eq!(t.grid().cursor().col().0, 0);
}

/// Ghostty: saveCursor doesn't modify hyperlink state.
#[test]
fn esc7_esc8_preserves_hyperlink() {
    let mut t = term();
    // Set a hyperlink.
    feed(&mut t, b"\x1b]8;;https://example.com\x07");
    let get_link = |t: &Term<crate::effect::VoidEffectSink>| {
        t.grid()
            .cursor()
            .template
            .extra
            .as_ref()
            .and_then(|e| e.hyperlink.clone())
    };
    let before = get_link(&t);
    assert!(before.is_some());
    // Save.
    feed(&mut t, b"\x1b7");
    assert_eq!(get_link(&t), before, "Save should not clear hyperlink");
    // Restore.
    feed(&mut t, b"\x1b8");
    assert_eq!(
        get_link(&t),
        before,
        "DECSC/DECRC should not modify hyperlink state",
    );
}

/// Ghostty: DEC special charset only maps ASCII-range characters.
#[test]
fn dec_special_charset_ignores_non_ascii() {
    let mut t = term();
    // Switch G0 to DEC special graphics.
    feed(&mut t, b"\x1b(0");
    // Print backtick (should map to ◆ in DEC special).
    feed(&mut t, b"`");
    let ch0 = t.grid()[crate::index::Line(0)][Column(0)].ch;
    assert_eq!(ch0, '◆');
    // Non-ASCII characters should pass through the VTE parser as-is.
    // The charset mapping only applies to single-byte ASCII-range chars.
    // Multi-byte UTF-8 chars won't match the DEC mapping table.
    feed(&mut t, b"\xc3\xa9"); // 'é' (U+00E9)
    let ch1 = t.grid()[crate::index::Line(0)][Column(1)].ch;
    assert_eq!(
        ch1, 'é',
        "Non-ASCII should not be affected by DEC special charset"
    );
}

// --- DECALN (ESC # 8) ---

#[test]
fn decaln_fills_screen_with_e() {
    let mut t = term();
    feed(&mut t, b"Hello World");
    // Send DECALN.
    feed(&mut t, b"\x1b#8");

    let grid = t.grid();
    for line in 0..grid.lines() {
        for col in 0..grid.cols() {
            assert_eq!(
                grid[crate::index::Line(line as i32)][Column(col)].ch,
                'E',
                "cell at ({line}, {col}) should be 'E'"
            );
        }
    }
}

#[test]
fn decaln_resets_scroll_region() {
    let mut t = term();
    // Set a non-default scroll region (lines 5-10).
    feed(&mut t, b"\x1b[5;10r");
    assert_ne!(t.grid().scroll_region(), &(0..24));

    // DECALN should reset scroll region to full screen.
    feed(&mut t, b"\x1b#8");
    assert_eq!(t.grid().scroll_region(), &(0..24));
}

#[test]
fn decaln_homes_cursor() {
    let mut t = term();
    // Move cursor to line 10, col 40.
    feed(&mut t, b"\x1b[11;41H");
    assert_eq!(t.grid().cursor().line(), 10);
    assert_eq!(t.grid().cursor().col(), Column(40));

    // DECALN should home the cursor.
    feed(&mut t, b"\x1b#8");
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn decaln_clears_cell_attributes() {
    let mut t = term();
    // Set bold and write some text.
    feed(&mut t, b"\x1b[1mBold\x1b[0m");
    // Verify bold flag is set.
    assert!(
        t.grid()[crate::index::Line(0)][Column(0)]
            .flags
            .contains(crate::cell::CellFlags::BOLD)
    );

    // DECALN should reset all attributes.
    feed(&mut t, b"\x1b#8");
    assert!(
        !t.grid()[crate::index::Line(0)][Column(0)]
            .flags
            .contains(crate::cell::CellFlags::BOLD)
    );
}

// --- RIS / DECSTR paired keyboard-mode-snapshot invariant (BUG-08-012) ---

/// Regression: BUG-08-012 — RIS fired mid-command must seed BOTH paired
/// snapshots to `Some(VecDeque::new())` (not `None`) so a child that later
/// pushes kitty modes and crashes can still have those pushes cleaned at
/// the next OSC 133 ; A / ; D.
/// See: bug-tracker/plans/completed/BUG-08-012/00-overview.md
#[test]
fn keyboard_mode_stack_ris_during_command_sets_saved_to_empty_snapshot() {
    use std::collections::VecDeque;
    use vte::ansi::KeyboardModes;

    let mut t = term();
    // Simulate OSC 133 ; C command-boundary snapshot.
    t.snapshot_keyboard_mode_stack();
    // Child pushes a kitty mode.
    feed(&mut t, b"\x1b[>1u");
    assert!(!t.keyboard_mode_stack().is_empty());

    // RIS fires mid-command.
    feed(&mut t, b"\x1bc");

    assert!(t.keyboard_mode_stack().is_empty());
    assert!(t.inactive_keyboard_mode_stack().is_empty());
    assert_eq!(
        t.pre_command_kb_stack_snapshot(),
        Some(&VecDeque::<KeyboardModes>::new()),
        "RIS must set pre-command snapshot to Some(empty), not None"
    );
    assert_eq!(
        t.inactive_pre_command_kb_stack_snapshot(),
        Some(&VecDeque::<KeyboardModes>::new()),
        "RIS must set inactive-pre-command snapshot to Some(empty), not None"
    );
}

/// Regression: BUG-08-012 — DECSTR variant of RIS snapshot seeding.
#[test]
fn keyboard_mode_stack_decstr_during_command_sets_saved_to_empty_snapshot() {
    use std::collections::VecDeque;
    use vte::ansi::KeyboardModes;

    let mut t = term();
    t.snapshot_keyboard_mode_stack();
    feed(&mut t, b"\x1b[>1u");
    assert!(!t.keyboard_mode_stack().is_empty());

    // DECSTR (CSI ! p).
    feed(&mut t, b"\x1b[!p");

    assert!(t.keyboard_mode_stack().is_empty());
    assert!(t.inactive_keyboard_mode_stack().is_empty());
    assert_eq!(
        t.pre_command_kb_stack_snapshot(),
        Some(&VecDeque::<KeyboardModes>::new())
    );
    assert_eq!(
        t.inactive_pre_command_kb_stack_snapshot(),
        Some(&VecDeque::<KeyboardModes>::new())
    );
}

/// Regression: BUG-08-012 — RIS mid-command followed by child pushes then
/// restore must clean the post-RIS child pushes. `Some(empty)` snapshot
/// enables this; `None` would leave post-reset pushes live.
#[test]
fn keyboard_mode_stack_ris_mid_command_then_child_push_then_a_cleans_pushes() {
    let mut t = term();
    t.snapshot_keyboard_mode_stack();
    feed(&mut t, b"\x1bc"); // RIS.
    // Child (post-reset) pushes kitty modes and then crashes.
    feed(&mut t, b"\x1b[>1u");
    feed(&mut t, b"\x1b[>3u");
    assert_eq!(t.keyboard_mode_stack().len(), 2);

    // Restore (next OSC 133 ; A / ; D).
    t.restore_keyboard_mode_stack();

    assert!(
        t.keyboard_mode_stack().is_empty(),
        "RIS-seeded Some(empty) snapshot must clean post-RIS child pushes"
    );
    assert!(
        !t.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
        "KITTY bits must be cleared after restore to empty snapshot"
    );
}

/// Regression: BUG-08-012 — DECSTR variant of the mid-command cleanup test.
#[test]
fn keyboard_mode_stack_decstr_mid_command_then_child_push_then_a_cleans_pushes() {
    let mut t = term();
    t.snapshot_keyboard_mode_stack();
    feed(&mut t, b"\x1b[!p"); // DECSTR.
    feed(&mut t, b"\x1b[>1u");
    feed(&mut t, b"\x1b[>3u");
    assert_eq!(t.keyboard_mode_stack().len(), 2);

    t.restore_keyboard_mode_stack();

    assert!(t.keyboard_mode_stack().is_empty());
    assert!(!t.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

/// Regression: BUG-08-017. DECALN (`ESC # 8`) fills every visible cell
/// with 'E'. Those cells are application-written and MUST carry
/// `CellFlags::DRAWN` so DECRQCRA sees them as drawn. Before the fix,
/// DECALN called `cell.reset(&default_template)` then `cell.ch = 'E'`
/// without setting DRAWN, so a post-DECALN DECRQCRA skipped every 'E'
/// and returned 0 instead of the 'E'-sum.
#[test]
fn decaln_sets_drawn_on_every_filled_cell() {
    let mut t = term();
    feed(&mut t, b"\x1b#8");

    let grid = t.grid();
    for line in 0..grid.lines() {
        for col in 0..grid.cols() {
            assert!(
                grid[crate::index::Line(line as i32)][Column(col)]
                    .flags
                    .contains(crate::cell::CellFlags::DRAWN),
                "DECALN-filled cell at ({line}, {col}) must carry DRAWN"
            );
        }
    }
}
