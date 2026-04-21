//! Catalog rows: ECMA48-CSI-REP, ECMA48-SGR-38, ECMA48-SGR-48, ECMA48-SGR-58,
//! ECMA48-SGR-53, ECMA48-SGR-55, ECMA48-SGR-73, ECMA48-SGR-74, ECMA48-SGR-75,
//! ECMA48-CSI-DECSTR, ECMA48-CSI-DECSED, ECMA48-CSI-DECSEL, ECMA48-CSI-SL,
//! ECMA48-CSI-SR, ECMA48-DCS-DECRQSS-DECSLRM,
//! XT-PUSHSGR, XT-POPSGR

use crate::index::Column;
use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_recorder, term_with_recorder_sized};

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- CSI scroll region tests ---

#[test]
fn decstbm_sets_scroll_region() {
    let mut t = term();
    // CSI 3;20 r — set scroll region lines 3–20 (1-based).
    feed(&mut t, b"\x1b[3;20r");

    let region = t.grid().scroll_region();
    assert_eq!(region.start, 2); // 3 - 1 = 2 (0-based).
    assert_eq!(region.end, 20); // 20 (half-open).
    // Cursor should be at origin after DECSTBM.
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- CHA under DECOM + DECLRMM matrix ---
// DEC STD 070 §4.6.10: CHA is absolute addressing unless DECOM is set.
// With DECOM active, the column parameter is relative to the left margin
// (when DECLRMM is also set) and clamped to the right margin.
// Matrix dimensions: 2 DECLRMM states x 2 DECOM states = 4 cells.

#[test]
fn cha_absolute_when_declrmm_off_decom_off() {
    let mut t = term();
    // Both modes off (default): CHA 5 → col 4.
    feed(&mut t, b"\x1b[5G");
    assert_eq!(t.grid().cursor().col(), Column(4));
}

#[test]
fn cha_absolute_when_declrmm_on_decom_off() {
    let mut t = term();
    // DECLRMM on, DECOM off, left margin = 10.
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[11;40s"); // DECSLRM: left=10, right=39 (0-based).
    // CHA 5 under DECOM off is ABSOLUTE — must land at col 4, NOT col 14.
    feed(&mut t, b"\x1b[5G");
    assert_eq!(
        t.grid().cursor().col(),
        Column(4),
        "CHA with DECOM off must be absolute even with DECLRMM active",
    );
}

#[test]
fn cha_absolute_when_declrmm_off_decom_on() {
    let mut t = term();
    // DECOM on without DECLRMM: no left margin to offset by, so CHA is
    // still absolute (left_margin is 0 by default).
    feed(&mut t, b"\x1b[?6h");
    feed(&mut t, b"\x1b[5G");
    assert_eq!(t.grid().cursor().col(), Column(4));
}

#[test]
fn cha_offsets_by_left_margin_when_declrmm_on_decom_on() {
    let mut t = term();
    // DECLRMM on with left margin 10, DECOM on.
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[11;40s"); // DECSLRM: left=10, right=39 (0-based).
    feed(&mut t, b"\x1b[?6h"); // DECOM on.
    // CHA 5: relative to left margin, so col = (5-1)+10 = 14.
    feed(&mut t, b"\x1b[5G");
    assert_eq!(
        t.grid().cursor().col(),
        Column(14),
        "CHA with DECOM+DECLRMM must offset by left_margin: col=5 → (5-1)+10=14",
    );
}

#[test]
fn cha_clamps_to_right_margin_under_decom_declrmm() {
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[11;40s"); // left=10, right=39 (0-based).
    feed(&mut t, b"\x1b[?6h");
    // CHA 100: (100-1)+10 = 109, clamped to right=39.
    feed(&mut t, b"\x1b[100G");
    assert_eq!(t.grid().cursor().col(), Column(39));
}

#[test]
fn cha_col_1_lands_at_left_margin_under_decom_declrmm() {
    // Positive edge-case pin: `CSI 1 G` (col=0 zero-based) under
    // DECOM+DECLRMM must resolve to `left_margin`. This test does NOT
    // distinguish the pre-fix and post-fix code paths — with col=0,
    // the pre-fix `Grid::move_to_column` clamp to `[left_margin,
    // right_margin]` coincidentally also landed at `left_margin`. The
    // true regression guard for the offset is
    // `cha_offsets_by_left_margin_when_declrmm_on_decom_on` (col=5 →
    // col=14), which fails on the pre-fix clamp path.
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[11;40s");
    feed(&mut t, b"\x1b[?6h");
    feed(&mut t, b"\x1b[1G");
    assert_eq!(t.grid().cursor().col(), Column(10));
}

// --- DECSC / DECRC full round-trip ---

#[test]
fn decsc_decrc_saves_and_restores_cursor_position() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;10H"); // CUP to line 4, col 9
    feed(&mut t, b"\x1b7"); // DECSC: save cursor
    feed(&mut t, b"\x1b[1;1H"); // Move somewhere else
    feed(&mut t, b"\x1b8"); // DECRC: restore cursor

    assert_eq!(t.grid().cursor().line(), 4);
    assert_eq!(t.grid().cursor().col(), Column(9));
}

// --- Unknown DECSET/DECRST mode is silently ignored ---

#[test]
fn unknown_decset_mode_is_silently_ignored() {
    let mut t = term();
    let before = t.mode();

    // Feed an unknown private mode via DECSET.
    feed(&mut t, b"\x1b[?9999h");

    // Mode should be unchanged, no panic.
    assert_eq!(t.mode(), before);

    // Terminal still functional.
    feed(&mut t, b"ok");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'o');
}

#[test]
fn unknown_decrst_mode_is_silently_ignored() {
    let mut t = term();
    // Set a known mode first.
    feed(&mut t, b"\x1b[?1000h");
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));

    // DECRST with unknown mode number.
    feed(&mut t, b"\x1b[?9999l");

    // Known mode should be unaffected.
    assert!(t.mode().contains(TermMode::MOUSE_REPORT_CLICK));
}

// --- DECCOLM (132-column mode) tests ---

#[test]
fn deccolm_set_clears_screen() {
    let mut t = term();
    // Write content, then set DECCOLM (CSI ? 3 h).
    feed(&mut t, b"Hello, world!");
    feed(&mut t, b"\x1b[?3h");
    // Screen should be cleared — first cell is blank.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, ' ');
    // Cursor should be at origin.
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn deccolm_reset_clears_screen() {
    let mut t = term();
    // Write content, then reset DECCOLM (CSI ? 3 l).
    feed(&mut t, b"Hello, world!");
    feed(&mut t, b"\x1b[?3l");
    // Screen should be cleared.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, ' ');
    // Cursor at origin.
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

#[test]
fn deccolm_preserves_grid_dimensions() {
    let mut t = term();
    let (lines, cols) = (t.grid().lines(), t.grid().cols());
    // Set DECCOLM — grid should NOT resize.
    feed(&mut t, b"\x1b[?3h");
    assert_eq!(t.grid().lines(), lines);
    assert_eq!(t.grid().cols(), cols);
    // Reset DECCOLM — grid still same size.
    feed(&mut t, b"\x1b[?3l");
    assert_eq!(t.grid().lines(), lines);
    assert_eq!(t.grid().cols(), cols);
}

#[test]
fn deccolm_resets_scroll_region() {
    let mut t = term();
    // Set a scroll region, then toggle DECCOLM.
    feed(&mut t, b"\x1b[5;15r"); // DECSTBM 5–15
    feed(&mut t, b"\x1b[?3h");
    // Scroll region should be reset to full screen.
    // Verify by writing at line 1 and checking the region doesn't constrain.
    let region = t.grid().scroll_region();
    assert_eq!(region.start, 0);
    assert_eq!(region.end, t.grid().lines());
}

#[test]
fn decawm_wrap_fills_line() {
    let mut t = term();
    // DECAWM is on by default. Write 81 chars — 80 fill row 0, 81st wraps.
    feed(&mut t, &[b'*'; 81]);
    // 81st char triggers wrap to row 1, then writes at col 0.
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(1));
    // Row 0 should be fully filled with '*'.
    for col in 0..80 {
        assert_eq!(
            t.grid()[crate::index::Line(0)][Column(col)].ch,
            '*',
            "col {col} should be '*'"
        );
    }
    // Row 1 col 0 should also be '*' (the 81st char).
    assert_eq!(t.grid()[crate::index::Line(1)][Column(0)].ch, '*');
}

#[test]
fn decawm_off_no_wrap() {
    let mut t = term();
    // Disable DECAWM (CSI ? 7 l).
    feed(&mut t, b"\x1b[?7l");
    // Write 85 chars — more than 80 columns.
    for i in 0..85 {
        let ch = b'A' + (i % 26);
        feed(&mut t, &[ch]);
    }
    // Cursor should stay on line 0, at wrap-pending position (col 80).
    assert_eq!(t.grid().cursor().line(), 0);
    // Last column should contain the last character written (85th char = 'K').
    assert_eq!(
        t.grid()[crate::index::Line(0)][Column(79)].ch,
        char::from(b'A' + (84 % 26))
    );
    // Row 1 should be empty — no wrap occurred.
    assert_eq!(t.grid()[crate::index::Line(1)][Column(0)].ch, ' ');
}

#[test]
fn decawm_with_control_chars() {
    let mut t = term();
    // DECAWM on (default). Write 78 chars, then BS, then 4 more chars.
    // BS at col 78 → col 77. Then 4 chars fill cols 77–80, wrapping.
    feed(&mut t, &[b'X'; 78]);
    feed(&mut t, b"\x08"); // BS
    feed(&mut t, b"ABCD");
    // After BS: col 77. A→77, B→78, C→79, D→wrap to line 1, col 0.
    // Wait, actually after 78 X's, cursor is at col 78. BS → col 77.
    // A→77 (cursor 78), B→78 (cursor 79), C→79 (cursor 80 = wrap pending).
    // D triggers wrap → line 1, col 0, write D.
    assert_eq!(t.grid().cursor().line(), 1);
    assert_eq!(t.grid().cursor().col(), Column(1));
    assert_eq!(t.grid()[crate::index::Line(1)][Column(0)].ch, 'D');
}

#[test]
fn deccolm_set_then_reset_roundtrip() {
    let mut t = term();
    let (lines, cols) = (t.grid().lines(), t.grid().cols());
    // Write content.
    feed(&mut t, b"ABCDEF");
    // Set DECCOLM.
    feed(&mut t, b"\x1b[?3h");
    // Write new content.
    feed(&mut t, b"XYZ");
    // Reset DECCOLM.
    feed(&mut t, b"\x1b[?3l");
    // Grid dimensions unchanged throughout.
    assert_eq!(t.grid().lines(), lines);
    assert_eq!(t.grid().cols(), cols);
    // Screen cleared by DECCOLM reset.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, ' ');
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col(), Column(0));
}

// --- DECSTBM scroll region with IL/DL ---

#[test]
fn scroll_region_fill_preserves_row_zero() {
    // Diagnostic: does filling rows with linefeeds inside a scroll region
    // corrupt row 0 (which is outside the region)?
    let mut t = term();

    // Write A's on row 0.
    feed(&mut t, b"AAAAAAAAAA");
    // Move to row 1 (CR+LF).
    feed(&mut t, b"\r\n");
    // Set scroll region: rows 2-24 (1-based), i.e., 0-based 1..24.
    feed(&mut t, b"\x1b[2;24r");
    // Home cursor to row 2, col 1 (inside scroll region).
    feed(&mut t, b"\x1b[2;1H");

    // Fill rows inside region with B's via prints + linefeeds.
    for _ in 0..30 {
        feed(&mut t, b"BBBBBBBBBB\n");
    }

    // Check row 0.
    let grid = t.grid();
    assert_eq!(
        grid[crate::index::Line(0)][Column(0)].ch,
        'A',
        "row 0 should still be A after fill with scroll region active"
    );
}

#[test]
fn il_within_scroll_region_preserves_row_zero() {
    let mut t = term();
    // Write A's on row 0.
    feed(&mut t, b"AAAAAAAAAA");
    feed(&mut t, b"\r\n");
    // Write B's on row 1.
    feed(&mut t, b"BBBBBBBBBB");
    // Set scroll region: rows 2-24 (1-based), i.e., 0-based 1..24.
    feed(&mut t, b"\x1b[2;24r");
    // Position cursor at row 2 col 1 (inside scroll region).
    feed(&mut t, b"\x1b[2;1H");
    // IL 1: insert 1 blank line at cursor.
    feed(&mut t, b"\x1b[1L");

    let grid = t.grid();
    assert_eq!(
        grid[crate::index::Line(0)][Column(0)].ch,
        'A',
        "row 0 should still be A after IL within scroll region"
    );
    assert_eq!(
        grid[crate::index::Line(1)][Column(0)].ch,
        ' ',
        "row 1 should be blank (inserted line)"
    );
}

#[test]
fn accordion_with_scroll_region_preserves_row_zero() {
    // Exact vttest menu 8 reproduction: first round fill → second round accordion.
    // vttest source: /tmp/vttest-20251205/main.c lines 961-986.
    let mut t = term(); // 24x80

    // First round fill (main.c:961-966): fill all 24 rows with A-X.
    feed(&mut t, b"\x1b[2J"); // ED: clear screen
    feed(&mut t, b"\x1b[1;1H"); // CUP(1,1)
    for row in 1..=24u8 {
        feed(&mut t, &format!("\x1b[{};1H", row).into_bytes());
        let ch = b'A' - 1 + row;
        let line: Vec<u8> = vec![ch; 80];
        feed(&mut t, &line);
    }
    // Prompt overlay (main.c:968-970).
    feed(&mut t, b"\x1b[4;1H");
    feed(
        &mut t,
        b"Screen accordion test (Insert & Delete Line). Push <RETURN>",
    );

    // Verify row 0 has A's.
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].ch, 'A');

    // Second round setup (main.c:972-975): RI, EL 2, DECSTBM, DECOM.
    feed(&mut t, b"\x1bM"); // RI: reverse index
    feed(&mut t, b"\x1b[2K"); // EL 2: erase entire line
    feed(&mut t, b"\x1b[2;23r"); // DECSTBM(2,23): scroll region rows 2-23
    feed(&mut t, b"\x1b[?6h"); // DECOM ON

    // CUP(1,1) with DECOM (main.c:976).
    feed(&mut t, b"\x1b[1;1H");

    // Accordion loop (main.c:977-980): IL(n), DL(n) for n=1..max_lines.
    for n in 1..=24u8 {
        feed(&mut t, &format!("\x1b[{}L", n).into_bytes());
        feed(&mut t, &format!("\x1b[{}M", n).into_bytes());
    }

    // Cleanup (main.c:981-982).
    feed(&mut t, b"\x1b[?6l"); // DECOM OFF
    feed(&mut t, b"\x1b[r"); // Reset DECSTBM

    // Row 0 should still have A's — it was outside the scroll region.
    assert_eq!(
        t.grid()[crate::index::Line(0)][Column(0)].ch,
        'A',
        "row 0 should still be A after second-round accordion"
    );
}

