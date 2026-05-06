//! Test wrappers for the modes scenarios.
//!
//! Const `ScenarioSpec`s, `PhaseSpec`s, and parsers live in
//! `oriterm_test_support::tack_framework::scenarios::modes`. This
//! file just defines `#[test] fn` wrappers that invoke
//! `ScenarioRunner` against those consts.
//!
//! # Per-cap phase scenarios — coded to spec, ignored at runtime
//!
//! Section 05.1 specifies 7 per-cap phase scenarios
//! (`tack_modes_phase_am`, `_bce`, `_bw`, `_km`, `_mir`, `_msgr`,
//! `_xenl`) that each capture tack's emission of the
//! corresponding `(cap)` line during the modes-test sweep. The
//! `PhaseSpec` consts and the test wrappers below are coded to
//! the plan's spec.
//!
//! Empirical investigation under tack v1.08 (verified against both
//! `extra/ori_term.info` AND the host's `xterm-256color` driven
//! interactively through `expect`) showed that tack v1.08's modes
//! test emits ONLY `(os)` content. The full captured output is:
//!
//! ```text
//! \x1B[H\x1B[2J(os) should be true, not false.
//! (os) should be false.
//! (os) over-strike is false in the data base. (os) Done
//! ```
//!
//! No `(am)`, `(bce)`, `(bw)`, `(km)`, `(mir)`, `(msgr)`, or
//! `(xenl)` is ever printed by tack v1.08. Tack tests these caps
//! INTERNALLY (sets up screens that exercise auto-margins,
//! back-color-erase, etc.) but doesn't surface per-cap status —
//! that's been tack's design since 1997. The `(os) Done` line is
//! the test terminator and the only visible signal that the
//! modes test ran successfully.
//!
//! **The 7 per-cap test wrappers below carry `#[ignore]`** with
//! the empirical-finding rationale on each one. The default
//! `cargo test` skips them so the suite stays green; running
//! `cargo test -- --ignored` attempts them against whatever tack
//! is installed. A future tack release that DOES emit per-cap
//! labels (or a new capture strategy that observes the
//! intermediate state) can simply remove the `#[ignore]` without
//! touching the rest of the spec.
//!
//! Section 04's `TACK_MODES_AM` (and its `parse_modes_screen`
//! parser with `KNOWN: &["os"]`) remains the always-active
//! end-to-end coverage of tack's modes screen. It is unchanged
//! from Section 04 and runs on every test invocation.
//!
//! The 05.0.b `PhaseSpec` / `ScenarioRunner::run_phase` /
//! `PtySession::drain_until` infrastructure is the speculative
//! future-use primitive these scenarios consume.

use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::modes::{
    TACK_MODES_AM, TACK_MODES_PHASE_AM, TACK_MODES_PHASE_BCE, TACK_MODES_PHASE_BW,
    TACK_MODES_PHASE_KM, TACK_MODES_PHASE_MIR, TACK_MODES_PHASE_MSGR, TACK_MODES_PHASE_XENL,
};

#[test]
fn tack_modes_am() {
    if !ScenarioRunner::available() {
        eprintln!("SKIP: tack or tic not installed");
        return;
    }

    let outcome = ScenarioRunner::run(&TACK_MODES_AM);

    // Programmatic semantic assertion: the parser found `os`
    // (over-strike) in the modes screen capability list. Tack lists
    // `os` last as the test terminator, so it's always visible in
    // the 24-row viewport at the moment the test reports "Done"
    // (earlier caps like `am`, `bce` scrolled off — Section 05
    // adds per-cap scenarios that capture the right viewport for
    // each). Uses the tokenized `grid_has_paren_token` helper
    // indirectly via `parse_modes_screen` — tack tags every modes
    // result with `(cap_name)` and `grid_has_paren_token` matches
    // exactly that form, so substring collisions cannot false-pass.
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "os"),
        "expected `os` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );

    // Insta snapshot of the full grid for visual regression catching.
    // Use the size-aware snapshot name so size-matrix runs in
    // Section 05 share the snapshot file when the screen is the same.
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

