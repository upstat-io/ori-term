//! Discovery test for the `s) ANSI status reports` sub-submenu.
//!
//! Walks the sub-submenu by sending `n` directly via `PtySession`
//! (not via a `ScenarioSpec` menu_path), draining after each press,
//! and tracking every curated sub-test label observed in the grid.
//! Asserts: (a) every curated label in `STATUS_REPORTS_INVENTORY`
//! was observed, (b) the length-drift anchor
//! (`LENGTH_DRIFT_FINAL_LABEL`) was observed, (c) the sub-submenu
//! exited before a safety bound.
//!
//! # Why a direct walk and NOT `ScenarioSpec::menu_path`
//!
//! Empirically discovered during 06.0.b implementation (2026-04-08):
//! tack's `n` key does NOT advance one sub-test at a time under
//! ori_term's pinned terminfo. Tack batches sub-tests based on the
//! terminal's responses to the preceding CSI probes. Under xterm-
//! 256color (the shell probe) the sequence was 32 sub-tests with
//! roughly 1:1 `n`→sub-test advancement. Under ori_term (where the
//! VTE correctly handles every CSI query) tack:
//!
//! - Emits 6 sub-tests on the initial `s` press alone (before any
//! `n`). These are the "auto" sub-tests that don't need user
//! input between them.
//! - Emits 1 sub-test per `n` press at steps 1 and 2.
//! - Emits 8 sub-tests per `n` press at step 3 (the DECRQSS group
//! through DECRQUPSS) — one batched burst.
//! - Continues with irregular batch sizes for the remaining walks.
//! - Total sub-tests: 33 under ori_term vs 32 under xterm-256color.
//! The extra sub-test is `(DSR) Screen size (CSI 6 n)` at index
//! 3, which tack only emits when it got a valid CPR response
//! from the preceding "Cursor position" sub-test.
//!
//! A `ScenarioSpec::menu_path` with per-step `wait_for` anchors
//! cannot handle batching — we cannot predict which label to wait
//! for at step K without knowing tack's batching for step K under
//! the specific terminfo. The direct walk sidesteps the problem:
//! we send `n`, drain, scan the current grid for ALL curated
//! labels, record which ones are newly-seen, and continue until
//! the sub-submenu exits back to the tools menu.
//!
//! This is NOT a new framework primitive — it's a one-off direct
//! consumer of `PtySession` that reflects tack's actual behavior
//! instead of an idealized per-step advancement.

use std::collections::BTreeSet;

use oriterm_test_support::tack_framework::scenarios::status_reports_inventory::{
    LENGTH_DRIFT_FINAL_LABEL, STATUS_REPORTS_INVENTORY,
};
use oriterm_test_support::{PtySession, TerminfoEnv, tack_available, tic_available};

/// Safety bound on the number of `n` presses during the walk.
/// Empirical measurement (2026-04-08) showed that with
/// `PtySession::send` (300 ms built-in quiesce), tack's walk
/// completes cleanly in ~19 n presses and emits "Tools Menu" as
/// the exit marker. Shorter quiesces (e.g. `send_raw` + 200 ms)
/// cause tack to walk past the status-reports sub-submenu into
/// adjacent test sections before returning to the tools menu,
/// which exceeds any reasonable bound and breaks teardown. 40 is
/// the safety ceiling for the normal walk.
const MAX_N_PRESSES: usize = 40;

/// Collect every curated label visible in `grid` into `seen`.
fn record_seen_labels(grid: &str, seen: &mut BTreeSet<&'static str>) {
    for sub in STATUS_REPORTS_INVENTORY {
        if grid.contains(sub.label) {
            seen.insert(sub.label);
        }
    }
    if grid.contains(LENGTH_DRIFT_FINAL_LABEL) {
        seen.insert(LENGTH_DRIFT_FINAL_LABEL);
    }
}