// --- DECLRMM (mode 69) plumbing tests (§08.3) ---

#[test]
fn mode_69_set_inserts_left_right_margin_flag() {
    use crate::term::TermMode;
    let mut t = term();
    assert!(!t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));
    feed(&mut t, b"\x1b[?69h");
    assert!(
        t.mode().contains(TermMode::LEFT_RIGHT_MARGIN),
        "DECSET ?69 must set LEFT_RIGHT_MARGIN flag"
    );
}

#[test]
fn mode_69_reset_removes_left_right_margin_flag() {
    use crate::term::TermMode;
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    assert!(t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));
    feed(&mut t, b"\x1b[?69l");
    assert!(
        !t.mode().contains(TermMode::LEFT_RIGHT_MARGIN),
        "DECRST ?69 must clear LEFT_RIGHT_MARGIN flag"
    );
}

#[test]
fn mode_69_decrqm_reports_correctly() {
    use crate::effect::Effect;
    use crate::effect::sink::EffectSink;

    let mut t = super::super::test_helpers::term_with_effect_sink();

    // Mode 69 defaults to reset (2).
    feed(&mut t, b"\x1b[?69$p");
    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    let response = effects
        .iter()
        .find_map(|e| match e {
            Effect::Pty(crate::effect::PtyEffect::Write { bytes, .. }) => {
                Some(String::from_utf8_lossy(bytes).to_string())
            }
            _ => None,
        })
        .expect("DECRQM should produce a PtyEffect::Write");
    assert_eq!(response, "\x1b[?69;2$y", "mode 69 should report reset (2)");

    // Enable mode 69, then query again — should report set (1).
    feed(&mut t, b"\x1b[?69h\x1b[?69$p");
    effects.clear();
    t.effect_sink().drain_into(&mut effects);
    let response = effects
        .iter()
        .find_map(|e| match e {
            Effect::Pty(crate::effect::PtyEffect::Write { bytes, .. }) => {
                Some(String::from_utf8_lossy(bytes).to_string())
            }
            _ => None,
        })
        .expect("DECRQM should produce a PtyEffect::Write after DECSET");
    assert_eq!(response, "\x1b[?69;1$y", "mode 69 should report set (1)");
}

