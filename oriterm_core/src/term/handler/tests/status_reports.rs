use crate::term::TermMode;

use super::super::test_helpers::{feed, term_with_recorder, term_with_recorder_sized};

// --- CSI device status tests ---

#[test]
fn dsr_produces_cursor_position_report() {
    let (mut t, listener) = term_with_recorder();
    // Move cursor to line 4, column 9 (0-based).
    feed(&mut t, b"\x1b[5;10H"); // CUP row 5, col 10 (1-based)
    // CSI 6 n — DSR: request cursor position.
    feed(&mut t, b"\x1b[6n");

    let events = listener.events();
    // CPR response: ESC [ 5 ; 10 R (1-based).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[5;10R)"));
}

#[test]
fn da1_produces_device_attributes() {
    let (mut t, listener) = term_with_recorder();
    // CSI c — primary device attributes.
    feed(&mut t, b"\x1b[c");

    let events = listener.events();
    // VT420-class (64) with ANSI color (6) and sixel (4).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[?64;6;4c)"));
}

// --- DSR code 5 and DA2 tests ---

#[test]
fn dsr_code_5_reports_terminal_ok() {
    let (mut t, listener) = term_with_recorder();
    // CSI 5 n — DSR: terminal status.
    feed(&mut t, b"\x1b[5n");

    let events = listener.events();
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[0n)"));
}

#[test]
fn da2_produces_secondary_device_attributes() {
    let (mut t, listener) = term_with_recorder();
    // CSI > c — secondary device attributes.
    feed(&mut t, b"\x1b[>c");

    let events = listener.events();
    // DA2 response: ESC [ > 0 ; version ; 1 c
    assert!(
        events
            .iter()
            .any(|e| e.starts_with("PtyWrite(\x1b[>0;") && e.ends_with(";1c)"))
    );
}

#[test]
fn da3_produces_tertiary_device_attributes() {
    let (mut t, listener) = term_with_recorder();
    // CSI = c — tertiary device attributes.
    feed(&mut t, b"\x1b[=c");

    let events = listener.events();
    // DA3 response: DCS ! | 00000000 ST.
    assert!(
        events
            .iter()
            .any(|e| e == "PtyWrite(\x1bP!|00000000\x1b\\)")
    );
}

// --- DECRPM (mode report) tests ---

#[test]
fn decrpm_reports_set_private_mode() {
    let (mut t, listener) = term_with_recorder();
    // SHOW_CURSOR is on by default.
    // CSI ? 25 $ p — query DECTCEM.
    feed(&mut t, b"\x1b[?25$p");

    let events = listener.events();
    // Response: CSI ? 25 ; 1 $ y (1 = set).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[?25;1$y)"));
}

#[test]
fn decrpm_reports_reset_private_mode() {
    let (mut t, listener) = term_with_recorder();
    // ALT_SCREEN is off by default.
    // CSI ? 1049 $ p — query alt screen.
    feed(&mut t, b"\x1b[?1049$p");

    let events = listener.events();
    // Response: CSI ? 1049 ; 2 $ y (2 = reset).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[?1049;2$y)"));
}

#[test]
fn decrpm_reports_ansi_mode() {
    let (mut t, listener) = term_with_recorder();
    // INSERT mode is off by default.
    // CSI 4 $ p — query IRM.
    feed(&mut t, b"\x1b[4$p");

    let events = listener.events();
    // Response: CSI 4 ; 2 $ y (2 = reset).
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[4;2$y)"));
}

// --- DSR cursor position report in ORIGIN mode ---

#[test]
fn dsr_reports_relative_position_in_origin_mode() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[5;15r"); // DECSTBM 5–15
    feed(&mut t, b"\x1b[?6h"); // ORIGIN mode
    feed(&mut t, b"\x1b[1;1H"); // CUP(1,1) → absolute line 4, col 0
    feed(&mut t, b"\x1b[6n"); // DSR

    let events = listener.events();
    // Per DEC spec, DECOM DSR 6 reports position relative to scroll
    // region origin. CUP(1,1) in region 5-15 → relative row 1, col 1.
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[1;1R)"));
}

