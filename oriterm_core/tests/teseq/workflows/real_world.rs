//! Real-world pattern workflow tests (shell prompt, clear/redraw, charset switching, status bar).

use super::run_scenario;

#[test]
fn real_shell_prompt() {
    let Some(outcome) = run_scenario("real_shell_prompt") else {
        return;
    };
    // OSC 0 should have emitted Title + IconName events.
    use super::super::harness::RecordedEvent;
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
    // 'l' -> U+250C, 'q' -> U+2500, 'k' -> U+2510, 'x' -> U+2502, 'm' -> U+2514, 'j' -> U+2518
    let line0: String = outcome.grid_chars[0].iter().collect();
    assert!(
        line0.contains('\u{250C}'),
        "expected box-drawing top-left corner on line 0, got: {line0:?}"
    );
    // "Text" should appear in ASCII between the box sides.
    assert!(
        outcome.grid_text.contains("Text"),
        "ASCII text inside box not found"
    );
}

// --- Status bar: base (80x24) ---

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

// --- Status bar: multi-size variants ---

#[test]
fn real_status_bar_97x33() {
    let Some(outcome) = run_scenario("real_status_bar_97x33") else {
        return;
    };
    // "Main content area" at row 0.
    assert!(
        outcome.grid_text.starts_with("Main content area"),
        "main content not at row 0"
    );
    // Status bar text at row 32 (last row of 97x33).
    let line32 = outcome.grid_text.lines().nth(32).unwrap_or("");
    assert!(
        line32.contains("Status: OK"),
        "status bar not found at row 32: {line32:?}"
    );
}

#[test]
fn real_status_bar_120x40() {
    let Some(outcome) = run_scenario("real_status_bar_120x40") else {
        return;
    };
    // "Main content area" at row 0.
    assert!(
        outcome.grid_text.starts_with("Main content area"),
        "main content not at row 0"
    );
    // Status bar text at row 39 (last row of 120x40).
    let line39 = outcome.grid_text.lines().nth(39).unwrap_or("");
    assert!(
        line39.contains("Status: OK"),
        "status bar not found at row 39: {line39:?}"
    );
}
