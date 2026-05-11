use log::{Level, LevelFilter};
use oriterm_test_support::log_capture::{CapturedRecord, with_capture};

use super::{DirtyLine, DirtyTracker};

const TARGET: &str = "oriterm_core::grid::dirty";

fn matching(records: &[CapturedRecord], substr: &str) -> Vec<CapturedRecord> {
    records
        .iter()
        .filter(|r| r.target == TARGET && r.level == Level::Trace && r.message.contains(substr))
        .cloned()
        .collect()
}

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

    // Why: Line 2 was individually marked with cols 10..20. Expected
    // semantics under `mark_cols(2, 10, 20)` + `all_dirty`: the per-line
    // bounds (10..20) are preserved, not widened to the full 0..79 range.
    // I.e. `all_dirty` + individually-marked yields the individual bounds.
    assert_eq!(items[2].left, 10);
    assert_eq!(items[2].right, 20);

    // Lines without individual marks get full-line bounds from all_dirty.
    assert_eq!(items[0].left, 0);
    assert_eq!(items[0].right, 79);
    assert_eq!(items[1].left, 0);
    assert_eq!(items[1].right, 79);
}

// Trace-emission tests

#[test]
fn mark_emits_trace_with_line() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(10, 80);
        tracker.mark(5);

        let recs = matching(&sink.records(), "mark line=5");
        assert_eq!(
            recs.len(),
            1,
            "expected one trace; got {:?}",
            sink.records()
        );
        assert!(recs[0].message.contains("mark line=5"));
    });
}

#[test]
fn mark_cols_emits_trace_with_line_and_range() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(10, 80);
        tracker.mark_cols(3, 10, 20);

        let recs = matching(&sink.records(), "mark_cols");
        assert_eq!(recs.len(), 1);
        assert!(recs[0].message.contains("line=3"));
        assert!(recs[0].message.contains("left=10"));
        assert!(recs[0].message.contains("right=20"));
    });
}

#[test]
fn mark_range_emits_trace_with_range() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(10, 80);
        tracker.mark_range(2..7);

        let recs = matching(&sink.records(), "mark_range");
        assert_eq!(recs.len(), 1);
        assert!(recs[0].message.contains("start=2"));
        assert!(recs[0].message.contains("end=7"));
    });
}

#[test]
fn mark_all_emits_trace() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(10, 80);
        tracker.mark_all();

        let recs = matching(&sink.records(), "mark_all");
        assert_eq!(recs.len(), 1);
    });
}

#[test]
fn resize_with_changed_dims_emits_trace() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(5, 80);
        tracker.resize(8, 120);

        let recs = matching(&sink.records(), "resize");
        assert_eq!(recs.len(), 1, "got records: {:?}", sink.records());
        let msg = &recs[0].message;
        assert!(msg.contains("old_lines=5"), "msg={msg}");
        assert!(msg.contains("new_lines=8"), "msg={msg}");
        assert!(msg.contains("old_cols=80"), "msg={msg}");
        assert!(msg.contains("new_cols=120"), "msg={msg}");
        assert!(msg.contains("changed=true"), "msg={msg}");
    });
}

#[test]
fn resize_with_unchanged_dims_emits_trace_with_changed_false() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(5, 80);
        tracker.resize(5, 80);

        let recs = matching(&sink.records(), "resize");
        assert_eq!(recs.len(), 1);
        assert!(recs[0].message.contains("changed=false"));
    });
}

#[test]
fn drain_drop_emits_summary_with_yielded_and_cleared_counts() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(10, 80);
        tracker.mark(2);
        tracker.mark(5);
        tracker.mark(7);
        let _consumed: Vec<DirtyLine> = tracker.drain().collect();

        let recs = matching(&sink.records(), "drain end");
        assert_eq!(recs.len(), 1);
        let msg = &recs[0].message;
        assert!(msg.contains("yielded=3"), "msg={msg}");
        assert!(msg.contains("drop_cleared=0"), "msg={msg}");
    });
}

#[test]
fn drain_drop_partial_iter_reports_cleared_remainder() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(10, 80);
        tracker.mark(1);
        tracker.mark(5);
        tracker.mark(9);
        {
            let mut iter = tracker.drain();
            let _first = iter.next().unwrap();
            // Drop iter here.
        }

        let recs = matching(&sink.records(), "drain end");
        assert_eq!(recs.len(), 1);
        let msg = &recs[0].message;
        assert!(msg.contains("yielded=1"), "msg={msg}");
        assert!(msg.contains("drop_cleared=2"), "msg={msg}");
    });
}

#[test]
fn drain_drop_with_mark_all_counts_remainder() {
    // When all_dirty is set, the drop-cleared count must include lines
    // that aren't individually marked but ARE logically dirty under the
    // all_dirty contract.
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(5, 80);
        tracker.mark_all();
        {
            let _iter = tracker.drain();
            // Drop without iterating any line.
        }

        let recs = matching(&sink.records(), "drain end");
        assert_eq!(recs.len(), 1);
        let msg = &recs[0].message;
        assert!(msg.contains("yielded=0"), "msg={msg}");
        assert!(msg.contains("drop_cleared=5"), "msg={msg}");
    });
}

#[test]
fn out_of_bounds_mark_emits_no_trace() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(5, 80);
        tracker.mark(100);

        let recs = matching(&sink.records(), "mark line=100");
        assert!(
            recs.is_empty(),
            "expected no trace for OOB mark; got {:?}",
            sink.records()
        );
    });
}

#[test]
fn out_of_bounds_mark_cols_emits_no_trace() {
    with_capture(LevelFilter::Trace, |sink| {
        let mut tracker = DirtyTracker::new(5, 80);
        tracker.mark_cols(100, 0, 5);

        let recs = matching(&sink.records(), "mark_cols line=100");
        assert!(recs.is_empty());
    });
}

#[test]
fn traces_disabled_at_warn_level_emit_nothing() {
    with_capture(LevelFilter::Warn, |sink| {
        let mut tracker = DirtyTracker::new(10, 80);
        tracker.mark(1);
        tracker.mark_cols(2, 0, 5);
        tracker.mark_range(3..6);
        tracker.mark_all();
        tracker.resize(12, 80);
        let _consumed: Vec<DirtyLine> = tracker.drain().collect();

        let recs = sink.records();
        assert!(
            recs.iter()
                .all(|r| r.target != TARGET || r.level < Level::Trace),
            "expected zero trace records at Warn level; got {:?}",
            recs
        );
    });
}
