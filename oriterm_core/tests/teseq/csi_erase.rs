//! CSI erase scenarios (ED and EL).

use std::path::Path;

use super::harness::{self, TeseqHarness, reseq_available};

fn run_scenario(name: &str) {
    if !reseq_available() {
        eprintln!("SKIP: reseq not installed");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/erase")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("csi_erase_{name}"));
}

#[test]
fn ed_below() {
    run_scenario("ed_below");
}

#[test]
fn ed_above() {
    run_scenario("ed_above");
}

#[test]
fn ed_all() {
    run_scenario("ed_all");
}

#[test]
fn ed_scrollback() {
    if !reseq_available() {
        eprintln!("SKIP: reseq not installed");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/erase/ed_scrollback.teseq");
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), "csi_erase_ed_scrollback");
    harness::assert_scrollback_empty(&outcome);
}

#[test]
fn el_right() {
    run_scenario("el_right");
}

#[test]
fn el_left() {
    run_scenario("el_left");
}

#[test]
fn el_all() {
    run_scenario("el_all");
}
