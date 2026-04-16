use crate::index::Column;
use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_recorder};

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- Print (input) tests ---

#[test]
fn hello_places_cells_and_advances_cursor() {
    let mut t = term();
    feed(&mut t, b"hello");

    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'h');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'e');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'l');
    assert_eq!(grid[crate::index::Line(0)][Column(3)].ch, 'l');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, 'o');
    assert_eq!(grid.cursor().col(), Column(5));
    assert_eq!(grid.cursor().line(), 0);
}

#[test]
fn hello_newline_world() {
    let mut t = term();
    feed(&mut t, b"hello\nworld");

    let grid = t.grid();
    // "hello" on line 0.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'h');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, 'o');
    // LF only moves down, column stays at 5. "world" starts at col 5 on line 1.
    assert_eq!(grid[crate::index::Line(1)][Column(5)].ch, 'w');
    assert_eq!(grid[crate::index::Line(1)][Column(9)].ch, 'd');
    assert_eq!(grid.cursor().line(), 1);
    assert_eq!(grid.cursor().col(), Column(10));
}

#[test]
fn carriage_return_overwrites() {
    let mut t = term();
    feed(&mut t, b"hello\rworld");

    let grid = t.grid();
    // "world" overwrites "hello" on line 0.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'w');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'o');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'r');
    assert_eq!(grid[crate::index::Line(0)][Column(3)].ch, 'l');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, 'd');
    assert_eq!(grid.cursor().col(), Column(5));
}

#[test]
fn tab_advances_to_column_8() {
    let mut t = term();
    feed(&mut t, b"\t");

    // Tab stops are at 0, 8, 16, ... — from col 0, next stop is col 8.
    assert_eq!(t.grid().cursor().col(), Column(8));
}

#[test]
fn tab_from_midline() {
    let mut t = term();
    feed(&mut t, b"ab\t");

    // From col 2, next tab stop is col 8.
    assert_eq!(t.grid().cursor().col(), Column(8));
}

#[test]
fn backspace_moves_left() {
    let mut t = term();
    feed(&mut t, b"abc\x08");

    // "abc" puts cursor at col 3; backspace moves to col 2.
    assert_eq!(t.grid().cursor().col(), Column(2));
}

#[test]
fn backspace_at_col_zero_is_noop() {
    let mut t = term();
    feed(&mut t, b"\x08");

    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn bell_triggers_event() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x07");

    let events = listener.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], "Bell");
}

#[test]
fn linefeed_moves_down() {
    let mut t = term();
    feed(&mut t, b"A\n");

    let grid = t.grid();
    assert_eq!(grid.cursor().line(), 1);
    // LF does not change column (unlike CR+LF).
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn vertical_tab_same_as_lf() {
    let mut t = term();
    feed(&mut t, b"A\x0B");

    // VT (0x0B) is treated identically to LF.
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(1));
}

#[test]
fn form_feed_same_as_lf() {
    let mut t = term();
    feed(&mut t, b"A\x0C");

    // FF (0x0C) is treated identically to LF.
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(1));
}

#[test]
fn so_activates_g1_charset() {
    let mut t = term();
    // SO = 0x0E activates G1.
    feed(&mut t, b"\x0E");

    assert_eq!(*t.charset().active(), vte::ansi::CharsetIndex::G1);
}

#[test]
fn si_activates_g0_charset() {
    let mut t = term();
    // SO then SI should restore G0.
    feed(&mut t, b"\x0E\x0F");

    assert_eq!(*t.charset().active(), vte::ansi::CharsetIndex::G0);
}

#[test]
fn crlf_moves_to_start_of_next_line() {
    let mut t = term();
    feed(&mut t, b"hello\r\n");

    let grid = t.grid();
    assert_eq!(grid.cursor().line(), 1);
    assert_eq!(grid.cursor().col(), Column(0));
}

#[test]
fn multiple_linefeeds() {
    let mut t = term();
    feed(&mut t, b"\n\n\n");

    assert_eq!(t.grid().cursor().line(), 3);
}

#[test]
fn substitute_writes_space() {
    let mut t = term();
    feed(&mut t, b"A\x1AB");

    let grid = t.grid();
    // SUB (0x1A) writes a space.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'B');
}

// --- CSI cursor movement tests ---

#[test]
fn cuu_moves_cursor_up_5() {
    let mut t = term();
    // Move cursor to line 10, then CUU 5.
    feed(&mut t, b"\x1b[11;1H"); // CUP to line 10 (1-based)
    feed(&mut t, b"\x1b[5A"); // CUU 5

    assert_eq!(t.grid().cursor().line(), 5);
}

#[test]
fn cup_moves_cursor_to_line_9_col_19() {
    let mut t = term();
    // CSI 10;20 H — CUP to row 10, column 20 (1-based → 0-based: 9, 19).
    feed(&mut t, b"\x1b[10;20H");

    assert_eq!(t.grid().cursor().line(), 9);
    assert_eq!(t.grid().cursor().col(), Column(19));
}

// --- CSI erase tests ---

#[test]
fn ed_clears_screen() {
    let mut t = term();
    feed(&mut t, b"ABCDE\r\nFGHIJ\r\nKLMNO");
    // CSI 2 J — erase entire display.
    feed(&mut t, b"\x1b[2J");

    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, ' ');
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, ' ');
    assert_eq!(grid[crate::index::Line(2)][Column(0)].ch, ' ');
}

#[test]
fn el_clears_to_end_of_line() {
    let mut t = term();
    feed(&mut t, b"ABCDE");
    // Move cursor to column 2, then EL 0 (clear to right).
    feed(&mut t, b"\x1b[3G"); // CHA column 3 (1-based) → col 2
    feed(&mut t, b"\x1b[K"); // EL (default = right)

    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'B');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(3)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, ' ');
}

// --- CSI insert / delete tests ---

#[test]
fn ich_inserts_5_blanks() {
    let mut t = term();
    feed(&mut t, b"ABCDE");
    // Move cursor to column 1, then ICH 5.
    feed(&mut t, b"\x1b[2G"); // CHA column 2 (1-based) → col 1
    feed(&mut t, b"\x1b[5@"); // ICH 5

    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    // 5 blanks inserted at col 1.
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(5)].ch, ' ');
    // 'B' shifted to col 6.
    assert_eq!(grid[crate::index::Line(0)][Column(6)].ch, 'B');
}

#[test]
fn dch_deletes_3_chars() {
    let mut t = term();
    feed(&mut t, b"ABCDEFGH");
    // Move cursor to column 2, then DCH 3.
    feed(&mut t, b"\x1b[3G"); // CHA col 3 (1-based) → col 2
    feed(&mut t, b"\x1b[3P"); // DCH 3

    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'B');
    // C, D, E deleted; F shifts to col 2.
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'F');
    assert_eq!(grid[crate::index::Line(0)][Column(3)].ch, 'G');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, 'H');
}