// --- CSI s / DECSLRM ambiguity (§08.5b) ---

#[test]
fn csi_s_zero_params_mode_69_off_saves_cursor() {
    let mut t = term();
    // Move cursor to (5, 10).
    feed(&mut t, b"\x1b[6;11H");
    assert_eq!(t.grid().cursor().line(), 5);
    assert_eq!(t.grid().cursor().col(), Column(10));
    // CSI s with no params, mode 69 OFF: should save cursor.
    feed(&mut t, b"\x1b[s");
    // Move cursor elsewhere.
    feed(&mut t, b"\x1b[1;1H");
    assert_eq!(t.grid().cursor().line(), 0);
    // Restore cursor: should go back to (5, 10).
    feed(&mut t, b"\x1b[u");
    assert_eq!(t.grid().cursor().line(), 5);
    assert_eq!(t.grid().cursor().col(), Column(10));
}

#[test]
fn csi_s_zero_params_mode_69_on_sets_default_margins() {
    let mut t = term();
    // Enable mode 69 (DECLRMM).
    feed(&mut t, b"\x1b[?69h");
    // Set non-default margins first.
    t.grid_mut().set_left_right_margins(5, 20);
    assert_eq!(t.grid().left_right_margins(), (5, 20));
    // CSI s with no params, mode 69 ON: should reset margins to full width.
    feed(&mut t, b"\x1b[s");
    assert_eq!(
        t.grid().left_right_margins(),
        (0, t.grid().cols() - 1),
        "CSI s with mode 69 ON should reset margins to full width"
    );
}

#[test]
fn csi_s_with_params_always_decslrm() {
    let mut t = term();
    // Enable mode 69.
    feed(&mut t, b"\x1b[?69h");
    // CSI 5 ; 20 s — should set margins to (4, 19) in 0-based.
    feed(&mut t, b"\x1b[5;20s");
    assert_eq!(
        t.grid().left_right_margins(),
        (4, 19),
        "CSI 5;20 s with mode 69 ON should set margins"
    );
}

#[test]
fn csi_s_with_params_mode_69_off_is_noop() {
    let mut t = term();
    // Mode 69 OFF (default).
    // Move cursor to (3, 7) and save it explicitly first.
    feed(&mut t, b"\x1b[4;8H");
    feed(&mut t, b"\x1b7"); // DECSC saves cursor.
    assert_eq!(t.grid().cursor().line(), 3);
    // Now try CSI 5 ; 20 s — with params but mode 69 OFF: should be no-op.
    feed(&mut t, b"\x1b[5;20s");
    // Margins should stay at full width.
    assert_eq!(
        t.grid().left_right_margins(),
        (0, t.grid().cols() - 1),
        "CSI 5;20 s with mode 69 OFF should be no-op"
    );
    // Also verify cursor was NOT saved by this — move cursor, restore,
    // and check we get back the DECSC save, not a CSI s save.
    feed(&mut t, b"\x1b[1;1H");
    feed(&mut t, b"\x1b8"); // DECRC restores.
    assert_eq!(
        t.grid().cursor().line(),
        3,
        "cursor should restore to DECSC save, not CSI s"
    );
}