#[test]
fn dsr_reports_absolute_position_without_origin_mode() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[5;15r"); // DECSTBM 5–15
    feed(&mut t, b"\x1b[5;10H"); // CUP → absolute line 4, col 9
    feed(&mut t, b"\x1b[6n"); // DSR

    let events = listener.events();
    // Without DECOM, DSR 6 reports absolute position.
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[5;10R)"));
}

// --- Text area size report ---

#[test]
fn text_area_size_chars_reports_dimensions() {
    let (mut t, listener) = term_with_recorder();
    // CSI 18 t — report text area size in characters.
    feed(&mut t, b"\x1b[18t");

    let events = listener.events();
    // Response: CSI 8 ; lines ; cols t.
    assert!(events.iter().any(|e| e == "PtyWrite(\x1b[8;24;80t)"));
}

// --- DA1 response format (vttest conformance) ---

#[test]
fn da1_response_indicates_vt220_class() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[c");

    let events = listener.events();
    let da1 = events
        .iter()
        .find(|e| e.starts_with("PtyWrite(\x1b[?"))
        .expect("DA1 response should be emitted");

    // vttest requires the DA1 response to indicate VT200+ class.
    // The response must start with CSI ? 62 (VT220), 63 (VT320),
    // or 64 (VT420) for vttest to send CSI 18t size queries.
    assert!(
        da1.contains("?62;") || da1.contains("?63;") || da1.contains("?64;"),
        "DA1 response must indicate VT220+ class (62/63/64), got: {da1}"
    );
}

#[test]
fn csi_18t_at_non_80_cols() {
    let (mut t, listener) = term_with_recorder_sized(40, 120);
    feed(&mut t, b"\x1b[18t");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[8;40;120t)"),
        "CSI 18t should report actual grid dimensions (40x120), got: {events:?}"
    );
}

#[test]
fn csi_18t_at_97x33() {
    let (mut t, listener) = term_with_recorder_sized(33, 97);
    feed(&mut t, b"\x1b[18t");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[8;33;97t)"),
        "CSI 18t should report actual grid dimensions (33x97), got: {events:?}"
    );
}

// --- Comprehensive DECRQM coverage for DEC private modes ---

/// Assert DECRQM response for a private mode.
///
/// Feeds optional `setup` bytes (e.g. DECSET to change the mode),
/// then sends the DECRQM query `CSI ? <mode_num> $ p` and checks
/// the response `CSI ? <mode_num> ; <expected_value> $ y`.
fn assert_decrqm_private(setup: &[u8], mode_num: u16, expected_value: u8) {
    let (mut t, listener) = term_with_recorder();
    if !setup.is_empty() {
        feed(&mut t, setup);
    }
    let query = format!("\x1b[?{mode_num}$p");
    feed(&mut t, query.as_bytes());
    let events = listener.events();
    let expected_response = format!("PtyWrite(\x1b[?{mode_num};{expected_value}$y)");
    assert!(
        events.iter().any(|e| *e == expected_response),
        "DECRQM ?{mode_num}: expected value {expected_value}, got events: {events:?}"
    );
}

// Mode 9 — X10 mouse.

#[test]
fn decrqm_x10_mouse_default_reset() {
    assert_decrqm_private(b"", 9, 2);
}

#[test]
fn decrqm_x10_mouse_after_set() {
    assert_decrqm_private(b"\x1b[?9h", 9, 1);
}

// Mode 1000 — report mouse clicks.

#[test]
fn decrqm_mouse_clicks_default_reset() {
    assert_decrqm_private(b"", 1000, 2);
}

#[test]
fn decrqm_mouse_clicks_after_set() {
    assert_decrqm_private(b"\x1b[?1000h", 1000, 1);
}

// Mode 1002 — report cell mouse motion.

#[test]
fn decrqm_cell_motion_default_reset() {
    assert_decrqm_private(b"", 1002, 2);
}

#[test]
fn decrqm_cell_motion_after_set() {
    assert_decrqm_private(b"\x1b[?1002h", 1002, 1);
}

// Mode 1003 — report all mouse motion.

