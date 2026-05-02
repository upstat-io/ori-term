use crate::effect::{Effect, PtyEffect, PtyWriteKind};
use crate::term::TermMode;

use super::super::test_helpers::{
    feed, term_with_effect_sink, term_with_recorder, term_with_recorder_sized,
};

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

// --- XTSMGRAPHICS (CSI ? Pi ; Pa ; Pv S) ---
//
// XTSMGRAPHICS is xterm's set-or-request graphics attribute query. Apps
// using sixel (notcurses, mlterm, libsixel) send these at startup to
// negotiate color-register count and graphics-area geometry. Without
// replies, those apps default to conservative sizes or skip sixel.
//
// Pi values per xterm ctlseqs / `charproc.c:5153-5279`:
//   1 = number of color registers
//   2 = sixel graphics geometry (pixels)
//   3 = ReGIS graphics geometry (unsupported — Ps=3 failure)
//
// Pa values:
//   1 = read, 2 = reset, 3 = set, 4 = read maximum
//
// Reply format: `CSI ? Pi ; Ps [; Pv [; Pv2]] S`. Ps=0 success, Ps=1
// bad-value (unknown Pi), Ps=2 bad-item (unknown Pa), Ps=3 failure.
//
// Test grid: default 24x80 with cell_pixel_width=8, cell_pixel_height=16,
// so Pi=2 geometry replies report 640x384 pixels.
//
// Regression: BUG-06-022 — XTSMGRAPHICS query had no reply path before
// this fix; vte CSI dispatch had no `('S', [b'?'])` arm and Handler
// trait had no `graphics_attribute` method.

// --- Pi=1 (color registers) Pa matrix ---

#[test]
fn xtsmgraphics_pi1_pa1_read_returns_count() {
    // Pa=1 read: reply with the current color-register count (default 256).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;256S)"),
        "Pi=1 Pa=1 read should reply ?1;0;256S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa2_reset_returns_count() {
    // Pa=2 reset: reset to default and reply with default count (256).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;2;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;256S)"),
        "Pi=1 Pa=2 reset should reply ?1;0;256S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa3_set_within_max_succeeds() {
    // Pa=3 set: succeed when 1 < Pv <= MAX, reply reflects the set state
    // per xterm `charproc.c:5181-5185`.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;100S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;100S)"),
        "Pi=1 Pa=3 set Pv=100 should reply ?1;0;100S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa3_set_at_max_succeeds() {
    // notcurses startup repro: `\x1b[?1;3;256S` is the first XTSMGRAPHICS
    // query notcurses sends. Without this reply, notcurses zeros
    // initdata->color_registers and disables sixel entirely.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;256S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;256S)"),
        "Pi=1 Pa=3 set Pv=256 (notcurses startup repro) should reply ?1;0;256S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa3_set_at_min_boundary_succeeds() {
    // Pv=2 is the smallest accepted value per xterm `Pv > 1`. Reply
    // reflects the SET state (2), not the MAX (256).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;2S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;2S)"),
        "Pi=1 Pa=3 set Pv=2 should reply ?1;0;2S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa4_read_max_returns_count() {
    // Pa=4 read max: always MAX, regardless of any prior set.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;100S"); // set to 100
    feed(&mut t, b"\x1b[?1;4;0S"); // read max

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;256S)"),
        "Pi=1 Pa=4 read max should always reply ?1;0;256S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_set_then_read_returns_set_value() {
    // Pa=3 set mutates `Term::color_register_count`. Subsequent Pa=1
    // read reflects the SET state per xterm `charproc.c:5181-5185`.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;100S");
    feed(&mut t, b"\x1b[?1;1;0S");

    let events = listener.events();
    let read_replies: Vec<_> = events
        .iter()
        .filter(|e| e.starts_with("PtyWrite(\x1b[?1;0;"))
        .collect();
    assert!(
        read_replies
            .iter()
            .any(|e| **e == "PtyWrite(\x1b[?1;0;100S)"),
        "Pi=1 Pa=1 read after set Pv=100 should reflect 100, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_set_then_reset_returns_default() {
    // Pa=2 reset restores default (256). Subsequent Pa=1 reads return MAX.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;100S"); // set to 100
    feed(&mut t, b"\x1b[?1;2;0S"); // reset
    feed(&mut t, b"\x1b[?1;1;0S"); // read

    let events = listener.events();
    let final_read_count = events
        .iter()
        .filter(|e| **e == "PtyWrite(\x1b[?1;0;256S)")
        .count();
    assert!(
        final_read_count >= 2,
        "Pa=2 reset followed by Pa=1 read should produce two ;0;256S replies (the reset + the read), got: {events:?}"
    );
}

