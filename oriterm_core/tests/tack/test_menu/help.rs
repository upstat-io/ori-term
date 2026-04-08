//! Real automated test for tack's `?) help` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::Duplicate { covered_by:
//! "tack_help_redisplays_begin_testing_menu in oriterm_core/tests/tack/test_menu/help.rs ..." }`
//! per `crates/oriterm_test_support/src/tack_framework/scenarios/begin_testing_inventory/mod.rs`.
//!
//! # What this test does
//!
//! `tack_help_redisplays_begin_testing_menu` (the `#[test] fn`
//! below) is the canonical home for verifying the empirical
//! claim that pressing `?` from tack's begin-testing menu
//! re-displays the same menu inline. The test:
//!
//! 1. Gates on `ScenarioRunner::available()` (the canonical
//!    AND-combine of `tack_available`, `tic_available`, and
//!    `tack_version_supported`) per TPR-05-019.
//! 2. Spawns tack via `PtySession`.
//! 3. Sends `n` to enter the begin-testing menu.
//! 4. Captures the baseline grid + sanity-checks the menu
//!    prompt is present.
//! 5. Sends `?`.
//! 6. Captures the post-`?` grid.
//! 7. Asserts every one of the 16 begin-testing menu entries
//!    is still visible (proves `?` did not navigate away).
//! 8. Snapshots the post-`?` grid via insta for byte-level
//!    visual regression.
//!
//! If a future tack release makes `?` a distinct help screen,
//! the per-entry assertions will fail and the insta snapshot
//! will diff — that signals an inventory reclassification is
//! needed (revert `?` from `Duplicate` back to `Scenario`).
//!
//! # Why this is a `Duplicate`, not a `Scenario`
//!
//! Originally classified as `Scenario` in the inventory, the
//! 05.4b empirical probe (2026-04-08) discovered that pressing
//! `?` from the begin-testing menu does NOT navigate to a
//! separate help screen — it simply re-displays the same
//! begin-testing menu inline. NOTE: the post-`?` grid is NOT
//! byte-identical to the pre-`?` grid (the post-`?` viewport
//! contains additional scroll history above the re-rendered
//! menu — that's why this test asserts "all 16 menu entries
//! still visible" rather than asserting byte equality with the
//! 05.0 inventory snapshot). The semantic claim is that `?`
//! does not navigate to a distinct screen, NOT that the buffers
//! match exactly. A separate `tack_help` `Scenario` would still
//! add no incremental signal — the per-entry assertions plus
//! the post-`?` insta snapshot are stronger than a structural
//! `Scenario` would be. (TPR-05-023 fix: previous version of
//! this rustdoc incorrectly said the grids were "byte-identical".)
//!
//! # Promotion history (TPR-05-017 -> TPR-05-022)
//!
//! Originally drafted as a doc-only stub citing the 05.0
//! `begin_testing_inventory` drift gate as covering help
//! behavior. TPR-05-017 (Codex /review-work iteration 2 of M2)
//! correctly noted that the drift gate only sends `n` and
//! never `?`, so the duplicate claim was unverified — promoted
//! to a real test in the same fix. TPR-05-019 then noted the
//! real test bypassed the version gate and added the
//! `ScenarioRunner::available()` route. TPR-05-022 then
//! cleaned up the doc-only-stub language in this rustdoc and
//! the inventory comment so the canonical owner is the real
//! test, not the (no-op) drift gate.

use oriterm_test_support::session::PtySession;
use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::terminfo::TerminfoEnv;

#[test]
fn tack_help_redisplays_begin_testing_menu() {
    // SEMANTIC PIN for TPR-05-019: gate on the canonical
    // `ScenarioRunner::available()` AND-combine (tack_available
    // + tic_available + tack_version_supported), NOT on the
    // bare `tack_available() && tic_available()` pair. The
    // version gate is part of Section 05's contract: every
    // tack-spawning test in this submodule must skip cleanly
    // on unsupported tack versions, not just on hosts missing
    // the binary entirely. The previous version of this test
    // bypassed the version gate by spawning tack directly via
    // PtySession, which would have caused snapshot churn or
    // hard failures when tack upgrades.
    if !ScenarioRunner::available() {
        eprintln!(
            "tack/tic unavailable or tack version unsupported, skipping \
             tack_help_redisplays_begin_testing_menu"
        );
        return;
    }

    let env = TerminfoEnv::compile();
    let mut session = PtySession::spawn_tack(&env, 80, 24);
    session.wait(500);

    // Step 1: enter the begin-testing menu via `n`. Same first
    // step as every Section 05 scenario.
    session.send(b"n");
    session.wait(500);

    // Sanity baseline: confirm we're in the begin-testing menu
    // before pressing `?`. If this fails, the tack invocation
    // shape itself drifted and the test is not measuring what
    // we think it's measuring.
    let baseline = session.grid_text();
    assert!(
        baseline.contains("tack/test [n] >"),
        "expected to be in the begin-testing menu prompt before pressing ?, got:\n{baseline}"
    );
    assert!(
        baseline.contains("x) test modes and glitches"),
        "begin-testing menu sanity check failed (no `x) test modes` entry), got:\n{baseline}"
    );

    // Step 2: press `?` and let tack respond.
    session.send(b"?");
    session.wait(500);

    // Step 3: capture the post-`?` grid and assert that EVERY
    // begin-testing menu entry is still visible. The empirical
    // claim from the 05.4b probe is that `?` re-displays the
    // same menu inline; if any of these per-entry assertions
    // fails, that claim is wrong and tack actually navigated
    // to a distinct screen — which means `?` should be
    // reclassified from Duplicate back to Scenario in
    // BEGIN_TESTING_INVENTORY.
    let post_help = session.grid_text();
    assert!(
        post_help.contains("tack/test [n] >"),
        "expected `?` to leave us at the begin-testing menu prompt, got:\n{post_help}"
    );

    // Pin every key inventory entry from the begin-testing
    // menu. These are all single-line `<key>) <label>` entries
    // verified by the begin_testing_inventory drift gate test.
    // If `?` ever navigates to a distinct screen, the post-`?`
    // grid would not contain these entries and at least one
    // assertion would fire.
    for entry in [
        "e) edit terminfo",
        "i) send reset and init",
        "x) test modes and glitches",
        "a) test alternate character set and graphic rendition",
        "c) test color",
        "m) test cursor movement",
        "f) test function keys",
        "p) test padding and string capabilities",
        "P) test printer",
        "/) test a specific capability",
        "t) auto generate pad delays",
        "n) run standard tests",
        "r) repeat test",
        "s) skip to next test",
        "q) quit",
        "?) help",
    ] {
        assert!(
            post_help.contains(entry),
            "post-`?` grid missing menu entry {entry:?} — `?` may have navigated to a distinct \
             screen, in which case BEGIN_TESTING_INVENTORY should reclassify `?` from Duplicate \
             back to Scenario. Got:\n{post_help}"
        );
    }

    // Snapshot the full post-`?` grid for visual regression.
    // This pins the byte-level state so any tack version drift
    // (changed wording, reordered entries, ANY divergence) shows
    // up as a snapshot diff alongside the per-entry assertions.
    insta::assert_snapshot!("tack_help_post_question_mark", post_help);
}