/// Regression: `CSI 0;0 s` has TWO explicit params (a semicolon was
/// parsed) and must be treated as DECSLRM regardless of
/// parameter values. With mode 69 OFF, this must be a no-op (like any
/// parameterized DECSLRM when DECLRMM is inactive), NOT save cursor.
///
/// Before the fix, `has_params` was computed from parameter VALUES only
/// (`left != 0 || right != 0`), so `CSI 0;0 s` with all-zero values was
/// collapsed into the save-cursor branch — a behavioral divergence from
/// WezTerm/Ghostty for a legal DECSLRM default-request sequence.
#[test]
fn csi_s_zero_zero_params_mode_69_off_is_noop_not_save_cursor() {
    let mut t = term();
    // DECSC explicitly so we can tell whether `CSI 0;0 s` overwrote it.
    feed(&mut t, b"\x1b[4;8H"); // cursor at (3, 7)
    feed(&mut t, b"\x1b7"); // DECSC
    assert_eq!(t.grid().cursor().line(), 3);
    // Mode 69 is OFF by default. `CSI 0;0 s` has params (semicolon seen)
    // → must route to DECSLRM → must be a no-op (mode 69 inactive).
    feed(&mut t, b"\x1b[0;0s");
    // Margins unchanged (still full width).
    assert_eq!(
        t.grid().left_right_margins(),
        (0, t.grid().cols() - 1),
        "CSI 0;0 s with mode 69 OFF must be a no-op",
    );
    // Cursor saved-state unchanged — DECRC should restore the DECSC save,
    // not a save cursor triggered spuriously by `CSI 0;0 s`.
    feed(&mut t, b"\x1b[1;1H");
    feed(&mut t, b"\x1b8"); // DECRC
    assert_eq!(
        t.grid().cursor().line(),
        3,
        "DECRC should restore DECSC save; CSI 0;0 s must not overwrite the save slot",
    );
}

/// Regression: `CSI 0;0 s` with mode 69 ON is treated as DECSLRM with
/// explicit default values, which per DEC STD
/// 070 resets margins to full width.
#[test]
fn csi_s_zero_zero_params_mode_69_on_resets_margins() {
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    t.grid_mut().set_left_right_margins(5, 40);
    // Explicit defaults: left=1 (1-based) = 0 (0-based), right=cols.
    // VT `0` in DECSLRM means "use default" per DEC STD 070 §4.6.10.
    feed(&mut t, b"\x1b[0;0s");
    assert_eq!(
        t.grid().left_right_margins(),
        (0, t.grid().cols() - 1),
        "CSI 0;0 s with mode 69 ON should reset margins to full width",
    );
}

// --- DECSC/DECRC scope: margins + DECLRMM are NOT in the save set ---
// Per DEC STD 070 §5.6.1 and cross-verified against wezterm / alacritty /
// ghostty: the DECSC save set is cursor position + attributes + charsets +
// wrap flag + DECOM flag. Left/right margins and the DECLRMM mode flag are
// NOT saved. Reset paths (RIS, DECSTR, DECCOLM, DECALN, resize, explicit
// DECRST ?69) handle margin clearing — DECSC/DECRC do not touch margins.

#[test]
fn decrc_does_not_restore_horizontal_margins() {
    // Negative pin: after DECSC saves the cursor, a subsequent DECSLRM
    // change to the margins must SURVIVE a DECRC — the margins are not
    // part of the save set.
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[5;40s"); // DECSLRM: left=5, right=40 → (4, 39) 0-based.
    assert_eq!(t.grid().left_right_margins(), (4, 39));
    feed(&mut t, b"\x1b7"); // DECSC
    feed(&mut t, b"\x1b[10;60s"); // change margins to (9, 59).
    assert_eq!(t.grid().left_right_margins(), (9, 59));
    feed(&mut t, b"\x1b8"); // DECRC
    assert_eq!(
        t.grid().left_right_margins(),
        (9, 59),
        "DECRC must NOT restore saved margins (margins are not in the DECSC save set)"
    );
}

#[test]
fn decrc_does_not_restore_declrmm_mode_flag() {
    use crate::term::TermMode;
    // Negative pin: DECSC with mode 69 on, then DECRST ?69 turns it off.
    // DECRC must leave mode 69 OFF — the mode flag is not part of the
    // save set.
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    assert!(t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));
    feed(&mut t, b"\x1b7"); // DECSC
    feed(&mut t, b"\x1b[?69l"); // disable DECLRMM
    assert!(!t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));
    feed(&mut t, b"\x1b8"); // DECRC
    assert!(
        !t.mode().contains(TermMode::LEFT_RIGHT_MARGIN),
        "DECRC must NOT resurrect DECLRMM mode (mode 69 is not in the DECSC save set)"
    );
}

#[test]
fn decrc_does_not_enable_declrmm_after_disabled_save() {
    use crate::term::TermMode;
    // Symmetric negative pin: DECSC with mode 69 off, then enable it.
    // DECRC must leave mode 69 ON — the restore cannot resurrect the
    // saved off-state either direction.
    let mut t = term();
    assert!(!t.mode().contains(TermMode::LEFT_RIGHT_MARGIN));
    feed(&mut t, b"\x1b7"); // DECSC with mode 69 off
    feed(&mut t, b"\x1b[?69h"); // enable DECLRMM
    feed(&mut t, b"\x1b8"); // DECRC
    assert!(
        t.mode().contains(TermMode::LEFT_RIGHT_MARGIN),
        "DECRC must not toggle DECLRMM mode off from the save slot"
    );
}

#[test]
fn decsc_decrc_restores_cursor_position_and_origin() {
    // Positive pin: verify that DECSC/DECRC DO restore the state that
    // IS in the save set (cursor, origin mode) — regression guard
    // ensuring the scope removal did not break the actual contract.
    use crate::term::TermMode;
    let mut t = term();
    feed(&mut t, b"\x1b[?6h"); // DECOM on.
    feed(&mut t, b"\x1b[5;10H"); // CUP under DECOM (origin-relative).
    let saved_line = t.grid().cursor().line();
    let saved_col = t.grid().cursor().col();
    feed(&mut t, b"\x1b7"); // DECSC
    // Move and toggle DECOM.
    feed(&mut t, b"\x1b[1;1H");
    feed(&mut t, b"\x1b[?6l");
    assert!(!t.mode().contains(TermMode::ORIGIN));
    feed(&mut t, b"\x1b8"); // DECRC
    assert_eq!(t.grid().cursor().line(), saved_line);
    assert_eq!(t.grid().cursor().col(), saved_col);
    assert!(
        t.mode().contains(TermMode::ORIGIN),
        "DECRC must restore DECOM flag (DECOM IS in the save set)",
    );
}

// --- Reset paths that clear margins (§08.5d) ---

#[test]
fn decrst_69_resets_margins_to_full_width() {
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    t.grid_mut().set_left_right_margins(5, 40);
    assert_eq!(t.grid().left_right_margins(), (5, 40));
    // DECRST ?69: disable DECLRMM → margins must reset to full width.
    feed(&mut t, b"\x1b[?69l");
    assert_eq!(
        t.grid().left_right_margins(),
        (0, t.grid().cols() - 1),
        "DECRST 69 must reset margins to full width"
    );
}