// --- Pi=1 negative pins (Pv boundary failures) ---

#[test]
fn xtsmgraphics_pi1_pa3_set_pv_zero_replies_status3_no_pv() {
    // Pv=0 fails `Pv > 1`; status=3, no Pv field per xterm.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;3S)"),
        "Pi=1 Pa=3 Pv=0 should reply ?1;3S (no Pv), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa3_set_pv_one_replies_status3_no_pv() {
    // Pv=1 fails `Pv > 1` (boundary).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;1S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;3S)"),
        "Pi=1 Pa=3 Pv=1 should reply ?1;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa3_set_pv_above_max_replies_status3_no_pv() {
    // Pv > MAX fails (Pv=257 > 256).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;257S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;3S)"),
        "Pi=1 Pa=3 Pv=257 should reply ?1;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_unknown_pa_replies_status2() {
    // Unknown Pa → Ps=2 (bad-item).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;99;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;2S)"),
        "Pi=1 unknown Pa should reply ?1;2S, got: {events:?}"
    );
}

// --- Pi=2 (sixel graphics geometry) Pa matrix ---

#[test]
fn xtsmgraphics_pi2_pa1_read_returns_width_height() {
    // Pa=1 read: `cols * cell_pixel_width` = 80*8 = 640;
    // `lines * cell_pixel_height` = 24*16 = 384.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?2;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?2;0;640;384S)"),
        "Pi=2 Pa=1 read should reply ?2;0;640;384S (80*8 x 24*16), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi2_pa4_read_max_returns_width_height() {
    // For oriterm, current geometry == max geometry (no separate max).
    // xterm separates them via screen->graphics_max_wide; oriterm
    // dynamically reports the current grid pixel size for both.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?2;4;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?2;0;640;384S)"),
        "Pi=2 Pa=4 read max should reply ?2;0;640;384S (current==max for oriterm), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi2_pa2_reset_replies_status3_no_geometry() {
    // xterm `charproc.c:5211` Pa=2 falls through to empty-block; status=3.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?2;2;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?2;3S)"),
        "Pi=2 Pa=2 reset should reply ?2;3S (xterm rejects geometry mutation), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi2_pa3_set_replies_status3_no_geometry() {
    // xterm rejects geometry set (empty block, status=3).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?2;3;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?2;3S)"),
        "Pi=2 Pa=3 set should reply ?2;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi2_unknown_pa_replies_status2() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?2;99;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?2;2S)"),
        "Pi=2 unknown Pa should reply ?2;2S, got: {events:?}"
    );
}

// --- Pi=3 (ReGIS — unsupported) Pa matrix ---

#[test]
fn xtsmgraphics_pi3_pa1_read_replies_status3() {
    // ReGIS unsupported regardless of Pa, status stays 3.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?3;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?3;3S)"),
        "Pi=3 Pa=1 (ReGIS read) should reply ?3;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi3_pa2_reset_replies_status3() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?3;2;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?3;3S)"),
        "Pi=3 Pa=2 (ReGIS reset) should reply ?3;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi3_pa3_set_replies_status3() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?3;3;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?3;3S)"),
        "Pi=3 Pa=3 (ReGIS set) should reply ?3;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi3_pa4_read_max_replies_status3() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?3;4;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?3;3S)"),
        "Pi=3 Pa=4 (ReGIS read max) should reply ?3;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi3_unknown_pa_replies_status2() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?3;99;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?3;2S)"),
        "Pi=3 unknown Pa should reply ?3;2S, got: {events:?}"
    );
}

// --- Unknown Pi × Pa matrix (outer-match short-circuit invariant) ---
//
// Per xterm `charproc.c:5258`, unknown Pi → status=1 regardless of Pa
// (the outer Pi switch fails before the inner Pa branches execute).
// These cells pin the outer-match short-circuit invariant — if a
// future refactor moves the Pa check ahead of the Pi check, all five
// cells would diverge.

