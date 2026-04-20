//! Tests for per-rung observer functions.

use super::*;
use crate::spec_chain::{
    DispatchExpectation, EffectExpectation, ParserExpectation, RenderableExpectation, SpecHarness,
    StateExpectation,
};

// Parser observer

#[test]
fn parser_observer_matches_csi_action() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H"); // CSI 'H' with params [5, 10]

    let expected = ParserExpectation::csi_with_params('H', &[5, 10]);
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        result.passed,
        "parser observer should match CSI H: {:?}",
        result.failure
    );
}

#[test]
fn parser_observer_fails_on_wrong_action() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H"); // CSI 'H'

    let expected = ParserExpectation::csi_with_params('J', &[5, 10]);
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        !result.passed,
        "parser observer should fail for wrong action"
    );
}

#[test]
fn parser_observer_fails_on_wrong_params() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H"); // CSI 'H' with params [5, 10]

    let expected = ParserExpectation::csi_with_params('H', &[1, 1]);
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        !result.passed,
        "parser observer should fail for wrong params"
    );
}

#[test]
fn parser_observer_matches_execute() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x07"); // BEL (0x07)

    let expected = ParserExpectation {
        action: '\x07',
        params: &[],
        intermediates: &[],
        osc_command: None,
    };
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        result.passed,
        "parser observer should match execute BEL: {:?}",
        result.failure
    );
}

#[test]
fn parser_observer_matches_dcs_hook() {
    let mut harness = SpecHarness::new();
    // DCS q (sixel introducer) — action 'q', no intermediates.
    harness.feed(b"\x1bPq\x1b\\");

    // VTE produces a default param of 0 for DCS q without explicit params.
    let expected = ParserExpectation::dcs('q', &[0]);
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        result.passed,
        "parser observer should match DCS Hook 'q': {:?}",
        result.failure
    );
}

#[test]
fn parser_observer_matches_osc_command() {
    let mut harness = SpecHarness::new();
    // OSC 0 ; title ST — set window title.
    harness.feed(b"\x1b]0;hello\x1b\\");

    let expected = ParserExpectation::osc(0);
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        result.passed,
        "parser observer should match OSC 0: {:?}",
        result.failure
    );
}

#[test]
fn parser_observer_distinguishes_osc_commands() {
    let mut harness = SpecHarness::new();
    // Feed OSC 0 (title set).
    harness.feed(b"\x1b]0;hello\x1b\\");

    // Expect OSC 52 (clipboard) — should NOT match OSC 0.
    let expected = ParserExpectation::osc(52);
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        !result.passed,
        "parser observer should not match OSC 52 when OSC 0 was sent"
    );
}

// Dispatch observer

#[test]
fn dispatch_observer_matches_method_name() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H"); // goto

    let expected = DispatchExpectation::method("goto");
    let result = observe_dispatch(harness.outcome(), &expected);
    assert!(
        result.passed,
        "dispatch observer should match 'goto': {:?}",
        result.failure
    );
}

#[test]
fn dispatch_observer_fails_on_wrong_method() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H"); // goto

    let expected = DispatchExpectation::method("scroll_up");
    let result = observe_dispatch(harness.outcome(), &expected);
    assert!(
        !result.passed,
        "dispatch observer should fail for wrong method"
    );
}

// State observer

#[test]
fn state_observer_matches_cursor_position() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H"); // CUP: row 5, col 10 → 0-based (4, 9)

    let expected = StateExpectation::cursor_at(4, 9);
    let result = observe_state(harness.term(), &expected);
    assert!(
        result.passed,
        "state observer should match cursor at (4,9): {:?}",
        result.failure
    );
}

#[test]
fn state_observer_fails_on_wrong_line() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H");

    let expected = StateExpectation::cursor_at(0, 9);
    let result = observe_state(harness.term(), &expected);
    assert!(!result.passed, "state observer should fail for wrong line");
}

#[test]
fn state_observer_fails_on_wrong_col() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H");

    let expected = StateExpectation::cursor_at(4, 0);
    let result = observe_state(harness.term(), &expected);
    assert!(!result.passed, "state observer should fail for wrong col");
}

// Effect observer

#[test]
fn effect_observer_matches_pty_effect() {
    let mut harness = SpecHarness::new();
    // DA1 query: CSI c → triggers identify_terminal → emits Pty effect.
    harness.feed(b"\x1b[c");

    let expected = EffectExpectation::family("Pty");
    let result = observe_effect(harness.outcome(), &expected);
    assert!(
        result.passed,
        "effect observer should match Pty effect: {:?}",
        result.failure
    );
}

#[test]
fn effect_observer_fails_on_wrong_variant() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[c");

    let expected = EffectExpectation::family("Ui");
    let result = observe_effect(harness.outcome(), &expected);
    assert!(
        !result.passed,
        "effect observer should fail for wrong variant"
    );
}

#[test]
fn effect_observer_fails_on_empty_effects() {
    let mut harness = SpecHarness::new();
    // Feed a CUP sequence — no effects emitted.
    harness.feed(b"\x1b[5;10H");

    let expected = EffectExpectation::family("Pty");
    let result = observe_effect(harness.outcome(), &expected);
    assert!(
        !result.passed,
        "effect observer should fail when no effects emitted"
    );
}

