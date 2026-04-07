//! Complex workflow scenarios (mode combinations, query-response, real-world patterns, edge cases).

use std::path::Path;

use super::harness::{
    self, RecordedListener, ScenarioOutcome, TeseqHarness, assert_cell_flags_contain,
    assert_mode_contains, assert_mode_not_contains, assert_pty_writes, assert_scrollback_empty,
    cell_bg_at, cell_fg_at, compute_da2_version, reseq_available,
};

mod edge;
mod mode;
mod query;
mod real_world;

/// Run a workflow scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform additional assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/workflows")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("workflows_{name}"));
    Some(outcome)
}
