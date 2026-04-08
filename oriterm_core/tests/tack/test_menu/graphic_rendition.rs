//! Test wrapper for the graphic-rendition (SGR) scenario.
//!
//! Const `ScenarioSpec` and parser live in
//! `oriterm_test_support::tack_framework::scenarios::graphic_rendition`.
//! This file just defines a `#[test] fn` wrapper that invokes
//! `ScenarioRunner::run` against the const.
//!
//! # Empirical reality (tack v1.08)
//!
//! Same screen as the ACS test (tack v1.08 combines the two
//! under one `a) test alternate character set and graphic
//! rendition` menu key). Tack only probes the `bel` capability
//! and reports `Done` — no SGR sample text appears on the
//! captured grid. See
//! `crates/oriterm_test_support/src/tack_framework/scenarios/graphic_rendition/mod.rs`
//! rustdoc for the empirical evidence and the hybrid coverage
//! strategy.
//!
//! This scenario shares the same captured grid content as
//! `tack_acs_graphic_chars` against tack v1.08, but uses a
//! different `screen_id` so the snapshots do not collide and
//! parses for SGR labels instead of line-drawing characters.
//! The two scenarios preserve the plan's spec structure and let
//! a future runner gain per-aspect coverage without
//! restructuring.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::graphic_rendition::TACK_GRAPHIC_RENDITION_SGR;

#[test]
fn tack_graphic_rendition_sgr() {
    if !ScenarioRunner::available() {
        eprintln!("tack or tic unavailable, skipping tack_graphic_rendition_sgr");
        return;
    }
    let outcome = ScenarioRunner::run(&TACK_GRAPHIC_RENDITION_SGR);

    assert!(
        outcome.grid_text.contains("Done"),
        "expected captured grid to contain 'Done' terminator, got:\n{}",
        outcome.grid_text
    );

    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
