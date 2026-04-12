//! Tests for shell integration state accessors and navigation.

use super::cwd_short_path;

// --- cwd_short_path ---

#[test]
fn cwd_short_path_extracts_last_component() {
    assert_eq!(cwd_short_path("/home/user/projects"), "projects");
}

#[test]
fn cwd_short_path_root_returns_slash() {
    assert_eq!(cwd_short_path("/"), "/");
}

#[test]
fn cwd_short_path_trailing_slash_stripped() {
    assert_eq!(cwd_short_path("/home/user/projects/"), "projects");
}

#[test]
fn cwd_short_path_single_component() {
    assert_eq!(cwd_short_path("/usr"), "usr");
}

#[test]
fn cwd_short_path_triple_slash_returns_slash() {
    assert_eq!(cwd_short_path("///"), "/");
}

#[test]
fn cwd_short_path_tilde_passthrough() {
    assert_eq!(cwd_short_path("~"), "~");
}

// --- Prompt marker management ---

use crate::Theme;
use crate::effect::VoidEffectSink;
use crate::term::Term;

/// Create a minimal term for shell state tests.
fn make_term() -> Term<VoidEffectSink> {
    Term::new(24, 80, 1000, Theme::default(), VoidEffectSink)
}

/// Feed raw bytes through the VTE processor to advance terminal state.
fn feed_bytes(term: &mut Term<VoidEffectSink>, data: &[u8]) {
    let mut processor: vte::ansi::Processor = vte::ansi::Processor::new();
    processor.advance(term, data);
}

#[test]
fn mark_prompt_row_records_absolute_position() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    assert!(!term.prompt_mark_pending(), "pending flag cleared");
    assert_eq!(term.prompt_markers().len(), 1);
    let marker = &term.prompt_markers()[0];
    assert!(marker.command.is_none());
    assert!(marker.output.is_none());
}

#[test]
fn mark_prompt_row_deduplicates_same_row() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    // Re-set pending and try to mark the same row again.
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    assert_eq!(
        term.prompt_markers().len(),
        1,
        "duplicate prompt at same row is deduplicated"
    );
}

#[test]
fn prune_prompt_markers_removes_evicted() {
    let mut term = make_term();
    // Mark a prompt at row 0 (cursor at line 0, scrollback len 0).
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    assert_eq!(term.prompt_markers().len(), 1);

    // Simulate eviction of 10 rows — the marker at row 0 should be removed.
    term.prune_prompt_markers(10);
    assert!(
        term.prompt_markers().is_empty(),
        "marker below eviction threshold is pruned"
    );
}

#[test]
fn prune_prompt_markers_shifts_surviving_indices() {
    let mut term = make_term();

    // Mark a prompt at row 0 (cursor at line 0, no scrollback).
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    // Move cursor down by feeding newlines, then mark another prompt.
    // 10 newlines move the cursor to line 10 (absolute row 10).
    feed_bytes(&mut term, b"\n\n\n\n\n\n\n\n\n\n");
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    assert_eq!(
        term.prompt_markers().len(),
        2,
        "two markers at different rows"
    );
    let row0 = term.prompt_markers()[0].prompt;
    let row1 = term.prompt_markers()[1].prompt;
    assert!(row1 > row0, "second marker at higher row: {row1} > {row0}");

    // Evict rows below the first marker — first marker is removed,
    // second marker's index is shifted down.
    let evict = row0 + 1; // evict enough to remove the first marker
    term.prune_prompt_markers(evict);

    assert_eq!(
        term.prompt_markers().len(),
        1,
        "first marker pruned, second survives"
    );
    let shifted = term.prompt_markers()[0].prompt;
    assert_eq!(
        shifted,
        row1 - evict,
        "surviving marker's prompt index shifted down by eviction count"
    );
}

#[test]
fn prune_prompt_markers_shifts_command_and_output() {
    let mut term = make_term();

    // Feed enough newlines to place the cursor at a high row.
    feed_bytes(&mut term, b"\n\n\n\n\n\n\n\n\n\n\n\n\n\n\n");
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    // Mark command start and output start at subsequent rows.
    feed_bytes(&mut term, b"\n");
    term.set_command_start_mark_pending(true);
    term.mark_command_start_row();

    feed_bytes(&mut term, b"\n");
    term.set_output_start_mark_pending(true);
    term.mark_output_start_row();

    let marker = &term.prompt_markers()[0];
    let orig_prompt = marker.prompt;
    let orig_cmd = marker.command.expect("command set");
    let orig_out = marker.output.expect("output set");

    // Evict 5 rows — all indices should shift down.
    term.prune_prompt_markers(5);
    assert_eq!(term.prompt_markers().len(), 1, "marker survives eviction");

    let shifted = &term.prompt_markers()[0];
    assert_eq!(shifted.prompt, orig_prompt - 5, "prompt shifted");
    assert_eq!(
        shifted.command,
        Some(orig_cmd.saturating_sub(5)),
        "command shifted"
    );
    assert_eq!(
        shifted.output,
        Some(orig_out.saturating_sub(5)),
        "output shifted"
    );
}

// --- Title state ---

#[test]
fn effective_title_prefers_explicit_over_cwd() {
    let mut term = make_term();
    // Set CWD — without explicit title, effective_title returns CWD short path.
    term.set_cwd(Some("/home/user/projects".to_string()));
    assert_eq!(term.effective_title(), "projects", "CWD fallback");

    // Now set explicit title via VTE (OSC 0).
    feed_bytes(&mut term, b"\x1b]0;My Custom Title\x07");
    assert_eq!(
        term.effective_title(),
        "My Custom Title",
        "explicit title takes precedence over CWD"
    );
}

#[test]
fn effective_title_falls_back_to_cwd_short_path() {
    let mut term = make_term();
    term.set_cwd(Some("/home/user/code".to_string()));
    assert_eq!(term.effective_title(), "code");
}

#[test]
fn effective_title_returns_empty_when_nothing_set() {
    let term = make_term();
    assert_eq!(term.effective_title(), "", "empty title when nothing set");
}

// --- Pending marks flags ---

#[test]
fn pending_marks_toggle() {
    let mut term = make_term();
    assert!(!term.prompt_mark_pending());
    assert!(!term.command_start_mark_pending());
    assert!(!term.output_start_mark_pending());

    term.set_prompt_mark_pending(true);
    assert!(term.prompt_mark_pending());

    term.set_command_start_mark_pending(true);
    assert!(term.command_start_mark_pending());

    term.set_output_start_mark_pending(true);
    assert!(term.output_start_mark_pending());

    term.set_prompt_mark_pending(false);
    assert!(!term.prompt_mark_pending());
}
