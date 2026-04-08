//! Sibling tests for the runtime tool-availability probes.
//!
//! Moved out of `session/tests.rs` in the M1 TPR cleanup
//! (TPR-05-006) per `.claude/rules/test-organization.md` rule
//! "one sibling tests.rs per source file."

use super::{infocmp_available, tack_available, tic_available, tool_available, vttest_available};

#[test]
fn tool_available_returns_false_for_nonexistent_binary() {
    assert!(!tool_available(
        "definitely_not_a_real_program_xyz_oriterm",
        "--version"
    ));
}

#[test]
fn tool_available_returns_false_when_binary_spawns_but_exits_nonzero() {
    // SEMANTIC PIN for TPR-05-005: tool_available must check
    // BOTH "spawn succeeded" AND "exit code is success." A binary
    // that launches but reports failure (wrong flag, missing
    // terminfo path, broken install) is NOT available — it would
    // slip past the skip gate and panic downstream instead.
    //
    // Pre-fix, this function used `Command::status().is_ok()`
    // which only tested whether the spawn syscall succeeded.
    // The fix tightens to also check `status.success()`. This
    // test would fail with the old impl because /bin/sh DOES
    // spawn successfully (even with a bogus flag), and only the
    // exit-code check distinguishes the broken probe.
    #[cfg(unix)]
    let result = tool_available("/bin/sh", "--definitely-not-a-real-flag-xyz");
    #[cfg(windows)]
    let result = tool_available("cmd.exe", "/Q/C/X/this-is-not-a-valid-flag");
    assert!(
        !result,
        "tool_available must reject a probe that spawns but exits non-zero \
         (TPR-05-005 regression — only checking Command::status().is_ok() \
         and missing the status.success() check)"
    );
}

#[test]
fn vttest_available_matches_tool_available() {
    assert_eq!(vttest_available(), tool_available("vttest", "--help"));
}

#[test]
fn tic_available_matches_tool_available() {
    assert_eq!(tic_available(), tool_available("tic", "-V"));
}

#[test]
fn infocmp_available_matches_tool_available() {
    assert_eq!(infocmp_available(), tool_available("infocmp", "-V"));
}

#[test]
fn tack_available_matches_tool_available() {
    assert_eq!(tack_available(), tool_available("tack", "-V"));
}
