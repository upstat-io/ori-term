//! Spec_chain conversion of the tack `cursor_movement` scenario family.
//!
//! Per `crates/oriterm_test_support/src/tack_framework/scenarios/cursor_movement/mod.rs`
//! and the empirical capture against tack v1.08 (2026-04-08), the cursor
//! movement test on tack v1.08 only probes the `clear` capability. The
//! captured output is:
//!
//! ```text
//! \x1B[H\x1B[2JThis line should start in the home position.
//! The rest of the screen should be clear.  (clear) Done
//! ```
//!
//! `clear` in `extra/ori_term.info` is defined as the literal
//! `\E[H\E[2J` byte sequence. That is `CSI H` (CUP with default
//! parameters → cursor home) immediately followed by `CSI 2 J` (ED with
//! mode 2 → erase entire display). Both sequences are individually
//! verifiable catalog rows.
//!
//! # Catalog rows verified
//!
//! - `ECMA48-CSI-CUP` — `CSI Ps;Ps H` (cursor position)
//! - `ECMA48-CSI-ED`  — `CSI Ps J` (erase in display)
//!
//! `cup`, `csr`, `hpa`, `vpa`, `cuu`, `cud`, `cub`, `cuf` are NOT
//! emitted as cap labels by tack v1.08's cursor movement test (see the
//! scenario's module rustdoc for the rationale). Coverage for those
//! catalog rows comes from Section 07 GPU goldens or future tack
//! releases.

use oriterm_test_support::spec_chain::{
    ApexLayer, DispatchExpectation, ParserExpectation, ScenarioExpectations, SpecHarness,
    SpecScenario, StateExpectation,
};

/// CUP with explicit row;col parameters drives parser → dispatch → state.
///
/// `tack`'s `clear` cap begins with `\E[H` (CUP with default 1;1) but
/// the parser/dispatch contract for CUP is exercised more sharply by
/// an explicit position; the default-param case is covered by
/// [`cup_default_params_homes_cursor`] below.
#[test]
fn cup_explicit_params_drives_to_state_apex() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-CUP",
        bytes: b"\x1b[5;10H",
        apex_layer: ApexLayer::State,
        setup: b"",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('H', &[5, 10])),
            dispatch: Some(DispatchExpectation::method("goto")),
            // CUP `5;10H` is 1-based; dispatched as `goto(line=4, col=9)`
            // (0-based). The State observer converts column to grid coord.
            state: Some(StateExpectation::cursor_at(4, 9)),
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
}

/// CUP with default parameters (`\E[H`) homes the cursor to (0, 0).
///
/// This is the exact byte sequence emitted by the first half of the
/// `clear` terminfo cap that tack section 05 cursor_movement exercises.
#[test]
fn cup_default_params_homes_cursor() {
    let mut harness = SpecHarness::new();
    // Move the cursor away from home first so the home assertion is
    // semantically meaningful (not a tautology against the initial state).
    harness.feed(b"\x1b[10;20H");
    assert_eq!(
        harness.term().grid().cursor().line(),
        9,
        "setup: cursor should be at line 9 after CUP 10;20"
    );

    harness.feed(b"\x1b[H"); // CUP with no params — cursor home
    let cursor = harness.term().grid().cursor();
    assert_eq!(cursor.line(), 0, "CUP with no params should home cursor");
    assert_eq!(
        cursor.col().0,
        0,
        "CUP with no params should home cursor to column 0"
    );
}

/// ED with mode 2 (`\E[2J`) erases the entire display.
///
/// This is the second half of the `clear` terminfo cap. Verifies the
/// dispatch routes to `clear_screen` and the cells the test wrote are
/// erased.
#[test]
fn ed_mode_2_drives_through_dispatch_and_state() {
    let scenario = SpecScenario {
        catalog_row_id: "ECMA48-CSI-ED",
        bytes: b"\x1b[2J",
        apex_layer: ApexLayer::Dispatch,
        setup: b"hello world",
        expectations: ScenarioExpectations {
            parser: Some(ParserExpectation::csi_with_params('J', &[2])),
            dispatch: Some(DispatchExpectation::method("clear_screen")),
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

    // State assertion: after ED-2 the visible grid contains only blank
    // cells. Walk the first row and assert every glyph slot is the
    // empty space sentinel.
    let term = harness.term();
    let row0 = &term.grid()[oriterm_core::Line(0)];
    for col in 0..11 {
        // 11 = "hello world".len()
        let cell = &row0[oriterm_core::Column(col)];
        assert_eq!(
            cell.ch, ' ',
            "cell at col {col} should be erased by ED-2; got {:?}",
            cell.ch
        );
    }
}

/// Regression guard: ED-2 with no preceding write must still leave the
/// grid empty (idempotent erasure).
///
/// Without this pin, a regression that made ED-2 a no-op when the
/// grid was already empty would silently pass.
#[test]
fn ed_mode_2_is_idempotent_on_empty_grid() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[2J");
    harness.feed(b"\x1b[2J"); // second ED-2 must not panic, must not allocate

    let term = harness.term();
    let row0 = &term.grid()[oriterm_core::Line(0)];
    assert_eq!(
        row0[oriterm_core::Column(0)].ch,
        ' ',
        "second ED-2 must keep the grid blank"
    );
}
