//! Sibling tests for `begin_testing_inventory`.
//!
//! These tests pin the [`super::assert_inventory_drift`] drift-gate
//! algorithm so a future refactor that silently weakens it (e.g.
//! collapses the comparison to `assert!(true)`) breaks here AND in
//! the integration test in
//! `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs`
//! at the same time. The two consumers share exactly one
//! algorithm — they cannot drift independently.

use std::collections::BTreeSet;

use super::{BEGIN_TESTING_INVENTORY, assert_inventory_drift};

/// Build a discovered set that exactly matches the pinned inventory.
/// Helper for the synthetic-mutation tests below — keeps each test
/// focused on its own delta instead of repeating boilerplate.
fn pinned_set() -> BTreeSet<char> {
    BEGIN_TESTING_INVENTORY.iter().map(|k| k.key).collect()
}

#[test]
fn assert_inventory_drift_passes_on_exact_match() {
    let discovered = pinned_set();
    assert!(
        assert_inventory_drift(&discovered).is_ok(),
        "exact match must pass; got {:?}",
        assert_inventory_drift(&discovered)
    );
}

#[test]
fn begin_testing_inventory_drift_gate_pin() {
    // Synthetic mutation: take the pinned set and inject ONE extra
    // char that is guaranteed not to collide with the real menu
    // (`@` is not used by tack v1.08's begin-testing menu — and if
    // tack ever adds an `@` entry the real drift gate fires and
    // forces this pin to pick a new sentinel anyway).
    //
    // The drift gate MUST detect the extra char and return Err.
    // If a future refactor weakens the comparison (silent
    // `assert!(true)`, sloppy set equality, etc.) this assertion
    // fails and the regression is caught at the unit level — long
    // before the integration test can hide the same defect under a
    // tack-unavailable skip.
    let mut discovered = pinned_set();
    discovered.insert('@');

    let err = assert_inventory_drift(&discovered)
        .expect_err("drift gate must reject a discovered set with an injected extra key");

    assert!(
        err.contains("drift detected"),
        "drift error message must mention 'drift detected'; got: {err}"
    );
    assert!(
        err.contains("'@'"),
        "drift error message must name the unexpected key '@'; got: {err}"
    );
    assert!(
        err.contains("Only in discovered"),
        "drift error must distinguish 'only in discovered'; got: {err}"
    );
}

#[test]
fn drift_gate_detects_missing_pinned_key() {
    // Inverse direction: discovered is MISSING a key that the
    // pinned inventory expects. Tack version regressions or terminfo
    // changes that drop a menu entry must be caught with the same
    // loud error.
    let mut discovered = pinned_set();
    let removed = *discovered.iter().next().expect("inventory is non-empty");
    discovered.remove(&removed);

    let err = assert_inventory_drift(&discovered)
        .expect_err("drift gate must reject a discovered set missing a pinned key");

    assert!(
        err.contains("Only in pinned"),
        "drift error must distinguish 'only in pinned'; got: {err}"
    );
    assert!(
        err.contains(&format!("{removed:?}")),
        "drift error must name the removed key {removed:?}; got: {err}"
    );
}

#[test]
fn drift_gate_detects_empty_discovered_set() {
    // Belt-and-braces: even an empty discovered set must fail the
    // drift gate (the original failing-first state during 05.0
    // implementation). Pinning this case prevents a future refactor
    // from accidentally introducing an "empty short-circuit" that
    // returns Ok on no input.
    let discovered: BTreeSet<char> = BTreeSet::new();
    let err = assert_inventory_drift(&discovered)
        .expect_err("drift gate must reject an empty discovered set");

    assert!(
        err.contains("Only in pinned"),
        "drift error on empty discovered set must list missing pinned keys; got: {err}"
    );
}

#[test]
fn pinned_inventory_is_non_empty() {
 // Regression guard: the inventory cannot regress to the failing-
    // first empty-array state without breaking this test. Without
    // this guard, a future PR that accidentally clears
    // BEGIN_TESTING_INVENTORY would make every drift-gate test
    // pass against a host without tack installed (the integration
    // test skips when tack is unavailable; only this unit test
    // would catch the regression).
    assert!(
        !BEGIN_TESTING_INVENTORY.is_empty(),
        "BEGIN_TESTING_INVENTORY must contain the pinned begin-testing menu entries"
    );
}
