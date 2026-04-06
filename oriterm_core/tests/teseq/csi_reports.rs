//! CSI report and response scenarios.

use std::path::Path;

use super::harness::{
    self, ScenarioOutcome, TeseqHarness, analyze_response, assert_pty_writes,
    assert_response_snapshot, pipe_through_command, reseq_available,
};

/// Run a report scenario and apply spec assertions.
///
/// Returns `None` when `reseq` is unavailable (graceful skip with visible message).
/// Returns the outcome for callers to perform additional response assertions.
fn run_scenario(name: &str) -> Option<ScenarioOutcome> {
    if !reseq_available() {
        eprintln!("reseq not installed, skipping");
        return None;
    }
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/teseq/scenarios/csi/reports")
        .join(format!("{name}.teseq"));
    let mut h = TeseqHarness::from_scenario(&path);
    let outcome = h.run(&path);
    harness::assert_spec(&outcome, h.spec(), &format!("csi_reports_{name}"));
    Some(outcome)
}

/// Replicate `crate_version_number()` from `handler/helpers.rs`.
///
/// Uses the same algorithm so tests track version bumps automatically.
/// Do NOT hardcode a version number — it changes on every release.
fn compute_da2_version() -> usize {
    let mut result = 0usize;
    let version = env!("CARGO_PKG_VERSION");
    let version = version.split('-').next().unwrap_or(version);
    for (i, part) in version.split('.').rev().enumerate() {
        let n = part.parse::<usize>().unwrap_or(0);
        result += n * 100usize.pow(i as u32);
    }
    result
}

// --- DA1 ---