#[test]
fn deccolm_resets_horizontal_margins() {
    let mut t = term();
    // Enable mode 40 (allow DECCOLM) first.
    feed(&mut t, b"\x1b[?40h");
    feed(&mut t, b"\x1b[?69h");
    t.grid_mut().set_left_right_margins(5, 40);
    assert_eq!(t.grid().left_right_margins(), (5, 40));
    // DECSET ?3 (132-column mode) triggers DECCOLM reset.
    feed(&mut t, b"\x1b[?3h");
    let cols = t.grid().cols();
    assert_eq!(
        t.grid().left_right_margins(),
        (0, cols - 1),
        "DECCOLM must reset horizontal margins"
    );
}

#[test]
fn ris_resets_horizontal_margins() {
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    t.grid_mut().set_left_right_margins(5, 40);
    assert_eq!(t.grid().left_right_margins(), (5, 40));
    // RIS (ESC c): full terminal reset.
    feed(&mut t, b"\x1bc");
    assert_eq!(
        t.grid().left_right_margins(),
        (0, t.grid().cols() - 1),
        "RIS must reset horizontal margins"
    );
}

#[test]
fn resize_resets_horizontal_margins() {
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    t.grid_mut().set_left_right_margins(5, 40);
    assert_eq!(t.grid().left_right_margins(), (5, 40));
    // Resize the grid.
    t.grid_mut().resize(24, 120, false);
    assert_eq!(
        t.grid().left_right_margins(),
        (0, 119),
        "resize must reset horizontal margins"
    );
}

#[test]
fn decaln_resets_horizontal_margins() {
    let mut t = term();
    feed(&mut t, b"\x1b[?69h");
    t.grid_mut().set_left_right_margins(5, 40);
    assert_eq!(t.grid().left_right_margins(), (5, 40));
    // DECALN (ESC # 8): alignment test.
    feed(&mut t, b"\x1b#8");
    assert_eq!(
        t.grid().left_right_margins(),
        (0, t.grid().cols() - 1),
        "DECALN must reset horizontal margins"
    );
}

// ── REP (CSI Ps b) edge cases — §08.7 ──────────────────────────────

/// REP with no preceding graphic character is a no-op.
///
/// Spec: ECMA-48 §8.3.103
#[test]
fn rep_no_preceding_char_is_noop() {
    let mut t = term();
    // No prior graphic character; issue REP 3.
    feed(&mut t, b"\x1b[3b");

    let grid = t.grid();
    // Grid should be entirely blank.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, ' ');
    // Cursor should not have moved.
    assert_eq!(grid.cursor().col(), Column(0));
}

/// REP after CR repeats the preceding graphic character (de-facto
/// behavior matching xterm, alacritty, wezterm, ghostty).
///
/// The VTE parser tracks `preceding_char` as the last printed graphic
/// character. C0 controls (including CR) do not clear it. This matches
/// all major terminal emulators.
#[test]
fn rep_after_cr_repeats_preceding() {
    let mut t = term();
    // Print 'A', CR, REP 3 — all in one feed (preceding_char is per-Processor).
    feed(&mut t, b"A\r\x1b[3b");

    let grid = t.grid();
    // REP repeats 'A' 3 times starting from cursor position (col 0).
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'A');
    // Cursor at col 3 after printing 3 chars.
    assert_eq!(grid.cursor().col(), Column(3));
}

/// REP after a wide character repeats the wide character.
///
/// Spec: ECMA-48 §8.3.103 — repeats "the graphic character".
/// CJK ideograph '漢' (U+6F22, width 2).
#[test]
fn rep_after_wide_char_repeats_wide() {
    use crate::cell::CellFlags;

    let mut t = term();
    // '漢' U+6F22 (3-byte UTF-8: E6 BC A2) + REP 2 in one feed call
    // (preceding_char is per-Processor, and feed() creates a fresh one).
    feed(&mut t, b"\xe6\xbc\xa2\x1b[2b");

    let grid = t.grid();
    // Original wide char at cols 0-1.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, '漢');
    assert!(
        grid[crate::index::Line(0)][Column(0)]
            .flags
            .contains(CellFlags::WIDE_CHAR)
    );
    // First repeat at cols 2-3.
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, '漢');
    assert!(
        grid[crate::index::Line(0)][Column(2)]
            .flags
            .contains(CellFlags::WIDE_CHAR)
    );
    // Second repeat at cols 4-5.
    assert_eq!(grid[crate::index::Line(0)][Column(4)].ch, '漢');
    assert!(
        grid[crate::index::Line(0)][Column(4)]
            .flags
            .contains(CellFlags::WIDE_CHAR)
    );
    // 3 wide chars × 2 columns = 6 columns, cursor at col 6.
    assert_eq!(grid.cursor().col(), Column(6));
}

/// REP uses the current SGR state, not the SGR at print time.
///
/// Spec: ECMA-48 §8.3.103 — "the effect of REP is as if the graphic
/// character were present in the data stream Ps times." The character
/// is present NOW, so it inherits the current SGR, not the original.
#[test]
fn rep_uses_current_sgr_not_original() {
    let mut t = term();
    // Print 'A' (default fg), set SGR 31 (red fg), then REP 2.
    feed(&mut t, b"A\x1b[31m\x1b[2b");

    let grid = t.grid();
    // Original 'A' at col 0 has default fg.
    let a_orig = &grid[crate::index::Line(0)][Column(0)];
    assert_eq!(a_orig.ch, 'A');
    // Repeated 'A' at col 1 has red fg (SGR 31 was active during REP).
    let a_rep = &grid[crate::index::Line(0)][Column(1)];
    assert_eq!(a_rep.ch, 'A');
    assert_eq!(
        a_rep.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
    // Original and repeated chars have different fg.
    assert_ne!(a_orig.fg, a_rep.fg);
}

/// REP at the right margin triggers auto-wrap.
#[test]
fn rep_at_right_margin_wraps() {
    // Use a narrow terminal so we can test wrapping easily.
    let (mut t, _rec) = term_with_recorder_sized(3, 10);
    // Move cursor to col 8, print 'X', then REP 3.
    // Col 8: 'X' printed, cursor at 9. REP 3: col 9, wrap to 0, col 1.
    feed(&mut t, b"\x1b[1;9H");
    feed(&mut t, b"X\x1b[3b");

    let grid = t.grid();
    // 'X' at col 8 (original).
    assert_eq!(grid[crate::index::Line(0)][Column(8)].ch, 'X');
    // 'X' at col 9 (first repeat, fills last column).
    assert_eq!(grid[crate::index::Line(0)][Column(9)].ch, 'X');
    // After wrapping to line 1: 'X' at cols 0, 1.
    assert_eq!(grid[crate::index::Line(1)][Column(0)].ch, 'X');
    assert_eq!(grid[crate::index::Line(1)][Column(1)].ch, 'X');
}

/// CSI 0 b repeats once (Ps=0 treated as default=1 per ECMA-48 §5.4).
///
/// Negative pin: verifies that 0 is not treated literally as "repeat 0
/// times" but mapped to the default of 1.
#[test]
fn rep_count_zero_repeats_once() {
    let mut t = term();
    feed(&mut t, b"A\x1b[0b");

    let grid = t.grid();
    // Original 'A' at col 0.
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    // One repeat at col 1.
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'A');
    // No further repeats.
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, ' ');
    assert_eq!(grid.cursor().col(), Column(2));
}