#[test]
fn xtsmgraphics_unknown_pi_pa1_replies_status1() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?99;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?99;1S)"),
        "unknown Pi Pa=1 should reply ?99;1S (Ps=1 unknown-Pi), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_unknown_pi_pa2_replies_status1() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?99;2;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?99;1S)"),
        "unknown Pi Pa=2 should reply ?99;1S (unknown-Pi short-circuits Pa), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_unknown_pi_pa3_replies_status1() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?99;3;100S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?99;1S)"),
        "unknown Pi Pa=3 (set) should reply ?99;1S — Pv ignored, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_unknown_pi_pa4_replies_status1() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?99;4;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?99;1S)"),
        "unknown Pi Pa=4 should reply ?99;1S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_unknown_pi_unknown_pa_replies_status1() {
    // Outer-match precedence: unknown Pi outranks unknown Pa.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?99;99;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?99;1S)"),
        "unknown Pi + unknown Pa should reply ?99;1S (Pi outranks Pa), got: {events:?}"
    );
}

// --- RIS-reset regression pin ---

#[test]
fn xtsmgraphics_ris_resets_color_register_count_to_default() {
    // RIS (`ESC c`) → `esc_reset_state` MUST reset
    // `Term::color_register_count` to `COLOR_REGISTERS_MAX` (256).
    // Without the reset wiring, this test fails with `?1;0;100S` after
    // RIS — indicating the field was not added to the canonical reset
    // path.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;100S"); // set to 100
    feed(&mut t, b"\x1bc"); // RIS — full reset
    feed(&mut t, b"\x1b[?1;1;0S"); // read

    let events = listener.events();
    // After RIS, the Pa=1 read MUST reflect 256 (default), NOT 100.
    assert!(
        events
            .iter()
            .filter(|e| **e == "PtyWrite(\x1b[?1;0;256S)")
            .count()
            >= 1,
        "Pa=1 read after RIS should reply ?1;0;256S (default restored), got: {events:?}"
    );
    // Negative pin: the post-RIS read MUST NOT reflect the pre-RIS set value.
    let post_ris_set_value = events
        .iter()
        .filter(|e| **e == "PtyWrite(\x1b[?1;0;100S)")
        .count();
    assert_eq!(
        post_ris_set_value, 1,
        "should be exactly one ?1;0;100S (the original set's reply), not two — RIS leaked the set value, events: {events:?}"
    );
}

// --- Arity edge cases (silent drop per xterm `charproc.c:5159`) ---
//
// xterm checks `nparam != 3` and silently drops malformed-arity
// queries — no reply is constructed. The vte dispatch arm enforces
// this; the handler tests verify no `\x1b[?...S` reply lands on the
// effect sink for malformed arity inputs.

#[test]
fn xtsmgraphics_empty_params_emits_no_reply() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?S");

    let events = listener.events();
    let xtsmgraphics_replies: Vec<_> = events
        .iter()
        .filter(|e| e.starts_with("PtyWrite(\x1b[?") && e.ends_with("S)"))
        .collect();
    assert!(
        xtsmgraphics_replies.is_empty(),
        "empty params should emit no XTSMGRAPHICS reply, got: {xtsmgraphics_replies:?}"
    );
}

#[test]
fn xtsmgraphics_one_param_emits_no_reply() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1S");

    let events = listener.events();
    let xtsmgraphics_replies: Vec<_> = events
        .iter()
        .filter(|e| e.starts_with("PtyWrite(\x1b[?") && e.ends_with("S)"))
        .collect();
    assert!(
        xtsmgraphics_replies.is_empty(),
        "one param should emit no XTSMGRAPHICS reply, got: {xtsmgraphics_replies:?}"
    );
}

#[test]
fn xtsmgraphics_two_params_emits_no_reply() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;1S");

    let events = listener.events();
    let xtsmgraphics_replies: Vec<_> = events
        .iter()
        .filter(|e| e.starts_with("PtyWrite(\x1b[?") && e.ends_with("S)"))
        .collect();
    assert!(
        xtsmgraphics_replies.is_empty(),
        "two params should emit no XTSMGRAPHICS reply, got: {xtsmgraphics_replies:?}"
    );
}

#[test]
fn xtsmgraphics_four_params_emits_no_reply() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;1;0;0S");

    let events = listener.events();
    let xtsmgraphics_replies: Vec<_> = events
        .iter()
        .filter(|e| e.starts_with("PtyWrite(\x1b[?") && e.ends_with("S)"))
        .collect();
    assert!(
        xtsmgraphics_replies.is_empty(),
        "four params should emit no XTSMGRAPHICS reply, got: {xtsmgraphics_replies:?}"
    );
}

