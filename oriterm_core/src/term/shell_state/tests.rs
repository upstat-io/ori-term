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

// --- Additional cwd_short_path edge case ---

#[test]
fn cwd_short_path_double_slash_returns_slash() {
    assert_eq!(cwd_short_path("//"), "/");
}

// --- Mark command/output start fills last marker ---

#[test]
fn mark_command_start_fills_last_marker() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    term.set_command_start_mark_pending(true);
    term.mark_command_start_row();

    let markers = term.prompt_markers();
    assert_eq!(markers.len(), 1);
    assert!(markers[0].command.is_some());
    assert!(markers[0].output.is_none());
}

#[test]
fn mark_output_start_fills_last_marker() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    term.set_output_start_mark_pending(true);
    term.mark_output_start_row();

    let markers = term.prompt_markers();
    assert_eq!(markers.len(), 1);
    assert!(markers[0].output.is_some());
}

// --- Prune-marker boundary cases ---

#[test]
fn prune_prompt_markers_zero_eviction_is_noop() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    let before = term.prompt_markers().len();
    term.prune_prompt_markers(0);
    assert_eq!(term.prompt_markers().len(), before);
}

#[test]
fn prune_prompt_markers_exact_boundary_evicts_at_threshold() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    term.prune_prompt_markers(1);
    assert!(
        term.prompt_markers().is_empty(),
        "marker at row 0 should be evicted when evicted=1"
    );
}

// --- Command output / input range ---

#[test]
fn command_output_range_returns_correct_bounds() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    term.set_command_start_mark_pending(true);
    term.mark_command_start_row();
    feed_bytes(&mut term, b"\r\n");
    term.set_output_start_mark_pending(true);
    term.mark_output_start_row();

    let output_start = term.prompt_markers()[0].output.unwrap();
    let range = term.command_output_range(0);
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(start, output_start);
    let cursor_row = term.grid().scrollback().len() + term.grid().cursor().line();
    assert_eq!(end, cursor_row);
}

#[test]
fn command_output_range_bounded_by_next_prompt() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    term.set_command_start_mark_pending(true);
    term.mark_command_start_row();
    feed_bytes(&mut term, b"\r\n");
    term.set_output_start_mark_pending(true);
    term.mark_output_start_row();

    feed_bytes(&mut term, b"\r\n\r\n\r\n");
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    let second_prompt_start = term.prompt_markers()[1].prompt;
    let range = term.command_output_range(0).unwrap();
    assert_eq!(range.1, second_prompt_start - 1);
}

#[test]
fn command_input_range_returns_correct_bounds() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    term.set_command_start_mark_pending(true);
    term.mark_command_start_row();
    feed_bytes(&mut term, b"\r\n");
    term.set_output_start_mark_pending(true);
    term.mark_output_start_row();

    let cmd_start = term.prompt_markers()[0].command.unwrap();
    let output_start = term.prompt_markers()[0].output.unwrap();
    let range = term.command_input_range(0);
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(start, cmd_start);
    assert_eq!(end, output_start - 1);
}

#[test]
fn range_returns_none_when_no_markers() {
    let term = make_term();
    assert!(term.command_output_range(0).is_none());
    assert!(term.command_input_range(0).is_none());
}

#[test]
fn range_returns_none_when_output_start_missing() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    assert!(term.command_output_range(0).is_none());
}

#[test]
fn range_returns_none_when_command_start_missing() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    assert!(term.command_input_range(0).is_none());
}

// --- Scroll to previous/next prompt ---

#[test]
fn scroll_to_previous_prompt_scrolls_viewport() {
    let mut term = Term::new(10, 80, 1000, Theme::default(), VoidEffectSink);
    for _ in 0..30 {
        feed_bytes(&mut term, b"\r\n");
    }
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    for _ in 0..20 {
        feed_bytes(&mut term, b"\r\n");
    }

    let scrolled = term.scroll_to_previous_prompt();
    assert!(scrolled);
    assert!(term.grid().display_offset() > 0);
}

#[test]
fn scroll_to_next_prompt_scrolls_viewport() {
    let mut term = Term::new(10, 80, 1000, Theme::default(), VoidEffectSink);
    for _ in 0..30 {
        feed_bytes(&mut term, b"\r\n");
    }
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    term.grid_mut().scroll_display(isize::MAX);

    let scrolled = term.scroll_to_next_prompt();
    assert!(scrolled);
}