// ── ISO 8613-6 SGR colon-separated subparameter forms — §08.8 ───────

const RGB_255_128_64: vte::ansi::Color = vte::ansi::Color::Spec(vte::ansi::Rgb {
    r: 255,
    g: 128,
    b: 64,
});
const INDEXED_123: vte::ansi::Color = vte::ansi::Color::Indexed(123);

// Matrix: 3 targets (fg=38, bg=48, underline=58) × 2 modes (truecolor=2, indexed=5) × 2 separators

#[test]
fn sgr_38_semicolon_truecolor() {
    let mut t = term();
    feed(&mut t, b"\x1b[38;2;255;128;64mA");
    assert_eq!(
        t.grid()[crate::index::Line(0)][Column(0)].fg,
        RGB_255_128_64
    );
}

#[test]
fn sgr_38_colon_truecolor_no_colorspace() {
    let mut t = term();
    feed(&mut t, b"\x1b[38:2::255:128:64mA");
    assert_eq!(
        t.grid()[crate::index::Line(0)][Column(0)].fg,
        RGB_255_128_64
    );
}

#[test]
fn sgr_38_colon_truecolor_with_colorspace() {
    let mut t = term();
    feed(&mut t, b"\x1b[38:2:0:255:128:64mA");
    assert_eq!(
        t.grid()[crate::index::Line(0)][Column(0)].fg,
        RGB_255_128_64
    );
}

#[test]
fn sgr_38_semicolon_indexed() {
    let mut t = term();
    feed(&mut t, b"\x1b[38;5;123mA");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].fg, INDEXED_123);
}

#[test]
fn sgr_38_colon_indexed() {
    let mut t = term();
    feed(&mut t, b"\x1b[38:5:123mA");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].fg, INDEXED_123);
}

#[test]
fn sgr_48_semicolon_truecolor() {
    let mut t = term();
    feed(&mut t, b"\x1b[48;2;255;128;64mA");
    assert_eq!(
        t.grid()[crate::index::Line(0)][Column(0)].bg,
        RGB_255_128_64
    );
}

#[test]
fn sgr_48_colon_truecolor() {
    let mut t = term();
    feed(&mut t, b"\x1b[48:2::255:128:64mA");
    assert_eq!(
        t.grid()[crate::index::Line(0)][Column(0)].bg,
        RGB_255_128_64
    );
}

#[test]
fn sgr_48_semicolon_indexed() {
    let mut t = term();
    feed(&mut t, b"\x1b[48;5;123mA");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].bg, INDEXED_123);
}

#[test]
fn sgr_48_colon_indexed() {
    let mut t = term();
    feed(&mut t, b"\x1b[48:5:123mA");
    assert_eq!(t.grid()[crate::index::Line(0)][Column(0)].bg, INDEXED_123);
}

