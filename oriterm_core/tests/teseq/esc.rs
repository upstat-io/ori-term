//! ESC sequence scenarios.

use std::path::Path;

use super::harness::{self, TeseqHarness, reseq_available};

fn run_scenario(name: &str) {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/esc")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("esc_{name}"));
}

#[test]
fn decsc_decrc() {
    run_scenario("decsc_decrc");
}

#[test]
fn ris() {
    run_scenario("ris");
}

#[test]
fn scs_g0() {
    run_scenario("scs_g0");
}

#[test]
fn ind() {
    run_scenario("ind");
}

#[test]
fn ri() {
    run_scenario("ri");
}