#[test]
fn il_inserts_2_lines() {
    let mut t = term();
    feed(&mut t, b"AAA\r\nBBB\r\nCCC\r\nDDD");
    // Move cursor to line 1 (0-based), then IL 2.
    feed(&mut t, b"\x1b[2;1H"); // CUP row 2 (1-based) → line 1
    feed(&mut t, b"\x1b[2L"); // IL 2

    let grid = t.grid();
    // Line 0: AAA (untouched).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    // Lines 1–2: blank (inserted).
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, ' ');
    assert_eq!(grid[crate::index::Line(2)][Column(0)].ch, ' ');
    // Line 3: BBB (pushed down from line 1).
    assert_eq!(grid[crate::index::Line(3)][Column(0)].ch, 'B');
}

#[test]
fn dl_deletes_3_lines() {
    let mut t = term();
    feed(&mut t, b"AAA\r\nBBB\r\nCCC\r\nDDD\r\nEEE");
    // Move cursor to line 1, then DL 3.
    feed(&mut t, b"\x1b[2;1H"); // CUP row 2 → line 1
    feed(&mut t, b"\x1b[3M"); // DL 3

    let grid = t.grid();
    // Line 0: AAA (untouched).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    // Lines 1–3 deleted, EEE moved from line 4 to line 1.
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'E');
    // Line 2 now blank.
    assert_eq!(grid[crate::index::Line(2)][Column(0)].ch, ' ');
}

// --- CSI mode tests ---

#[test]
fn dectcem_hides_cursor() {
    let mut t = term();
    // CSI ? 25 l — hide cursor.
    feed(&mut t, b"\x1b[?25l");

    assert!(!t.mode().contains(TermMode::SHOW_CURSOR));
}

#[test]
fn dectcem_shows_cursor() {
    let mut t = term();
    // First hide, then show.
    feed(&mut t, b"\x1b[?25l");
    feed(&mut t, b"\x1b[?25h");

    assert!(t.mode().contains(TermMode::SHOW_CURSOR));
}

#[test]
fn decset_alt_screen_switches_to_alt() {
    let mut t = term();
    feed(&mut t, b"hello"); // Write on primary.
    // CSI ? 1049 h — switch to alt screen.
    feed(&mut t, b"\x1b[?1049h");

    assert!(t.mode().contains(TermMode::ALT_SCREEN));
    // Alt screen should be clear.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, ' ');
}

#[test]
fn decrst_alt_screen_switches_back() {
    let mut t = term();
    feed(&mut t, b"hello");
    feed(&mut t, b"\x1b[?1049h"); // Enter alt.
    feed(&mut t, b"alt");
    feed(&mut t, b"\x1b[?1049l"); // Leave alt.

    assert!(!t.mode().contains(TermMode::ALT_SCREEN));
    // Primary screen content restored.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'h');
}

// --- ORIGIN mode tests ---

#[test]
fn origin_mode_cup_relative_to_scroll_region() {
    let mut t = term();
    // Set scroll region rows 5–15 (1-based), enable ORIGIN mode.
    feed(&mut t, b"\x1b[5;15r"); // DECSTBM
    feed(&mut t, b"\x1b[?6h"); // DECSET ORIGIN

    // CUP(1,1) in ORIGIN mode → absolute line 4 (region.start), col 0.
    feed(&mut t, b"\x1b[1;1H");
    assert_eq!(t.grid().cursor().line(), 4);
    assert_eq!(t.grid().cursor().col(), Column(0));

    // CUP(3,5) → absolute line 6, col 4.
    feed(&mut t, b"\x1b[3;5H");
    assert_eq!(t.grid().cursor().line(), 6);
    assert_eq!(t.grid().cursor().col(), Column(4));
}

#[test]
fn origin_mode_cup_clamps_to_scroll_region() {
    let mut t = term();
    // Scroll region rows 5–10 (1-based → lines 4..10).
    feed(&mut t, b"\x1b[5;10r");
    feed(&mut t, b"\x1b[?6h");

    // CUP(99,1) should clamp to bottom of region (line 9).
    feed(&mut t, b"\x1b[99;1H");
    assert_eq!(t.grid().cursor().line(), 9);
}

#[test]
fn origin_mode_vpa_relative_to_scroll_region() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;15r"); // DECSTBM 5–15
    feed(&mut t, b"\x1b[?6h"); // ORIGIN mode
    feed(&mut t, b"\x1b[1;10H"); // Start at col 9

    // VPA(2) in ORIGIN mode → absolute line 5 (region.start + 1).
    feed(&mut t, b"\x1b[2d");
    assert_eq!(t.grid().cursor().line(), 5);
    // Column preserved.
    assert_eq!(t.grid().cursor().col(), Column(9));
}

#[test]
fn origin_mode_disabled_cup_uses_full_screen() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;15r"); // DECSTBM
    feed(&mut t, b"\x1b[?6h"); // Enable ORIGIN
    feed(&mut t, b"\x1b[?6l"); // Disable ORIGIN

    // CUP(1,1) without ORIGIN → absolute line 0, col 0.
    feed(&mut t, b"\x1b[1;1H");
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- IRM (Insert/Replace Mode) tests ---

#[test]
fn irm_insert_mode_shifts_content_right() {
    let mut t = term();
    feed(&mut t, b"foo");
    feed(&mut t, b"\x1b[1;1H"); // CUP to origin
    feed(&mut t, b"\x1b[4h"); // SM: set IRM (Insert mode)
    feed(&mut t, b"BAR");

    let grid = t.grid();
    // "BAR" inserted before "foo", shifting it right.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'B');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'R');
    assert_eq!(grid[crate::index::Line(0)][Column(3)].ch, 'f');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, 'o');
    assert_eq!(grid[crate::index::Line(0)][Column(5)].ch, 'o');
}

#[test]
fn irm_replace_mode_overwrites() {
    let mut t = term();
    feed(&mut t, b"foo");
    feed(&mut t, b"\x1b[1;1H"); // CUP to origin
    feed(&mut t, b"\x1b[4h"); // SM: set IRM
    feed(&mut t, b"\x1b[4l"); // RM: reset IRM (back to replace)
    feed(&mut t, b"BAR");

    let grid = t.grid();
    // "BAR" overwrites "foo".
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'B');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'R');
    assert_eq!(grid.cursor().col(), Column(3));
}

// --- LNM (Line Feed / New Line Mode) tests ---