// ============================================================
// 05.1 Per-cap modes phase scenarios — coded to spec, ignored
// at runtime against tack v1.08.
//
// See the file-level rustdoc for the empirical-finding rationale.
// Run `cargo test -- --ignored` to attempt these against an
// alternate tack version. The PhaseSpec consts in
// `crates/oriterm_test_support/src/tack_framework/scenarios/modes/mod.rs`
// are coded to the plan's spec; these wrappers are the runtime
// vehicle that becomes useful the moment tack starts emitting
// per-cap labels.
// ============================================================

const PHASE_IGNORE_REASON: &str = "tack v1.08 does not emit per-cap modes labels — see file rustdoc; \
     run with --ignored to attempt against an alternate tack version";

#[test]
#[ignore = "tack v1.08 does not emit per-cap modes labels — run with --ignored to attempt"]
fn tack_modes_phase_am() {
    if !ScenarioRunner::available() {
        eprintln!("tack/tic unavailable or wrong version, skipping tack_modes_phase_am");
        return;
    }
    let _ = PHASE_IGNORE_REASON; // referenced for documentation discoverability
    let outcome = ScenarioRunner::run_phase(&TACK_MODES_PHASE_AM);
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "am"),
        "expected `am` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
#[ignore = "tack v1.08 does not emit per-cap modes labels — run with --ignored to attempt"]
fn tack_modes_phase_bce() {
    if !ScenarioRunner::available() {
        eprintln!("tack/tic unavailable or wrong version, skipping tack_modes_phase_bce");
        return;
    }
    let outcome = ScenarioRunner::run_phase(&TACK_MODES_PHASE_BCE);
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "bce"),
        "expected `bce` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
#[ignore = "tack v1.08 does not emit per-cap modes labels — run with --ignored to attempt"]
fn tack_modes_phase_bw() {
    if !ScenarioRunner::available() {
        eprintln!("tack/tic unavailable or wrong version, skipping tack_modes_phase_bw");
        return;
    }
    let outcome = ScenarioRunner::run_phase(&TACK_MODES_PHASE_BW);
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "bw"),
        "expected `bw` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
#[ignore = "tack v1.08 does not emit per-cap modes labels — run with --ignored to attempt"]
fn tack_modes_phase_km() {
    if !ScenarioRunner::available() {
        eprintln!("tack/tic unavailable or wrong version, skipping tack_modes_phase_km");
        return;
    }
    let outcome = ScenarioRunner::run_phase(&TACK_MODES_PHASE_KM);
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "km"),
        "expected `km` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
#[ignore = "tack v1.08 does not emit per-cap modes labels — run with --ignored to attempt"]
fn tack_modes_phase_mir() {
    if !ScenarioRunner::available() {
        eprintln!("tack/tic unavailable or wrong version, skipping tack_modes_phase_mir");
        return;
    }
    let outcome = ScenarioRunner::run_phase(&TACK_MODES_PHASE_MIR);
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "mir"),
        "expected `mir` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
#[ignore = "tack v1.08 does not emit per-cap modes labels — run with --ignored to attempt"]
fn tack_modes_phase_msgr() {
    if !ScenarioRunner::available() {
        eprintln!("tack/tic unavailable or wrong version, skipping tack_modes_phase_msgr");
        return;
    }
    let outcome = ScenarioRunner::run_phase(&TACK_MODES_PHASE_MSGR);
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "msgr"),
        "expected `msgr` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}

#[test]
#[ignore = "tack v1.08 does not emit per-cap modes labels — run with --ignored to attempt"]
fn tack_modes_phase_xenl() {
    if !ScenarioRunner::available() {
        eprintln!("tack/tic unavailable or wrong version, skipping tack_modes_phase_xenl");
        return;
    }
    let outcome = ScenarioRunner::run_phase(&TACK_MODES_PHASE_XENL);
    assert!(
        outcome.parsed.capability_labels.iter().any(|c| c == "xenl"),
        "expected `xenl` in capability_labels, got {:?}\nGrid:\n{}",
        outcome.parsed.capability_labels,
        outcome.grid_text,
    );
    insta::assert_snapshot!(outcome.snapshot_name(), outcome.grid_text);
}
