//! Sibling tests for `status_reports/mod.rs`.
//!
//! Pins the response-extraction helper + per-sub-test content
//! predicates against synthesized grids modeled on the real tack
//! v1.08 walker output captured at
//! `oriterm_core/tests/tack/tools_menu/snapshots/tack__tools_menu__status_reports__tack_status_reports_walker_history_80x24.snap`.
//! Synthetic rather than live-walk so the parser can be exercised
//! cross-platform and without a tack/terminfo round-trip.

use super::{
    extract_response_for_label, extract_response_from_snapshot, is_dsr_cursor_position_response,
    is_dsr_terminal_status_response, is_primary_da_response, is_secondary_da_response,
    is_tertiary_da_response, last_segment_after_gap,
};

/// Synthesized tack status-reports snapshot row for DA1, modeled
/// after the real walker output (row 5 of snapshot 0 in the
/// diagnostic capture).
const SYNTHETIC_SNAPSHOT: &str = "\
Begin ANSI status report testing.  Parity bit set will be displayed in reverse
video.  If the terminal hangs, hit any alphabetic key.  Use n to continue

(DA) Primary device attributes (CSI 0 c)         ^[[?64;6;4c
(DSR) Terminal status (CSI 5 n)                  ^[[0n
(DSR) Cursor position (CSI 6 n)                  ^[[7;50R
(DA) Secondary device attributes (CSI > 0 c)     ^[[>0;200;1c
(DA) Tertiary device attributes (CSI = 0 c)      ^[P!|00000000^[\\
(DSR) Printer status (CSI ? 15 n)
                                                                                ";

#[test]
fn extract_response_finds_da1_on_label_line() {
    let response = extract_response_from_snapshot(SYNTHETIC_SNAPSHOT, "Primary device attributes")
        .expect("response should be extracted");
    assert_eq!(response, "^[[?64;6;4c");
}

#[test]
fn extract_response_finds_dsr_terminal_status() {
    let response = extract_response_from_snapshot(SYNTHETIC_SNAPSHOT, "Terminal status")
        .expect("response should be extracted");
    assert_eq!(response, "^[[0n");
}

#[test]
fn extract_response_finds_dsr_cursor_position() {
    let response = extract_response_from_snapshot(SYNTHETIC_SNAPSHOT, "Cursor position")
        .expect("response should be extracted");
    assert_eq!(response, "^[[7;50R");
}

#[test]
fn extract_response_finds_da2_with_angle_bracket_prefix() {
    let response =
        extract_response_from_snapshot(SYNTHETIC_SNAPSHOT, "Secondary device attributes")
            .expect("response should be extracted");
    assert_eq!(response, "^[[>0;200;1c");
}

#[test]
fn extract_response_finds_da3_dcs_form() {
    let response = extract_response_from_snapshot(SYNTHETIC_SNAPSHOT, "Tertiary device attributes")
        .expect("response should be extracted");
    assert!(
        response.starts_with("^[P!|"),
        "DA3 response should be in DCS P!| form, got {response:?}"
    );
}

#[test]
fn extract_response_returns_none_for_illegal_response_row() {
    // Tack's "Printer status" row has NO response text under
    // ori_term — the response column is padded blank. The
    // parser must not fabricate a response.
    let response = extract_response_from_snapshot(SYNTHETIC_SNAPSHOT, "Printer status");
    assert!(
        response.is_none(),
        "illegal-response rows must return None, got {response:?}"
    );
}

#[test]
fn extract_response_returns_none_when_label_absent() {
    assert!(extract_response_from_snapshot(SYNTHETIC_SNAPSHOT, "NOT A REAL LABEL").is_none());
}

#[test]
fn extract_response_for_label_scans_history_for_first_hit() {
    let empty_snapshot =
        "                                                                                \n";
    let history = vec![empty_snapshot.to_string(), SYNTHETIC_SNAPSHOT.to_string()];
    let response = extract_response_for_label(&history, "Primary device attributes")
        .expect("DA1 should be found in the second snapshot");
    assert_eq!(response, "^[[?64;6;4c");
}

#[test]
fn extract_response_for_label_returns_none_when_no_snapshot_has_it() {
    let history = vec!["no matching content here\n".to_string()];
    assert!(extract_response_for_label(&history, "Primary device attributes").is_none());
}

#[test]
fn last_segment_after_gap_finds_rightmost_run() {
    assert_eq!(
        last_segment_after_gap("label text   response value"),
        Some("response value")
    );
}

#[test]
fn last_segment_after_gap_returns_none_for_single_space_lines() {
    // A label with ONLY single-space separation has no discernible
    // response column — the parser MUST reject it rather than
    // returning the last word.
    assert_eq!(last_segment_after_gap("one two three four"), None);
}

#[test]
fn last_segment_after_gap_returns_none_for_blank_lines() {
    assert_eq!(last_segment_after_gap("                    "), None);
    assert_eq!(last_segment_after_gap(""), None);
}

#[test]
fn last_segment_after_gap_strips_trailing_padding() {
    assert_eq!(
        last_segment_after_gap("label   response                     "),
        Some("response")
    );
}

// --- Response-content predicate tests (per-sub-test semantic pins) ---

#[test]
fn is_primary_da_response_accepts_canonical_form() {
    assert!(is_primary_da_response("^[[?64;6;4c"));
    assert!(is_primary_da_response("^[[?1;2c"));
}

#[test]
fn is_primary_da_response_rejects_dsr_form() {
    assert!(!is_primary_da_response("^[[0n"));
    assert!(!is_primary_da_response("^[[7;50R"));
}

#[test]
fn is_dsr_terminal_status_accepts_0n_and_3n() {
    assert!(is_dsr_terminal_status_response("^[[0n"));
    assert!(is_dsr_terminal_status_response("^[[3n"));
}

#[test]
fn is_dsr_terminal_status_rejects_cpr_form() {
    // CPR ends with `R`, not `n` — must reject.
    assert!(!is_dsr_terminal_status_response("^[[7;50R"));
}

#[test]
fn is_dsr_cursor_position_accepts_row_col_r() {
    assert!(is_dsr_cursor_position_response("^[[7;50R"));
    assert!(is_dsr_cursor_position_response("^[[1;1R"));
}

#[test]
fn is_dsr_cursor_position_rejects_without_semicolon() {
    // A bare `^[[5R` is not a valid CPR — it must have row;col.
    assert!(!is_dsr_cursor_position_response("^[[5R"));
}

#[test]
fn is_secondary_da_accepts_angle_bracket_prefix() {
    assert!(is_secondary_da_response("^[[>0;200;1c"));
    assert!(is_secondary_da_response("^[[>41;344;0c"));
}

#[test]
fn is_secondary_da_rejects_primary_da_form() {
    // Primary DA has `^[[?` prefix, not `^[[>` — must reject.
    assert!(!is_secondary_da_response("^[[?64;6;4c"));
}

#[test]
fn is_tertiary_da_accepts_dcs_p_bang_pipe_form() {
    assert!(is_tertiary_da_response("^[P!|00000000^[\\"));
    assert!(is_tertiary_da_response("^[P!|DEADBEEF^[\\"));
}

#[test]
fn is_tertiary_da_rejects_csi_forms() {
    assert!(!is_tertiary_da_response("^[[?64;6;4c"));
    assert!(!is_tertiary_da_response("^[[>0;200;1c"));
}