#[test]
fn lnm_mode_lf_acts_as_crlf() {
    let mut t = term();
    feed(&mut t, b"\x1b[20h"); // SM: set LNM
    feed(&mut t, b"hello\n"); // LF should also perform CR

    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn lnm_mode_off_lf_preserves_column() {
    let mut t = term();
    feed(&mut t, b"\x1b[20h"); // Enable LNM
    feed(&mut t, b"\x1b[20l"); // Disable LNM
    feed(&mut t, b"hello\n");

    assert_eq!(t.grid().cursor().line(), 1);
    // Column stays at 5 (normal LF behavior).
    assert_eq!(t.grid().cursor().col(), Column(5));
}

// --- CHA edge case tests ---

#[test]
fn cha_default_param_goes_to_column_0() {
    let mut t = term();
    feed(&mut t, b"hello");
    // CSI G — default param is 1 (1-based → col 0).
    feed(&mut t, b"\x1b[G");

    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn cha_overflow_clamps_to_last_column() {
    let mut t = term();
    // CSI 999 G — should clamp to col 79 on an 80-column terminal.
    feed(&mut t, b"\x1b[999G");

    assert_eq!(t.grid().cursor().col(), Column(79));
}

// --- CNL / CPL tests ---

#[test]
fn cnl_moves_down_and_to_column_0() {
    let mut t = term();
    feed(&mut t, b"hello");
    // CSI 3 E — move down 3 lines, column 0.
    feed(&mut t, b"\x1b[3E");

    assert_eq!(t.grid().cursor().line(), 3);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn cpl_moves_up_and_to_column_0() {
    let mut t = term();
    feed(&mut t, b"\x1b[10;15H"); // CUP to line 9, col 14
    // CSI 3 F — move up 3 lines, column 0.
    feed(&mut t, b"\x1b[3F");

    assert_eq!(t.grid().cursor().line(), 6);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- ECH edge case tests ---

#[test]
fn ech_overflow_clamps_to_line_end() {
    let mut t = term();
    feed(&mut t, b"ABCDE");
    feed(&mut t, b"\x1b[2G"); // CHA col 2 → col 1
    // ECH 999 — should erase from col 1 to end of line.
    feed(&mut t, b"\x1b[999X");

    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, ' ');
}

// --- Scroll up/down through VTE bytes ---

#[test]
fn su_scrolls_content_up() {
    let mut t = term();
    feed(&mut t, b"AAA\r\nBBB\r\nCCC");
    // CSI 1 S — scroll up 1.
    feed(&mut t, b"\x1b[1S");

    let grid = t.grid();
    // Line 0 now has BBB (was line 1).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'B');
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'C');
}

#[test]
fn sd_scrolls_content_down() {
    let mut t = term();
    feed(&mut t, b"AAA\r\nBBB\r\nCCC");
    // CSI 1 T — scroll down 1.
    feed(&mut t, b"\x1b[1T");

    let grid = t.grid();
    // Line 0 is blank (new).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, ' ');
    // AAA moved from line 0 to line 1.
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(2)][Column(0)].ch, 'B');
}

// --- RI (Reverse Index) through VTE bytes ---

#[test]
fn ri_at_top_of_scroll_region_scrolls_down() {
    let mut t = term();
    feed(&mut t, b"AAA\r\nBBB\r\nCCC");
    feed(&mut t, b"\x1b[1;1H"); // CUP to origin (top of region)
    // ESC M — reverse index.
    feed(&mut t, b"\x1bM");

    let grid = t.grid();
    // Line 0 is blank (scrolled down).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, ' ');
    // AAA pushed to line 1.
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(2)][Column(0)].ch, 'B');
}

#[test]
fn ri_in_middle_moves_cursor_up() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;1H"); // CUP to line 4
    // ESC M — reverse index (not at region top → just moves up).
    feed(&mut t, b"\x1bM");

    assert_eq!(t.grid().cursor().line(), 3);
}

// --- Tab CSI tests ---

#[test]
fn cht_forward_tab_by_count() {
    let mut t = term();
    // CSI 2 I — forward tab 2 times (from col 0 → 8 → 16).
    feed(&mut t, b"\x1b[2I");

    assert_eq!(t.grid().cursor().col(), Column(16));
}

#[test]
fn cbt_backward_tab_by_count() {
    let mut t = term();
    feed(&mut t, b"\x1b[20G"); // CHA col 20 → col 19
    // CSI 2 Z — backward tab 2 times (col 19 → 16 → 8).
    feed(&mut t, b"\x1b[2Z");

    assert_eq!(t.grid().cursor().col(), Column(8));
}

#[test]
fn tbc_clears_all_tab_stops() {
    let mut t = term();
    // CSI 3 g — clear all tab stops.
    feed(&mut t, b"\x1b[3g");
    // Now tab from col 0 should go to last column (no stops).
    feed(&mut t, b"\t");

    assert_eq!(t.grid().cursor().col(), Column(79));
}

// --- NEL (Next Line) test ---

#[test]
fn nel_performs_cr_and_lf() {
    let mut t = term();
    feed(&mut t, b"hello");
    // ESC E — NEL: next line (CR + LF).
    feed(&mut t, b"\x1bE");

    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- DECSCUSR (cursor shape) tests ---

#[test]
fn decscusr_1_sets_blinking_block() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1 q");

    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Block);
    assert!(
        t.mode().contains(TermMode::CURSOR_BLINKING),
        "CSI 1 q should enable blinking"
    );
}

#[test]
fn decscusr_2_sets_steady_block() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[2 q");

    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Block);
    assert!(
        !t.mode().contains(TermMode::CURSOR_BLINKING),
        "CSI 2 q should disable blinking"
    );
}

#[test]
fn decscusr_5_sets_blinking_bar() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[5 q");

    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Bar);
    assert!(
        t.mode().contains(TermMode::CURSOR_BLINKING),
        "CSI 5 q should enable blinking"
    );
}

#[test]
fn decscusr_6_sets_steady_bar() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[6 q");

    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Bar);
    assert!(
        !t.mode().contains(TermMode::CURSOR_BLINKING),
        "CSI 6 q should disable blinking"
    );
}

#[test]
fn decscusr_3_sets_blinking_underline() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[3 q");

    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Underline);
    assert!(t.mode().contains(TermMode::CURSOR_BLINKING));
}

#[test]
fn decscusr_4_sets_steady_underline() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[4 q");

    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Underline);
    assert!(!t.mode().contains(TermMode::CURSOR_BLINKING));
}

#[test]
fn decscusr_0_resets_to_default() {
    let (mut t, _listener) = term_with_recorder();
    // Set to bar first.
    feed(&mut t, b"\x1b[5 q");
    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Bar);

    // Reset.
    feed(&mut t, b"\x1b[0 q");
    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Block);
}

#[test]
fn decscusr_fires_cursor_blinking_change_event() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[5 q");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("CursorBlinkingChange")),
        "DECSCUSR should fire CursorBlinkingChange event"
    );
}

// --- Unhandled sequences ---

#[test]
fn unknown_csi_does_not_panic() {
    let mut t = term();
    // Random unknown CSI.
    feed(&mut t, b"\x1b[999z");
    // Should not panic — grid still functional.
    feed(&mut t, b"ok");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'o');
}

#[test]
fn unknown_osc_does_not_panic() {
    let mut t = term();
    // Unknown OSC number.
    feed(&mut t, b"\x1b]9999;data\x07");
    feed(&mut t, b"ok");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'o');
}

