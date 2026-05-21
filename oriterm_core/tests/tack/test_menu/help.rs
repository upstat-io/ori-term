//! Real automated test for tack's `?) help` begin-testing entry.
//!
//! Classification: `BeginTestingStatus::Duplicate { covered_by:
//! "tack_help_redisplays_begin_testing_menu in oriterm_core/tests/tack/test_menu/help.rs..." }`
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
//! AND-combine of `tack_available`, `tic_available`, and
//! `tack_version_supported`) per
//! 2. Spawns tack via `PtySession`.
//! 3. Sends `n` and waits for the begin-testing menu prompt
//! (`tack/test [n] >`) via the framework's `wait_for(...)`
//! contract — NO fixed sleeps, per
//! 4. Sends `?` and waits for the menu prompt to re-appear via
//! `wait_for(...)`.
//! 5. Asserts every entry from `BEGIN_TESTING_INVENTORY` (the
//! SSOT — NOT a hardcoded copy, per ) is still
//! visible (proves `?` did not navigate away).
//! 6. Snapshots the post-`?` grid via insta for byte-level
//! visual regression.
//! 7. Quits tack via the framework's `quit_tack(5)` contract
//! and asserts the child exits cleanly via `success()` —
//! matches every other Section 05 scenario teardown path,
//! per
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
//! menu — that's why this test asserts "all menu entries
//! still visible" rather than asserting byte equality with the
//! 05.0 inventory snapshot). The semantic claim is that `?`
//! does not navigate to a distinct screen, NOT that the buffers
//! match exactly. A separate `tack_help` `Scenario` would still
//! add no incremental signal — the per-entry assertions plus
//! the post-`?` insta snapshot are stronger than a structural
//! `Scenario` would be. (fix: previous version of
//! this rustdoc incorrectly said the grids were "byte-identical".)
//!
//! # Promotion history
//!
//! Originally drafted as a doc-only stub citing the 05.0
//! `begin_testing_inventory` drift gate as covering help
//! behavior. (Codex review-work iteration 2 of M2)
//! correctly noted that the drift gate only sends `n` and
//! never `?`, so the duplicate claim was unverified — promoted
//! to a real test in the same fix. then noted the
//! real test bypassed the version gate and added the
//! `ScenarioRunner::available()` route. then
//! cleaned up the doc-only-stub language in this rustdoc and
//! the inventory comment so the canonical owner is the real
//! test, not the (no-op) drift gate. (final TPR
//! iter 4) noted the test was hand-driving tack with fixed
//! `wait(500)` sleeps and never asserting clean exit, bypassing
//! the Section 04 framework contract — fix uses `wait_for(...)`
//! for synchronization and `quit_tack(5).success()` for the
//! teardown gate. (same iter) noted the test
//! hardcoded the 16 menu entries — fix iterates
//! `BEGIN_TESTING_INVENTORY` directly, eliminating the second
//! source of truth.

use oriterm_test_support::session::PtySession;
use oriterm_test_support::tack_framework::ScenarioRunner;
use oriterm_test_support::tack_framework::scenarios::begin_testing_inventory::{
 BEGIN_TESTING_INVENTORY, BeginTestingStatus,
};
use oriterm_test_support::terminfo::TerminfoEnv;

