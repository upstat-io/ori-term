//! Assertion helpers for teseq scenario outcomes.
//!
//! Integrates with insta for golden snapshot comparison and provides
//! convenience methods for cursor, event, and scrollback assertions.

use super::events::RecordedEvent;
use super::loader::ScenarioSpec;
use super::reseq::teseq_available;
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

/// Assert PtyWrite response bytes match expected values exactly.
///
/// This is the canonical response assertion — raw bytes are the oracle,
/// not teseq output. Each entry in `expected` is compared verbatim against
/// the corresponding PtyWrite event payload.
pub fn assert_pty_writes(outcome: &ScenarioOutcome, expected: &[&str]) {
    let actual: Vec<&str> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            RecordedEvent::PtyWrite(s) => Some(s.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        actual.len(),
        expected.len(),
        "expected {} PtyWrite events, got {}: {:?}",
        expected.len(),
        actual.len(),
        actual
    );
    for (i, (got, want)) in actual.iter().zip(expected).enumerate() {
        assert_eq!(
            got,
            want,
            "PtyWrite[{i}] mismatch:\n  got:  {:02x?}\n  want: {:02x?}",
            got.as_bytes(),
            want.as_bytes()
        );
    }
}

/// Snapshot PtyWrite response bytes for golden comparison.
///
/// Snapshots the raw response bytes (hex-escaped for readability).
/// This is a secondary assertion — `assert_pty_writes` is the primary
/// canonical check. The snapshot catches unexpected format changes.
pub fn assert_response_snapshot(outcome: &ScenarioOutcome, name: &str) {
    let pty_writes: Vec<String> = outcome
        .events
        .iter()
        .filter_map(|e| match e {
            RecordedEvent::PtyWrite(s) => Some(format!("{:02x?}", s.as_bytes())),
            _ => None,
        })
        .collect();
    insta::assert_snapshot!(format!("{name}_responses"), pty_writes.join("\n"));
}

/// Pipe response bytes through teseq for human-readable debug output.
///
/// This is NOT an oracle — it is a debug aid for understanding response
/// content when tests fail. Never use the return value as a golden
/// assertion target. Falls back to hex dump if teseq is unavailable.
pub fn analyze_response(response_bytes: &str) -> Result<String, String> {
    if !teseq_available() {
        return Ok(format!("hex: {:02x?}", response_bytes.as_bytes()));
    }
    pipe_through_command("teseq", &[], response_bytes)
}

/// Pipe bytes through a subprocess and return its stdout.
///
/// Returns `Err` if the subprocess exits non-zero or fails to spawn.
/// Extracted from `analyze_response` for testability (the teseq binary
/// cannot be faked in tests because `deny(unsafe_code)` prevents
/// `std::env::set_var` for PATH manipulation).
pub fn pipe_through_command(command: &str, args: &[&str], input: &str) -> Result<String, String> {
    use std::io::Write as _;

    let mut child = std::process::Command::new(command)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn {command}: {e}"))?;

    // take() returns Option<ChildStdin>; safe to unwrap because we set piped().
    // The temporary ChildStdin is dropped at statement end, closing the pipe
    // and signaling EOF to the subprocess.
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .map_err(|e| format!("failed to write to {command}: {e}"))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("{command} failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "{command} exited with {}: {}",
            output.status, stderr
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
