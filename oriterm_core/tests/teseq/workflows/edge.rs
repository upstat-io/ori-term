//! Edge case workflow tests (rapid mode toggle, zero/large params, erase with attrs, chunked feed).

use super::{RecordedListener, assert_mode_not_contains, cell_bg_at, run_scenario};
use oriterm_core::TermMode;

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
    // CUP 0;0 -> home, write "At origin via zeros\n".
    // CUP (omitted) -> home, write "At origin via omit\n" (overwrites).
    // CUU 0 -> treated as CUU 1, moves up to row 0. Write "CUU zero" overwrites cols 0-7.
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

// --- Chunked-feed edge cases (pure Rust, no .teseq files) ---

#[test]
fn edge_chunked_osc() {
    use super::super::harness::RecordedEvent;
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
