//! Unit tests for `compute_redraw_predicates`.
//!
//! Regression: BUG-06-053 — single-pane content_changed predicate was
//! missing preedit_revision input. The split-predicate output is what
//! these tests pin; the destructive overlay correctness emerges from the
//! re-extract-on-preedit-only behavior the split enables.
//!
//! See: bug-tracker/plans/completed/BUG-06-053/

use super::{RedrawPredicateInputs, RedrawPredicates, compute_redraw_predicates};

fn base_inputs() -> RedrawPredicateInputs {
    RedrawPredicateInputs {
        snap_is_none: false,
        snap_dirty: false,
        pane_changed: false,
        preedit_revision: 0,
        prev_preedit_revision: 0,
    }
}

/// Regression: BUG-06-053 — common cursor-blink frame: snapshot clean, no
/// preedit change. Both predicates false; no refresh/swap/re-extract fires.
#[test]
fn redraw_predicates_clean_snapshot_no_preedit_returns_both_false() {
    let preds = compute_redraw_predicates(base_inputs());
    assert_eq!(
        preds,
        RedrawPredicates {
            snapshot_changed: false,
            content_changed: false,
        }
    );
}

/// Regression: BUG-06-053 — snap_dirty propagates into BOTH predicates.
#[test]
fn redraw_predicates_snap_dirty_sets_both_true() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        snap_dirty: true,
        ..base_inputs()
    });
    assert!(preds.snapshot_changed);
    assert!(preds.content_changed);
}

/// Regression: BUG-06-053 — pane_changed propagates into BOTH predicates.
#[test]
fn redraw_predicates_pane_changed_sets_both_true() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        pane_changed: true,
        ..base_inputs()
    });
    assert!(preds.snapshot_changed);
    assert!(preds.content_changed);
}

/// Regression: BUG-06-053 — snap_is_none propagates into BOTH predicates.
#[test]
fn redraw_predicates_snap_is_none_sets_both_true() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        snap_is_none: true,
        ..base_inputs()
    });
    assert!(preds.snapshot_changed);
    assert!(preds.content_changed);
}

/// Regression: BUG-06-053 — the BUG-06-053 fix surface: preedit_revision
/// alone changes (all snapshot inputs false). content_changed=true forces
/// re-extract; snapshot_changed=false avoids the stale-swap path.
#[test]
fn redraw_predicates_preedit_revision_changed_alone_sets_content_only() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        preedit_revision: 5,
        prev_preedit_revision: 4,
        ..base_inputs()
    });
    assert_eq!(
        preds,
        RedrawPredicates {
            snapshot_changed: false,
            content_changed: true,
        }
    );
}

/// Regression: BUG-06-053 — preedit_revision unchanged: predicates reflect
/// snapshot inputs only.
#[test]
fn redraw_predicates_preedit_unchanged_returns_false() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        preedit_revision: 7,
        prev_preedit_revision: 7,
        ..base_inputs()
    });
    assert!(!preds.snapshot_changed);
    assert!(!preds.content_changed);
}

/// Regression: BUG-06-053 — every signal active. Both true.
#[test]
fn redraw_predicates_all_signals_active_both_true() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        snap_is_none: true,
        snap_dirty: true,
        pane_changed: true,
        preedit_revision: 1,
        prev_preedit_revision: 0,
    });
    assert!(preds.snapshot_changed);
    assert!(preds.content_changed);
}

/// Regression: BUG-06-053 — wrap (u64::MAX → 0) treated as a change. Any
/// `!=` qualifies; consistent with the multi-pane fingerprint behavior at
/// `gpu/prepare/gates.rs:140`. Must-not-fire pin against accidentally
/// tolerating wrap (which would skip re-extract on a real preedit change
/// at the u64 boundary).
#[test]
fn redraw_predicates_preedit_revision_wrap_treated_as_change() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        preedit_revision: 0,
        prev_preedit_revision: u64::MAX,
        ..base_inputs()
    });
    assert!(preds.content_changed);
    assert!(!preds.snapshot_changed);
}

/// Regression: BUG-06-053 — load-bearing semantic must-not-fire pin on
/// the split-predicate's gating decisions.
///
/// When preedit_revision is the ONLY change, the callers must NOT call
/// `mux.refresh_pane_snapshot` or `mux.swap_renderable_content`. The
/// former is a no-op when snapshot is clean, but the LATTER does a blind
/// `std::mem::swap` on its renderable cache (per
/// `oriterm_mux/src/backend/embedded/mod.rs:440-442`) and would surface
/// the prior frame's destructive preedit-overlayed buffer — exactly the
/// the prior-frame destructive-overlay corruption surface.
///
/// This pin asserts the BEHAVIOR the split enables, not just the
/// boolean. Defends against a future refactor that re-merges the two
/// predicates into one (which would re-introduce the destructive-overlay
/// reuse this fix prevents).
#[test]
fn redraw_predicates_preedit_only_change_pins_split_gating_decision() {
    let preds = compute_redraw_predicates(RedrawPredicateInputs {
        preedit_revision: 1,
        prev_preedit_revision: 0,
        ..base_inputs()
    });
    assert!(
        !preds.snapshot_changed,
        "preedit-only change MUST NOT set snapshot_changed — \
         callers gate refresh_pane_snapshot and swap_renderable_content \
         on this flag; firing either on preedit-only would surface the \
         stale destructively-overlayed cache buffer (per regression anchor)"
    );
    assert!(
        preds.content_changed,
        "preedit-only change MUST set content_changed — \
         callers gate re-extract from snapshot on this flag; without it, \
         the destructive overlay's prior cell mutations are reused"
    );
}
