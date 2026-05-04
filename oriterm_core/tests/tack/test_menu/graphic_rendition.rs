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

 // Verifies for same as the ACS wrapper. Tack
    // v1.08 combines the ACS and graphic-rendition tests under one
    // menu key (`a`) and the test only probes (bel). No SGR labels
    // (bold/dim/underline/blink/reverse/invis) appear on the screen
    // — verified empirically. The only honest semantic claim is
    // that (bel) was invoked. This wrapper has a distinct screen_id
    // from the ACS wrapper so snapshots do not collide and the
    // parser path differs (SGR-label scan vs. line-drawing-char
    // count), but the asserted facts on tack v1.08 are the same.
    // SGR cap coverage will come from a different source (Section
    // 07's GPU goldens for actual SGR rendering, or vttest menu
    // entries that DO emit SGR sample text).
    assert!(
        outcome.grid_text.contains("Testing bell"),
        "expected captured grid to contain 'Testing bell' header (bel cap pin), got:\n{}",
        outcome.grid_text
    );
    assert!(
        outcome.grid_text.contains("(bel)"),
        "expected captured grid to contain '(bel)' parenthesized cap (bel cap pin), got:\n{}",
        outcome.grid_text
    );

    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
