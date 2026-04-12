//! Sibling tests for the runtime tool-availability probes.
//!
//! Moved out of `session/tests.rs` in the M1 TPR cleanup
//! per `.claude/rules/test-organization.md` rule
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
    // SEMANTIC PIN for tool_available must check
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
         (regression — only checking Command::status().is_ok() \
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
    // NOTE: tack uses `-h` (not `-V`) because tack v1.08's `-V` flag
    // exits with status 1 (it prints the version banner to stdout
    // but the binary then exits non-zero). After tightened
    // tool_available to require `status.success()`, the `-V` probe
    // would always report tack as unavailable on every dev/CI host.
    // tack -h prints usage to stderr and exits 0 (verified empirically).
    // Other tools (tic, infocmp, vttest) exit 0 from their canonical
    // version flag and use the same probe pattern unchanged.
    assert_eq!(tack_available(), tool_available("tack", "-h"));
}

#[test]
fn tack_available_pinned_to_h_probe_via_direct_spawn() {
    // SEMANTIC PIN for `tack_available()` must agree with
    // a DIRECT `tack -h` spawn — independent ground truth, decoupled
    // from `tool_available`.
    //
    // Why a separate test from `tack_available_matches_tool_available`:
    // the existing test compares `tack_available()` to
    // `tool_available("tack", "-h")`. On a host where tack IS installed,
    // that DOES catch a `-V` revert (LHS becomes false, RHS stays
    // true → assert_eq fails). On a host WITHOUT tack, both sides
    // return false and the assertion passes vacuously — the
    // regression slips through any CI lane that lacks tack. Codex
    // /review-work flagged the gap as a silent-skip
    // vector that would let the original `-V` regression come back
    // unnoticed in CI.
    //
    // This test strengthens the pin by spawning `tack -h` DIRECTLY
    // (not through `tool_available`, so the truth source cannot
    // co-vary with any future `tool_available` change), then
    // asserting `tack_available()` agrees on hosts where the bare
    // probe succeeds.
    //
    // ALSO: when both `tack -h` succeeds AND `tack -V` fails (the
    // tack v1.08 reality on every dev/CI host that ships ncurses
    // tack), the test additionally asserts `tack_available()` returns
    // true even though the tighter `-V` path would say false. This
    // is the behavioral contract: tack_available() reflects whether
    // tack is RUNNABLE, not whether `-V` happens to exit zero. The
    // regression it catches: a future commit that "simplifies"
    // `tack_available()` back to `tool_available("tack", "-V")`.
    //
    // On hosts WITHOUT tack installed, the test is silently a no-op
    // via early return — same caveat as the existing tack probe
    // tests. The dev environment (Linux/WSL) and Linux CI both have
    // tack, which is where the regression is caught in practice.
    let h_succeeds = std::process::Command::new("tack")
        .arg("-h")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !h_succeeds {
        // tack not installed on this host (or `-h` doesn't exit 0
        // for some reason); cannot pin the behavioral contract here.
        return;
    }

    // tack -h works → tack is installed and runnable. tack_available()
    // MUST agree, regardless of which flag it chose internally.
    assert!(
        tack_available(),
        "tack_available() returned false on a host where `tack -h` \
         exits 0 — the probe flag is wrong (likely reverted to `-V`, \
         which exits 1 on tack v1.08, fails the status.success() \
         check, and silently skips every tack integration test)."
    );

    // Belt-and-braces: explicitly catch the -V revert by checking
    // whether `tack -V` ALSO exits zero. On tack v1.08 (the empirical
    // reality), `-V` exits 1; if it ever exits 0 on this host, the
    // -V probe choice would be just as valid as -h and the test
    // becomes a no-op for that case (we cannot distinguish a correct
    // -V revert from a buggy one).
    let v_succeeds = std::process::Command::new("tack")
        .arg("-V")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);

    if !v_succeeds {
        // tack v1.08 reality: -V exits non-zero. Pin the contract
        // hard: tack_available() must NOT mirror the -V failure. If
        // anyone reverts tack_available() to `tool_available("tack",
        // "-V")`, the function will return false and the assertion
        // above will already have fired — but this comment block
        // documents the regression vector explicitly.
        debug_assert!(
            tack_available(),
            "tack -V exits 1 but tack_available() returned false: \
             probe is using -V (regression)"
        );
    }
}