#[test]
fn da1() {
    let Some(outcome) = run_scenario("da1") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[?64;6;4c"]);
    assert_response_snapshot(&outcome, "csi_reports_da1");
}

// --- DA2 ---

#[test]
fn da2_version_drift_check() {
    // Validates compute_da2_version() matches production crate_version_number()
    // output by feeding DA2 query directly through Term (no reseq dependency).
    use super::harness::RecordedListener;
    use oriterm_core::Theme;
    use oriterm_core::term::Term;

    let listener = RecordedListener::new();
    let mut term = Term::new(24, 80, 0, Theme::Dark, listener.clone());

    // Feed DA2 query: ESC [ > c
    let mut processor: vte::ansi::Processor = vte::ansi::Processor::new();
    processor.advance(&mut term, b"\x1b[>c");

    let writes = listener.pty_writes();
    assert_eq!(writes.len(), 1, "expected 1 PtyWrite, got {:?}", writes);
    let expected = format!("\x1b[>0;{};1c", compute_da2_version());
    assert_eq!(
        writes[0], expected,
        "DA2 drift: compute_da2_version() disagrees with production response"
    );
}

#[test]
fn da2() {
    let Some(outcome) = run_scenario("da2") else {
        return;
    };
    let expected = format!("\x1b[>0;{};1c", compute_da2_version());
    assert_pty_writes(&outcome, &[&expected]);
    assert_response_snapshot(&outcome, "csi_reports_da2");
}

// --- DA3 ---

#[test]
fn da3() {
    let Some(outcome) = run_scenario("da3") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1bP!|00000000\x1b\\"]);
    assert_response_snapshot(&outcome, "csi_reports_da3");
}

// --- DSR device status (arg 5) ---

#[test]
fn dsr_device_status() {
    let Some(outcome) = run_scenario("dsr_device_status") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[0n"]);
    assert_response_snapshot(&outcome, "csi_reports_dsr_device_status");
}

// --- DSR cursor position at home (arg 6) ---

#[test]
fn dsr_cursor_home() {
    let Some(outcome) = run_scenario("dsr_cursor_home") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[1;1R"]);
    assert_response_snapshot(&outcome, "csi_reports_dsr_cursor_home");
}

// --- DSR cursor after movement (arg 6) ---

#[test]
fn dsr_cursor_moved() {
    let Some(outcome) = run_scenario("dsr_cursor_moved") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[10;20R"]);
    assert_response_snapshot(&outcome, "csi_reports_dsr_cursor_moved");
}

// --- DSR with origin mode (arg 6 + DECOM) ---

#[test]
fn dsr_origin_mode() {
    let Some(outcome) = run_scenario("dsr_origin_mode") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[3;10R"]);
    assert_response_snapshot(&outcome, "csi_reports_dsr_origin_mode");
}

// --- DECRQM private: DECTCEM default (mode 25, set = cursor visible) ---

#[test]
fn decrqm_dectcem() {
    let Some(outcome) = run_scenario("decrqm_dectcem") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[?25;1$y"]);
    assert_response_snapshot(&outcome, "csi_reports_decrqm_dectcem");
}

// --- DECRQM private: DECTCEM after reset (mode 25, reset = cursor hidden) ---

#[test]
fn decrqm_dectcem_off() {
    let Some(outcome) = run_scenario("decrqm_dectcem_off") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[?25;2$y"]);
    assert_response_snapshot(&outcome, "csi_reports_decrqm_dectcem_off");
}

// --- DECRQM private: DECAWM default (mode 7, set = auto-wrap enabled) ---

#[test]
fn decrqm_decawm() {
    let Some(outcome) = run_scenario("decrqm_decawm") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[?7;1$y"]);
    assert_response_snapshot(&outcome, "csi_reports_decrqm_decawm");
}

// --- DECRQM private: unrecognized mode (mode 9999 = not recognized) ---

#[test]
fn decrqm_unknown() {
    let Some(outcome) = run_scenario("decrqm_unknown") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[?9999;0$y"]);
    assert_response_snapshot(&outcome, "csi_reports_decrqm_unknown");
}

// --- ANSI mode: IRM default (mode 4, reset = replace mode) ---

#[test]
fn ansi_mode_irm_default() {
    let Some(outcome) = run_scenario("ansi_mode_irm_default") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[4;2$y"]);
    assert_response_snapshot(&outcome, "csi_reports_ansi_mode_irm_default");
}

// --- ANSI mode: IRM after enable (mode 4, set = insert mode) ---

#[test]
fn ansi_mode_irm_set() {
    let Some(outcome) = run_scenario("ansi_mode_irm_set") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[4;1$y"]);
    assert_response_snapshot(&outcome, "csi_reports_ansi_mode_irm_set");
}

// --- ANSI mode: LNM default (mode 20, reset = line feed mode) ---

#[test]
fn ansi_mode_lnm_default() {
    let Some(outcome) = run_scenario("ansi_mode_lnm_default") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[20;2$y"]);
    assert_response_snapshot(&outcome, "csi_reports_ansi_mode_lnm_default");
}

// --- ANSI mode: unknown (mode 99 = not recognized) ---

#[test]
fn ansi_mode_unknown() {
    let Some(outcome) = run_scenario("ansi_mode_unknown") else {
        return;
    };
    assert_pty_writes(&outcome, &["\x1b[99;0$y"]);
    assert_response_snapshot(&outcome, "csi_reports_ansi_mode_unknown");
}

// --- analyze_response coverage ---

#[test]
fn analyze_response_produces_output() {
    // Exercises analyze_response: teseq happy path if available, hex fallback otherwise.
    let result = analyze_response("\x1b[?64;6;4c");
    assert!(result.is_ok(), "analyze_response failed: {:?}", result);
    let output = result.unwrap();
    assert!(!output.is_empty(), "analyze_response returned empty string");
}

#[test]
fn pipe_through_command_returns_err_on_nonzero_exit() {
    // Tests the non-zero-exit error path extracted into pipe_through_command.
    // Uses platform-appropriate commands that always exit non-zero.
    #[cfg(unix)]
    let result = pipe_through_command("false", &[], "");
    #[cfg(windows)]
    let result = pipe_through_command("cmd", &["/C", "exit", "1"], "");

    assert!(result.is_err(), "expected Err for non-zero exit");
    let err_msg = result.unwrap_err();
    assert!(
        err_msg.contains("exited with"),
        "unexpected error: {err_msg}"
    );
}