#[test]
fn xtsmgraphics_subparam_in_first_param_uses_first_subvalue() {
    // 3 top-level params (the first has subparam `:2` which is silently
    // ignored by `next_param_or`). vte's `params.iter().count()` counts
    // PARAMETER GROUPS, so this passes the arity check (count=3).
    // `next_param_or` takes the first sub-value per param, so this
    // dispatches as `graphics_attribute(1, 1, 0)`.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1:2;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;256S)"),
        "subparam in first should dispatch as Pi=1 Pa=1 → ?1;0;256S, got: {events:?}"
    );
}

// --- Dispatch isolation negative pins ---

#[test]
fn csi_ps_s_without_question_mark_invokes_scroll_up_no_xtsmgraphics_reply() {
    // Critical regression guard: ('S', []) (scroll-up) and ('S', [b'?'])
    // (XTSMGRAPHICS) must dispatch correctly without bleeding into each
    // other. SU `\x1b[3S` must NOT trigger a GraphicsAttributeReport.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[3S");

    let events = listener.events();
    let xtsmgraphics_replies: Vec<_> = events
        .iter()
        .filter(|e| e.starts_with("PtyWrite(\x1b[?") && e.ends_with("S)"))
        .collect();
    assert!(
        xtsmgraphics_replies.is_empty(),
        "plain SU should not emit any XTSMGRAPHICS reply, got: {xtsmgraphics_replies:?}"
    );
}

#[test]
fn csi_question_pi_pa_pv_s_invokes_xtsmgraphics_no_scroll() {
    // Inverse pin: `\x1b[?1;1;0S` MUST emit a GraphicsAttributeReport
    // and MUST NOT trigger SU. We verify the reply landed; the absence
    // of scroll is implicit (SU emits no PTY reply).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;256S)"),
        "XTSMGRAPHICS should emit a reply, got: {events:?}"
    );
}

// --- PtyWriteKind sync pin (registration sync point) ---

#[test]
fn xtsmgraphics_reply_uses_graphics_attribute_report_kind() {
    // Verifies the new `PtyWriteKind::GraphicsAttributeReport` variant
    // is wired correctly and not silently fallback to a peer (e.g.,
    // DeviceAttribute or Other). Uses `term_with_effect_sink` to
    // inspect raw `Effect` variants — `RecordingListener`'s legacy
    // string format drops the kind.
    use crate::effect::sink::EffectSink;
    let mut t = term_with_effect_sink();
    feed(&mut t, b"\x1b[?1;1;0S");

    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);

    let xtsm_writes: Vec<_> = effects
        .iter()
        .filter_map(|e| match e {
            Effect::Pty(PtyEffect::Write { bytes, kind }) => {
                let s = String::from_utf8_lossy(bytes);
                if s.starts_with("\x1b[?1;0;") && s.ends_with('S') {
                    Some(*kind)
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();

    assert!(
        !xtsm_writes.is_empty(),
        "expected at least one XTSMGRAPHICS PtyEffect::Write, got effects: {effects:?}"
    );
    assert!(
        xtsm_writes
            .iter()
            .all(|k| *k == PtyWriteKind::GraphicsAttributeReport),
        "XTSMGRAPHICS replies must use GraphicsAttributeReport kind, got: {xtsm_writes:?}"
    );
    // Negative pin: must NOT use DeviceAttribute (which is for DA1/DA2/DA3).
    assert!(
        xtsm_writes
            .iter()
            .all(|k| *k != PtyWriteKind::DeviceAttribute),
        "XTSMGRAPHICS must NOT use DeviceAttribute kind, got: {xtsm_writes:?}"
    );
}

// --- Image-protocol-disabled gate (3-of-3 reviewer agreement) ---
//
// Per xterm `charproc.c:5198-5200` (Pi=1) + `:5226-5227` (Pi=2): when
// sixel/ReGIS is disabled, success replies downgrade to Ps=3 and drop
// the Pv field. oriterm's `image_protocol_enabled` field is the
// canonical sixel-disabled gate (peer pattern at
// `term/handler/image/sixel.rs:28`, `iterm2.rs:35`,
// `kitty/mod.rs:61`).

#[test]
fn xtsmgraphics_pi1_pa1_replies_status3_when_image_protocol_disabled() {
    let (mut t, listener) = term_with_recorder();
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?1;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;3S)"),
        "Pi=1 Pa=1 with image protocol disabled should reply ?1;3S (no Pv), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi1_pa3_set_replies_status3_when_image_protocol_disabled() {
    let (mut t, listener) = term_with_recorder();
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?1;3;256S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;3S)"),
        "Pi=1 Pa=3 set with image protocol disabled should reply ?1;3S (set is gated), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi2_pa1_replies_status3_when_image_protocol_disabled() {
    let (mut t, listener) = term_with_recorder();
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?2;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?2;3S)"),
        "Pi=2 Pa=1 with image protocol disabled should reply ?2;3S (no W;H), got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi2_pa4_replies_status3_when_image_protocol_disabled() {
    let (mut t, listener) = term_with_recorder();
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?2;4;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?2;3S)"),
        "Pi=2 Pa=4 with image protocol disabled should reply ?2;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_pi3_unaffected_by_image_protocol_disabled() {
    // Pi=3 is already always Ps=3 since ReGIS isn't implemented; verify
    // behavior is the same regardless of `image_protocol_enabled`.
    let (mut t, listener) = term_with_recorder();
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?3;1;0S");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?3;3S)"),
        "Pi=3 with image protocol disabled should still reply ?3;3S, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_disabled_pa3_set_does_not_leak_color_register_count() {
    // Regression: round-0 TPR (opencode F1 high-severity finding) — state
    // mutation MUST NOT execute when image_protocol_enabled is false.
    // Per xterm `charproc.c:5198-5200`, the gate sits at the TOP of the
    // Pi=1 block, BEFORE any Pa dispatch. If the gate fires AFTER the
    // match, Pa=3 set leaks `color_register_count` even though the reply
    // is downgraded to Ps=3.
    //
    // Repro: disable image protocol → Pa=3 set Pv=100 → re-enable →
    // Pa=1 read MUST return 256 (default), NOT 100 (the leaked value).
    let (mut t, listener) = term_with_recorder();
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?1;3;100S"); // attempted set under disabled gate
    t.set_image_protocol_enabled(true);
    feed(&mut t, b"\x1b[?1;1;0S"); // read

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1b[?1;0;256S)"),
        "Pa=1 read after disabled-set + re-enable must return default 256, got: {events:?}"
    );
    let leaked_replies: Vec<_> = events
        .iter()
        .filter(|e| **e == "PtyWrite(\x1b[?1;0;100S)")
        .collect();
    assert!(
        leaked_replies.is_empty(),
        "no reply may carry leaked Pv=100 — state mutation under disabled gate violates xterm `charproc.c:5198-5200`, got: {events:?}"
    );
}

