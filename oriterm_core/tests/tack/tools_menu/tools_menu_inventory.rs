//! Discovery test for tack's `t) tools` submenu.
//!
//! Spawns tack under the pinned terminfo, sends `t` to enter the
//! tools submenu, captures the screen via insta, and asserts every
//! `<key>) <label>` row matches the pinned
//! `TOOLS_MENU_INVENTORY`. New keys in tack output without an
//! inventory entry OR removed keys both fail the test — this is the
//! drift gate that protects every Section 06 scenario from tack
//! version drift. Mirrors Section 05.0's begin-testing inventory
//! pattern.

use std::collections::BTreeSet;

use oriterm_test_support::tack_framework::scenarios::menu_inventory::{
    assert_menu_drift, collect_menu_keys,
};
use oriterm_test_support::tack_framework::scenarios::tools_menu_inventory::TOOLS_MENU_INVENTORY;
use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec};

/// Snapshot-only scenario that lands on tack's tools submenu.
///
/// The anchor `tack/tools [q] >` is the unique sub-menu prompt
/// produced after sending `t` from the main menu — empirically
/// verified against tack v1.08. The pre-existing-anchor guard in
/// `TackNavigator` rejects any anchor already present in the prior
/// grid; the main menu shows `tack [n] >`, NOT `tack/tools [q] >`,
/// so the substring check fires correctly.
const TACK_TOOLS_MENU: ScenarioSpec = ScenarioSpec::snapshot_only(
    "tack_tools_menu",
    "tack_tools_menu",
    &[MenuStep::new(b"t", "tack/tools [q] >")],
    "tack/tools [q] >",
);

#[test]
fn tack_tools_menu_inventory() {
    if !ScenarioRunner::available() {
        eprintln!("SKIP: tack or tic not installed");
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_TOOLS_MENU);

    // Snapshot the captured tools menu so the FIRST run (with
    // `INSTA_UPDATE=1`) pins the screen as a versioned artifact.
    // Subsequent runs compare against the pinned snapshot
    // byte-for-byte.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);

    // Drift gate: scan the captured grid for `<key>) <label>`
    // patterns (tack's standard menu format) and build the
    // discovered key set. The shared `assert_menu_drift` helper
    // compares this against `TOOLS_MENU_INVENTORY` — identical
    // algorithm to the sibling unit test in the inventory module so
    // a future weakening shows up in both.
    let discovered: BTreeSet<char> = collect_menu_keys(&outcome.grid_text);
    let pinned: BTreeSet<char> = TOOLS_MENU_INVENTORY.iter().map(|k| k.key).collect();
    if let Err(msg) = assert_menu_drift(&discovered, &pinned, "tools menu") {
        panic!("{msg}\nGrid:\n{grid}", grid = outcome.grid_text);
    }

    // Defensive cross-check: TOOLS_MENU_INVENTORY MUST contain every
    // discovered key (covered by `assert_menu_drift`) AND vice versa.
    // The pinned-side check is implicit in the helper; this
    // debug_assert pins the inverse so a future refactor that drops
    // one direction surfaces immediately.
    debug_assert_eq!(
        discovered.len(),
        TOOLS_MENU_INVENTORY.len(),
        "discovered key count must match pinned inventory length"
    );
}