#[test]
fn unknown_esc_does_not_panic() {
    let mut t = term();
    // Unknown ESC final.
    feed(&mut t, b"\x1bZ");
    feed(&mut t, b"ok");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'o');
}

#[test]
fn ris_clears_keyboard_mode_stack_and_flags() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[>3u");
    assert!(!t.keyboard_mode_stack().is_empty());

    // RIS.
    feed(&mut t, b"\x1bc");
    assert!(t.keyboard_mode_stack().is_empty());
    assert!(!t.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

#[test]
fn ris_resets_cursor_shape() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[5 q");
    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Bar);

    feed(&mut t, b"\x1bc");
    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Block);
}

#[test]
fn query_keyboard_mode_empty_stack_reports_zero() {
    let (mut t, listener) = term_with_recorder();
    // Query with nothing on the stack.
    feed(&mut t, b"\x1b[?u");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("[?0u")),
        "empty stack should report mode 0: {events:?}"
    );
}

#[test]
fn query_keyboard_mode_reports_bitmask() {
    let (mut t, listener) = term_with_recorder();
    // Mode 3 = DISAMBIGUATE_ESC_CODES (1) | REPORT_EVENT_TYPES (2).
    feed(&mut t, b"\x1b[>3u");
    feed(&mut t, b"\x1b[?u");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("[?3u")),
        "should report combined bitmask 3: {events:?}"
    );
}

#[test]
fn pop_more_than_stack_depth_clamps() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[>1u");
    feed(&mut t, b"\x1b[>3u");
    assert_eq!(t.keyboard_mode_stack().len(), 2);

    // Pop 999 from a stack of 2 — should clamp to empty.
    feed(&mut t, b"\x1b[<999u");
    assert!(t.keyboard_mode_stack().is_empty());
    assert!(!t.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL));
}

#[test]
fn keyboard_mode_stack_survives_alt_screen_swap() {
    let (mut t, _listener) = term_with_recorder();
    // Push mode on primary screen.
    feed(&mut t, b"\x1b[>1u");
    assert_eq!(t.keyboard_mode_stack().len(), 1);

    // Switch to alt screen — primary stack is swapped out.
    feed(&mut t, b"\x1b[?1049h");
    assert!(
        t.keyboard_mode_stack().is_empty(),
        "alt screen should have its own empty keyboard mode stack"
    );

    // Push a different mode on alt screen.
    feed(&mut t, b"\x1b[>3u");
    assert_eq!(t.keyboard_mode_stack().len(), 1);

    // Switch back to primary — original mode should be restored.
    feed(&mut t, b"\x1b[?1049l");
    assert_eq!(t.keyboard_mode_stack().len(), 1);
    assert!(
        t.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
        "primary stack mode should be restored after alt screen exit"
    );
}

#[test]
fn decscusr_set_same_shape_twice_is_idempotent() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[5 q");
    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Bar);
    assert!(t.mode().contains(TermMode::CURSOR_BLINKING));

    // Set the same shape again.
    feed(&mut t, b"\x1b[5 q");
    assert_eq!(t.cursor_shape(), crate::grid::CursorShape::Bar);
    assert!(t.mode().contains(TermMode::CURSOR_BLINKING));
}

#[test]
fn ris_restores_default_cursor_blinking() {
    let (mut t, _listener) = term_with_recorder();
    // Disable blinking, then RIS should restore to default (blinking on).
    feed(&mut t, b"\x1b[2 q");
    assert!(!t.mode().contains(TermMode::CURSOR_BLINKING));

    feed(&mut t, b"\x1bc");
    assert!(
        t.mode().contains(TermMode::CURSOR_BLINKING),
        "RIS should restore default cursor blinking (on)"
    );
}

// --- Zero-width / combining mark tests ---

#[test]
fn combining_mark_appends_to_previous_cell() {
    let mut t = term();
    // 'e' followed by U+0301 (combining acute accent).
    feed(&mut t, "e\u{0301}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'e');
    let zw = cell
        .extra
        .as_ref()
        .expect("should have extra")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0301}']);
    // Cursor stays at col 1 (zero-width doesn't advance).
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn multiple_combining_marks_append_to_same_cell() {
    let mut t = term();
    // 'a' + U+0300 (grave) + U+0301 (acute) + U+0302 (circumflex).
    feed(&mut t, "a\u{0300}\u{0301}\u{0302}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'a');
    let zw = cell
        .extra
        .as_ref()
        .expect("should have extra")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0300}', '\u{0301}', '\u{0302}']);
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn zerowidth_at_col_zero_discarded() {
    let mut t = term();
    // Feed a combining mark at column 0 with no previous cell.
    feed(&mut t, "\u{0301}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    // Cell should remain the default space — combining mark was discarded.
    assert_eq!(cell.ch, ' ');
    assert!(cell.extra.is_none());
    assert_eq!(grid.cursor().col(), Column(0));
}

#[test]
fn combining_mark_on_wide_char() {
    use crate::cell::CellFlags;

    let mut t = term();
    // CJK ideograph '漢' (width 2) + combining acute accent.
    feed(&mut t, "漢\u{0301}".as_bytes());

    let grid = t.grid();
    let base = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(base.ch, '漢');
    assert!(base.flags.contains(CellFlags::WIDE_CHAR));
    let zw = base
        .extra
        .as_ref()
        .expect("combining mark on base cell")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0301}']);

    // Spacer at col 1 must NOT have the combining mark.
    let spacer = &grid[crate::index::Line(0)][Column(1)];
    assert!(spacer.flags.contains(CellFlags::WIDE_CHAR_SPACER));
    assert!(spacer.extra.is_none());

    // Cursor at col 2 (wide char width), unaffected by combining mark.
    assert_eq!(grid.cursor().col(), Column(2));
}

#[test]
fn combining_mark_at_wrap_pending() {
    // 5-column terminal: write "abcde" to fill the line.
    // After 'e', cursor is at col 5 (== cols), i.e. wrap-pending.
    // A combining mark should attach to 'e' at col 4, not trigger a wrap.
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, "abcde\u{0300}".as_bytes());

    let grid = t.grid();
    let cell_e = &grid[crate::index::Line(0)][Column(4)];
    assert_eq!(cell_e.ch, 'e');
    let zw = cell_e
        .extra
        .as_ref()
        .expect("combining mark on wrap-pending cell")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0300}']);

    // Cursor stays wrap-pending at col 5 — combining mark didn't advance it.
    assert_eq!(grid.cursor().col(), Column(5));
    // Still on line 0 — no wrap occurred.
    assert_eq!(grid.cursor().line(), 0);
}

