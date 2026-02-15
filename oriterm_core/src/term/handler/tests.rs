//! Tests for VTE handler (Print, Execute, and CSI sequences).
//!
//! Feed raw bytes through `vte::ansi::Processor` → `Term<RecordingListener>`
//! and verify grid state and events.

use std::sync::{Arc, Mutex};

use vte::ansi::Processor;

use crate::event::{Event, EventListener};
use crate::index::Column;
use crate::term::Term;

/// Event listener that records all events for assertions.
#[derive(Clone)]
struct RecordingListener {
    events: Arc<Mutex<Vec<String>>>,
}

impl RecordingListener {
    fn new() -> Self {
        Self { events: Arc::new(Mutex::new(Vec::new())) }
    }

    fn events(&self) -> Vec<String> {
        self.events.lock().expect("lock poisoned").clone()
    }
}

impl EventListener for RecordingListener {
    fn send_event(&self, event: Event) {
        self.events.lock().expect("lock poisoned").push(format!("{event:?}"));
    }
}

/// Create a Term with 24 lines, 80 columns, and a recording listener.
fn term_with_recorder() -> (Term<RecordingListener>, RecordingListener) {
    let listener = RecordingListener::new();
    let term = Term::new(24, 80, 0, listener.clone());
    (term, listener)
}

/// Create a Term with VoidListener (when events don't matter).
fn term() -> Term<crate::event::VoidListener> {
    Term::new(24, 80, 0, crate::event::VoidListener)
}

/// Feed raw bytes through the VTE processor.
fn feed(term: &mut impl vte::ansi::Handler, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
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
    feed(&mut t, b"\x1b[5A");    // CUU 5

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
    feed(&mut t, b"\x1b[K");  // EL (default = right)

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
    feed(&mut t, b"\x1b[2G");  // CHA column 2 (1-based) → col 1
    feed(&mut t, b"\x1b[5@");  // ICH 5

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
    feed(&mut t, b"\x1b[3G");  // CHA col 3 (1-based) → col 2
    feed(&mut t, b"\x1b[3P");  // DCH 3

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
    feed(&mut t, b"\x1b[2L");   // IL 2

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
    feed(&mut t, b"\x1b[3M");   // DL 3

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

    assert!(!t.mode().contains(crate::term::TermMode::SHOW_CURSOR));
}

#[test]
fn dectcem_shows_cursor() {
    let mut t = term();
    // First hide, then show.
    feed(&mut t, b"\x1b[?25l");
    feed(&mut t, b"\x1b[?25h");

    assert!(t.mode().contains(crate::term::TermMode::SHOW_CURSOR));
}

#[test]
fn decset_alt_screen_switches_to_alt() {
    let mut t = term();
    feed(&mut t, b"hello"); // Write on primary.
    // CSI ? 1049 h — switch to alt screen.
    feed(&mut t, b"\x1b[?1049h");

    assert!(t.mode().contains(crate::term::TermMode::ALT_SCREEN));
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

    assert!(!t.mode().contains(crate::term::TermMode::ALT_SCREEN));
    // Primary screen content restored.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'h');
}

// --- CSI scroll region tests ---

