//! Section 06.1 walker test for tack's `s) ANSI status reports`
//! sub-submenu.
//!
//! **Design note (plan deviation from 06.1 as originally written).**
//! The original 06.1 plan designed per-sub-test `ScenarioSpec` values
//! with fixed menu_paths like `[t, s, n, n]`, assuming `n` presses
//! advance one sub-test at a time. 06.0.b's empirical discovery
//! established that tack BATCHES sub-tests under `ori_term` terminfo
//! (the VTE correctly answers every CSI probe, so tack races through
//! multiple sub-tests per keypress), so per-step `wait_for` anchors
//! cannot target a specific sub-test. 06.1 therefore mirrors 06.0.b's
//! direct-walker pattern: spawn tack, walk the sub-submenu via `send`
//! (300 ms quiesce), capture the grid after each `n` press into a
//! history vec, and extract each curated sub-test's response from the
//! accumulated history at the end. Response parsing uses helpers in
//! `oriterm_test_support::tack_framework::scenarios::status_reports`.
//!
//! Section 06 task-checkbox traceability:
//! - Declare per-sub-test ScenarioSpec consts → SUBSUMED (batching).
//! - Sibling parser tests → land in
//! `scenarios::status_reports::tests`.
//! - `#[test] fn` wrappers → collapsed into one walker test here that
//! asserts every curated sub-test has a parseable response.
//! - Cap coverage extension → `cap_coverage/section_06.rs` moves `u6`
//! + `u7` from `exempt` to `covered`.

use std::collections::BTreeMap;

use oriterm_test_support::tack_framework::scenarios::status_reports::{
    STATUS_REPORTS_RESPONSE_EXPECTATIONS, extract_response_for_label,
};
use oriterm_test_support::{PtySession, TerminfoEnv, tack_available, tic_available};

/// Safety bound on the number of `n` presses during the walk.
/// Matches the 06.0.b walker ceiling — empirically ~19 presses
/// under ori_term terminfo with `PtySession::send`'s 300 ms
/// quiesce, 40 gives plenty of headroom.
const MAX_N_PRESSES: usize = 40;

#[test]
fn tack_tools_status_reports_walker() {
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
    // automatically on entry under ori_term before waiting for the
    // first `n` press.
    session.send(b"s");

    // History of every grid snapshot, indexed by `n`-press count
    // (index 0 is the pre-`n`-press snapshot captured right after
    // `s`). We scan the accumulated history for response regions
    // because individual sub-tests may scroll off the 24-row
    // viewport before their response fields are extracted.
    let mut history: Vec<String> = Vec::new();
    history.push(session.grid_text());

    let mut n_press_count = 0_usize;
    let mut exited_cleanly = false;
    while n_press_count < MAX_N_PRESSES {
        session.send(b"n");
        let grid = session.grid_text();
        let finished = grid.contains("Tools Menu");
        history.push(grid);
        n_press_count += 1;
        if finished {
            exited_cleanly = true;
            break;
        }
    }

    assert!(
        exited_cleanly,
        "status reports sub-submenu did not exit within {MAX_N_PRESSES} n presses"
    );
    eprintln!(
        "tack_tools_status_reports_walker: walked {n_press_count} n presses, \
         captured {snapshots} grid snapshots",
        snapshots = history.len()
    );

    // Quit cleanly before running assertions so a later failure
    // does not leak the tack child.
    let _exit = session.quit_tack(5);

    // Extract each curated sub-test's response from the walker
    // history. The parser locates the sub-test's label on a line
    // and returns the rightmost column segment (tack's response
    // column). Sub-tests whose response region is blank (tack's
    // "illegal response" marker) return `None`.
    let mut extracted: BTreeMap<&'static str, String> = BTreeMap::new();
    for expectation in STATUS_REPORTS_RESPONSE_EXPECTATIONS {
        if let Some(response) = extract_response_for_label(&history, expectation.label) {
            extracted.insert(expectation.label, response);
        }
    }

    // Drift gate: snapshot the set of curated labels that produced
    // a parseable response under `ori_term` terminfo. This is
    // stable across runs (raw response content like CPR row;col
    // values vary with cursor placement) and pins which sub-tests
    // successfully round-trip with ori_term's VTE. A future tack
    // release that changes sub-test availability, or an ori_term
    // regression that breaks DA/DSR response formatting, will
    // flip this snapshot.
    let mut responded_labels: Vec<&'static str> = STATUS_REPORTS_RESPONSE_EXPECTATIONS
        .iter()
        .filter(|exp| extracted.contains_key(exp.label))
        .map(|exp| exp.label)
        .collect();
    responded_labels.sort_unstable();
    insta::assert_snapshot!(
        "tack_status_reports_responded_labels_80x24",
        &responded_labels.join("\n")
    );

    // Every curated sub-test must have produced a parseable response
    // matching its content predicate. The per-sub-test predicates
    // (Primary DA → ^[[?...c, DSR status → ^[[...n, DSR CPR →
    // ^[[...;...R, DA2 → ^[[>...c, DA3 → ^[P!|...^[\) catch
    // ori_term regressions that break response formatting.
    for expectation in STATUS_REPORTS_RESPONSE_EXPECTATIONS {
        let response = extracted.get(expectation.label).unwrap_or_else(|| {
            panic!(
                "status reports walker: no response extracted for curated sub-test \
                 {label:?} (mnemonic {mnemonic}). The sub-test label did not appear \
                 in any captured grid snapshot or its response column was empty \
                 (tack's 'illegal response' marker). Either tack's output format \
                 changed or ori_term's VTE regressed on this sub-test.",
                label = expectation.label,
                mnemonic = expectation.mnemonic,
            )
        });
        assert!(
            !response.trim().is_empty(),
            "status reports walker: response region for {label:?} was empty",
            label = expectation.label,
        );
        assert!(
            (expectation.response_check)(response),
            "status reports walker: response for {label:?} failed content check. \
             Extracted response: {response:?}",
            label = expectation.label,
        );
    }
}