#[test]
fn zerowidth_joiner_at_col_zero_discarded() {
    let mut t = term();
    // U+200D (zero-width joiner) at column 0 with no previous cell.
    feed(&mut t, "\u{200D}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, ' ');
    assert!(cell.extra.is_none());
    assert_eq!(grid.cursor().col(), Column(0));
}

// --- Extended zero-width character tests (from Ghostty/Alacritty reference patterns) ---

#[test]
fn zerowidth_space_appends_to_previous_cell() {
    let mut t = term();
    // 'a' + U+200B (zero-width space).
    feed(&mut t, "a\u{200B}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'a');
    let zw = cell
        .extra
        .as_ref()
        .expect("should have extra")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{200B}']);
    // Cursor stays at col 1.
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn word_joiner_appends_to_previous_cell() {
    let mut t = term();
    // 'b' + U+2060 (word joiner).
    feed(&mut t, "b\u{2060}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'b');
    let zw = cell
        .extra
        .as_ref()
        .expect("should have extra")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{2060}']);
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn variation_selector_15_appends_to_previous_cell() {
    let mut t = term();
    // '☔' (U+2614, umbrella with rain, width 2) + U+FE0E (VS15).
    // VS15 is zero-width; without mode 2027 it's stored as a combining mark.
    feed(&mut t, "\u{2614}\u{FE0E}".as_bytes());

    let grid = t.grid();
    let base = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(base.ch, '\u{2614}');
    let zw = base
        .extra
        .as_ref()
        .expect("VS15 stored")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{FE0E}']);
    // Without mode 2027, width stays at 2.
    assert_eq!(grid.cursor().col(), Column(2));
}

#[test]
fn variation_selector_16_appends_to_previous_cell() {
    let mut t = term();
    // '❤' (U+2764, heavy black heart) + U+FE0F (VS16).
    // VS16 is zero-width; stored as combining mark without mode 2027.
    feed(&mut t, "\u{2764}\u{FE0F}".as_bytes());

    let grid = t.grid();
    let base = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(base.ch, '\u{2764}');
    let zw = base
        .extra
        .as_ref()
        .expect("VS16 stored")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{FE0F}']);
}

#[test]
fn vs16_on_ascii_stored_as_zerowidth() {
    let mut t = term();
    // 'x' + U+FE0F (VS16, invalid for ASCII — silently stored as zerowidth).
    feed(&mut t, "x\u{FE0F}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'x');
    let zw = cell
        .extra
        .as_ref()
        .expect("VS16 stored on ASCII")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{FE0F}']);
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn zjw_appends_to_previous_cell() {
    let mut t = term();
    // 'a' + U+200D (ZWJ).
    feed(&mut t, "a\u{200D}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'a');
    let zw = cell
        .extra
        .as_ref()
        .expect("ZWJ stored")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{200D}']);
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn zjw_emoji_sequence_stores_each_emoji_separately() {
    use crate::cell::CellFlags;

    let mut t = term();
    // 👨‍👩‍👧 = U+1F468 + U+200D + U+1F469 + U+200D + U+1F467
    // Without mode 2027, each emoji is placed as a separate wide char.
    // ZWJ chars get appended as zerowidth to the preceding emoji.
    feed(
        &mut t,
        "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}".as_bytes(),
    );

    let grid = t.grid();
    // 👨 at col 0-1 (wide).
    let man = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(man.ch, '\u{1F468}');
    assert!(man.flags.contains(CellFlags::WIDE_CHAR));
    // ZWJ appended to 👨.
    let zw = man.extra.as_ref().expect("ZWJ on man").zerowidth.as_slice();
    assert_eq!(zw, &['\u{200D}']);

    // Spacer at col 1.
    assert!(
        grid[crate::index::Line(0)][Column(1)]
            .flags
            .contains(CellFlags::WIDE_CHAR_SPACER)
    );

    // 👩 at col 2-3 (wide).
    let woman = &grid[crate::index::Line(0)][Column(2)];
    assert_eq!(woman.ch, '\u{1F469}');
    assert!(woman.flags.contains(CellFlags::WIDE_CHAR));
    // ZWJ appended to 👩.
    let zw = woman
        .extra
        .as_ref()
        .expect("ZWJ on woman")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{200D}']);

    // 👧 at col 4-5 (wide).
    let girl = &grid[crate::index::Line(0)][Column(4)];
    assert_eq!(girl.ch, '\u{1F467}');
    assert!(girl.flags.contains(CellFlags::WIDE_CHAR));

    // Cursor at col 6 (3 wide chars * 2).
    assert_eq!(grid.cursor().col(), Column(6));
}

#[test]
fn vs16_then_combining_mark_both_stored() {
    let mut t = term();
    // 'n' + U+FE0F (VS16) + U+0303 (combining tilde).
    // Both are zero-width and should be stored on 'n'.
    feed(&mut t, "n\u{FE0F}\u{0303}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'n');
    let zw = cell
        .extra
        .as_ref()
        .expect("both stored")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{FE0F}', '\u{0303}']);
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn four_combining_marks_all_stored() {
    let mut t = term();
    // 'o' + 4 combining marks (grave, acute, circumflex, tilde).
    feed(&mut t, "o\u{0300}\u{0301}\u{0302}\u{0303}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'o');
    let zw = cell
        .extra
        .as_ref()
        .expect("4 marks stored")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0300}', '\u{0301}', '\u{0302}', '\u{0303}']);
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn mixed_zerowidth_types_on_same_cell() {
    let mut t = term();
    // 'a' + combining acute + ZWJ + VS16.
    feed(&mut t, "a\u{0301}\u{200D}\u{FE0F}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'a');
    let zw = cell
        .extra
        .as_ref()
        .expect("mixed zw types")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0301}', '\u{200D}', '\u{FE0F}']);
    assert_eq!(grid.cursor().col(), Column(1));
}

#[test]
fn combining_mark_after_line_wrap() {
    // 5-column terminal. Write "ABCDE" to fill line 0, then "F" wraps to line 1.
    // Then a combining mark should attach to 'F' on line 1.
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, "ABCDEF\u{0301}".as_bytes());

    let grid = t.grid();
    // 'F' is on line 1, col 0. Combining mark attaches to it.
    let cell = &grid[crate::index::Line(1)][Column(0)];
    assert_eq!(cell.ch, 'F');
    let zw = cell
        .extra
        .as_ref()
        .expect("combining on wrapped cell")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0301}']);
    assert_eq!(grid.cursor().col(), Column(1));
    assert_eq!(grid.cursor().line(), 1);
}

#[test]
fn wide_char_at_boundary_sets_leading_spacer() {
    use crate::cell::CellFlags;

    // 5-column terminal. Write "ABCD" (fills cols 0-3), then a wide char at col 4
    // can't fit (needs 2 cells, only 1 left). The boundary cell should become
    // LEADING_WIDE_CHAR_SPACER, and the wide char goes to the next line.
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, b"ABCD");
    feed(&mut t, "漢".as_bytes());

    let grid = t.grid();

    // Line 0, col 4: boundary padding (LEADING_WIDE_CHAR_SPACER + WRAP).
    let boundary = &grid[crate::index::Line(0)][Column(4)];
    assert!(
        boundary.flags.contains(CellFlags::LEADING_WIDE_CHAR_SPACER),
        "boundary cell should be LEADING_WIDE_CHAR_SPACER"
    );
    assert!(
        boundary.flags.contains(CellFlags::WRAP),
        "boundary cell should also have WRAP"
    );

    // Line 1, col 0: the wide char.
    let wide = &grid[crate::index::Line(1)][Column(0)];
    assert_eq!(wide.ch, '漢');
    assert!(wide.flags.contains(CellFlags::WIDE_CHAR));
}

#[test]
fn combining_mark_on_wide_char_after_wrap() {
    use crate::cell::CellFlags;

    // 5-column terminal. Write "ABC" (3 cols), then a wide char wraps to next line.
    // Then a combining mark should attach to the wide char base, not the spacer.
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, b"ABCD");
    // Wide char at col 4 can't fit → wraps to line 1.
    feed(&mut t, "漢\u{0301}".as_bytes());

    let grid = t.grid();
    let base = &grid[crate::index::Line(1)][Column(0)];
    assert_eq!(base.ch, '漢');
    assert!(base.flags.contains(CellFlags::WIDE_CHAR));
    let zw = base
        .extra
        .as_ref()
        .expect("combining on wide after wrap")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0301}']);

    // Spacer must not have the combining mark.
    let spacer = &grid[crate::index::Line(1)][Column(1)];
    assert!(spacer.flags.contains(CellFlags::WIDE_CHAR_SPACER));
    assert!(spacer.extra.is_none());
}

#[test]
fn zerowidth_space_at_col_zero_discarded() {
    let mut t = term();
    // U+200B at column 0 with no previous cell.
    feed(&mut t, "\u{200B}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, ' ');
    assert!(cell.extra.is_none());
    assert_eq!(grid.cursor().col(), Column(0));
}

#[test]
fn variation_selector_at_col_zero_discarded() {
    let mut t = term();
    // U+FE0F (VS16) at column 0 — no previous cell to attach to.
    feed(&mut t, "\u{FE0F}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, ' ');
    assert!(cell.extra.is_none());
    assert_eq!(grid.cursor().col(), Column(0));
}

#[test]
fn combining_mark_does_not_trigger_wrap() {
    // 5-column terminal, fill line with "abcde" (wrap pending at col 5).
    // Multiple combining marks should attach to 'e' without wrapping.
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, "abcde\u{0300}\u{0301}\u{0302}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(4)];
    assert_eq!(cell.ch, 'e');
    let zw = cell
        .extra
        .as_ref()
        .expect("3 marks on wrap-pending")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{0300}', '\u{0301}', '\u{0302}']);
    // Still on line 0, cursor at col 5 (wrap pending). No wrap occurred.
    assert_eq!(grid.cursor().line(), 0);
    assert_eq!(grid.cursor().col(), Column(5));
}

#[test]
fn zjw_between_wide_chars_stored_correctly() {
    use crate::cell::CellFlags;

    let mut t = term();
    // Two CJK chars with ZWJ between them: 漢 + ZWJ + 字
    feed(&mut t, "漢\u{200D}字".as_bytes());

    let grid = t.grid();
    // 漢 at col 0 with ZWJ.
    let c1 = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(c1.ch, '漢');
    assert!(c1.flags.contains(CellFlags::WIDE_CHAR));
    let zw = c1
        .extra
        .as_ref()
        .expect("ZWJ between wide chars")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{200D}']);

    // 字 at col 2.
    let c2 = &grid[crate::index::Line(0)][Column(2)];
    assert_eq!(c2.ch, '字');
    assert!(c2.flags.contains(CellFlags::WIDE_CHAR));

    assert_eq!(grid.cursor().col(), Column(4));
}

#[test]
fn emoji_with_vs16_and_combining() {
    let mut t = term();
    // '❤' (U+2764) + VS16 (U+FE0F) + combining enclosing keycap (U+20E3).
    // Both zero-width chars stored on the heart.
    feed(&mut t, "\u{2764}\u{FE0F}\u{20E3}".as_bytes());

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, '\u{2764}');
    let zw = cell
        .extra
        .as_ref()
        .expect("VS16 + combining")
        .zerowidth
        .as_slice();
    assert_eq!(zw, &['\u{FE0F}', '\u{20E3}']);
}

#[test]
fn dirty_tracked_for_combining_mark() {
    let mut t = term();
    // Write 'a', drain dirty, then add combining mark.
    feed(&mut t, b"a");
    t.grid_mut().dirty_mut().drain().for_each(drop);

    // Combining mark should mark line 0 dirty.
    feed(&mut t, "\u{0301}".as_bytes());

    let dirty_lines: Vec<usize> = t.grid_mut().dirty_mut().drain().map(|d| d.line).collect();
    assert!(
        dirty_lines.contains(&0),
        "combining mark should mark line dirty: {dirty_lines:?}"
    );
}

#[test]
fn dirty_tracked_for_zerowidth_space() {
    let mut t = term();
    feed(&mut t, b"x");
    t.grid_mut().dirty_mut().drain().for_each(drop);

    // Zero-width space should mark line 0 dirty.
    feed(&mut t, "\u{200B}".as_bytes());

    let dirty_lines: Vec<usize> = t.grid_mut().dirty_mut().drain().map(|d| d.line).collect();
    assert!(
        dirty_lines.contains(&0),
        "zero-width space should mark line dirty: {dirty_lines:?}"
    );
}

// --- VT handler edge cases (tmux audit) ---

#[test]
fn decstbm_top_greater_than_bottom_is_ignored() {
    let mut t = term();
    // First set a valid region so we can verify it doesn't change.
    feed(&mut t, b"\x1b[5;20r");
    let region_before = t.grid().scroll_region().clone();

    // CSI 10;5r — top > bottom: should be silently ignored.
    feed(&mut t, b"\x1b[10;5r");
    assert_eq!(
        t.grid().scroll_region().clone(),
        region_before,
        "invalid DECSTBM (top > bottom) should be ignored"
    );
}

#[test]
fn decstbm_equal_top_and_bottom_is_ignored() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;20r");
    let region_before = t.grid().scroll_region().clone();

    // CSI 10;10r — top == bottom (single line): should be ignored.
    feed(&mut t, b"\x1b[10;10r");
    assert_eq!(
        t.grid().scroll_region().clone(),
        region_before,
        "DECSTBM with top == bottom should be ignored"
    );
}

#[test]
fn decstbm_reset_with_no_params_restores_full_screen() {
    let mut t = term();
    // Set a sub-region.
    feed(&mut t, b"\x1b[5;20r");
    assert_ne!(t.grid().scroll_region().start, 0);

    // CSI r — no params: reset to full screen.
    feed(&mut t, b"\x1b[r");
    assert_eq!(t.grid().scroll_region().start, 0);
    assert_eq!(t.grid().scroll_region().end, 24);
}

#[test]
fn cht_with_count_zero_treated_as_one() {
    let mut t = term();
    feed(&mut t, b"\x1b[3;1H"); // Move to col 0
    feed(&mut t, b"ABC"); // Now at col 3

    // CSI 0 I — CHT with count=0, should act as count=1.
    feed(&mut t, b"\x1b[0I");
    // Next tab stop after col 3 is col 8.
    assert_eq!(t.grid().cursor().col(), Column(8));
}

#[test]
fn cht_with_count_three_advances_three_stops() {
    let mut t = term();
    // CSI 3 I — advance 3 tab stops from col 0 (stops at 8, 16, 24).
    feed(&mut t, b"\x1b[3I");
    assert_eq!(t.grid().cursor().col(), Column(24));
}

#[test]
fn cbt_at_col_past_end_goes_to_last_stop() {
    let mut t = term();
    // Fill the line to trigger wrap-pending.
    let text: String = (0..80).map(|_| 'A').collect();
    feed(&mut t, text.as_bytes());
    assert_eq!(t.grid().cursor().col(), Column(80)); // wrap-pending

    // CSI Z — CBT from wrap-pending should snap and go to previous stop.
    feed(&mut t, b"\x1b[Z");
    assert_eq!(t.grid().cursor().col(), Column(72));
}

#[test]
fn alt_screen_preserves_and_restores_cursor_position() {
    let mut t = term();
    // Move to a known position on primary screen.
    feed(&mut t, b"\x1b[10;30H"); // Row 10, Col 30 (1-based)
    assert_eq!(t.grid().cursor().line(), 9);
    assert_eq!(t.grid().cursor().col(), Column(29));

    // Enter alt screen (mode 1049 saves cursor).
    feed(&mut t, b"\x1b[?1049h");
    // Alt screen starts at origin.
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));

    // Move in alt screen.
    feed(&mut t, b"\x1b[5;15H");
    assert_eq!(t.grid().cursor().line(), 4);

    // Exit alt screen — cursor should be restored to primary position.
    feed(&mut t, b"\x1b[?1049l");
    assert_eq!(t.grid().cursor().line(), 9);
    assert_eq!(t.grid().cursor().col(), Column(29));
}

#[test]
fn scroll_up_count_exceeds_region_via_handler() {
    let mut t = term();
    feed(&mut t, b"AAAAA");
    // CSI 100 S — scroll up by 100 (exceeds screen height).
    feed(&mut t, b"\x1b[100S");
    // All visible lines should be blank.
    for line in 0..24 {
        assert!(
            t.grid()[crate::index::Line(line)][Column(0)].is_empty(),
            "line {line} should be empty after massive scroll"
        );
    }
}

#[test]
fn scroll_down_count_exceeds_region_via_handler() {
    let mut t = term();
    feed(&mut t, b"AAAAA");
    // CSI 100 T — scroll down by 100.
    feed(&mut t, b"\x1b[100T");
    // All visible lines should be blank.
    for line in 0..24 {
        assert!(
            t.grid()[crate::index::Line(line)][Column(0)].is_empty(),
            "line {line} should be empty after massive scroll"
        );
    }
}

#[test]
fn insert_delete_lines_outside_scroll_region_noop() {
    let mut t = term();
    // Fill with content.
    for i in 0..24 {
        feed(
            &mut t,
            format!("\x1b[{};1H{}", i + 1, (b'A' + (i as u8 % 26)) as char).as_bytes(),
        );
    }
    // Set scroll region 5-20.
    feed(&mut t, b"\x1b[5;20r");
    // Move cursor to line 1 (outside region).
    feed(&mut t, b"\x1b[1;1H");
    let ch_before = t.grid()[crate::index::Line(0)][Column(0)].ch;

    // IL and DL should be noop outside scroll region.
    feed(&mut t, b"\x1b[5L"); // Insert 5 lines
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, ch_before);
    feed(&mut t, b"\x1b[5M"); // Delete 5 lines
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, ch_before);
}

// --- Wide character (CJK) placement ---

#[test]
fn wide_char_occupies_two_cells_with_spacer() {
    let mut t = term();
    // U+4E16 '世' is a CJK character with display width 2.
    feed(&mut t, "世".as_bytes());

    let grid = t.grid();
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, '世');
    assert!(
        grid[line][Column(0)]
            .flags
            .contains(crate::cell::CellFlags::WIDE_CHAR),
        "base cell should have WIDE_CHAR flag"
    );
    assert_eq!(grid[line][Column(1)].ch, ' ');
    assert!(
        grid[line][Column(1)]
            .flags
            .contains(crate::cell::CellFlags::WIDE_CHAR_SPACER),
        "next cell should be WIDE_CHAR_SPACER"
    );
    // Cursor should advance by 2.
    assert_eq!(grid.cursor().col(), Column(2));
}

#[test]
fn multiple_wide_chars_place_correctly() {
    let mut t = term();
    // '世界' — two CJK chars, each width 2.
    feed(&mut t, "世界".as_bytes());

    let grid = t.grid();
    let line = crate::index::Line(0);
    assert_eq!(grid[line][Column(0)].ch, '世');
    assert_eq!(grid[line][Column(2)].ch, '界');
    assert_eq!(grid.cursor().col(), Column(4));
}

#[test]
fn wide_char_at_last_column_wraps_to_next_line() {
    // 10-column terminal: wide char at col 9 can't fit, wraps.
    let mut t = Term::new(5, 10, 0, Theme::default(), crate::effect::VoidEffectSink);
    // Fill to col 9 (last column).
    feed(&mut t, b"123456789");
    assert_eq!(t.grid().cursor().col(), Column(9));

    // Write a wide char — doesn't fit in 1 remaining column.
    feed(&mut t, "世".as_bytes());

    let grid = t.grid();
    // Col 9 should be a LEADING_WIDE_CHAR_SPACER (padding before wrap).
    assert!(
        grid[crate::index::Line(0)][Column(9)]
            .flags
            .contains(crate::cell::CellFlags::LEADING_WIDE_CHAR_SPACER),
        "boundary cell should be LEADING_WIDE_CHAR_SPACER"
    );
    // Wide char should be on the next line, col 0.
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, '世');
    assert!(
        grid[crate::index::Line(1)][Column(0)]
            .flags
            .contains(crate::cell::CellFlags::WIDE_CHAR)
    );
    assert_eq!(grid.cursor().col(), Column(2));
    assert_eq!(grid.cursor().line(), 1);
}

#[test]
fn wide_char_on_single_column_grid_is_skipped() {
    // Width-2 char on a 1-column grid — can never fit.
    let mut t = Term::new(5, 1, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, "世".as_bytes());

    // Cursor shouldn't have moved (char was skipped).
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- Line wrap at column boundary ---

#[test]
fn printing_past_last_column_wraps_to_next_line() {
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, b"ABCDE");
    // After writing 5 chars in a 5-col grid, cursor is at col 5 (wrap-pending).
    assert_eq!(t.grid().cursor().col(), Column(5));

    // Next char triggers wrap.
    feed(&mut t, b"F");
    let grid = t.grid();
    assert_eq!(grid.cursor().line(), 1);
    assert_eq!(grid.cursor().col(), Column(1));
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'F');
    // First line should have WRAP flag on last cell.
    assert!(
        grid[crate::index::Line(0)][Column(4)]
            .flags
            .contains(crate::cell::CellFlags::WRAP)
    );
}

