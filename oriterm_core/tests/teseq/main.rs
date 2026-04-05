//! Teseq-based escape sequence conformance tests.
//!
//! Uses GNU teseq/reseq to author human-readable escape sequence scenarios,
//! feeds them through `Term<RecordedListener>`, and validates terminal state
//! against insta golden snapshots.
//!
//! Requires `reseq` installed (`sudo apt install teseq`).
//! Tests gracefully skip when reseq is unavailable.
//!
//! # Commands
//!
//! - Run: `cargo test -p oriterm_core --test teseq`
//! - Update snapshots: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq`

// Harness utilities are built in Section 01 and consumed by Sections 02-07.
// Suppress dead_code warnings for the incrementally-built test harness.
#![allow(dead_code)]

use std::path::Path;

mod harness;

use harness::TeseqHarness;

#[test]
fn smoke_bel() {
    if !harness::reseq_available() {
        eprintln!("reseq not installed, skipping teseq tests");
        return;
    }
    let scenario_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/teseq/scenarios/c0/bel.teseq");
    let mut h = TeseqHarness::from_scenario(&scenario_path);
    let outcome = h.run(&scenario_path);

    harness::assert_grid_snapshot(&outcome, "smoke_bel_grid");
    harness::assert_event_snapshot(&outcome, "smoke_bel_events");
    harness::assert_cursor(&outcome, 11, 0);
}