#[test]
fn effect_observer_matches_pty_sub_variant() {
    let mut harness = SpecHarness::new();
    // DA1 query emits Pty::Write { kind: DeviceAttribute }.
    harness.feed(b"\x1b[c");

    let expected = EffectExpectation::pty("DeviceAttribute");
    let result = observe_effect(harness.outcome(), &expected);
    assert!(
        result.passed,
        "effect observer should match Pty::DeviceAttribute: {:?}",
        result.failure
    );
}

#[test]
fn effect_observer_fails_on_wrong_sub_variant() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[c"); // DA1 → DeviceAttribute

    let expected = EffectExpectation::pty("CursorReport");
    let result = observe_effect(harness.outcome(), &expected);
    assert!(
        !result.passed,
        "effect observer should fail for wrong sub-variant"
    );
}

// Renderable observer
//
// Stub-regression pins. With the original `RungResult::pass(rung)`-only
// stub, the mismatch tests below would PASS the assertion (stub returned
// `passed: true`), so the test would FAIL — the negative pin is what
// proves the observer actually inspects the snapshot. After the observer
// implementation lands, both the matching and the mismatched expectations
// behave correctly: positive case passes, negative case fails.

#[test]
fn renderable_observer_hyperlink_matches() {
    let mut harness = SpecHarness::new();
    // OSC 8 ; ; http://right.com ST X OSC 8 ; ; ST — attach URI, write 'X', clear.
    harness.feed(b"\x1b]8;;http://right.com\x1b\\X\x1b]8;;\x1b\\");

    let expected = RenderableExpectation {
        hyperlink_at: Some((0, 0, "http://right.com")),
        ..Default::default()
    };
    let result = observe_renderable(harness.term(), expected);
    assert!(
        result.passed,
        "renderable observer should match correct hyperlink URI: {:?}",
        result.failure
    );
}

#[test]
fn renderable_observer_fails_on_wrong_hyperlink_uri() {
    // STUB-REGRESSION PIN: if `observe_renderable` regresses to the
    // `RungResult::pass(rung)`-only stub, this assertion FAILS — the stub
    // returns `passed: true` against any expectation, including a wrong
    // URI. The observer MUST detect the mismatch and return `passed: false`.
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]8;;http://right.com\x1b\\X\x1b]8;;\x1b\\");

    let expected = RenderableExpectation {
        hyperlink_at: Some((0, 0, "http://wrong.com")),
        ..Default::default()
    };
    let result = observe_renderable(harness.term(), expected);
    assert!(
        !result.passed,
        "renderable observer must fail when expected URI differs from snapshot URI \
         (if this passes, the observer regressed to the stub)"
    );
}

#[test]
fn renderable_observer_fails_when_hyperlink_absent() {
    // Negative pin: cell exists but has NO hyperlink attached; expecting
    // one must fail (the stub would have passed this too).
    let mut harness = SpecHarness::new();
    harness.feed(b"X");

    let expected = RenderableExpectation {
        hyperlink_at: Some((0, 0, "http://example.com")),
        ..Default::default()
    };
    let result = observe_renderable(harness.term(), expected);
    assert!(
        !result.passed,
        "renderable observer must fail when cell has no hyperlink but expectation does"
    );
}

#[test]
fn renderable_observer_cursor_position_matches() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H"); // CUP row 5 col 10 → 0-based (4, 9)

    let expected = RenderableExpectation {
        cursor_position: Some((4, 9)),
        ..Default::default()
    };
    let result = observe_renderable(harness.term(), expected);
    assert!(
        result.passed,
        "renderable observer should match cursor at (4,9): {:?}",
        result.failure
    );
}

#[test]
fn renderable_observer_fails_on_wrong_cursor_position() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b[5;10H");

    let expected = RenderableExpectation {
        cursor_position: Some((0, 0)),
        ..Default::default()
    };
    let result = observe_renderable(harness.term(), expected);
    assert!(
        !result.passed,
        "renderable observer must fail when cursor position differs from snapshot"
    );
}

#[test]
fn renderable_observer_cell_char_matches() {
    let mut harness = SpecHarness::new();
    harness.feed(b"X");

    let expected = RenderableExpectation {
        cells: Some(&[(0, 0, 'X')]),
        ..Default::default()
    };
    let result = observe_renderable(harness.term(), expected);
    assert!(
        result.passed,
        "renderable observer should match cell 'X' at (0,0): {:?}",
        result.failure
    );
}

#[test]
fn renderable_observer_fails_on_wrong_cell_char() {
    let mut harness = SpecHarness::new();
    harness.feed(b"X");

    let expected = RenderableExpectation {
        cells: Some(&[(0, 0, 'Y')]),
        ..Default::default()
    };
    let result = observe_renderable(harness.term(), expected);
    assert!(
        !result.passed,
        "renderable observer must fail when expected cell char differs from snapshot"
    );
}

#[test]
fn renderable_observer_default_expectation_passes() {
    // Empty expectation (no fields set) must always pass — there is
    // nothing to disprove. This guards against an over-eager observer
    // that fails on `Default::default()`.
    let harness = SpecHarness::new();
    let result = observe_renderable(harness.term(), RenderableExpectation::default());
    assert!(
        result.passed,
        "renderable observer with empty expectation must pass: {:?}",
        result.failure
    );
}