#[test]
fn wrap_pending_cleared_by_cursor_movement() {
    let mut t = Term::new(5, 5, 0, Theme::default(), crate::effect::VoidEffectSink);
    feed(&mut t, b"ABCDE");
    // Wrap pending — cursor at col 5 (one past last).
    assert_eq!(t.grid().cursor().col(), Column(5));

    // CUB (cursor back 1) clamps to last column first, then moves back by 1.
    feed(&mut t, b"\x1b[D");
    assert_eq!(t.grid().cursor().col(), Column(4));
    assert_eq!(t.grid().cursor().line(), 0);

    // Another CUB moves further back.
    feed(&mut t, b"\x1b[D");
    assert_eq!(t.grid().cursor().col(), Column(3));
}

// --- RIS grid content verification ---

#[test]
fn ris_clears_grid_content() {
    let mut t = term();
    feed(&mut t, b"Hello, World!");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'H');

    // RIS (ESC c).
    feed(&mut t, b"\x1bc");

    // Grid should be cleared — all cells should be default (space or null).
    let grid = t.grid();
    for col in 0..80 {
        let ch = grid[crate::index::Line(0)][Column(col)].ch;
        assert!(
            ch == ' ' || ch == '\0',
            "cell at col {col} should be blank after RIS, got {ch:?}"
        );
    }
    // Cursor should be at origin.
    assert_eq!(grid.cursor().col(), Column(0));
    assert_eq!(grid.cursor().line(), 0);
}

