//! Unit tests for the extract phase.

use oriterm_core::{Column, FairMutex, Rgb, Term, Theme, VoidListener};

use super::{extract_frame, extract_frame_into};
use crate::font::CellMetrics;
use crate::gpu::frame_input::ViewportSize;

fn make_terminal(rows: usize, cols: usize) -> FairMutex<Term<VoidListener>> {
    FairMutex::new(Term::new(rows, cols, 100, Theme::Dark, VoidListener))
}

const CELL: CellMetrics = CellMetrics {
    width: 8.0,
    height: 16.0,
    baseline: 12.0,
};

// --- extract_frame ---

#[test]
fn extract_returns_correct_viewport_and_cell_size() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    assert_eq!(frame.viewport, viewport);
    assert_eq!(frame.cell_size, CELL);
}

#[test]
fn extract_captures_all_visible_cells() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    // 24 rows × 80 cols = 1920 cells.
    assert_eq!(frame.content.cells.len(), 24 * 80);
}

#[test]
fn extract_captures_cursor_state() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    // Default terminal: cursor at (0, 0), visible (SHOW_CURSOR set by default).
    assert!(frame.content.cursor.visible);
    assert_eq!(frame.content.cursor.line, 0);
    assert_eq!(frame.content.cursor.column, Column(0));
}

#[test]
fn extract_captures_palette_colors() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    // Dark theme has non-black foreground and cursor.
    assert_ne!(frame.palette.foreground, Rgb { r: 0, g: 0, b: 0 });
    assert_ne!(frame.palette.cursor_color, Rgb { r: 0, g: 0, b: 0 });
}

#[test]
fn extract_selection_is_none() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    // Selection is a placeholder until Section 9.
    assert!(frame.selection.is_none());
}

#[test]
fn extract_search_matches_are_empty() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    // Search matches are a placeholder until Section 11.
    assert!(frame.search_matches.is_empty());
}

#[test]
fn extract_does_not_hold_lock_after_return() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let _frame = extract_frame(&terminal, viewport, CELL);

    // If the lock were still held, this would deadlock.
    let _guard = terminal.lock();
}

#[test]
fn extract_captures_damage_info() {
    let terminal = make_terminal(24, 80);

    // Mark all dirty so the snapshot sees it.
    terminal.lock().grid_mut().dirty_mut().mark_all();

    let viewport = ViewportSize::new(640, 384);
    let frame = extract_frame(&terminal, viewport, CELL);

    assert!(frame.content.all_dirty);
}

#[test]
fn extract_fresh_terminal_not_all_dirty() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    // Fresh terminal starts clean — no lines marked dirty.
    assert!(!frame.content.all_dirty);
}

#[test]
fn extract_captures_terminal_mode() {
    use oriterm_core::TermMode;

    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let frame = extract_frame(&terminal, viewport, CELL);

    // Default mode includes SHOW_CURSOR and LINE_WRAP.
    assert!(frame.content.mode.contains(TermMode::SHOW_CURSOR));
}

// --- extract_frame_into ---

#[test]
fn extract_into_reuses_allocation() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    // First extraction allocates.
    let mut frame = extract_frame(&terminal, viewport, CELL);
    let first_capacity = frame.content.cells.capacity();

    // Second extraction reuses the buffer.
    extract_frame_into(&terminal, &mut frame, viewport, CELL);

    // Capacity should not have decreased (Vec reuse).
    assert!(frame.content.cells.capacity() >= first_capacity);
    assert_eq!(frame.content.cells.len(), 24 * 80);
}

#[test]
fn extract_into_updates_viewport() {
    let terminal = make_terminal(24, 80);
    let original = ViewportSize::new(640, 384);
    let updated = ViewportSize::new(1024, 768);

    let mut frame = extract_frame(&terminal, original, CELL);
    assert_eq!(frame.viewport, original);

    extract_frame_into(&terminal, &mut frame, updated, CELL);
    assert_eq!(frame.viewport, updated);
}

#[test]
fn extract_into_clears_search_matches() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let mut frame = extract_frame(&terminal, viewport, CELL);
    // Simulate leftover search matches from a previous frame.
    frame.search_matches.push(());

    extract_frame_into(&terminal, &mut frame, viewport, CELL);
    assert!(frame.search_matches.is_empty());
}

#[test]
fn extract_into_does_not_hold_lock() {
    let terminal = make_terminal(24, 80);
    let viewport = ViewportSize::new(640, 384);

    let mut frame = extract_frame(&terminal, viewport, CELL);
    extract_frame_into(&terminal, &mut frame, viewport, CELL);

    // If the lock were still held, this would deadlock.
    let _guard = terminal.lock();
}
