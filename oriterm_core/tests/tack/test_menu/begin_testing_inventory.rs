//! Discovery test: spawns tack, navigates to the begin-testing
//! submenu, captures the screen via insta, and asserts every key
//! shown matches the pinned inventory table in
//! `oriterm_test_support::tack_framework::scenarios::begin_testing_inventory`.
//!
//! New keys appearing in tack output without being added to the
//! inventory = test fail. Removed keys = test fail. This is the
//! drift gate that protects every Section 05 scenario from tack
//! version drift.
//!
//! Reuses the canonical drift-gate helper `assert_inventory_drift`
//! from the inventory module so there is exactly one drift-gate
//! algorithm in the workspace (algorithmic DRY).

use std::collections::BTreeSet;

use oriterm_test_support::tack_framework::scenarios::begin_testing_inventory::{
    BEGIN_TESTING_INVENTORY, assert_inventory_drift,
};
use oriterm_test_support::tack_framework::{MenuStep, ScenarioRunner, ScenarioSpec};

/// Snapshot-only scenario that lands on tack's begin-testing menu.
///
/// The anchor `tack/test [n] >` is the unique sub-menu prompt
/// produced after sending `n` from the main menu — verified by
/// `oriterm_test_support::tack_framework::scenarios::modes::TACK_MODES_AM`
/// (see its `MenuStep` step 0). The pre-existing-anchor guard in
/// `TackNavigator` rejects any anchor already present in the prior
/// grid; the main menu shows `tack [n] >`, NOT `tack/test [n] >`,
/// so the substring check fires correctly.
const TACK_BEGIN_TESTING_MENU: ScenarioSpec = ScenarioSpec::snapshot_only(
    "tack_begin_testing_menu",
    "tack_begin_testing_menu",
    &[MenuStep::new(b"n", "tack/test [n] >")],
    "tack/test [n] >",
);

#[test]
fn tack_begin_testing_inventory() {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic not installed, skipping tack_begin_testing_inventory");
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_BEGIN_TESTING_MENU);

    // Snapshot the captured begin-testing menu so the FIRST run
    // (with `INSTA_UPDATE=1`) pins the screen as a versioned
    // artifact. Subsequent runs compare against the pinned
    // snapshot byte-for-byte.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);

    // Drift gate: scan the captured grid for `<key>) <label>`
    // patterns (tack's standard menu format) and build the
    // discovered key set. The shared `assert_inventory_drift`
    // helper compares this against `BEGIN_TESTING_INVENTORY` —
    // identical algorithm to the sibling unit test in the
    // inventory module so a future weakening shows up in both.
    let discovered = collect_menu_keys(&outcome.grid_text);
    if let Err(msg) = assert_inventory_drift(&discovered) {
        panic!("{msg}\nGrid:\n{grid}", grid = outcome.grid_text);
    }

    // Defensive cross-check: BEGIN_TESTING_INVENTORY MUST contain
    // every discovered key (covered by `assert_inventory_drift`)
    // AND vice versa. The pinned-side check is implicit in the
    // helper; this debug_assert pins the inverse so a future
    // refactor that drops one direction surfaces immediately.
    debug_assert_eq!(
        discovered.len(),
        BEGIN_TESTING_INVENTORY.len(),
        "discovered key count must match pinned inventory length"
    );
}

/// Extract every `<key>)` menu entry from the grid by scanning each
/// non-blank line for a single non-whitespace key character
/// followed by `)`.
///
/// Tack's menu format is fixed: `<spaces><key>) <label>` per row.
/// We trim leading whitespace, take the first character, and accept
/// it only when followed by `)` and not whitespace itself. Other
/// formats (table headers, prompt lines, blank rows) are ignored.
///
/// **Case-sensitive on purpose.** Tack's begin-testing menu uses
/// BOTH `p) test padding ...` and `P) test printer` — distinct
/// commands that map to different scenarios. Lowercasing here would
/// collapse them and silently hide the printer entry from the drift
/// gate.
///
/// **Punctuation keys accepted on purpose.** Tack uses `/) test a
/// specific capability` and `?) help`. Both are real menu keys with
/// the same `<key>)` syntax. Restricting to ASCII-alphabetic would
/// drop them from the discovered set and silently hide drift in
/// either entry.
fn collect_menu_keys(grid: &str) -> BTreeSet<char> {
    grid.lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            let mut chars = trimmed.chars();
            let key = chars.next()?;
            if chars.next() == Some(')') && !key.is_whitespace() {
                Some(key)
            } else {
                None
            }
        })
        .collect()
}