#[test]
fn ris_clears_all_visible_lines() {
    let mut t = term();
    // Write content on multiple lines.
    feed(&mut t, b"Line 0\r\nLine 1\r\nLine 2");

    feed(&mut t, b"\x1bc");

    let grid = t.grid();
    for line in 0..3 {
        let ch = grid[crate::index::Line(line)][Column(0)].ch;
        assert!(
            ch == ' ' || ch == '\0',
            "line {line} col 0 should be blank after RIS, got {ch:?}"
        );
    }
}

// --- ASCII fast path tests (23.2) ---

#[test]
fn ascii_fast_path_writes_cells_correctly() {
    let mut t = term();
    feed(&mut t, b"ABC");

    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'B');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'C');
    assert_eq!(grid.cursor().col(), Column(3));
}

#[test]
fn ascii_fast_path_preserves_sgr_attributes() {
    let mut t = term();
    // ESC[31m = red foreground, then write ASCII.
    feed(&mut t, b"\x1b[31mX");

    let grid = t.grid();
    let cell = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'X');
    // Foreground should not be the default (it should be red / color index 1).
    assert_ne!(
        cell.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Foreground)
    );
}

#[test]
fn ascii_fast_path_falls_through_for_insert_mode() {
    let mut t = term();
    // Write "AB", then ESC[4h (INSERT mode), position at col 1, write "X".
    feed(&mut t, b"AB\x1b[4h\x1b[1GX");

    let grid = t.grid();
    // INSERT mode: "X" inserted at col 0, shifting "AB" right.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'X');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'B');
}

