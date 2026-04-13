//! Tests for per-rung observer functions.

use super::*;
use crate::spec_chain::{
    DispatchExpectation, EffectExpectation, ParserExpectation, SpecHarness, StateExpectation,
};

// ── Parser observer ─────────────────────────────────────────────────

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
    let expected = ParserExpectation {
        action: 'q',
        params: &[0],
        intermediates: &[],
    };
    let result = observe_parser(harness.outcome(), &expected);
    assert!(
        result.passed,
        "parser observer should match DCS Hook 'q': {:?}",
        result.failure
    );
}

// ── Dispatch observer ───────────────────────────────────────────────

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

// ── State observer ──────────────────────────────────────────────────

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

// ── Effect observer ─────────────────────────────────────────────────

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
