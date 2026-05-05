//! Spec_chain conversion of the tack `sgr_modes` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/sgr_modes/mod.rs`,
//! tack's `t -> g) ANSI SGR modes` tool draws a stable 8×10 table of
//! `Mode 0`..`Mode 79` labels, each wearing its corresponding SGR
//! attribute (tack emits `CSI N m` for every N in 0..80 as it draws
//! each label). The test asserts ori_term parsed each SGR emit into
//! the right `CellFlags` / `Color` on the cursor template.
//!
//! # Catalog rows verified
//!
//! - `ECMA48-SGR-0` — Reset all attributes (`CSI 0 m`, also `CSI m`)
//! - `ECMA48-SGR-1` — Bold (`CSI 1 m`)
//! - `ECMA48-SGR-4` — Underline (`CSI 4 m`)
//! - `ECMA48-SGR-7` — Reverse (`CSI 7 m`)
//!
//! The remaining 76 modes tack emits are covered by direct-VTE SGR
//! xcheck tests (`oriterm_core/src/term/handler/tack_cap_xcheck/`) and
//! unit tests on `sgr::apply` (`oriterm_core/src/term/handler/sgr.rs`).
//! Replaying all 80 here would be `LEAK:algorithmic-duplication` —
//! the canonical per-mode coverage already lives at the handler
//! layer. The four modes verified here are the most visually
//! distinctive mode labels on tack's SGR screen (reset, bold,
//! underline, reverse), anchoring the spec_chain layer while leaving
//! per-mode exhaustiveness where it belongs.

use oriterm_core::cell::CellFlags;
use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, ParserExpectation, ScenarioExpectations, SpecHarness,
    SpecScenario,
};

// --- SGR 1 (Bold) ----------------------------------------------------------

/// `CSI 1 m` sets the BOLD flag on the cursor template.
///
/// Default template flags are empty; the assertion proves the SGR 1
/// handler actually transitions the flag. Without the post-assertion,
/// a no-op handler would still pass the "flags include BOLD" check
/// via a regression that left BOLD pre-set in some other code path.
#[test]
fn sgr_1_sets_bold_flag() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-SGR-1",
        bytes: b"\x1b[1m",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('m', &[1])),
            dispatch: Some(DispatchExpectation::method("terminal_attribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    assert!(
        !harness
            .term()
            .grid()
            .cursor()
            .template()
            .flags
            .contains(CellFlags::BOLD),
        "default cursor template must NOT include BOLD"
    );

    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert!(
        harness
            .term()
            .grid()
            .cursor()
            .template()
            .flags
            .contains(CellFlags::BOLD),
        "after CSI 1 m, cursor template must include BOLD"
    );
}

// --- SGR 4 (Underline) -----------------------------------------------------

/// `CSI 4 m` sets the UNDERLINE flag on the cursor template.
///
/// Underline sets the base `UNDERLINE` flag — ori_term's underline
/// variants (curly / dotted / dashed / double) are separate flags that
/// are NOT set by plain SGR 4; the assertion below confirms the base
/// flag transitions without accidentally touching the variants.
#[test]
fn sgr_4_sets_underline_flag() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-SGR-4",
        bytes: b"\x1b[4m",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('m', &[4])),
            dispatch: Some(DispatchExpectation::method("terminal_attribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    let flags = harness.term().grid().cursor().template().flags;
    assert!(
        flags.contains(CellFlags::UNDERLINE),
        "after CSI 4 m, UNDERLINE must be set; flags={flags:?}"
    );
    assert!(
        !flags.contains(CellFlags::DOUBLE_UNDERLINE),
        "SGR 4 must not accidentally set DOUBLE_UNDERLINE; flags={flags:?}"
    );
    assert!(
        !flags.contains(CellFlags::CURLY_UNDERLINE),
        "SGR 4 must not accidentally set CURLY_UNDERLINE; flags={flags:?}"
    );
}

// --- SGR 7 (Reverse) -------------------------------------------------------

/// `CSI 7 m` sets the INVERSE flag on the cursor template.
///
/// Tack's SGR table renders `Mode 7` with reversed video; this pin
/// verifies the flag is actually set, not merely that the sequence
/// was parsed.
#[test]
fn sgr_7_sets_inverse_flag() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-SGR-7",
        bytes: b"\x1b[7m",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('m', &[7])),
            dispatch: Some(DispatchExpectation::method("terminal_attribute")),
            ..ScenarioExpectations::default()
        },
    };

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert!(
        harness
            .term()
            .grid()
            .cursor()
            .template()
            .flags
            .contains(CellFlags::INVERSE),
        "after CSI 7 m, cursor template must include INVERSE"
    );
}