#[test]
fn decstbm_sets_scroll_region() {
    let mut t = term();
    // CSI 3;20 r — set scroll region lines 3–20 (1-based).
    feed(&mut t, b"\x1b[3;20r");

    let region = t.grid().scroll_region();
    assert_eq!(region.start, 2); // 3 - 1 = 2 (0-based).
    assert_eq!(region.end, 20);  // 20 (half-open).
    // Cursor should be at origin after DECSTBM.
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- CSI device status tests ---

#[test]
fn dsr_produces_cursor_position_report() {
    let (mut t, listener) = term_with_recorder();
    // Move cursor to line 4, column 9 (0-based).
    feed(&mut t, b"\x1b[5;10H"); // CUP row 5, col 10 (1-based)
    // CSI 6 n — DSR: request cursor position.
    feed(&mut t, b"\x1b[6n");

    let events = listener.events();
    // CPR response: ESC [ 5 ; 10 R (1-based).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[5;10R)"));
}

#[test]
fn da1_produces_device_attributes() {
    let (mut t, listener) = term_with_recorder();
    // CSI c — primary device attributes.
    feed(&mut t, b"\x1b[c");

    let events = listener.events();
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[?6c)"));
}

// --- ORIGIN mode tests ---

#[test]
fn origin_mode_cup_relative_to_scroll_region() {
    let mut t = term();
    // Set scroll region rows 5–15 (1-based), enable ORIGIN mode.
    feed(&mut t, b"\x1b[5;15r");   // DECSTBM
    feed(&mut t, b"\x1b[?6h");     // DECSET ORIGIN

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
    feed(&mut t, b"\x1b[5;15r");   // DECSTBM 5–15
    feed(&mut t, b"\x1b[?6h");     // ORIGIN mode
    feed(&mut t, b"\x1b[1;10H");   // Start at col 9

    // VPA(2) in ORIGIN mode → absolute line 5 (region.start + 1).
    feed(&mut t, b"\x1b[2d");
    assert_eq!(t.grid().cursor().line(), 5);
    // Column preserved.
    assert_eq!(t.grid().cursor().col(), Column(9));
}

#[test]
fn origin_mode_disabled_cup_uses_full_screen() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;15r");   // DECSTBM
    feed(&mut t, b"\x1b[?6h");     // Enable ORIGIN
    feed(&mut t, b"\x1b[?6l");     // Disable ORIGIN

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
    feed(&mut t, b"\x1b[1;1H");  // CUP to origin
    feed(&mut t, b"\x1b[4h");    // SM: set IRM (Insert mode)
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
    feed(&mut t, b"\x1b[1;1H");  // CUP to origin
    feed(&mut t, b"\x1b[4h");    // SM: set IRM
    feed(&mut t, b"\x1b[4l");    // RM: reset IRM (back to replace)
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
    feed(&mut t, b"\x1b[20h");   // SM: set LNM
    feed(&mut t, b"hello\n");    // LF should also perform CR

    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn lnm_mode_off_lf_preserves_column() {
    let mut t = term();
    feed(&mut t, b"\x1b[20h");   // Enable LNM
    feed(&mut t, b"\x1b[20l");   // Disable LNM
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
    feed(&mut t, b"\x1b[10;15H");  // CUP to line 9, col 14
    // CSI 3 F — move up 3 lines, column 0.
    feed(&mut t, b"\x1b[3F");

    assert_eq!(t.grid().cursor().line(), 6);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- DSR code 5 and DA2 tests ---

#[test]
fn dsr_code_5_reports_terminal_ok() {
    let (mut t, listener) = term_with_recorder();
    // CSI 5 n — DSR: terminal status.
    feed(&mut t, b"\x1b[5n");

    let events = listener.events();
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[0n)"));
}

#[test]
fn da2_produces_secondary_device_attributes() {
    let (mut t, listener) = term_with_recorder();
    // CSI > c — secondary device attributes.
    feed(&mut t, b"\x1b[>c");

    let events = listener.events();
    // DA2 response: ESC [ > 0 ; version ; 1 c
    assert!(events.iter().any(|e| e.starts_with("PtyWrite(\x1b[>0;") && e.ends_with(";1c)")));
}

// --- DECRPM (mode report) tests ---

#[test]
fn decrpm_reports_set_private_mode() {
    let (mut t, listener) = term_with_recorder();
    // SHOW_CURSOR is on by default.
    // CSI ? 25 $ p — query DECTCEM.
    feed(&mut t, b"\x1b[?25$p");

    let events = listener.events();
    // Response: CSI ? 25 ; 1 $ y (1 = set).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[?25;1$y)"));
}

#[test]
fn decrpm_reports_reset_private_mode() {
    let (mut t, listener) = term_with_recorder();
    // ALT_SCREEN is off by default.
    // CSI ? 1049 $ p — query alt screen.
    feed(&mut t, b"\x1b[?1049$p");

    let events = listener.events();
    // Response: CSI ? 1049 ; 2 $ y (2 = reset).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[?1049;2$y)"));
}

#[test]
fn decrpm_reports_ansi_mode() {
    let (mut t, listener) = term_with_recorder();
    // INSERT mode is off by default.
    // CSI 4 $ p — query IRM.
    feed(&mut t, b"\x1b[4$p");

    let events = listener.events();
    // Response: CSI 4 ; 2 $ y (2 = reset).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[4;2$y)"));
}

// --- ECH edge case tests ---

#[test]
fn ech_overflow_clamps_to_line_end() {
    let mut t = term();
    feed(&mut t, b"ABCDE");
    feed(&mut t, b"\x1b[2G");  // CHA col 2 → col 1
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
    feed(&mut t, b"\x1b[1;1H");   // CUP to origin (top of region)
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
    feed(&mut t, b"\x1b[5;1H");  // CUP to line 4
    // ESC M — reverse index (not at region top → just moves up).
    feed(&mut t, b"\x1bM");

    assert_eq!(t.grid().cursor().line(), 3);
}

// --- DECSC / DECRC full round-trip ---

#[test]
fn decsc_decrc_saves_and_restores_cursor_position() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;10H");  // CUP to line 4, col 9
    feed(&mut t, b"\x1b7");       // DECSC: save cursor
    feed(&mut t, b"\x1b[1;1H");   // Move somewhere else
    feed(&mut t, b"\x1b8");       // DECRC: restore cursor

    assert_eq!(t.grid().cursor().line(), 4);
    assert_eq!(t.grid().cursor().col(), Column(9));
}

// --- DSR cursor position report in ORIGIN mode ---

#[test]
fn dsr_reports_absolute_position_even_in_origin_mode() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[5;15r");   // DECSTBM 5–15
    feed(&mut t, b"\x1b[?6h");     // ORIGIN mode
    feed(&mut t, b"\x1b[1;1H");    // CUP(1,1) → absolute line 4, col 0
    feed(&mut t, b"\x1b[6n");      // DSR

    let events = listener.events();
    // CPR: absolute line 4 + 1 = 5, col 0 + 1 = 1.
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[5;1R)"));
}

// --- Text area size report ---

#[test]
fn text_area_size_chars_reports_dimensions() {
    let (mut t, listener) = term_with_recorder();
    // CSI 18 t — report text area size in characters.
    feed(&mut t, b"\x1b[18t");

    let events = listener.events();
    // Response: CSI 8 ; lines ; cols t.
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[8;24;80t)"));
}

// --- Keypad mode tests ---

#[test]
fn deckpam_sets_application_keypad() {
    let mut t = term();
    // ESC = — DECKPAM.
    feed(&mut t, b"\x1b=");

    assert!(t.mode().contains(crate::term::TermMode::APP_KEYPAD));
}

#[test]
fn deckpnm_resets_application_keypad() {
    let mut t = term();
    feed(&mut t, b"\x1b=");   // Enable
    // ESC > — DECKPNM.
    feed(&mut t, b"\x1b>");

    assert!(!t.mode().contains(crate::term::TermMode::APP_KEYPAD));
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
    feed(&mut t, b"\x1b[20G");  // CHA col 20 → col 19
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

// --- SGR (Select Graphic Rendition) tests ---

#[test]
fn sgr_bold_sets_flag_on_cursor_template() {
    let mut t = term();
    // ESC[1m — set bold.
    feed(&mut t, b"\x1b[1m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::BOLD));
}

#[test]
fn sgr_fg_red_sets_ansi_color() {
    let mut t = term();
    // ESC[31m — set fg to red (ANSI 1).
    feed(&mut t, b"\x1b[31m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
}

#[test]
fn sgr_256color_fg() {
    let mut t = term();
    // ESC[38;5;196m — set fg to 256-color index 196.
    feed(&mut t, b"\x1b[38;5;196m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(fg, vte::ansi::Color::Indexed(196));
}

#[test]
fn sgr_truecolor_fg() {
    let mut t = term();
    // ESC[38;2;255;128;0m — set fg to RGB(255, 128, 0).
    feed(&mut t, b"\x1b[38;2;255;128;0m");

    let fg = t.grid().cursor().template.fg;
    assert_eq!(fg, vte::ansi::Color::Spec(vte::ansi::Rgb { r: 255, g: 128, b: 0 }));
}

#[test]
fn sgr_reset_clears_all_attributes() {
    let mut t = term();
    // Set bold + red fg + green bg, then reset.
    feed(&mut t, b"\x1b[1;31;42m");
    feed(&mut t, b"\x1b[0m");

    let template = &t.grid().cursor().template;
    assert_eq!(template.flags, crate::cell::CellFlags::empty());
    assert_eq!(template.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Foreground));
    assert_eq!(template.bg, vte::ansi::Color::Named(vte::ansi::NamedColor::Background));
}

#[test]
fn sgr_compound_bold_red_fg_green_bg() {
    let mut t = term();
    // ESC[1;31;42m — bold + red fg + green bg in one sequence.
    feed(&mut t, b"\x1b[1;31;42m");

    let template = &t.grid().cursor().template;
    assert!(template.flags.contains(crate::cell::CellFlags::BOLD));
    assert_eq!(template.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
    assert_eq!(template.bg, vte::ansi::Color::Named(vte::ansi::NamedColor::Green));
}

#[test]
fn sgr_curly_underline() {
    let mut t = term();
    // ESC[4:3m — curly underline (sub-param style).
    feed(&mut t, b"\x1b[4:3m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::CURLY_UNDERLINE));
    // Should not have regular underline.
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_underline_color_truecolor() {
    let mut t = term();
    // ESC[58;2;255;0;0m — set underline color to red (CellExtra).
    feed(&mut t, b"\x1b[58;2;255;0;0m");

    let template = &t.grid().cursor().template;
    let extra = template.extra.as_ref().expect("CellExtra should be allocated");
    assert_eq!(
        extra.underline_color,
        Some(vte::ansi::Color::Spec(vte::ansi::Rgb { r: 255, g: 0, b: 0 }))
    );
}

#[test]
fn sgr_59_clears_underline_color() {
    let mut t = term();
    // Set underline color, then clear it.
    feed(&mut t, b"\x1b[58;2;255;0;0m");
    feed(&mut t, b"\x1b[59m");

    let template = &t.grid().cursor().template;
    // CellExtra should be dropped (no other extra data).
    assert!(template.extra.is_none());
}

// --- SGR individual attribute flag tests ---

#[test]
fn sgr_dim_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[2m");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::DIM));
}

#[test]
fn sgr_italic_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[3m");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::ITALIC));
}

#[test]
fn sgr_blink_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[5m");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::BLINK));
}

#[test]
fn sgr_inverse_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[7m");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::INVERSE));
}

#[test]
fn sgr_hidden_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[8m");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::HIDDEN));
}

#[test]
fn sgr_strikethrough_sets_flag() {
    let mut t = term();
    feed(&mut t, b"\x1b[9m");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::STRIKETHROUGH));
}

// --- SGR cancel attribute tests ---

#[test]
fn sgr_22_cancels_bold_and_dim() {
    let mut t = term();
    // Set both bold and dim, then cancel both with SGR 22.
    feed(&mut t, b"\x1b[1;2m");
    feed(&mut t, b"\x1b[22m");

    let flags = t.grid().cursor().template.flags;
    assert!(!flags.contains(crate::cell::CellFlags::BOLD));
    assert!(!flags.contains(crate::cell::CellFlags::DIM));
}

#[test]
fn sgr_23_cancels_italic() {
    let mut t = term();
    feed(&mut t, b"\x1b[3m");
    feed(&mut t, b"\x1b[23m");

    assert!(!t.grid().cursor().template.flags.contains(crate::cell::CellFlags::ITALIC));
}

#[test]
fn sgr_24_cancels_all_underlines() {
    let mut t = term();
    // Set curly underline, then cancel.
    feed(&mut t, b"\x1b[4:3m");
    feed(&mut t, b"\x1b[24m");

    let flags = t.grid().cursor().template.flags;
    assert!(!flags.contains(crate::cell::CellFlags::CURLY_UNDERLINE));
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_25_cancels_blink() {
    let mut t = term();
    feed(&mut t, b"\x1b[5m");
    feed(&mut t, b"\x1b[25m");

    assert!(!t.grid().cursor().template.flags.contains(crate::cell::CellFlags::BLINK));
}

#[test]
fn sgr_27_cancels_inverse() {
    let mut t = term();
    feed(&mut t, b"\x1b[7m");
    feed(&mut t, b"\x1b[27m");

    assert!(!t.grid().cursor().template.flags.contains(crate::cell::CellFlags::INVERSE));
}

#[test]
fn sgr_28_cancels_hidden() {
    let mut t = term();
    feed(&mut t, b"\x1b[8m");
    feed(&mut t, b"\x1b[28m");

    assert!(!t.grid().cursor().template.flags.contains(crate::cell::CellFlags::HIDDEN));
}

#[test]
fn sgr_29_cancels_strikethrough() {
    let mut t = term();
    feed(&mut t, b"\x1b[9m");
    feed(&mut t, b"\x1b[29m");

    assert!(!t.grid().cursor().template.flags.contains(crate::cell::CellFlags::STRIKETHROUGH));
}

// --- SGR underline mutual exclusion tests ---

#[test]
fn sgr_underline_replaces_curly() {
    let mut t = term();
    // Set curly, then single — single should replace curly.
    feed(&mut t, b"\x1b[4:3m");
    feed(&mut t, b"\x1b[4m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::UNDERLINE));
    assert!(!flags.contains(crate::cell::CellFlags::CURLY_UNDERLINE));
}

#[test]
fn sgr_double_underline_replaces_single() {
    let mut t = term();
    // Single underline, then double via sub-param ESC[4:2m.
    feed(&mut t, b"\x1b[4m");
    feed(&mut t, b"\x1b[4:2m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::DOUBLE_UNDERLINE));
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_dotted_underline() {
    let mut t = term();
    feed(&mut t, b"\x1b[4:4m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::DOTTED_UNDERLINE));
}

#[test]
fn sgr_dashed_underline() {
    let mut t = term();
    feed(&mut t, b"\x1b[4:5m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::DASHED_UNDERLINE));
}

// --- SGR cancel preserves unrelated attributes ---

#[test]
fn sgr_cancel_underline_preserves_bold() {
    let mut t = term();
    // Bold + underline, then cancel underline — bold should remain.
    feed(&mut t, b"\x1b[1;4m");
    feed(&mut t, b"\x1b[24m");

    let flags = t.grid().cursor().template.flags;
    assert!(flags.contains(crate::cell::CellFlags::BOLD));
    assert!(!flags.contains(crate::cell::CellFlags::UNDERLINE));
}

#[test]
fn sgr_cancel_bold_preserves_italic_and_color() {
    let mut t = term();
    // Bold + italic + red fg, then cancel bold.
    feed(&mut t, b"\x1b[1;3;31m");
    feed(&mut t, b"\x1b[22m");

    let template = &t.grid().cursor().template;
    assert!(!template.flags.contains(crate::cell::CellFlags::BOLD));
    assert!(template.flags.contains(crate::cell::CellFlags::ITALIC));
    assert_eq!(template.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
}

// --- SGR color tests ---

#[test]
fn sgr_bg_256color() {
    let mut t = term();
    feed(&mut t, b"\x1b[48;5;42m");

    assert_eq!(t.grid().cursor().template.bg, vte::ansi::Color::Indexed(42));
}

#[test]
fn sgr_bg_truecolor() {
    let mut t = term();
    feed(&mut t, b"\x1b[48;2;0;128;255m");

    assert_eq!(
        t.grid().cursor().template.bg,
        vte::ansi::Color::Spec(vte::ansi::Rgb { r: 0, g: 128, b: 255 })
    );
}

#[test]
fn sgr_bright_fg() {
    let mut t = term();
    // ESC[91m — bright red foreground (ANSI 8–15 range).
    feed(&mut t, b"\x1b[91m");

    assert_eq!(
        t.grid().cursor().template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::BrightRed)
    );
}

#[test]
fn sgr_bright_bg() {
    let mut t = term();
    // ESC[102m — bright green background.
    feed(&mut t, b"\x1b[102m");

    assert_eq!(
        t.grid().cursor().template.bg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::BrightGreen)
    );
}

#[test]
fn sgr_39_resets_fg_only() {
    let mut t = term();
    // Red fg + green bg, then reset fg only.
    feed(&mut t, b"\x1b[31;42m");
    feed(&mut t, b"\x1b[39m");

    let template = &t.grid().cursor().template;
    assert_eq!(template.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Foreground));
    assert_eq!(template.bg, vte::ansi::Color::Named(vte::ansi::NamedColor::Green));
}

#[test]
fn sgr_49_resets_bg_only() {
    let mut t = term();
    // Red fg + green bg, then reset bg only.
    feed(&mut t, b"\x1b[31;42m");
    feed(&mut t, b"\x1b[49m");

    let template = &t.grid().cursor().template;
    assert_eq!(template.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
    assert_eq!(template.bg, vte::ansi::Color::Named(vte::ansi::NamedColor::Background));
}

// --- SGR character inheritance tests ---

#[test]
fn printed_char_inherits_bold() {
    let mut t = term();
    // Bold, then print 'A'.
    feed(&mut t, b"\x1b[1mA");

    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.ch, 'A');
    assert!(cell.flags.contains(crate::cell::CellFlags::BOLD));
}

#[test]
fn printed_char_inherits_fg_color() {
    let mut t = term();
    feed(&mut t, b"\x1b[31mA");

    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert_eq!(cell.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
}

#[test]
fn reset_between_chars_gives_different_attrs() {
    let mut t = term();
    // Bold 'A', then reset + 'B'.
    feed(&mut t, b"\x1b[1mA\x1b[0mB");

    let a = &t.grid()[crate::index::Line(0)][Column(0)];
    let b = &t.grid()[crate::index::Line(0)][Column(1)];
    assert!(a.flags.contains(crate::cell::CellFlags::BOLD));
    assert!(!b.flags.contains(crate::cell::CellFlags::BOLD));
}

// --- SGR persistence tests ---

#[test]
fn sgr_persists_across_cursor_movement() {
    let mut t = term();
    // Set bold, then move cursor down 5.
    feed(&mut t, b"\x1b[1m");
    feed(&mut t, b"\x1b[5B");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::BOLD));
}

#[test]
fn sgr_stacks_across_separate_sequences() {
    let mut t = term();
    // Bold in one sequence, underline in another, color in a third.
    feed(&mut t, b"\x1b[1m");
    feed(&mut t, b"\x1b[4m");
    feed(&mut t, b"\x1b[31m");

    let template = &t.grid().cursor().template;
    assert!(template.flags.contains(crate::cell::CellFlags::BOLD));
    assert!(template.flags.contains(crate::cell::CellFlags::UNDERLINE));
    assert_eq!(template.fg, vte::ansi::Color::Named(vte::ansi::NamedColor::Red));
}

// --- SGR edge case tests ---

#[test]
fn sgr_empty_params_resets() {
    let mut t = term();
    // Set bold, then ESC[m (no params) should reset like SGR 0.
    feed(&mut t, b"\x1b[1m");
    feed(&mut t, b"\x1b[m");

    assert_eq!(t.grid().cursor().template.flags, crate::cell::CellFlags::empty());
}

#[test]
fn sgr_last_color_wins() {
    let mut t = term();
    // ESC[30;31m — black then red in same sequence; red should win.
    feed(&mut t, b"\x1b[30;31m");

    assert_eq!(
        t.grid().cursor().template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
}

#[test]
fn sgr_fast_blink_uses_blink_flag() {
    let mut t = term();
    // SGR 6 (fast blink) — mapped to same BLINK flag as slow blink.
    feed(&mut t, b"\x1b[6m");

    assert!(t.grid().cursor().template.flags.contains(crate::cell::CellFlags::BLINK));
}

#[test]
fn sgr_underline_color_survives_underline_type_change() {
    let mut t = term();
    // Set underline color to red, then switch from single to curly.
    feed(&mut t, b"\x1b[4m");
    feed(&mut t, b"\x1b[58;2;255;0;0m");
    feed(&mut t, b"\x1b[4:3m");

    let template = &t.grid().cursor().template;
    assert!(template.flags.contains(crate::cell::CellFlags::CURLY_UNDERLINE));
    let extra = template.extra.as_ref().expect("underline color should survive");
    assert_eq!(
        extra.underline_color,
        Some(vte::ansi::Color::Spec(vte::ansi::Rgb { r: 255, g: 0, b: 0 }))
    );
}

#[test]
fn sgr_underline_color_256() {
    let mut t = term();
    // ESC[58;5;196m — set underline color to 256-color index 196.
    feed(&mut t, b"\x1b[58;5;196m");

    let extra = t
        .grid()
        .cursor()
        .template
        .extra
        .as_ref()
        .expect("CellExtra should be allocated");
    assert_eq!(extra.underline_color, Some(vte::ansi::Color::Indexed(196)));
}

#[test]
fn sgr_reset_clears_underline_color() {
    let mut t = term();
    // Set underline color, then full reset.
    feed(&mut t, b"\x1b[58;2;255;0;0m");
    feed(&mut t, b"\x1b[0m");

    assert!(t.grid().cursor().template.extra.is_none());
}
