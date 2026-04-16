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