// --- SGR 0 (Reset) ---------------------------------------------------------

/// `CSI 0 m` resets all SGR flags after they have been set.
///
/// Matrix: feed SGR 1 + 4 + 7 first to set BOLD + UNDERLINE + INVERSE,
/// then feed SGR 0 and assert every flag returns to the default
/// (empty) state. This guarantees SGR 0 actually resets, not just
/// leaves flags unchanged.
#[test]
fn sgr_0_clears_all_set_flags() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-SGR-0",
        bytes: b"\x1b[0m",
        apex_layer: ApexLayer::State,
        setup: b"\x1b[1m\x1b[4m\x1b[7m",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('m', &[0])),
            dispatch: Some(DispatchExpectation::method("terminal_attribute")),
            ..ScenarioExpectations::default()
        },
    };

    // Pre-flight the setup on a separate harness so we can pin that
    // the setup bytes actually set BOLD + UNDERLINE + INVERSE (without
    // this pin, a regression that silently broke SGR 1/4/7 would let
    // the reset check pass vacuously — empty → empty).
    let mut pre_harness = SpecHarness::new();
    pre_harness.feed(scenario.setup);
    assert!(
        pre_harness
            .term()
            .grid()
            .cursor()
            .template()
            .flags
            .contains(CellFlags::BOLD | CellFlags::UNDERLINE | CellFlags::INVERSE),
        "setup bytes must set BOLD + UNDERLINE + INVERSE on a fresh harness"
    );

    let mut harness = SpecHarness::new();
    let results = harness.run_scenario(&scenario);
    for r in &results {
        assert!(
            r.passed,
            "rung {:?} failed: {}",
            r.rung_name,
            r.failure.as_deref().unwrap_or("(no message)")
        );
    }

    assert!(
        harness.term().grid().cursor().template().flags.is_empty(),
        "after CSI 0 m, cursor template flags must be empty; got {:?}",
        harness.term().grid().cursor().template().flags
    );
}

/// Empty SGR (`CSI m`) is equivalent to `CSI 0 m` per ECMA-48 §8.3.117.
///
/// Regression guard that doubles as the "bare SGR resets" positive: after
/// setting BOLD, a lone `CSI m` must clear it. A regression that
/// treated empty-param SGR as a no-op would fail here.
#[test]
fn empty_sgr_resets_like_sgr_0() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[1m");
    assert!(
        harness
            .term()
            .grid()
            .cursor()
            .template()
            .flags
            .contains(CellFlags::BOLD)
    );

    harness.feed(b"\x1b[m");

    assert!(
        harness.term().grid().cursor().template().flags.is_empty(),
        "`CSI m` (empty param) must reset like `CSI 0 m`"
    );
}

// --- Negative pins ---------------------------------------------------------

/// Regression guard: SGR 1 must NOT set INVERSE (cross-mode isolation).
///
/// Without this pin, a regression that bulk-set multiple flags on any
/// SGR sequence would pass the positive per-mode pins above (each
/// checks only its own flag).
#[test]
fn sgr_1_does_not_set_inverse() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[1m");
    assert!(
        !harness
            .term()
            .grid()
            .cursor()
            .template()
            .flags
            .contains(CellFlags::INVERSE),
        "SGR 1 must not set INVERSE"
    );
}