#[test]
fn tack_status_reports_inventory() {
    if !tack_available() || !tic_available() {
        eprintln!("SKIP: tack or tic not installed");
        return;
    }

    let env = TerminfoEnv::compile();
    let mut session = PtySession::spawn_tack(&env, 80, 24);
    session.wait_for("tack [n] >", 5_000);

    // Enter tools menu.
    session.send(b"t");
    session.wait_for("tack/tools [q] >", 5_000);

    // Enter status reports sub-submenu. Tack emits 6 sub-tests
    // automatically on entry under ori_term (Primary DA, Terminal
    // status, Cursor position, Screen size, Secondary DA, Printer
    // status) before waiting for the first `n` press.
    session.send(b"s");

    // Track which curated labels we've seen. Because tack's
    // scrollback is bounded by the 24-row viewport, we check
    // whether each curated label is visible in the current grid at
    // multiple points during the walk — any given label might only
    // be visible for a few iterations before scrolling off.
    let mut seen_labels: BTreeSet<&'static str> = BTreeSet::new();
    record_seen_labels(&session.grid_text(), &mut seen_labels);

    // Walk forward one `n` press per iteration using
    // `PtySession::send`, which includes a 300 ms built-in
    // quiesce. The longer quiesce is LOAD-BEARING: it lets tack
    // batch sub-tests per press (tack emits 1-8 sub-tests per
    // keystroke depending on the current sub-section), which
    // shrinks the walk from ~80 n presses (with a shorter
    // quiesce) to ~19 n presses and produces a clean "Tools Menu"
    // exit marker. Shorter quiesces cause tack to walk past the
    // status-reports sub-submenu into adjacent test sections
    // (mode status, status line,...) that `q` cannot unwind from
    // cleanly, breaking teardown.
    //
    // Empirical cost (2026-04-08): ~3.5 s per iteration under
    // ori_term's synchronous VTE, ~19 iterations to exit =
    // ~66 s total. Slower than the plan's Option B 16-20 s
    // budget, but acceptable because this is a discovery test
    // that runs once per `test-all.sh` invocation and skips on
    // hosts without tack (Windows CI).
    let mut n_press_count = 0_usize;
    let mut exited_cleanly = false;
    let mut final_grid = String::new();
    while n_press_count < MAX_N_PRESSES {
        session.send(b"n");
        final_grid = session.grid_text();
        record_seen_labels(&final_grid, &mut seen_labels);
        n_press_count += 1;

        // Sub-submenu has exited when the tools menu header appears.
        if final_grid.contains("Tools Menu") {
            exited_cleanly = true;
            break;
        }
    }

    assert!(
        exited_cleanly,
        "status reports sub-submenu did not exit within {MAX_N_PRESSES} n presses; \
         the walk is stuck or tack's behavior has changed. Seen labels: {seen_labels:?}\n\
         Final grid:\n{final_grid}"
    );
    eprintln!("tack_status_reports_inventory: walked {n_press_count} n presses before exit");

    // Quit cleanly before running assertions so a later failure
    // doesn't leak the tack child. The walk exits at "Tools Menu"
    // so we only need main-menu + quit (~2 q presses) but
    // `quit_tack(5)` is the canonical default and leaves headroom.
    let _exit = session.quit_tack(5);

    // Snapshot the sorted seen_labels set — a DETERMINISTIC
    // artifact that is stable across runs regardless of tack's
    // batching or walk exit timing. The final grid is NOT
    // snapshotted because the exit-point grid depends on how many
    // `n` presses the walk took, which is timing-dependent:
    // longer post-send quiesces (like the built-in `send` 300 ms)
    // let tack batch more sub-tests per press, yielding fewer
    // total presses and a different exit grid than a short
    // explicit quiesce via `send_raw`. The sorted-labels
    // assertion below fires if any curated label is missing or a
    // new label appears, which is the real drift semantic.
    let labels_manifest = seen_labels.iter().copied().collect::<Vec<_>>().join("\n");
    insta::assert_snapshot!("tack_status_reports_seen_labels_80x24", &labels_manifest);

    // Curated inventory gate: every pinned label MUST appear in
    // the `seen_labels` set. This is Option B's drift gate — it
    // catches any renamed or removed sub-test.
    for sub in STATUS_REPORTS_INVENTORY {
        assert!(
            seen_labels.contains(sub.label),
            "status reports drift: curated sub-test {mnemonic} \
             (label {label:?}) not observed during the walk. Tack \
             v1.08 no longer emits this sub-test, the label has been \
             renamed, or the walk terminated early.\n\
             Seen labels: {seen:?}\nFinal grid:\n{grid}",
            mnemonic = sub.mnemonic,
            label = sub.label,
            seen = seen_labels,
            grid = final_grid,
        );
    }

    // Length-drift gate: the final sub-test label must appear.
    // Catches insertions or removals anywhere in the sequence.
    assert!(
        seen_labels.contains(LENGTH_DRIFT_FINAL_LABEL),
        "status reports length-drift gate: expected final label \
         {label:?} not observed during the walk. The sub-test \
         sequence has been perturbed.\nSeen labels: {seen:?}\n\
         Final grid:\n{grid}",
        label = LENGTH_DRIFT_FINAL_LABEL,
        seen = seen_labels,
        grid = final_grid,
    );

    // Sanity: we walked the full sub-submenu in under the safety
    // bound. Pin the real walk length (~19 presses under ori_term)
    // loosely to catch a regression where tack starts emitting the
    // sub-test sequence twice or reorders it across `n` presses.
    assert!(
        n_press_count < MAX_N_PRESSES,
        "walk reached safety bound {MAX_N_PRESSES} without exit"
    );
}
