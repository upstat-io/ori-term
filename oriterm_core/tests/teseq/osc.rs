//! OSC scenarios (title, icon name, clipboard, color query).

use std::path::Path;

use super::harness::{self, ScenarioOutcome, TeseqHarness, reseq_available};

/// Run an OSC scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform additional event assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/osc")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("osc_{name}"));
    Some(outcome)
}
