//! Sibling tests for `status_reports_inventory`.
//!
//! These tests pin the curated inventory's invariants so a future
//! refactor that accidentally empties the list, reorders it, or
//! breaks the length-drift final-label constant fails here at the
//! unit level — before the integration test can mask the regression
//! behind a tack-unavailable skip.

use super::{LENGTH_DRIFT_FINAL_LABEL, STATUS_REPORTS_INVENTORY, TOTAL_SUB_TESTS};

#[test]
fn pinned_inventory_is_non_empty() {
 // Regression guard: the curated inventory cannot regress to an
    // empty slice without breaking this test. Without this guard, a
    // future PR that cleared `STATUS_REPORTS_INVENTORY` would make
    // the integration test trivially pass (empty loop) on hosts
    // without tack installed.
    assert!(
        !STATUS_REPORTS_INVENTORY.is_empty(),
        "STATUS_REPORTS_INVENTORY must contain the curated sub-test entries"
    );
}

#[test]
fn pinned_inventory_covers_expected_mnemonics() {
    // The curated 8-entry set was chosen to cover DA-family,
    // canonical DSR, DECRQSS bounds, and the length-drift anchor.
    // This pin asserts those mnemonics are all present — catches a
    // refactor that drops one in passing without noticing the
    // cap-coverage implication.
    let mnemonics: Vec<&str> = STATUS_REPORTS_INVENTORY
        .iter()
        .map(|s| s.mnemonic)
        .collect();
    for required in [
        "da1",
        "da2",
        "da3",
        "dsr_status",
        "dsr_cpr",
        "decrqss_first",
        "decrqss_last",
        "decrqde_window",
    ] {
        assert!(
            mnemonics.contains(&required),
            "STATUS_REPORTS_INVENTORY missing curated mnemonic {required:?}; \
             present: {mnemonics:?}"
        );
    }
}

#[test]
fn pinned_entries_are_in_ascending_index_order() {
    // The curated entries are listed in tack's walk order. The
    // integration test relies on this to assemble menu_paths in the
    // right order. A future edit that reorders entries without
    // updating `index` fields would break the pre-existing-anchor
    // guard at runtime.
    for window in STATUS_REPORTS_INVENTORY.windows(2) {
        assert!(
            window[0].index < window[1].index,
            "STATUS_REPORTS_INVENTORY entries must be strictly ascending by index; \
             found {:?} before {:?}",
            window[0],
            window[1],
        );
    }
}

#[test]
fn every_entry_has_non_empty_label_and_mnemonic() {
    for entry in STATUS_REPORTS_INVENTORY {
        assert!(!entry.label.is_empty(), "entry {entry:?} has empty label");
        assert!(
            !entry.mnemonic.is_empty(),
            "entry {entry:?} has empty mnemonic"
        );
    }
}

#[test]
fn every_entry_index_is_within_total_sub_tests() {
    // The curated entries point into tack's 32-entry sub-submenu
    // walk. An entry with `index >= TOTAL_SUB_TESTS` would cause the
    // scenario builder to send `n` presses past the end, landing on
    // the tools menu instead of a sub-test screen. Catch that here.
    for entry in STATUS_REPORTS_INVENTORY {
        assert!(
            entry.index < TOTAL_SUB_TESTS,
            "entry {entry:?} has index >= TOTAL_SUB_TESTS ({TOTAL_SUB_TESTS})"
        );
    }
}

#[test]
fn length_drift_final_label_matches_last_curated_entry() {
    // The curated inventory's last entry is the length-drift anchor:
    // the sub-test that should land at `TOTAL_SUB_TESTS - 1` after a
    // full walk. If a refactor changes `LENGTH_DRIFT_FINAL_LABEL`
    // without also updating the curated inventory's last entry, the
    // length-drift scenario and the per-entry scenarios would
    // diverge on what they consider "the final sub-test" — catch it
    // here so they stay in lockstep.
    let last = STATUS_REPORTS_INVENTORY
        .last()
        .expect("inventory is non-empty (covered by pinned_inventory_is_non_empty)");
    assert_eq!(
        last.index,
        TOTAL_SUB_TESTS - 1,
        "last curated entry must land at the final sub-test index; \
         got index {} but TOTAL_SUB_TESTS-1 = {}",
        last.index,
        TOTAL_SUB_TESTS - 1,
    );
    assert_eq!(
        last.label, LENGTH_DRIFT_FINAL_LABEL,
        "last curated entry's label must match LENGTH_DRIFT_FINAL_LABEL \
         so the length-drift scenario and per-entry scenario agree"
    );
}

#[test]
fn total_sub_tests_is_plausible() {
    // Sanity check: the empirically measured count must be within a
    // sane range. A future edit that sets `TOTAL_SUB_TESTS = 0` or
    // `TOTAL_SUB_TESTS = 1_000_000` is almost certainly a typo; this
    // pin catches it. The real tack v1.08 number is 32.
    assert!(
        (8..=200).contains(&TOTAL_SUB_TESTS),
        "TOTAL_SUB_TESTS = {TOTAL_SUB_TESTS} is outside the plausible range 8..=200"
    );
}