#[test]
fn xtsmgraphics_disabled_pa2_reset_does_not_leak_color_register_count() {
    // Companion to the Pa=3 leak test — Pa=2 reset also mutates state
    // and must be gated. Repro: set to 100 (enabled) → disable → Pa=2
    // reset (gated) → re-enable → Pa=1 read MUST return 100, NOT 256.
    // The Pa=2 reset under disabled gate must be a no-op; only the
    // re-enabled read should reflect the prior set state.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;3;100S"); // enabled set to 100
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?1;2;0S"); // attempted reset under disabled gate
    t.set_image_protocol_enabled(true);
    feed(&mut t, b"\x1b[?1;1;0S"); // read — must reflect 100, not 256

    let events = listener.events();
    let final_read = events
        .iter()
        .filter(|e| e.starts_with("PtyWrite(\x1b[?1;0;"))
        .last()
        .map(|s| s.as_str())
        .unwrap_or("(no read reply)");
    assert_eq!(
        final_read, "PtyWrite(\x1b[?1;0;100S)",
        "Pa=1 read after disabled-reset + re-enable must reflect prior set (100), got: {final_read}"
    );
}

#[test]
fn xtsmgraphics_set_image_protocol_enabled_re_enables_replies() {
    // Toggle true → false → true; verify reply behavior tracks the
    // gate (idempotency / state-restore).
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?1;1;0S"); // enabled (default) → ?1;0;256S
    t.set_image_protocol_enabled(false);
    feed(&mut t, b"\x1b[?1;1;0S"); // disabled → ?1;3S
    t.set_image_protocol_enabled(true);
    feed(&mut t, b"\x1b[?1;1;0S"); // re-enabled → ?1;0;256S

    let events = listener.events();
    let success_count = events
        .iter()
        .filter(|e| **e == "PtyWrite(\x1b[?1;0;256S)")
        .count();
    let downgrade_count = events
        .iter()
        .filter(|e| **e == "PtyWrite(\x1b[?1;3S)")
        .count();
    assert_eq!(
        success_count, 2,
        "should be 2 success replies (initial + re-enable), got: {events:?}"
    );
    assert_eq!(
        downgrade_count, 1,
        "should be 1 downgrade reply (during disabled), got: {events:?}"
    );
}
