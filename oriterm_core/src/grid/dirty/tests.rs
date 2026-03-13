use super::{DirtyLine, DirtyTracker};

#[test]
fn new_tracker_is_clean() {
    let tracker = DirtyTracker::new(10, 80);
    assert!(!tracker.is_any_dirty());
    for i in 0..10 {
        assert!(!tracker.is_dirty(i));
    }
}

#[test]
fn mark_single_line() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark(5);

    assert!(tracker.is_dirty(5));
    assert!(tracker.is_any_dirty());

    // Other lines remain clean.
    assert!(!tracker.is_dirty(0));
    assert!(!tracker.is_dirty(4));
    assert!(!tracker.is_dirty(6));
    assert!(!tracker.is_dirty(9));
}

#[test]
fn mark_reports_full_line_bounds() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark(3);

    let items: Vec<DirtyLine> = tracker.drain().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].line, 3);
    assert_eq!(items[0].left, 0);
    assert_eq!(items[0].right, 79);
}

#[test]
fn mark_all_makes_everything_dirty() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_all();

    assert!(tracker.is_any_dirty());
    for i in 0..10 {
        assert!(tracker.is_dirty(i));
    }
}

#[test]
fn drain_returns_dirty_lines() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark(2);
    tracker.mark(7);
    tracker.mark(7); // duplicate mark is idempotent

    let indices: Vec<usize> = tracker.drain().map(|d| d.line).collect();
    assert_eq!(indices, vec![2, 7]);
}

#[test]
fn drain_resets_to_clean() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark(3);
    tracker.mark(8);

    // Consume all dirty lines.
    let _: Vec<DirtyLine> = tracker.drain().collect();

    // Everything should be clean now.
    assert!(!tracker.is_any_dirty());
    for i in 0..10 {
        assert!(!tracker.is_dirty(i));
    }
}

#[test]
fn drain_mark_all_yields_every_line() {
    let mut tracker = DirtyTracker::new(5, 80);
    tracker.mark_all();

    let indices: Vec<usize> = tracker.drain().map(|d| d.line).collect();
    assert_eq!(indices, vec![0, 1, 2, 3, 4]);

    // Clean after drain.
    assert!(!tracker.is_any_dirty());
}

#[test]
fn resize_marks_all_dirty() {
    let mut tracker = DirtyTracker::new(5, 80);
    assert!(!tracker.is_any_dirty());

    tracker.resize(8, 120);
    assert!(tracker.is_any_dirty());
    for i in 0..8 {
        assert!(tracker.is_dirty(i));
    }

    // Drain and verify 8 lines.
    let indices: Vec<usize> = tracker.drain().map(|d| d.line).collect();
    assert_eq!(indices, vec![0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn drain_drop_clears_remaining() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark(1);
    tracker.mark(5);
    tracker.mark(9);

    // Only consume the first dirty line, then drop the iterator.
    {
        let mut iter = tracker.drain();
        assert_eq!(iter.next().unwrap().line, 1);
        // Drop iter here — lines 5 and 9 should still be cleared.
    }

    // Tracker should be fully clean despite partial iteration.
    assert!(!tracker.is_any_dirty());
    assert!(!tracker.is_dirty(5));
    assert!(!tracker.is_dirty(9));
}

#[test]
fn mark_range_marks_only_target_lines() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_range(3..7);

    // Lines inside the range are dirty.
    for i in 3..7 {
        assert!(tracker.is_dirty(i), "line {i} should be dirty");
    }

    // Lines outside the range are clean.
    for i in (0..3).chain(7..10) {
        assert!(!tracker.is_dirty(i), "line {i} should be clean");
    }
}

#[test]
fn mark_range_empty_range_is_noop() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_range(5..5);
    assert!(!tracker.is_any_dirty());
}

#[test]
fn mark_range_drain_yields_only_range() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_range(2..5);

    let indices: Vec<usize> = tracker.drain().map(|d| d.line).collect();
    assert_eq!(indices, vec![2, 3, 4]);
}

#[test]
fn mark_range_full_sets_all_dirty() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_range(0..10);

    // Full-range mark_range should set the all_dirty flag.
    assert!(tracker.is_all_dirty());

    // Drain should yield every line.
    let indices: Vec<usize> = tracker.drain().map(|d| d.line).collect();
    assert_eq!(indices, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
}

