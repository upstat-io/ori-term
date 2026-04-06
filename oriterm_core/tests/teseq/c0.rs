//! C0 control character scenarios.

use std::path::Path;

use super::harness::{self, TeseqHarness, reseq_available};

fn run_scenario(name: &str) {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/c0")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("c0_{name}"));
}

#[test]
fn cr() {
    run_scenario("cr");
}

#[test]
fn lf() {
    run_scenario("lf");
}

#[test]
fn bs() {
    run_scenario("bs");
}

#[test]
fn tab() {
    run_scenario("tab");
}

#[test]
fn bel() {
    run_scenario("bel");
}

#[test]
fn ff() {
    run_scenario("ff");
}

#[test]
fn vt() {
    run_scenario("vt");
}

#[test]
fn so_si() {
    run_scenario("so_si");
}