#[test]
fn ascii_fast_path_falls_through_for_non_ascii_charset() {
    let mut t = term();
    // ESC(0 = switch G0 to DEC Special Graphics. '`' (0x60) maps to diamond.
    feed(&mut t, b"\x1b(0`");

    let grid = t.grid();
    // '`' in DEC Special Graphics maps to U+25C6 (diamond).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, '\u{25C6}');
}

#[test]
fn ascii_fast_path_handles_wrap_at_line_end() {
    // 10-column terminal.
    let mut t = Term::new(5, 10, 0, Theme::default(), crate::effect::VoidEffectSink);
    // Write exactly 10 chars to fill the line, then one more to trigger wrap.
    feed(&mut t, b"0123456789A");

    let grid = t.grid();
    // First line filled.
    assert_eq!(grid[crate::index::Line(0)][Column(9)].ch, '9');
    // 'A' wraps to line 1, col 0.
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'A');
    assert_eq!(grid.cursor().col(), Column(1));
    assert_eq!(grid.cursor().line(), 1);
}

#[test]
fn ascii_fast_path_overwriting_wide_char_falls_to_slow_path() {
    let mut t = term();
    // Write a CJK character (width 2), then move cursor back and overwrite with ASCII.
    // U+4E16 = '世' (width 2), encoded as UTF-8: E4 B8 96
    feed(&mut t, b"\xe4\xb8\x96\x1b[1GA");

    let grid = t.grid();
    // ASCII 'A' at col 0 should replace the wide char (slow path handles cleanup).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    // Col 1 should be cleared (spacer removed by slow path).
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, ' ');
}

// --- Origin mode additional tests (Section 02) ---

#[test]
fn decaln_while_origin_mode_active() {
    let mut t = term();
    // Set narrow scroll region and enable DECOM.
    feed(&mut t, b"\x1b[5;15r");
    feed(&mut t, b"\x1b[?6h");
    // DECALN: fills screen with 'E', resets scroll region to full screen.
    feed(&mut t, b"\x1b#8");
    // After DECALN, scroll region is full screen. CUP(1,1) in DECOM
    // should go to absolute line 0 (full-screen region start = 0).
    feed(&mut t, b"\x1b[1;1H");
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
    // Screen should be filled with 'E'.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'E');
    assert_eq!(t.grid()[crate::index::Line(23)][Column(79)].ch, 'E');
}

#[test]
fn origin_mode_preserves_column() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;15r"); // DECSTBM 5–15
    feed(&mut t, b"\x1b[?6h"); // ORIGIN mode
    // CUP(3,40) — row 3 in region is absolute line 6, col 39.
    feed(&mut t, b"\x1b[3;40H");
    assert_eq!(t.grid().cursor().line(), 6);
    // Column is NOT offset by DECOM — always 0-based from screen left.
    assert_eq!(t.grid().cursor().col(), Column(39));
}

#[test]
fn origin_mode_cup_row_zero_maps_to_region_start() {
    let mut t = term();
    feed(&mut t, b"\x1b[10;20r"); // DECSTBM 10–20
    feed(&mut t, b"\x1b[?6h"); // ORIGIN mode
    // CUP with row=1 (1-based minimum) → absolute line 9 (region start).
    feed(&mut t, b"\x1b[1;1H");
    assert_eq!(t.grid().cursor().line(), 9);
}
