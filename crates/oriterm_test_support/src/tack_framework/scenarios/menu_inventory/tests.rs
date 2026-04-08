//! Sibling tests for [`super::assert_menu_drift`] and
//! [`super::collect_menu_keys`].
//!
//! These tests pin the drift-gate algorithm and the menu-key scanner
//! so a future refactor that silently weakens either (e.g. collapses
//! the comparison to `assert!(true)`, or drops punctuation keys from
//! the scanner) breaks here AND in the Section 06 integration tests
//! at the same time. The three consumers (tools_menu_inventory,
//! status_reports_inventory, and the integration tests) share
//! exactly one algorithm — they cannot drift independently.

use std::collections::BTreeSet;

use super::{assert_menu_drift, collect_menu_keys};

fn set(chars: &str) -> BTreeSet<char> {
    chars.chars().collect()
}

#[test]
fn assert_menu_drift_passes_on_exact_match() {
    let pinned = set("abcq?");
    let discovered = set("abcq?");
    assert!(
        assert_menu_drift(&discovered, &pinned, "test menu").is_ok(),
        "exact match must pass"
    );
}

#[test]
fn assert_menu_drift_rejects_extra_key_in_discovered() {
    // Synthetic mutation: discovered has ONE extra key the pinned
    // inventory does not know about. The drift gate MUST detect it
    // and return Err naming both the source label AND the extra key.
    // If a future refactor weakens the comparison to silent
    // `assert!(true)`, this pin fires.
    let pinned = set("abc");
    let discovered = set("abcZ");

    let err = assert_menu_drift(&discovered, &pinned, "tools menu")
        .expect_err("drift gate must reject extra key in discovered");

    assert!(
        err.contains("tools menu drift detected"),
        "error must name the source label; got: {err}"
    );
    assert!(
        err.contains("'Z'"),
        "error must name the unexpected key 'Z'; got: {err}"
    );
    assert!(
        err.contains("Only in discovered"),
        "error must distinguish 'only in discovered'; got: {err}"
    );
}

#[test]
fn assert_menu_drift_rejects_missing_pinned_key() {
    // Inverse direction: discovered is MISSING a key that the pinned
    // inventory expects. Tack version regressions or terminfo changes
    // that drop a menu entry must be caught with the same loud error.
    let pinned = set("abc");
    let discovered = set("ab");

    let err = assert_menu_drift(&discovered, &pinned, "tools menu")
        .expect_err("drift gate must reject missing pinned key");

    assert!(
        err.contains("Only in pinned"),
        "error must distinguish 'only in pinned'; got: {err}"
    );
    assert!(
        err.contains("'c'"),
        "error must name the missing key 'c'; got: {err}"
    );
}

#[test]
fn assert_menu_drift_rejects_empty_discovered() {
    // Belt-and-braces: even an empty discovered set must fail the
    // drift gate (the failing-first state during 06.0 implementation).
    // Pinning this prevents a future refactor from introducing an
    // "empty short-circuit" that returns Ok on no input.
    let pinned = set("abc");
    let discovered: BTreeSet<char> = BTreeSet::new();

    let err = assert_menu_drift(&discovered, &pinned, "tools menu")
        .expect_err("drift gate must reject empty discovered set");

    assert!(
        err.contains("Only in pinned"),
        "empty discovered must list missing pinned keys; got: {err}"
    );
}

#[test]
fn assert_menu_drift_rejects_empty_pinned() {
    // Inverse edge: empty pinned inventory with non-empty discovered.
    // A regression that leaves `TOOLS_MENU_INVENTORY` at its initial
    // empty-array start state would be caught here via the
    // `pinned_inventory_is_non_empty` pin in tools_menu_inventory, but
    // this test pins the helper's behavior on the empty-pinned side
    // so any caller that bypasses the non-empty pin still gets a loud
    // error.
    let pinned: BTreeSet<char> = BTreeSet::new();
    let discovered = set("abc");

    let err = assert_menu_drift(&discovered, &pinned, "tools menu")
        .expect_err("drift gate must reject empty pinned set when discovered has entries");

    assert!(
        err.contains("Only in discovered"),
        "empty pinned must list the unexpected discovered keys; got: {err}"
    );
}

#[test]
fn collect_menu_keys_extracts_letters_digits_and_punctuation() {
    // Realistic tools-menu capture: letters, menu-meta punctuation.
    let grid = " s) ANSI status reports\n g) ANSI SGR modes\n c) ANSI character sets\n q) quit\n ?) help\n";
    let keys = collect_menu_keys(grid);
    assert_eq!(keys, set("sgcq?"));
}

#[test]
fn collect_menu_keys_ignores_non_menu_lines() {
    // The capture includes header lines, blank rows, and a prompt
    // line. Only the `<key>)` rows should be scanned.
    let grid = "Tools Menu\n\n s) ANSI status reports\n g) ANSI SGR modes\n\ntack/tools [q] > \n";
    let keys = collect_menu_keys(grid);
    assert_eq!(keys, set("sg"));
}

#[test]
fn collect_menu_keys_is_case_sensitive() {
    // Upper vs lower case are distinct keys (tack's begin-testing
    // submenu uses `p` vs `P`). This test pins that the tools-menu
    // scanner keeps the same invariant even though tack v1.08's
    // tools menu does not currently expose a case-paired key.
    let grid = " p) test padding\n P) test printer\n";
    let keys = collect_menu_keys(grid);
    assert_eq!(keys, set("pP"));
}

#[test]
fn collect_menu_keys_rejects_whitespace_key() {
    // Degenerate row where the "key" is whitespace followed by `)`.
    // Must be rejected so the scanner cannot be tricked into
    // inserting a sentinel space character into the discovered set.
    let grid = "  ) not a real entry\n s) real\n";
    let keys = collect_menu_keys(grid);
    assert_eq!(keys, set("s"));
}

#[test]
fn collect_menu_keys_empty_grid_returns_empty_set() {
    let keys = collect_menu_keys("");
    assert!(keys.is_empty());
}