#[test]
fn mark_range_superset_sets_all_dirty() {
    let mut tracker = DirtyTracker::new(5, 80);
    // Range extends beyond dirty.len() — still triggers all_dirty.
    tracker.mark_range(0..100);

    assert!(tracker.is_all_dirty());
}

#[test]
fn mark_range_partial_does_not_set_all_dirty() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_range(0..9);

    // Partial range should NOT set all_dirty.
    assert!(!tracker.is_all_dirty());
    assert!(tracker.is_any_dirty());
}

#[test]
fn mark_out_of_bounds_is_safe() {
    let mut tracker = DirtyTracker::new(5, 80);
    tracker.mark(100); // no panic, no effect
    assert!(!tracker.is_any_dirty());
    assert!(!tracker.is_dirty(100));
}

// Column-level damage bounds tests.

#[test]
fn mark_cols_single_char() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_cols(3, 10, 10);

    assert!(tracker.is_dirty(3));
    let items: Vec<DirtyLine> = tracker.drain().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].line, 3);
    assert_eq!(items[0].left, 10);
    assert_eq!(items[0].right, 10);
}

#[test]
fn mark_cols_expands_range() {
    let mut tracker = DirtyTracker::new(10, 80);
    // Two writes at different columns on the same line.
    tracker.mark_cols(3, 10, 10);
    tracker.mark_cols(3, 50, 50);

    let items: Vec<DirtyLine> = tracker.drain().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].left, 10);
    assert_eq!(items[0].right, 50);
}

#[test]
fn mark_cols_erase_range() {
    let mut tracker = DirtyTracker::new(10, 80);
    // Erase chars 20..39 (inclusive).
    tracker.mark_cols(5, 20, 39);

    let items: Vec<DirtyLine> = tracker.drain().collect();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].line, 5);
    assert_eq!(items[0].left, 20);
    assert_eq!(items[0].right, 39);
}

#[test]
fn mark_full_line_reports_full_width() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark(3);

    let items: Vec<DirtyLine> = tracker.drain().collect();
    assert_eq!(items[0].left, 0);
    assert_eq!(items[0].right, 79);
}

#[test]
fn mark_cols_then_mark_full_expands_to_full() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_cols(3, 10, 20);
    tracker.mark(3); // full-line mark should expand bounds

    let items: Vec<DirtyLine> = tracker.drain().collect();
    assert_eq!(items[0].left, 0);
    assert_eq!(items[0].right, 79);
}

#[test]
fn col_bounds_returns_none_for_clean_line() {
    let tracker = DirtyTracker::new(10, 80);
    assert_eq!(tracker.col_bounds(3), None);
}

#[test]
fn col_bounds_returns_marked_range() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_cols(3, 15, 25);

    assert_eq!(tracker.col_bounds(3), Some((15, 25)));
    // Clean line still returns None.
    assert_eq!(tracker.col_bounds(4), None);
}

#[test]
fn col_bounds_with_all_dirty_returns_full() {
    let mut tracker = DirtyTracker::new(10, 80);
    tracker.mark_all();

    assert_eq!(tracker.col_bounds(3), Some((0, 79)));
}

#[test]
fn mark_cols_out_of_bounds_is_safe() {
    let mut tracker = DirtyTracker::new(5, 80);
    tracker.mark_cols(100, 10, 20); // no panic, no effect
    assert!(!tracker.is_any_dirty());
}

#[test]
fn all_dirty_yields_full_line_bounds_for_unmarked_lines() {
    let mut tracker = DirtyTracker::new(5, 80);
    // Mark only one line with specific columns.
    tracker.mark_cols(2, 10, 20);
    // Then mark all dirty.
    tracker.mark_all();

    let items: Vec<DirtyLine> = tracker.drain().collect();
    assert_eq!(items.len(), 5);

    // Line 2 was individually marked with cols 10..20, but expanded to
    // include 0..79 since mark_cols(2, 10, 20) + all_dirty should still
    // yield the per-line bounds (which were 10..20), not necessarily full.
    // Actually: all_dirty + individually marked → yields the individual bounds.
    assert_eq!(items[2].left, 10);
    assert_eq!(items[2].right, 20);

    // Lines without individual marks get full-line bounds from all_dirty.
    assert_eq!(items[0].left, 0);
    assert_eq!(items[0].right, 79);
    assert_eq!(items[1].left, 0);
    assert_eq!(items[1].right, 79);
}
