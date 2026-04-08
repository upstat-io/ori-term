//! Doc-only stub for tack's `?) help` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::Duplicate` per
//! `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//! Covered by: the begin_testing_inventory drift gate.
//!
//! # Why this is a duplicate, not a separate scenario
//!
//! Originally classified as `Scenario` in the inventory, the
//! 05.4b empirical probe (2026-04-08) discovered that pressing
//! `?` from the begin-testing menu does NOT navigate to a
//! separate help screen — it simply re-displays the same
//! begin-testing menu inline. The captured grid after pressing
//! `?` is byte-identical to the captured grid after pressing
//! the menu's prior key (or any other no-op key that doesn't
//! advance state).
//!
//! # Where the equivalent coverage lives
//!
//! The `oriterm_core/tests/tack/test_menu/begin_testing_inventory.rs`
//! drift-gate test snapshots the entire begin-testing menu
//! rendering at 80x24 and asserts that:
//! - The set of single-character menu keys matches the
//!   `BEGIN_TESTING_INVENTORY` constant exactly (no drift in
//!   either direction).
//! - The captured grid_text matches the pinned insta snapshot
//!   byte-for-byte (any tack version bump that changes the menu
//!   labels fails the snapshot).
//!
//! That coverage IS the help screen test — adding a separate
//! `tack_help` scenario would re-snapshot the same menu rendering
//! with no incremental signal. Reclassifying `?` as Duplicate
//! makes the deduplication explicit in the inventory and avoids
//! the false claim that 05.4b adds a fresh "help screen" test.