#[test]
fn decrqm_all_motion_default_reset() {
    assert_decrqm_private(b"", 1003, 2);
}

#[test]
fn decrqm_all_motion_after_set() {
    assert_decrqm_private(b"\x1b[?1003h", 1003, 1);
}

// Mode 1004 — focus in/out.

#[test]
fn decrqm_focus_default_reset() {
    assert_decrqm_private(b"", 1004, 2);
}

#[test]
fn decrqm_focus_after_set() {
    assert_decrqm_private(b"\x1b[?1004h", 1004, 1);
}

// Mode 1005 — UTF-8 mouse encoding.

#[test]
fn decrqm_utf8_mouse_default_reset() {
    assert_decrqm_private(b"", 1005, 2);
}

#[test]
fn decrqm_utf8_mouse_after_set() {
    assert_decrqm_private(b"\x1b[?1005h", 1005, 1);
}

// Mode 1006 — SGR mouse encoding.

#[test]
fn decrqm_sgr_mouse_default_reset() {
    assert_decrqm_private(b"", 1006, 2);
}

#[test]
fn decrqm_sgr_mouse_after_set() {
    assert_decrqm_private(b"\x1b[?1006h", 1006, 1);
}

// Mode 1007 — alternate scroll (default: SET, in default mode).

#[test]
fn decrqm_alternate_scroll_default_set() {
    // ALTERNATE_SCROLL is in the default mode.
    assert_decrqm_private(b"", 1007, 1);
}

#[test]
fn decrqm_alternate_scroll_after_reset() {
    assert_decrqm_private(b"\x1b[?1007l", 1007, 2);
}

// Mode 1015 — URXVT mouse encoding.

#[test]
fn decrqm_urxvt_mouse_default_reset() {
    assert_decrqm_private(b"", 1015, 2);
}

#[test]
fn decrqm_urxvt_mouse_after_set() {
    assert_decrqm_private(b"\x1b[?1015h", 1015, 1);
}

// Mode 1042 — urgency hints.

#[test]
fn decrqm_urgency_default_reset() {
    assert_decrqm_private(b"", 1042, 2);
}

#[test]
fn decrqm_urgency_after_set() {
    assert_decrqm_private(b"\x1b[?1042h", 1042, 1);
}

// Mode 1047 — alt screen option (maps to ALT_SCREEN, default off).

#[test]
fn decrqm_alt_screen_opt_default_reset() {
    assert_decrqm_private(b"", 1047, 2);
}

#[test]
fn decrqm_alt_screen_opt_after_set() {
    assert_decrqm_private(b"\x1b[?1047h", 1047, 1);
}

// Mode 1048 — save cursor (maps to None in named_private_mode_flag → value 0).

#[test]
fn decrqm_save_cursor_returns_unrecognized() {
    assert_decrqm_private(b"", 1048, 0);
}

// Mode 80 — sixel scrolling (default: SET).

#[test]
fn decrqm_sixel_scrolling_default_set() {
    // SIXEL_SCROLLING is in the default mode.
    assert_decrqm_private(b"", 80, 1);
}

#[test]
fn decrqm_sixel_scrolling_after_reset() {
    assert_decrqm_private(b"\x1b[?80l", 80, 2);
}

// Mode 8452 — sixel cursor right.

#[test]
fn decrqm_sixel_cursor_right_default_reset() {
    assert_decrqm_private(b"", 8452, 2);
}

#[test]
fn decrqm_sixel_cursor_right_after_set() {
    assert_decrqm_private(b"\x1b[?8452h", 8452, 1);
}

// Mode 9001 — Win32 input.

#[test]
fn decrqm_win32_input_default_reset() {
    assert_decrqm_private(b"", 9001, 2);
}

#[test]
fn decrqm_win32_input_after_set() {
    assert_decrqm_private(b"\x1b[?9001h", 9001, 1);
}

// Mode 2026 — synchronized update.

#[test]
fn decrqm_sync_update_default_reset() {
    assert_decrqm_private(b"", 2026, 2);
}

#[test]
fn decrqm_sync_update_after_set() {
    assert_decrqm_private(b"\x1b[?2026h", 2026, 1);
}

