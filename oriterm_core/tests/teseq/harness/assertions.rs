//! Assertion helpers for teseq scenario outcomes.
//!
//! Integrates with insta for golden snapshot comparison and provides
//! convenience methods for cursor, event, and scrollback assertions.

use super::loader::ScenarioSpec;
use super::runner::ScenarioOutcome;

/// Assert grid state matches an insta golden snapshot.
pub fn assert_grid_snapshot(outcome: &ScenarioOutcome, name: &str) {
    insta::assert_snapshot!(name, outcome.grid_text);
}

/// Assert event sequence matches an insta golden snapshot.
pub fn assert_event_snapshot(outcome: &ScenarioOutcome, name: &str) {
    let event_text = outcome
        .events
        .iter()
        .map(|e| format!("{e:?}"))
        .collect::<Vec<_>>()
        .join("\n");
    insta::assert_snapshot!(name, event_text);
}

/// Assert cursor is at the expected position.
pub fn assert_cursor(outcome: &ScenarioOutcome, col: usize, line: usize) {
    assert_eq!(
        (outcome.cursor_col, outcome.cursor_line),
        (col, line),
        "cursor at col={}, line={} but expected col={col}, line={line}",
        outcome.cursor_col,
        outcome.cursor_line
    );
}

/// Run all assertions specified in the `ScenarioSpec`.
pub fn assert_spec(outcome: &ScenarioOutcome, spec: &ScenarioSpec, name: &str) {
    if spec.expect.grid_snapshot {
        assert_grid_snapshot(outcome, &format!("{name}_grid"));
    }
    if spec.expect.event_snapshot {
        assert_event_snapshot(outcome, &format!("{name}_events"));
    }
    if let Some(cursor) = &spec.expect.cursor {
        assert_cursor(outcome, cursor.col, cursor.line);
    }
    // Event name matching: each expected string is matched via contains()
    // against the Debug output of RecordedEvent variants.
    for expected_event in &spec.expect.events {
        assert!(
            outcome
                .events
                .iter()
                .any(|e| format!("{e:?}").contains(expected_event)),
            "expected event containing {expected_event:?} not found in {:?}",
            outcome.events
        );
    }
}

/// Assert scrollback buffer is empty (e.g., after ED 3).
pub fn assert_scrollback_empty(outcome: &ScenarioOutcome) {
    assert_eq!(
        outcome.scrollback_len, 0,
        "expected empty scrollback, got {} lines",
        outcome.scrollback_len
    );
}