#[test]
fn tack_help_redisplays_begin_testing_menu() {
 // Verifies for gate on the canonical
 // `ScenarioRunner::available()` AND-combine (tack_available
 // + tic_available + tack_version_supported), NOT on the
 // bare `tack_available() && tic_available()` pair. The
 // version gate is part of Section 05's contract: every
 // tack-spawning test in this submodule must skip cleanly
 // on unsupported tack versions, not just on hosts missing
 // the binary entirely.
 if !ScenarioRunner::available() {
 eprintln!(
 "tack/tic unavailable or tack version unsupported, skipping \
 tack_help_redisplays_begin_testing_menu"
 );
 return;
 }

 let env = TerminfoEnv::compile();
 let mut session = PtySession::spawn_tack(&env, 80, 24);

 // Step 0: wait for tack's initial main-menu prompt
 // (`tack [n] >`) before sending any input. tack prints the
 // terminfo header banner + main menu listing on startup;
 // synchronizing on the prompt ensures we don't race the
 // banner output. 5 s timeout matches the standard Section
 // 05 navigation budget.
 session.wait_for("tack [n] >", 5_000);

 // Step 1: enter the begin-testing menu via `n`. Wait for
 // the begin-testing menu prompt via the framework's
 // wait_for contract (NO fixed sleeps).
 session.send_raw(b"n");
 session.wait_for("tack/test [n] >", 5_000);

 // Sanity baseline: confirm the begin-testing menu is
 // visible. If this fails, the tack invocation shape itself
 // drifted and the test is not measuring what we think it
 // is measuring.
 let baseline = session.grid_text();
 assert!(
 baseline.contains("tack/test [n] >"),
 "expected to be in the begin-testing menu prompt before pressing ?, got:\n{baseline}"
 );
 assert!(
 baseline.contains("x) test modes and glitches"),
 "begin-testing menu sanity check failed (no `x) test modes` entry), got:\n{baseline}"
 );

 // Step 2: press `?` and wait for the menu prompt to
 // re-appear (the empirical claim is that `?` re-displays
 // the same menu inline). The wait_for contract drives the
 // synchronization without a fixed sleep.
 session.send_raw(b"?");
 session.wait_for("tack/test [n] >", 5_000);

 // Step 3: capture the post-`?` grid and assert that EVERY
 // begin-testing menu entry is still visible. Per
 // we iterate `BEGIN_TESTING_INVENTORY` directly instead of
 // hardcoding the 16 entries — the inventory IS the SSOT for
 // the menu content, and the per-entry-format string here
 // (`<key>) <label>`) matches tack's menu output convention
 // (verified by 05.0's drift-gate snapshot).
 let post_help = session.grid_text();
 assert!(
 post_help.contains("tack/test [n] >"),
 "expected `?` to leave us at the begin-testing menu prompt, got:\n{post_help}"
 );

 for entry in BEGIN_TESTING_INVENTORY {
 // Skip the synthetic `os` and similar entries that
 // aren't real menu rows. The current inventory has no
 // such entries, but the filter future-proofs the test:
 // if a future inventory adds a Synthetic-class entry
 // for documentation purposes, it shouldn't break this
 // assertion.
 match entry.status {
 BeginTestingStatus::Scenario
 | BeginTestingStatus::DelegatedToSection { .. }
 | BeginTestingStatus::ExcludedInteractive { .. }
 | BeginTestingStatus::Duplicate { .. } => {}
 }
 let formatted = format!("{}) {}", entry.key, entry.label);
 assert!(
 post_help.contains(&formatted),
 "post-`?` grid missing menu entry {formatted:?} — `?` may have navigated to a \
 distinct screen, in which case BEGIN_TESTING_INVENTORY should reclassify `?` from \
 Duplicate back to Scenario. Got:\n{post_help}"
 );
 }

 // Snapshot the full post-`?` grid for visual regression.
 // This pins the byte-level state so any tack version drift
 // (changed wording, reordered entries, ANY divergence) shows
 // up as a snapshot diff alongside the per-entry assertions.
 insta::assert_snapshot!("tack_help_post_question_mark", post_help);

 // TEARDOWN: quit tack cleanly via the framework's
 // `quit_tack` contract and assert the child exits with
 // success status. This matches every other
 // Section 05 scenario's teardown path — `ScenarioRunner::run`
 // calls the same `quit_tack(TACK_QUIT_MAX_ITERATIONS)` and
 // asserts the result via `assert_quit_status_success`.
 // Hand-driving the test through `PtySession` directly means
 // we have to call this ourselves; doing so closes the loop
 // on the Section 04 LiveSession::finish contract.
 let exit_status = session.quit_tack(5);
 assert!(
 exit_status.success(),
 "tack child exited non-zero after quit_tack: {exit_status:?}"
 );
}
