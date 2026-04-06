//! Complex workflow scenarios (mode combinations, query-response, real-world patterns, edge cases).

use std::path::Path;

use oriterm_core::TermMode;

use super::harness::{
    self, RecordedListener, ScenarioOutcome, TeseqHarness, assert_cell_flags_contain,
    assert_mode_contains, assert_mode_not_contains, assert_pty_writes, assert_scrollback_empty,
    cell_bg_at, cell_fg_at, reseq_available,
};

/// Run a workflow scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform additional assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/workflows")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("workflows_{name}"));
    Some(outcome)
}

// Mode combination workflows

#[test]
fn mode_scroll_origin_fill() {
    let Some(outcome) = run_scenario("mode_scroll_origin_fill") else {
        return;
    };
    assert_mode_not_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn mode_alt_with_modes() {
    let Some(outcome) = run_scenario("mode_alt_with_modes") else {
        return;
    };
    // Origin mode should survive the alt screen roundtrip.
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    // Primary screen content should be restored.
    assert!(
        outcome.grid_text.contains("Primary with modes"),
        "primary screen content not restored after alt screen exit"
    );
}

#[test]
fn mode_deccolm_full_cycle() {
    let Some(outcome) = run_scenario("mode_deccolm_full_cycle") else {
        return;
    };
    // After DECCOLM 3l (back to 80 cols), grid should be 80 columns.
    assert_eq!(outcome.cols, 80, "expected 80 columns after DECCOLM off");
    // DECCOLM 3l clears the display and homes cursor.
    // "Back to 80" should be visible at row 0.
    assert!(
        outcome.grid_text.contains("Back to 80"),
        "content after DECCOLM off not found"
    );
    // Origin mode should have been preserved across DECCOLM transitions.
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn mode_decsc_attrs() {
    let Some(outcome) = run_scenario("mode_decsc_attrs") else {
        return;
    };
    // After DECRC, cursor should return to saved position (line 4, col 9).
    assert_eq!(outcome.cursor_line, 4, "DECRC should restore cursor line");
    assert_eq!(outcome.cursor_col, 24, "cursor col after 'q after restore'");
    // The 'q' at col 9 should render as a DEC Special Graphics character
    // if charset was properly saved/restored. Under DEC Special Graphics,
    // 'q' maps to U+2500 HORIZONTAL LINE (─).
    let ch = outcome.grid_chars[4][9];
    assert_eq!(
        ch, '\u{2500}',
        "expected DEC Special Graphics horizontal line at (4,9), got {ch:?} — \
         charset was not saved/restored by DECSC/DECRC"
    );
    // Bold flag should be restored on the text after DECRC.
    assert_cell_flags_contain(&outcome, 4, 9, oriterm_core::cell::CellFlags::BOLD);
    // Red foreground should be restored — resolved Rgb red component should be nonzero.
    let fg = cell_fg_at(&outcome, 4, 9);
    assert!(
        fg.r > 100,
        "expected red-ish fg after DECRC restore, got r={} g={} b={}",
        fg.r,
        fg.g,
        fg.b
    );
}

#[test]
fn mode_decsc_origin_flag() {
    let Some(outcome) = run_scenario("mode_decsc_origin_flag") else {
        return;
    };
    // "absolute" was written at row 0 (origin mode off, CUP 1;1).
    assert!(
        outcome.grid_text.starts_with("absolute"),
        "expected 'absolute' at row 0"
    );
    // After DECRC, origin mode should be restored (enabled).
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    // "relative" should appear within the scroll region (row 4, the region top).
    let line4 = &outcome.grid_text.lines().nth(4).unwrap_or("");
    assert!(
        line4.trim_end().starts_with("relative"),
        "expected 'relative' at scroll region top (row 4), got: {line4:?}"
    );
}

// Multi-size variants (97x33)

#[test]
fn mode_scroll_origin_fill_97x33() {
    let Some(outcome) = run_scenario("mode_scroll_origin_fill_97x33") else {
        return;
    };
    assert_mode_not_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn mode_deccolm_full_cycle_97x33() {
    let Some(outcome) = run_scenario("mode_deccolm_full_cycle_97x33") else {
        return;
    };
    // DECCOLM ?3l restores to the original terminal width (97).
    assert_eq!(outcome.cols, 97, "expected 97 columns after DECCOLM off");
    assert!(
        outcome.grid_text.contains("Back to 80"),
        "content after DECCOLM off not found"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn mode_alt_with_modes_97x33() {
    let Some(outcome) = run_scenario("mode_alt_with_modes_97x33") else {
        return;
    };
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert!(
        outcome.grid_text.contains("Primary with modes"),
        "primary screen content not restored after alt screen exit"
    );
}

#[test]
fn mode_decsc_attrs_97x33() {
    let Some(outcome) = run_scenario("mode_decsc_attrs_97x33") else {
        return;
    };
    // After DECRC, cursor should return to saved position (line 9, col 19).
    assert_eq!(outcome.cursor_line, 9, "DECRC should restore cursor line");
    assert_eq!(outcome.cursor_col, 34, "cursor col after 'q after restore'");
    // DEC Special Graphics 'q' -> U+2500 at the save position.
    let ch = outcome.grid_chars[9][19];
    assert_eq!(
        ch, '\u{2500}',
        "expected DEC Special Graphics horizontal line at (9,19), got {ch:?} — \
         charset was not saved/restored by DECSC/DECRC"
    );
    assert_cell_flags_contain(&outcome, 9, 19, oriterm_core::cell::CellFlags::BOLD);
    let fg = cell_fg_at(&outcome, 9, 19);
    assert!(
        fg.r > 100,
        "expected red-ish fg after DECRC restore, got r={} g={} b={}",
        fg.r,
        fg.g,
        fg.b
    );
}

#[test]
fn mode_decsc_origin_flag_97x33() {
    let Some(outcome) = run_scenario("mode_decsc_origin_flag_97x33") else {
        return;
    };
    assert!(
        outcome.grid_text.starts_with("absolute"),
        "expected 'absolute' at row 0"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    let line4 = &outcome.grid_text.lines().nth(4).unwrap_or("");
    assert!(
        line4.trim_end().starts_with("relative"),
        "expected 'relative' at scroll region top (row 4), got: {line4:?}"
    );
}

// Multi-size variants (120x40)

#[test]
fn mode_scroll_origin_fill_120x40() {
    let Some(outcome) = run_scenario("mode_scroll_origin_fill_120x40") else {
        return;
    };
    assert_mode_not_contains(&outcome, TermMode::ORIGIN);
    assert_scrollback_empty(&outcome);
}

#[test]
fn mode_deccolm_full_cycle_120x40() {
    let Some(outcome) = run_scenario("mode_deccolm_full_cycle_120x40") else {
        return;
    };
    // DECCOLM ?3l restores to the original terminal width (120).
    assert_eq!(outcome.cols, 120, "expected 120 columns after DECCOLM off");
    assert!(
        outcome.grid_text.contains("Back to 80"),
        "content after DECCOLM off not found"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
}

#[test]
fn mode_alt_with_modes_120x40() {
    let Some(outcome) = run_scenario("mode_alt_with_modes_120x40") else {
        return;
    };
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    assert!(
        outcome.grid_text.contains("Primary with modes"),
        "primary screen content not restored after alt screen exit"
    );
}

#[test]
fn mode_decsc_attrs_120x40() {
    let Some(outcome) = run_scenario("mode_decsc_attrs_120x40") else {
        return;
    };
    // After DECRC, cursor should return to saved position (line 14, col 29).
    assert_eq!(outcome.cursor_line, 14, "DECRC should restore cursor line");
    assert_eq!(outcome.cursor_col, 44, "cursor col after 'q after restore'");
    // DEC Special Graphics 'q' -> U+2500 at the save position.
    let ch = outcome.grid_chars[14][29];
    assert_eq!(
        ch, '\u{2500}',
        "expected DEC Special Graphics horizontal line at (14,29), got {ch:?} — \
         charset was not saved/restored by DECSC/DECRC"
    );
    assert_cell_flags_contain(&outcome, 14, 29, oriterm_core::cell::CellFlags::BOLD);
    let fg = cell_fg_at(&outcome, 14, 29);
    assert!(
        fg.r > 100,
        "expected red-ish fg after DECRC restore, got r={} g={} b={}",
        fg.r,
        fg.g,
        fg.b
    );
}

#[test]
fn mode_decsc_origin_flag_120x40() {
    let Some(outcome) = run_scenario("mode_decsc_origin_flag_120x40") else {
        return;
    };
    assert!(
        outcome.grid_text.starts_with("absolute"),
        "expected 'absolute' at row 0"
    );
    assert_mode_contains(&outcome, TermMode::ORIGIN);
    let line4 = &outcome.grid_text.lines().nth(4).unwrap_or("");
    assert!(
        line4.trim_end().starts_with("relative"),
        "expected 'relative' at scroll region top (row 4), got: {line4:?}"
    );
}

// Query-response workflows

/// Replicate `crate_version_number()` from `handler/helpers.rs`.
fn compute_da2_version() -> usize {
    let mut result = 0usize;
    let version = env!("CARGO_PKG_VERSION");
    let version = version.split('-').next().unwrap_or(version);
    for (i, part) in version.split('.').rev().enumerate() {
        let n = part.parse::<usize>().unwrap_or(0);
        result += n * 100usize.pow(i as u32);
    }
    result
}

#[test]
fn query_da_handshake() {
    let Some(outcome) = run_scenario("query_da_handshake") else {
        return;
    };
    let da2_version = compute_da2_version();
    assert_pty_writes(
        &outcome,
        &[
            "\x1b[?64;6;4c",
            &format!("\x1b[>0;{da2_version};1c"),
            "\x1bP!|00000000\x1b\\",
        ],
    );
}

#[test]
fn query_cursor_tracking() {
    let Some(outcome) = run_scenario("query_cursor_tracking") else {
        return;
    };
    // CUP 5;10 → DSR → CUU 3 → DSR → CUF 20 → DSR
    // DSR reports 1-based coordinates.
    assert_pty_writes(&outcome, &["\x1b[5;10R", "\x1b[2;10R", "\x1b[2;30R"]);
}

// Real-world pattern workflows

#[test]
fn real_shell_prompt() {
    let Some(outcome) = run_scenario("real_shell_prompt") else {
        return;
    };
    // OSC 0 should have emitted Title + IconName events.
    use super::harness::RecordedEvent;
    let has_title = outcome
        .events
        .iter()
        .any(|e| matches!(e, RecordedEvent::Title(t) if t == "user@host:~"));
    assert!(has_title, "expected Title event from OSC 0");
    // Grid should show the colored prompt.
    assert!(
        outcome.grid_text.contains("user@host:~$"),
        "prompt text not found in grid"
    );
}

#[test]
fn real_clear_and_redraw() {
    let Some(outcome) = run_scenario("real_clear_and_redraw") else {
        return;
    };
    // Old content should be gone (ED 2 cleared it).
    assert!(
        !outcome.grid_text.contains("Old content"),
        "old content should be erased by ED 2"
    );
    // New content should be present.
    assert!(
        outcome.grid_text.contains("New content line 1"),
        "new content not found"
    );
}

#[test]
fn real_charset_switching() {
    let Some(outcome) = run_scenario("real_charset_switching") else {
        return;
    };
    // DEC Special Graphics box characters should appear in the grid.
    // 'l' -> ┌, 'q' -> ─, 'k' -> ┐, 'x' -> │, 'm' -> └, 'j' -> ┘
    let line0: String = outcome.grid_chars[0].iter().collect();
    assert!(
        line0.contains('\u{250C}'),
        "expected box-drawing top-left corner (┌) on line 0, got: {line0:?}"
    );
    // "Text" should appear in ASCII between the box sides.
    assert!(
        outcome.grid_text.contains("Text"),
        "ASCII text inside box not found"
    );
}

#[test]
fn real_status_bar() {
    let Some(outcome) = run_scenario("real_status_bar") else {
        return;
    };
    // "Main content area" at row 0.
    assert!(
        outcome.grid_text.starts_with("Main content area"),
        "main content not at row 0"
    );
    // Status bar text at row 23 (last row of 80x24).
    let line23 = outcome.grid_text.lines().nth(23).unwrap_or("");
    assert!(
        line23.contains("Status: OK"),
        "status bar not found at row 23: {line23:?}"
    );
}

// Edge case scenarios

#[test]
fn edge_rapid_mode_toggle() {
    let Some(outcome) = run_scenario("edge_rapid_mode_toggle") else {
        return;
    };
    // After rapid toggling, origin mode should be off (last toggle was ?6l).
    assert_mode_not_contains(&outcome, TermMode::ORIGIN);
    // "After toggles" at row 4, col 9 (CUP 5;10 with 1-based coords).
    assert!(
        outcome.grid_text.contains("After toggles"),
        "text after mode toggles not found"
    );
}

#[test]
fn edge_zero_params() {
    let Some(outcome) = run_scenario("edge_zero_params") else {
        return;
    };
    // CUP 0;0 → home, write "At origin via zeros\n".
    // CUP (omitted) → home, write "At origin via omit\n" (overwrites).
    // CUU 0 → treated as CUU 1, moves up to row 0. Write "CUU zero" overwrites cols 0-7.
    // Row 0 should start with "CUU zero".
    assert!(
        outcome.grid_text.starts_with("CUU zero"),
        "expected row 0 to start with 'CUU zero' (CUU 0 treated as 1)"
    );
}

#[test]
fn edge_large_params() {
    let Some(outcome) = run_scenario("edge_large_params") else {
        return;
    };
    // CUP 99999;99999 clamps to bottom-right corner, "Clamped" wraps.
    // CUU 99999 clamps to row 0 but keeps column. "Top" appears on row 0.
    let line0 = outcome.grid_text.lines().next().unwrap_or("");
    assert!(
        line0.contains("Top"),
        "expected 'Top' somewhere on row 0 after CUU 99999 clamp, got: {line0:?}"
    );
    // "Clamped" should be visible near the bottom (clamped to last row/col).
    assert!(
        outcome.grid_text.contains("lamped"),
        "expected 'Clamped' near bottom-right after CUP 99999;99999"
    );
}

#[test]
fn edge_erase_with_attrs() {
    let Some(outcome) = run_scenario("edge_erase_with_attrs") else {
        return;
    };
    // Cells 0-3 retain original content "AAAA" with default bg.
    let ch0 = outcome.grid_chars[0][0];
    assert_eq!(ch0, 'A', "expected 'A' at (0,0)");
    // Cells 4+ were erased with green bg active (SGR 42).
    // Erased cells should inherit the cursor template's green bg.
    let bg_erased = cell_bg_at(&outcome, 0, 4);
    let bg_default = cell_bg_at(&outcome, 0, 0);
    assert_ne!(
        bg_erased, bg_default,
        "erased cell bg should differ from default (green vs default)"
    );
}

// Chunked-feed edge cases (pure Rust, no .teseq files)

#[test]
fn edge_chunked_osc() {
    use super::harness::RecordedEvent;
    use oriterm_core::{Term, Theme};
    let listener = RecordedListener::new();
    let mut term = Term::new(24, 80, 0, Theme::default(), listener.clone());
    let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
    // Split an OSC title sequence across two advance() calls.
    let chunk1 = b"\x1b]0;MyT";
    let chunk2 = b"itle\x07";
    proc.advance(&mut term, chunk1);
    proc.advance(&mut term, chunk2);
    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RecordedEvent::Title(t) if t == "MyTitle")),
        "expected Title('MyTitle') from chunked OSC, got: {events:?}"
    );
}

#[test]
fn edge_chunked_csi() {
    use oriterm_core::{Term, Theme};
    let listener = RecordedListener::new();
    let mut term = Term::new(24, 80, 0, Theme::default(), listener.clone());
    let mut proc = vte::ansi::Processor::<vte::ansi::StdSyncHandler>::new();
    // Split CSI CUP 5;10H across two advance() calls.
    let chunk1 = b"\x1b[5;";
    let chunk2 = b"10H";
    proc.advance(&mut term, chunk1);
    proc.advance(&mut term, chunk2);
    let content = term.renderable_content();
    assert_eq!(
        content.cursor.column.0, 9,
        "cursor col should be 9 (0-based) after CUP 5;10"
    );
    assert_eq!(
        content.cursor.line, 4,
        "cursor line should be 4 (0-based) after CUP 5;10"
    );
}