// --- RIS clears shell integration state ---

#[test]
fn ris_clears_prompt_state() {
    let mut term = make_term();
    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    term.set_command_start_mark_pending(true);
    term.mark_command_start_row();

    assert_eq!(term.prompt_markers().len(), 1);
    assert_eq!(term.prompt_state(), super::PromptState::None);

    term.set_prompt_state(super::PromptState::CommandStart);

    feed_bytes(&mut term, b"\x1bc");

    assert_eq!(term.prompt_state(), super::PromptState::None);
    assert!(term.prompt_markers().is_empty());
    assert!(!term.prompt_mark_pending());
    assert!(!term.command_start_mark_pending());
    assert!(!term.output_start_mark_pending());
}

#[test]
fn ris_clears_cwd_and_title_state() {
    let mut term = make_term();
    term.set_cwd(Some("/home/user".to_string()));
    term.set_has_explicit_title(true);
    term.mark_title_dirty();

    feed_bytes(&mut term, b"\x1bc");

    assert!(term.cwd().is_none());
    assert!(!term.has_explicit_title());
    assert_eq!(term.effective_title(), "");
}

#[test]
fn ris_clears_command_timing() {
    let mut term = make_term();
    term.set_command_start(std::time::Instant::now());
    let _ = term.finish_command(None);

    assert!(term.last_command_duration().is_some());

    feed_bytes(&mut term, b"\x1bc");

    assert!(term.last_command_duration().is_none());
}

// --- Multiple prompt starts without completion ---

#[test]
fn multiple_prompt_starts_without_completion_create_separate_markers() {
    let mut term = make_term();

    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    feed_bytes(&mut term, b"\r\n\r\n");

    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    assert_eq!(term.prompt_markers().len(), 2);
    assert!(term.prompt_markers()[0].prompt < term.prompt_markers()[1].prompt);
    assert!(term.prompt_markers()[0].command.is_none());
    assert!(term.prompt_markers()[0].output.is_none());
}

// --- Prompt markers surviving subsequent output ---

#[test]
fn prompt_markers_survive_subsequent_output() {
    let mut term = make_term();

    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    assert_eq!(term.prompt_markers().len(), 1);

    feed_bytes(&mut term, b"\r\nhello world\r\nmore output\r\n");

    assert_eq!(
        term.prompt_markers().len(),
        1,
        "prompt marker should survive subsequent output"
    );
}

#[test]
fn prompt_markers_survive_scrolling() {
    let mut term = Term::new(5, 20, 100, Theme::default(), VoidEffectSink);

    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();

    for i in 0..10 {
        let msg = format!("\r\nline {i}");
        feed_bytes(&mut term, msg.as_bytes());
    }

    assert_eq!(
        term.prompt_markers().len(),
        1,
        "prompt marker should survive scrolling within scrollback capacity"
    );
}

#[test]
fn prompt_markers_evicted_by_manual_prune() {
    let mut term = Term::new(3, 10, 5, Theme::default(), VoidEffectSink);

    term.set_prompt_mark_pending(true);
    term.mark_prompt_row();
    assert_eq!(term.prompt_markers().len(), 1);

    term.prune_prompt_markers(5);
    assert!(
        term.prompt_markers().is_empty(),
        "prompt marker at row 0 should be evicted when 5 rows are pruned"
    );
}

// --- finish_command (injectable clock for deterministic duration) ---

#[test]
fn finish_command_uses_injected_now_for_deterministic_duration() {
    use std::time::{Duration, Instant};

    let mut term = make_term();
    let t0 = Instant::now();
    term.set_command_start(t0);

    let duration = term.finish_command(Some(t0 + Duration::from_millis(1500)));
    assert_eq!(duration, Some(Duration::from_millis(1500)));
    assert_eq!(
        term.last_command_duration(),
        Some(Duration::from_millis(1500))
    );
}

#[test]
fn finish_command_without_start_returns_none_regardless_of_now() {
    use std::time::{Duration, Instant};

    let mut term = make_term();
    let d = term.finish_command(Some(Instant::now() + Duration::from_millis(10)));
    assert!(d.is_none());
    assert!(term.last_command_duration().is_none());
}
