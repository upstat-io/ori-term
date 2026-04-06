//! SGR & color scenarios (attributes, underline styles, colors, selective resets).

use std::path::Path;

use super::harness::{self, ScenarioOutcome, TeseqHarness, reseq_available};

mod attributes;
mod colors;
mod combinations;
mod edge_cases;
mod resets;
mod underlines;

/// Run an SGR scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform cell attribute assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/sgr")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("sgr_{name}"));
    Some(outcome)
}
