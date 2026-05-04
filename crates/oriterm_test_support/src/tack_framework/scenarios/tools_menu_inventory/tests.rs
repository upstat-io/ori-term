//! Sibling tests for `tools_menu_inventory`.
//!
//! These tests pin the drift-gate integration + the inventory
//! non-empty invariant so a future refactor that accidentally clears
//! `TOOLS_MENU_INVENTORY` or collapses the drift-gate path breaks
//! here AND in the integration test at
//! `oriterm_core/tests/tack/tools_menu/tools_menu_inventory.rs`
//! simultaneously. Without this unit-level pin, a regression that
//! zeroed the inventory would silently pass on hosts without tack
//! installed (the integration test skips there).

use std::collections::BTreeSet;

use super::{TOOLS_MENU_INVENTORY, ToolsMenuStatus};
use crate::tack_framework::scenarios::menu_inventory::assert_menu_drift;

/// Build the pinned key set from `TOOLS_MENU_INVENTORY`. Helper for
/// the synthetic-mutation tests below — keeps each test focused on
/// its own delta instead of repeating boilerplate.
fn pinned_set() -> BTreeSet<char> {
    TOOLS_MENU_INVENTORY.iter().map(|k| k.key).collect()
}

#[test]
fn pinned_inventory_is_non_empty() {
    // Regression guard: the inventory cannot regress to the failing-
    // first empty-array state without breaking this test. Without
    // this guard, a future PR that accidentally cleared
    // TOOLS_MENU_INVENTORY would make every drift-gate assertion
    // pass against a host without tack installed (the integration
    // test skips when tack is unavailable; only this unit test
    // would catch the regression).
    assert!(
        !TOOLS_MENU_INVENTORY.is_empty(),
        "TOOLS_MENU_INVENTORY must contain the pinned tools-menu entries"
    );
}

#[test]
fn pinned_inventory_matches_captured_tack_v1_08_menu() {
    // Empirical pin: the tools menu captured from the live tack v1.08
    // binary during plan authorship had exactly these 12 keys. If a
    // tack release changes the menu, this pin fires at the unit level
    // (no tack needed) so contributors can see the regression locally
    // before CI runs the integration test against a tack-less host.
    //
    // This is a SNAPSHOT of the menu as of 2026-04-08; a future tack
    // release that legitimately changes the menu should update this
    // pin AND the snapshot AND the inventory in lockstep.
    let expected: BTreeSet<char> = "sgchereupidq?".chars().collect();
    let actual = pinned_set();
    assert_eq!(
        actual, expected,
        "pinned tools-menu key set drifted from the empirical v1.08 capture"
    );
}

#[test]
fn drift_gate_detects_injected_extra_key() {
    // Property for the shared `assert_menu_drift` helper, but
    // routed through `TOOLS_MENU_INVENTORY` to prove the wiring
    // works end-to-end. Inject one extra key that is guaranteed not
    // to collide with the real menu (`@` is not used by tack v1.08
    // — and if tack ever adds an `@` entry the real drift gate fires
    // and forces this pin to pick a new sentinel anyway).
    let mut discovered = pinned_set();
    discovered.insert('@');

    let err = assert_menu_drift(&discovered, &pinned_set(), "tools menu")
        .expect_err("drift gate must reject a discovered set with an injected extra key");

    assert!(
        err.contains("tools menu drift detected"),
        "error must name source label; got: {err}"
    );
    assert!(
        err.contains("'@'"),
        "error must name the unexpected key '@'; got: {err}"
    );
}

#[test]
fn drift_gate_detects_missing_pinned_key() {
    // Inverse direction: discovered is missing a key the pinned
    // inventory expects. Tack version regressions or terminfo
    // changes that drop a menu entry must be caught with the same
    // loud error.
    let mut discovered = pinned_set();
    let removed = *discovered.iter().next().expect("inventory is non-empty");
    discovered.remove(&removed);

    let err = assert_menu_drift(&discovered, &pinned_set(), "tools menu")
        .expect_err("drift gate must reject a discovered set missing a pinned key");

    assert!(
        err.contains("Only in pinned"),
        "error must distinguish 'only in pinned'; got: {err}"
    );
    assert!(
        err.contains(&format!("{removed:?}")),
        "error must name the removed key {removed:?}; got: {err}"
    );
}

#[test]
fn every_scenario_key_has_a_label() {
    // Defensive pin: each `Scenario`-classified entry must have a
    // non-empty label so the drift-gate diagnostic never produces
    // `'s' ()` or similar. Also catches a future refactor that
    // accidentally replaces a label with `""`.
    for entry in TOOLS_MENU_INVENTORY
        .iter()
        .filter(|k| matches!(k.status, ToolsMenuStatus::Scenario))
    {
        assert!(
            !entry.label.is_empty(),
            "Scenario-classified key {:?} has empty label",
            entry.key
        );
    }
}

#[test]
fn every_excluded_interactive_has_stub_file() {
    // Each `ExcludedInteractive` entry must name a stub file so
    // 06.6 can locate it. Catches a future refactor that zeroes the
    // `stub_file` field.
    for entry in TOOLS_MENU_INVENTORY {
        if let ToolsMenuStatus::ExcludedInteractive { stub_file } = entry.status {
            assert!(
                !stub_file.is_empty(),
                "ExcludedInteractive key {:?} has empty stub_file",
                entry.key
            );
            assert!(
                stub_file.ends_with(".rs"),
                "ExcludedInteractive key {:?} stub_file must be a .rs file; got {stub_file:?}",
                entry.key
            );
        }
    }
}