#[test]
fn sgr_58_semicolon_truecolor() {
    let mut t = term();
    feed(&mut t, b"\x1b[58;2;255;128;64mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    let extra = cell.extra.as_ref().expect("CellExtra allocated");
    assert_eq!(extra.underline_color, Some(RGB_255_128_64));
}

#[test]
fn sgr_58_colon_truecolor() {
    let mut t = term();
    feed(&mut t, b"\x1b[58:2::255:128:64mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    let extra = cell.extra.as_ref().expect("CellExtra allocated");
    assert_eq!(extra.underline_color, Some(RGB_255_128_64));
}

#[test]
fn sgr_58_semicolon_indexed() {
    let mut t = term();
    feed(&mut t, b"\x1b[58;5;123mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    let extra = cell.extra.as_ref().expect("CellExtra allocated");
    assert_eq!(extra.underline_color, Some(INDEXED_123));
}

#[test]
fn sgr_58_colon_indexed() {
    let mut t = term();
    feed(&mut t, b"\x1b[58:5:123mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    let extra = cell.extra.as_ref().expect("CellExtra allocated");
    assert_eq!(extra.underline_color, Some(INDEXED_123));
}

/// Mixed colon+semicolon separators produce incomplete params.
/// `38:2::255;128;64` splits as `[38,2,0,255]` + `[128]` + `[64]` — the
/// colon group has only 3 values after the mode byte, not enough for RGB.
#[test]
fn sgr_38_mixed_separators_does_not_parse() {
    let mut t = term();
    feed(&mut t, b"\x1b[38:2::255;128;64mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert_ne!(
        cell.fg, RGB_255_128_64,
        "mixed separators must not produce correct RGB"
    );
}

/// `38:2::R:G:B` and `38:2:0:R:G:B` are indistinguishable at dispatch
/// time because the VTE parser represents `::` as `:0:`.
#[test]
fn sgr_38_double_colon_vs_zero_indistinguishable() {
    let mut t1 = term();
    feed(&mut t1, b"\x1b[38:2::255:128:64mA");
    let mut t2 = term();
    feed(&mut t2, b"\x1b[38:2:0:255:128:64mA");
    assert_eq!(
        t1.grid()[crate::index::Line(0)][Column(0)].fg,
        t2.grid()[crate::index::Line(0)][Column(0)].fg,
        "empty colorspace-id (::) and explicit zero (:0:) must produce identical color"
    );
    assert_eq!(
        t1.grid()[crate::index::Line(0)][Column(0)].fg,
        RGB_255_128_64
    );
}

// ── Section 08.8b: remaining catalog rows ───────────────────────────

// SGR 53/55 — overline

#[test]
fn sgr_53_sets_overline() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[53mA");
    assert!(
        t.grid()[crate::index::Line(0)][Column(0)]
            .flags
            .contains(CellFlags::OVERLINE)
    );
}

#[test]
fn sgr_55_resets_overline() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[53m\x1b[55mA");
    assert!(
        !t.grid()[crate::index::Line(0)][Column(0)]
            .flags
            .contains(CellFlags::OVERLINE)
    );
}

// SGR 73/74/75 — superscript/subscript

#[test]
fn sgr_73_sets_superscript() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[73mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert!(cell.flags.contains(CellFlags::SUPERSCRIPT));
    assert!(!cell.flags.contains(CellFlags::SUBSCRIPT));
}

#[test]
fn sgr_74_sets_subscript() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[74mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert!(cell.flags.contains(CellFlags::SUBSCRIPT));
    assert!(!cell.flags.contains(CellFlags::SUPERSCRIPT));
}

#[test]
fn sgr_73_clears_subscript() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[74m\x1b[73mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert!(cell.flags.contains(CellFlags::SUPERSCRIPT));
    assert!(!cell.flags.contains(CellFlags::SUBSCRIPT));
}

#[test]
fn sgr_75_resets_super_subscript() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[73m\x1b[75mA");
    let cell = &t.grid()[crate::index::Line(0)][Column(0)];
    assert!(!cell.flags.contains(CellFlags::SUPERSCRIPT));
    assert!(!cell.flags.contains(CellFlags::SUBSCRIPT));
}

// DECSTR — soft terminal reset (CSI ! p)

#[test]
fn decstr_resets_terminal_state() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[1m\x1b[31m");
    assert!(t.grid().cursor().template.flags.contains(CellFlags::BOLD));
    feed(&mut t, b"\x1b[!p");
    assert!(!t.grid().cursor().template.flags.contains(CellFlags::BOLD));
}

/// DECSTR must clear the DECSC saved cursor — `ESC 7 / CSI ! p / ESC 8`
/// must NOT resurrect the pre-reset cursor position.
#[test]
fn decstr_clears_saved_cursor() {
    let mut t = term();
    // Move cursor to (5, 10), save via DECSC (ESC 7).
    feed(&mut t, b"\x1b[6;11H\x1b7");
    // Roundtrip proof: move away, DECRC, assert restoration proves DECSC populated.
    feed(&mut t, b"\x1b[1;1H");
    feed(&mut t, b"\x1b8");
    assert_eq!(
        t.grid().cursor().line(),
        5,
        "DECSC must have populated saved cursor (roundtrip proof)"
    );
    assert_eq!(
        t.grid().cursor().col(),
        Column(10),
        "DECSC must have populated saved cursor (roundtrip proof)"
    );
    // Re-save the restored position so DECSTR has work to clear.
    feed(&mut t, b"\x1b7");
    // Move cursor away, then DECSTR (soft reset).
    feed(&mut t, b"\x1b[1;1H\x1b[!p");
    // DECRC (ESC 8) must NOT restore the saved position — saved_cursor is cleared.
    feed(&mut t, b"\x1b8");
    let grid = t.grid();
    assert_eq!(grid.cursor().line(), 0, "DECSTR must clear saved cursor");
    assert_eq!(
        grid.cursor().col(),
        Column(0),
        "DECSTR must clear saved cursor"
    );
}

/// DECSTR must clear the XTPUSHSGR stack — `CSI # { / CSI ! p / CSI # }`
/// must NOT resurrect the pre-reset SGR state.
#[test]
fn decstr_clears_sgr_stack() {
    use crate::cell::CellFlags;
    let mut t = term();
    // Set bold, push SGR stack.
    feed(&mut t, b"\x1b[1m\x1b[#{");
    assert!(t.grid().cursor().template.flags.contains(CellFlags::BOLD));
    // Roundtrip proof: clear bold, pop, assert bold restored — proves push populated.
    feed(&mut t, b"\x1b[0m");
    assert!(
        !t.grid().cursor().template.flags.contains(CellFlags::BOLD),
        "SGR reset should clear bold from template"
    );
    feed(&mut t, b"\x1b[#}");
    assert!(
        t.grid().cursor().template.flags.contains(CellFlags::BOLD),
        "XTPUSHSGR must have populated SGR stack (roundtrip proof)"
    );
    // Re-push the restored bold so DECSTR has work to clear.
    feed(&mut t, b"\x1b[#{");
    assert_eq!(
        t.grid().sgr_stack_len(),
        1,
        "XTPUSHSGR reseed must succeed before DECSTR"
    );
    // Soft reset.
    feed(&mut t, b"\x1b[!p");
    assert!(!t.grid().cursor().template.flags.contains(CellFlags::BOLD));
    // Pop SGR stack — must be a no-op (stack cleared by DECSTR).
    feed(&mut t, b"\x1b[#}");
    assert!(
        !t.grid().cursor().template.flags.contains(CellFlags::BOLD),
        "DECSTR must clear XTPUSHSGR stack"
    );
}

/// DECSTR while on alt screen must reset BOTH grids. After DECSTR drops
/// ALT_SCREEN, the primary grid becomes active and its prior state
/// (cursor, margins, scroll region, saved cursor, keyboard mode stack)
/// must all be cleared.
#[test]
fn decstr_clears_primary_state_when_fired_on_alt_screen() {
    let mut t = term();
    // On primary: set margins (DECLRMM), scroll region, keyboard mode,
    // move cursor, save via DECSC.
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[6;41s");
    feed(&mut t, b"\x1b[3;20r");
    feed(&mut t, b"\x1b[>1u");
    feed(&mut t, b"\x1b[10;20H\x1b7");
    let (pre_left, pre_right) = t.grid().left_right_margins();
    assert_eq!(pre_left, 5);
    assert_eq!(pre_right, 40);
    assert_eq!(t.keyboard_mode_stack().len(), 1);

    // Enter alt screen (swaps primary keyboard stack to inactive).
    feed(&mut t, b"\x1b[?1049h");
    assert!(t.keyboard_mode_stack().is_empty());
    // Seed alt-screen state (scroll region, margins, cursor, saved cursor,
    // SGR stack, keyboard mode) BEFORE DECSTR so a single DECSTR must
    // clear ALL alt-grid surfaces. DECSTR drops ALT_SCREEN; we re-enter
    // later (without DECSTR) to inspect the alt grid.
    feed(&mut t, b"\x1b[?69h");
    feed(&mut t, b"\x1b[2;30s"); // alt margins
    feed(&mut t, b"\x1b[5;15r"); // alt scroll region
    feed(&mut t, b"\x1b[8;15H\x1b7"); // alt cursor + DECSC (saves cursor)
    feed(&mut t, b"\x1b[1m\x1b[#{"); // alt SGR bold + XTPUSHSGR (pushes bold)
    feed(&mut t, b"\x1b[>3u");
    assert_eq!(t.keyboard_mode_stack().len(), 1);
    assert_eq!(t.grid().scroll_region(), &(4..15));
    assert_eq!(t.grid().left_right_margins(), (1, 29));
    // Verify the alt-side DECSC saved-cursor slot ACTUALLY populated by
    // moving away and DECRC-restoring back. Without this round-trip,
    // post-DECSTR DECRC == (0,0) would pass even if DECSC silently no-op'd.
    feed(&mut t, b"\x1b[1;1H"); // move cursor to (0,0)
    feed(&mut t, b"\x1b8"); // DECRC — should restore to (7, 14)
    assert_eq!(
        t.grid().cursor().line(),
        7,
        "alt DECSC must have populated saved cursor"
    );
    assert_eq!(
        t.grid().cursor().col(),
        Column(14),
        "alt DECSC must have populated saved cursor"
    );
    // Verify the alt-side XTPUSHSGR stack ACTUALLY populated by clearing
    // bold then XTPOPSGR — should restore bold. Without this round-trip,
    // post-DECSTR XTPOPSGR == not-bold would pass even if XTPUSHSGR no-op'd.
    feed(&mut t, b"\x1b[0m"); // clear SGR
    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD),
        "SGR reset should clear bold from template"
    );
    feed(&mut t, b"\x1b[#}"); // XTPOPSGR — should restore bold
    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD),
        "alt XTPUSHSGR must have populated SGR stack"
    );
    // Re-establish the seed state for DECSTR to clear: DECSC the current
    // (7, 14) cursor + push bold back onto stack.
    feed(&mut t, b"\x1b7");
    feed(&mut t, b"\x1b[#{");
    assert_eq!(
        t.grid().sgr_stack_len(),
        1,
        "XTPUSHSGR reseed must succeed before DECSTR"
    );

    // DECSTR while on alt screen.
    feed(&mut t, b"\x1b[!p");

    // DECSTR should have dropped ALT_SCREEN — primary grid is active.
    // Cursor at (0,0), margins cleared, both keyboard stacks empty.
    let grid = t.grid();
    assert_eq!(
        grid.cursor().line(),
        0,
        "DECSTR must reset cursor on primary"
    );
    assert_eq!(
        grid.cursor().col(),
        Column(0),
        "DECSTR must reset cursor on primary"
    );
    // DECRC must NOT resurrect the pre-DECSTR saved cursor.
    feed(&mut t, b"\x1b8");
    let grid = t.grid();
    assert_eq!(
        grid.cursor().line(),
        0,
        "DECSTR must clear primary saved cursor"
    );
    assert_eq!(
        grid.cursor().col(),
        Column(0),
        "DECSTR must clear primary saved cursor"
    );

    // Primary margins must be cleared too.
    let (post_left, post_right) = t.grid().left_right_margins();
    assert_eq!(post_left, 0, "DECSTR must clear primary left margin");
    assert_eq!(
        post_right,
        t.grid().cols() - 1,
        "DECSTR must clear primary right margin"
    );

    // Primary scroll region must be reset to full height.
    let lines = t.grid().lines();
    assert_eq!(
        t.grid().scroll_region(),
        &(0..lines),
        "DECSTR must clear primary scroll region"
    );

    // Both keyboard mode stacks must be empty.
    assert!(
        t.keyboard_mode_stack().is_empty(),
        "DECSTR must clear primary keyboard mode stack"
    );
    // Re-enter alt screen to inspect alt-grid state — DO NOT fire DECSTR
    // again (that would drop ALT_SCREEN and re-check primary). Verify
    // all alt-grid surfaces were cleared: cursor, margins, scroll region,
    // saved cursor (via DECRC no-op), SGR stack (via XTPOPSGR no-op),
    // keyboard mode stack.
    feed(&mut t, b"\x1b[?1049h");
    let alt_lines = t.grid().lines();
    let alt_cols = t.grid().cols();
    assert_eq!(t.grid().cursor().line(), 0, "DECSTR must clear alt cursor");
    assert_eq!(
        t.grid().cursor().col(),
        Column(0),
        "DECSTR must clear alt cursor"
    );
    assert_eq!(
        t.grid().left_right_margins(),
        (0, alt_cols - 1),
        "DECSTR must clear alt margins"
    );
    assert_eq!(
        t.grid().scroll_region(),
        &(0..alt_lines),
        "DECSTR must clear alt scroll region"
    );
    // DECRC on alt screen must NOT resurrect the pre-DECSTR saved cursor.
    feed(&mut t, b"\x1b8");
    assert_eq!(
        t.grid().cursor().line(),
        0,
        "DECSTR must clear alt saved cursor"
    );
    assert_eq!(
        t.grid().cursor().col(),
        Column(0),
        "DECSTR must clear alt saved cursor"
    );
    // XTPOPSGR on alt screen must NOT resurrect the pre-DECSTR bold.
    feed(&mut t, b"\x1b[#}");
    assert!(
        !t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD),
        "DECSTR must clear alt SGR stack"
    );
    assert!(
        t.keyboard_mode_stack().is_empty(),
        "DECSTR must clear alt keyboard mode stack"
    );
}

// DECSED — selective erase in display (CSI ? J)

#[test]
fn decsed_below_clears_from_cursor() {
    let mut t = term();
    feed(&mut t, b"ABCDE\x1b[1;3H\x1b[?J");
    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'B');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, ' ');
}

// DECSEL — selective erase in line (CSI ? K)

#[test]
fn decsel_right_clears_to_end_of_line() {
    let mut t = term();
    feed(&mut t, b"ABCDE\x1b[1;3H\x1b[?K");
    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, 'B');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(3)].ch, ' ');
}