// Mode 2004 — bracketed paste.

#[test]
fn decrqm_bracketed_paste_default_reset() {
    assert_decrqm_private(b"", 2004, 2);
}

#[test]
fn decrqm_bracketed_paste_after_set() {
    assert_decrqm_private(b"\x1b[?2004h", 2004, 1);
}

// Mode 3 — column mode (maps to None → value 0).

#[test]
fn decrqm_column_mode_returns_unrecognized() {
    assert_decrqm_private(b"", 3, 0);
}

// Mode 40 — enable mode 3 (DECNRCM gate).

#[test]
fn decrqm_enable_mode_3_default_reset() {
    assert_decrqm_private(b"", 40, 2);
}

#[test]
fn decrqm_enable_mode_3_after_set() {
    assert_decrqm_private(b"\x1b[?40h", 40, 1);
}

// Mode 5 — reverse video.

#[test]
fn decrqm_reverse_video_default_reset() {
    assert_decrqm_private(b"", 5, 2);
}

#[test]
fn decrqm_reverse_video_after_set() {
    assert_decrqm_private(b"\x1b[?5h", 5, 1);
}

// Mode 69 — left-right margin mode.

#[test]
fn decrqm_left_right_margin_default_reset() {
    assert_decrqm_private(b"", 69, 2);
}

#[test]
fn decrqm_left_right_margin_after_set() {
    assert_decrqm_private(b"\x1b[?69h", 69, 1);
}

// --- DECRQM round-trip (set → query → reset → query) ---

#[test]
fn decrqm_round_trip_set_then_reset() {
    // Verify full round-trip: default=reset, DECSET=set, DECRST=reset.
    let (mut t, listener) = term_with_recorder();

    // Default: mode 2004 (bracketed paste) is reset.
    feed(&mut t, b"\x1b[?2004$p");
    let events = listener.events();
    assert!(
        events.iter().any(|e| *e == "PtyWrite(\x1b[?2004;2$y)"),
        "Bracketed paste should default to reset, events: {events:?}"
    );

    // DECSET → should report set.
    feed(&mut t, b"\x1b[?2004h");
    feed(&mut t, b"\x1b[?2004$p");
    let events = listener.events();
    assert!(
        events.iter().any(|e| *e == "PtyWrite(\x1b[?2004;1$y)"),
        "Bracketed paste should be set after DECSET, events: {events:?}"
    );

    // DECRST → should report reset again.
    feed(&mut t, b"\x1b[?2004l");
    feed(&mut t, b"\x1b[?2004$p");
    let events = listener.events();
    // There should be two ;2$ entries: default query and post-DECRST query.
    let reset_count = events
        .iter()
        .filter(|e| *e == "PtyWrite(\x1b[?2004;2$y)")
        .count();
    assert!(
        reset_count >= 2,
        "Bracketed paste should be reset after DECRST, events: {events:?}"
    );
}

// --- DECRQM for unknown private mode ---

#[test]
fn decrqm_unknown_mode_returns_unrecognized() {
    // An unknown mode number should return value 0.
    assert_decrqm_private(b"", 9999, 0);
}

// --- Default mode flags cross-check ---

#[test]
fn decrqm_verifies_default_mode_flags() {
    // Verify that modes in TermMode::default() report as set,
    // and modes NOT in the default report as reset.
    let default = TermMode::default();
    assert!(default.contains(TermMode::SHOW_CURSOR));
    assert!(default.contains(TermMode::LINE_WRAP));
    assert!(default.contains(TermMode::ALTERNATE_SCROLL));
    assert!(default.contains(TermMode::SIXEL_SCROLLING));
    assert!(default.contains(TermMode::CURSOR_BLINKING));

    // SHOW_CURSOR (mode 25) should be set by default.
    assert_decrqm_private(b"", 25, 1);
    // LINE_WRAP (mode 7) should be set by default.
    assert_decrqm_private(b"", 7, 1);
    // CURSOR_BLINKING (mode 12) should be set by default.
    assert_decrqm_private(b"", 12, 1);
}
