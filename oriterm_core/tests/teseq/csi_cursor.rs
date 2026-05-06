//! CSI cursor movement scenarios.

use std::path::Path;

use super::harness::{self, TeseqHarness, reseq_available};

fn run_scenario(name: &str) {
    if !reseq_available() {
        eprintln!("SKIP: reseq not installed");
        return;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/cursor")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("csi_cursor_{name}"));
}

#[test]
fn cup_basic() {
    run_scenario("cup_basic");
}

#[test]
fn cup_origin() {
    run_scenario("cup_origin");
}

#[test]
fn cup_clamp() {
    run_scenario("cup_clamp");
}

#[test]
fn cup_clamp_97x33() {
    run_scenario("cup_clamp_97x33");
}

#[test]
fn cup_clamp_120x40() {
    run_scenario("cup_clamp_120x40");
}

#[test]
fn cuu_cud() {
    run_scenario("cuu_cud");
}

#[test]
fn cuf_cub() {
    run_scenario("cuf_cub");
}

#[test]
fn vpa() {
    run_scenario("vpa");
}

#[test]
fn hpa() {
    run_scenario("hpa");
}

#[test]
fn cha() {
    run_scenario("cha");
}