// SL — scroll left (CSI Ps SP @)

#[test]
fn scroll_left_shifts_content() {
    let (mut t, _rec) = term_with_recorder_sized(3, 10);
    feed(&mut t, b"ABCDEFGHIJ");
    feed(&mut t, b"\x1b[2 @");
    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, 'C');
    assert_eq!(grid[crate::index::Line(0)][Column(7)].ch, 'J');
    assert_eq!(grid[crate::index::Line(0)][Column(8)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(9)].ch, ' ');
}

// SR — scroll right (CSI Ps SP A)

#[test]
fn scroll_right_shifts_content() {
    let (mut t, _rec) = term_with_recorder_sized(3, 10);
    feed(&mut t, b"ABCDEFGHIJ");
    feed(&mut t, b"\x1b[2 A");
    let grid = t.grid();
    assert_eq!(grid[crate::index::Line(0)][Column(0)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(1)].ch, ' ');
    assert_eq!(grid[crate::index::Line(0)][Column(2)].ch, 'A');
    assert_eq!(grid[crate::index::Line(0)][Column(9)].ch, 'H');
}

// DECRQSS for DECSLRM

#[test]
fn decrqss_decslrm_reports_margins() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[?69h");
    t.grid_mut().set_left_right_margins(5, 40);
    feed(&mut t, b"\x1bP$qs\x1b\\");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("6;41s")),
        "DECRQSS DECSLRM should report 1-based margins 6;41: {events:?}"
    );
}

// XTPUSHSGR / XTPOPSGR (CSI # { / CSI # })

#[test]
fn xtpushsgr_saves_and_restores_sgr() {
    use crate::cell::CellFlags;
    let mut t = term();
    feed(&mut t, b"\x1b[1m\x1b[31m");
    assert!(t.grid().cursor().template.flags.contains(CellFlags::BOLD));
    feed(&mut t, b"\x1b[#{");
    feed(&mut t, b"\x1b[0m");
    assert!(!t.grid().cursor().template.flags.contains(CellFlags::BOLD));
    feed(&mut t, b"\x1b[#}");
    assert!(t.grid().cursor().template.flags.contains(CellFlags::BOLD));
    assert_eq!(
        t.grid().cursor().template.fg,
        vte::ansi::Color::Named(vte::ansi::NamedColor::Red)
    );
}

#[test]
fn xtpopsgr_on_empty_stack_is_noop() {
    let mut t = term();
    feed(&mut t, b"\x1b[1m");
    feed(&mut t, b"\x1b[#}");
    assert!(
        t.grid()
            .cursor()
            .template
            .flags
            .contains(crate::cell::CellFlags::BOLD)
    );
}
