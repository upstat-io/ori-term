//! CSI insert/delete scenarios (ICH, DCH, IL, DL).

use std::path::Path;

use super::harness::{self, TeseqHarness, reseq_available};

fn run_scenario(name: &str) {
    if !reseq_available() {
        eprintln!("SKIP: reseq not installed");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/insert_delete")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("csi_insert_delete_{name}"));
}

#[test]
fn ich() {
    run_scenario("ich");
}

#[test]
fn dch() {
    run_scenario("dch");
}

#[test]
fn il() {
    run_scenario("il");
}

#[test]
fn dl() {
    run_scenario("dl");
}
