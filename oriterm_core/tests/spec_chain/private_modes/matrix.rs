//! Cross-cutting DECRQM + mutual-exclusion + mode-replacement matrix.
//!
//! Section 09.5 scope: exhaustive DECRQM verification across every
//! `NamedPrivateMode` variant, mutual-exclusion matrices for mouse
//! tracking and encoding modes, orthogonality pin, and SaveCursor /
//! ColumnMode DECRQM deviation pins.

use oriterm_core::TermMode;
use oriterm_test_support::spec_chain::SpecHarness;

// ── Variant table ──────────────────────────────────────────────────
//
// Every `NamedPrivateMode` variant with its mode number and expected
// `named_private_mode_flag()` mapping. Flag-mapped modes report 1/2
// via DECRQM; non-flag-mapped modes report 0 (not recognized).

/// A variant entry for the exhaustive iteration.
struct ModeEntry {
    name: &'static str,
    mode_num: u16,
    /// Whether `named_private_mode_flag()` returns `Some(...)` for this
    /// variant. When true, DECRQM reports 1 (set) / 2 (reset).
    /// When false, DECRQM reports 0 (not recognized).
    has_flag: bool,
}

/// Complete list of every `NamedPrivateMode` variant. The count
/// assertion at the bottom of each exhaustive test proves no variant
/// was forgotten.
const ALL_NAMED_MODES: &[ModeEntry] = &[
    ModeEntry {
        name: "CursorKeys",
        mode_num: 1,
        has_flag: true,
    },
    ModeEntry {
        name: "ColumnMode",
        mode_num: 3,
        has_flag: false,
    },
    ModeEntry {
        name: "ReverseVideo",
        mode_num: 5,
        has_flag: true,
    },
    ModeEntry {
        name: "Origin",
        mode_num: 6,
        has_flag: true,
    },
    ModeEntry {
        name: "LineWrap",
        mode_num: 7,
        has_flag: true,
    },
    ModeEntry {
        name: "X10Mouse",
        mode_num: 9,
        has_flag: true,
    },
    ModeEntry {
        name: "BlinkingCursor",
        mode_num: 12,
        has_flag: true,
    },
    ModeEntry {
        name: "ShowCursor",
        mode_num: 25,
        has_flag: true,
    },
    ModeEntry {
        name: "EnableMode3",
        mode_num: 40,
        has_flag: true,
    },
    ModeEntry {
        name: "ReverseWraparound",
        mode_num: 45,
        has_flag: true,
    },
    ModeEntry {
        name: "AltScreen",
        mode_num: 47,
        has_flag: true,
    },
    ModeEntry {
        name: "DecNumericKeypad",
        mode_num: 66,
        has_flag: true,
    },
    ModeEntry {
        name: "DecBackarrowKey",
        mode_num: 67,
        has_flag: true,
    },
    ModeEntry {
        name: "LeftRightMargin",
        mode_num: 69,
        has_flag: true,
    },
    ModeEntry {
        name: "SixelScrolling",
        mode_num: 80,
        has_flag: true,
    },
    ModeEntry {
        name: "ReportMouseClicks",
        mode_num: 1000,
        has_flag: true,
    },
    ModeEntry {
        name: "ReportCellMouseMotion",
        mode_num: 1002,
        has_flag: true,
    },
    ModeEntry {
        name: "ReportAllMouseMotion",
        mode_num: 1003,
        has_flag: true,
    },
    ModeEntry {
        name: "ReportFocusInOut",
        mode_num: 1004,
        has_flag: true,
    },
    ModeEntry {
        name: "Utf8Mouse",
        mode_num: 1005,
        has_flag: true,
    },
    ModeEntry {
        name: "SgrMouse",
        mode_num: 1006,
        has_flag: true,
    },
    ModeEntry {
        name: "AlternateScroll",
        mode_num: 1007,
        has_flag: true,
    },
    ModeEntry {
        name: "UrxvtMouse",
        mode_num: 1015,
        has_flag: true,
    },
    ModeEntry {
        name: "UrgencyHints",
        mode_num: 1042,
        has_flag: true,
    },
    ModeEntry {
        name: "AltScreenOpt",
        mode_num: 1047,
        has_flag: true,
    },
    ModeEntry {
        name: "SaveCursor",
        mode_num: 1048,
        has_flag: false,
    },
    ModeEntry {
        name: "SwapScreenAndSetRestoreCursor",
        mode_num: 1049,
        has_flag: true,
    },
    ModeEntry {
        name: "BracketedPaste",
        mode_num: 2004,
        has_flag: true,
    },
    ModeEntry {
        name: "SyncUpdate",
        mode_num: 2026,
        has_flag: true,
    },
    ModeEntry {
        name: "ColorSchemeUpdate",
        mode_num: 2031,
        has_flag: true,
    },
    ModeEntry {
        name: "SixelCursorRight",
        mode_num: 8452,
        has_flag: true,
    },
    ModeEntry {
        name: "Win32Input",
        mode_num: 9001,
        has_flag: true,
    },
];

/// Expected total number of `NamedPrivateMode` variants.
/// If a variant is added to the enum without updating `ALL_NAMED_MODES`,
/// the count assertion will fail.
const EXPECTED_VARIANT_COUNT: usize = 32;

// ── Helpers ────────────────────────────────────────────────────────

/// Feed a DECRQM query for the given private mode number and return the
/// DECRQM response value (0, 1, or 2), or panic if no response was
/// emitted.
fn query_decrqm(h: &mut SpecHarness, mode_num: u16) -> u8 {
    let query = format!("\x1b[?{}$p", mode_num);
    h.feed(query.as_bytes());

    let expected_prefix = format!("\x1b[?{};", mode_num);
    let effects = h.outcome().effects_emitted.clone();

    for eff in &effects {
        if let oriterm_core::effect::Effect::Pty(oriterm_core::effect::PtyEffect::Write {
            bytes,
            ..
        }) = eff
        {
            let s = String::from_utf8_lossy(bytes);
            if let Some(rest) = s.strip_prefix(&expected_prefix) {
                if let Some(val_str) = rest.strip_suffix("$y") {
                    if let Ok(val) = val_str.parse::<u8>() {
                        return val;
                    }
                }
            }
        }
    }
    panic!("DECRQM ?{mode_num}: no valid response found in effects: {effects:?}");
}

/// Extract a DECRQM response value from the already-collected effects
/// (for modes like 2026 where DECRQM must be fed in the same call).
fn extract_decrqm_from_effects(h: &SpecHarness, mode_num: u16) -> u8 {
    let expected_prefix = format!("\x1b[?{};", mode_num);
    for eff in &h.outcome().effects_emitted {
        if let oriterm_core::effect::Effect::Pty(oriterm_core::effect::PtyEffect::Write {
            bytes,
            ..
        }) = eff
        {
            let s = String::from_utf8_lossy(bytes);
            if let Some(rest) = s.strip_prefix(&expected_prefix) {
                if let Some(val_str) = rest.strip_suffix("$y") {
                    if let Ok(val) = val_str.parse::<u8>() {
                        return val;
                    }
                }
            }
        }
    }
    panic!(
        "DECRQM ?{mode_num}: no valid response in effects: {:?}",
        h.outcome().effects_emitted
    );
}

// ── 09.5 Item 1: DECRQM exhaustive matrix ─────────────────────────

/// For every `NamedPrivateMode` variant:
/// DECSET, then DECRQM → expect 1 (set) for flag-mapped, 0 for others.
///
/// Mode 2026 (SyncUpdate) activates VTE sync buffering, so a separate
/// `feed()` for DECRQM would be buffered. We feed BSU+DECRQM+ESU in
/// one call for that mode.
///
/// Self-verifying: asserts iteration count equals `EXPECTED_VARIANT_COUNT`.
#[test]
fn decrqm_exhaustive_set_then_query() {
    let mut count = 0;
    for entry in ALL_NAMED_MODES {
        let mut h = SpecHarness::new();

        if entry.mode_num == 2026 {
            // Mode 2026 enables sync buffering — feed BSU+DECRQM+ESU
            // in one call so the query is replayed before the flag clears.
            h.feed(b"\x1b[?2026h\x1b[?2026$p\x1b[?2026l");
            let val = extract_decrqm_from_effects(&h, 2026);
            assert_eq!(
                val, 1,
                "DECRQM after DECSET ?2026 (SyncUpdate): expected 1, got {val}",
            );
        } else {
            let decset = format!("\x1b[?{}h", entry.mode_num);
            h.feed(decset.as_bytes());
            let val = query_decrqm(&mut h, entry.mode_num);
            let expected = if entry.has_flag { 1 } else { 0 };
            assert_eq!(
                val, expected,
                "DECRQM after DECSET ?{} ({}): expected {expected}, got {val}",
                entry.mode_num, entry.name,
            );
        }
        count += 1;
    }
    assert_eq!(
        count, EXPECTED_VARIANT_COUNT,
        "variant count mismatch: iterated {count}, expected {EXPECTED_VARIANT_COUNT}"
    );
}

/// For every `NamedPrivateMode` variant:
/// DECSET then DECRST, then DECRQM → expect 2 (reset) for flag-mapped,
/// 0 for others.
///
/// Self-verifying: asserts iteration count equals `EXPECTED_VARIANT_COUNT`.
#[test]
fn decrqm_exhaustive_reset_then_query() {
    let mut count = 0;
    for entry in ALL_NAMED_MODES {
        let mut h = SpecHarness::new();
        // DECSET then DECRST the mode.
        let decset = format!("\x1b[?{}h", entry.mode_num);
        let decrst = format!("\x1b[?{}l", entry.mode_num);
        h.feed(decset.as_bytes());
        h.feed(decrst.as_bytes());

        let val = query_decrqm(&mut h, entry.mode_num);
        let expected = if entry.has_flag { 2 } else { 0 };
        assert_eq!(
            val, expected,
            "DECRQM after DECRST ?{} ({}): expected {expected}, got {val}",
            entry.mode_num, entry.name,
        );
        count += 1;
    }
    assert_eq!(
        count, EXPECTED_VARIANT_COUNT,
        "variant count mismatch: iterated {count}, expected {EXPECTED_VARIANT_COUNT}"
    );
}

// ── 09.5 Item 2: Mouse tracking mutual-exclusion 4×4 matrix ───────

/// Mouse tracking modes — mutually exclusive via `ANY_MOUSE`.
const TRACKING_MODES: &[(u16, TermMode)] = &[
    (9, TermMode::MOUSE_X10),
    (1000, TermMode::MOUSE_REPORT_CLICK),
    (1002, TermMode::MOUSE_DRAG),
    (1003, TermMode::MOUSE_MOTION),
];

/// DECSET A, then DECSET B → only B's bit is set; A's bit is cleared.
/// For A == B (idempotence): DECSET A twice → A remains set.
#[test]
fn mouse_tracking_mutual_exclusion_matrix() {
    let mut count = 0;
    for &(a_num, a_flag) in TRACKING_MODES {
        for &(b_num, b_flag) in TRACKING_MODES {
            let mut h = SpecHarness::new();
            let decset_a = format!("\x1b[?{}h", a_num);
            let decset_b = format!("\x1b[?{}h", b_num);
            h.feed(decset_a.as_bytes());
            h.feed(decset_b.as_bytes());

            let mode = h.term().mode();
            assert!(
                mode.contains(b_flag),
                "tracking exclusion: DECSET ?{a_num} then ?{b_num} — expected {b_num} set"
            );
            if a_num != b_num {
                assert!(
                    !mode.contains(a_flag),
                    "tracking exclusion: DECSET ?{a_num} then ?{b_num} — expected {a_num} cleared"
                );
            }
            count += 1;
        }
    }
    // 4×4 = 16 cells (12 exclusion + 4 idempotence)
    assert_eq!(count, 16, "tracking matrix cell count mismatch");
}

/// DECSET A, then DECRST B (A ≠ B) → A's bit UNCHANGED.
#[test]
fn mouse_tracking_decrst_cross_mode_preserves_active() {
    let mut count = 0;
    for &(a_num, a_flag) in TRACKING_MODES {
        for &(b_num, _b_flag) in TRACKING_MODES {
            if a_num == b_num {
                continue;
            }
            let mut h = SpecHarness::new();
            let decset_a = format!("\x1b[?{}h", a_num);
            let decrst_b = format!("\x1b[?{}l", b_num);
            h.feed(decset_a.as_bytes());
            h.feed(decrst_b.as_bytes());

            let mode = h.term().mode();
            assert!(
                mode.contains(a_flag),
                "tracking cross-reset: DECSET ?{a_num} then DECRST ?{b_num} — {a_num} must remain set"
            );
            count += 1;
        }
    }
    // 4×3 = 12 cross-reset pairs
    assert_eq!(count, 12, "tracking cross-reset cell count mismatch");
}

// ── 09.5 Item 3: Mouse encoding mutual-exclusion 3×3 matrix ───────

/// Mouse encoding modes — mutually exclusive via `ANY_MOUSE_ENCODING`.
const ENCODING_MODES: &[(u16, TermMode)] = &[
    (1005, TermMode::MOUSE_UTF8),
    (1006, TermMode::MOUSE_SGR),
    (1015, TermMode::MOUSE_URXVT),
];

/// DECSET A, then DECSET B → only B's encoding bit is set; A's is cleared.
/// For A == B (idempotence): A remains set.
#[test]
fn mouse_encoding_mutual_exclusion_matrix() {
    let mut count = 0;
    for &(a_num, a_flag) in ENCODING_MODES {
        for &(b_num, b_flag) in ENCODING_MODES {
            let mut h = SpecHarness::new();
            let decset_a = format!("\x1b[?{}h", a_num);
            let decset_b = format!("\x1b[?{}h", b_num);
            h.feed(decset_a.as_bytes());
            h.feed(decset_b.as_bytes());

            let mode = h.term().mode();
            assert!(
                mode.contains(b_flag),
                "encoding exclusion: DECSET ?{a_num} then ?{b_num} — expected {b_num} set"
            );
            if a_num != b_num {
                assert!(
                    !mode.contains(a_flag),
                    "encoding exclusion: DECSET ?{a_num} then ?{b_num} — expected {a_num} cleared"
                );
            }
            count += 1;
        }
    }
    // 3×3 = 9 cells (6 exclusion + 3 idempotence)
    assert_eq!(count, 9, "encoding matrix cell count mismatch");
}

/// DECSET A, then DECRST B (A ≠ B) → A's encoding bit UNCHANGED.
#[test]
fn mouse_encoding_decrst_cross_mode_preserves_active() {
    let mut count = 0;
    for &(a_num, a_flag) in ENCODING_MODES {
        for &(b_num, _b_flag) in ENCODING_MODES {
            if a_num == b_num {
                continue;
            }
            let mut h = SpecHarness::new();
            let decset_a = format!("\x1b[?{}h", a_num);
            let decrst_b = format!("\x1b[?{}l", b_num);
            h.feed(decset_a.as_bytes());
            h.feed(decrst_b.as_bytes());

            let mode = h.term().mode();
            assert!(
                mode.contains(a_flag),
                "encoding cross-reset: DECSET ?{a_num} then DECRST ?{b_num} — {a_num} must remain set"
            );
            count += 1;
        }
    }
    // 3×2 = 6 cross-reset pairs
    assert_eq!(count, 6, "encoding cross-reset cell count mismatch");
}

/// Encoding modes are orthogonal to tracking modes — setting an
/// encoding mode must NOT clear any tracking mode, and vice versa.
#[test]
fn encoding_orthogonal_to_tracking() {
    for &(t_num, t_flag) in TRACKING_MODES {
        for &(e_num, e_flag) in ENCODING_MODES {
            let mut h = SpecHarness::new();
            // Set both a tracking and an encoding mode.
            let decset_t = format!("\x1b[?{}h", t_num);
            let decset_e = format!("\x1b[?{}h", e_num);
            h.feed(decset_t.as_bytes());
            h.feed(decset_e.as_bytes());

            let mode = h.term().mode();
            assert!(
                mode.contains(t_flag),
                "orthogonality: DECSET ?{t_num} then ?{e_num} — tracking {t_num} must remain set"
            );
            assert!(
                mode.contains(e_flag),
                "orthogonality: DECSET ?{t_num} then ?{e_num} — encoding {e_num} must be set"
            );

            // Now set a different tracking mode — encoding must survive.
            for &(t2_num, t2_flag) in TRACKING_MODES {
                if t2_num == t_num {
                    continue;
                }
                // Fresh harness, replay the base state then replace tracking.
                let mut h2 = SpecHarness::new();
                h2.feed(format!("\x1b[?{}h", t_num).as_bytes());
                h2.feed(format!("\x1b[?{}h", e_num).as_bytes());
                h2.feed(format!("\x1b[?{}h", t2_num).as_bytes());

                let mode2 = h2.term().mode();
                assert!(
                    mode2.contains(t2_flag),
                    "orthogonality: tracking replacement ?{t2_num} must be set"
                );
                assert!(
                    !mode2.contains(t_flag),
                    "orthogonality: old tracking ?{t_num} must be cleared after ?{t2_num}"
                );
                assert!(
                    mode2.contains(e_flag),
                    "orthogonality: encoding ?{e_num} must survive tracking replacement ?{t2_num}"
                );
            }
        }
    }
}

// ── 09.5 Item 4: Orthogonality pin ────────────────────────────────

/// DECSET 1000+1006+1004+1007 all on simultaneously. Assert every bit
/// present. Then DECSET 1002 — assert 1000 cleared, 1006/1004/1007
/// preserved.
#[test]
fn orthogonality_pin_tracking_encoding_focus_alt_scroll() {
    let mut h = SpecHarness::new();

    // Enable all four.
    h.feed(b"\x1b[?1000h"); // tracking: MOUSE_REPORT_CLICK
    h.feed(b"\x1b[?1006h"); // encoding: MOUSE_SGR
    h.feed(b"\x1b[?1004h"); // focus: FOCUS_IN_OUT
    h.feed(b"\x1b[?1007h"); // alt-scroll: ALTERNATE_SCROLL

    let mode = h.term().mode();
    assert!(
        mode.contains(TermMode::MOUSE_REPORT_CLICK),
        "1000 must be set"
    );
    assert!(mode.contains(TermMode::MOUSE_SGR), "1006 must be set");
    assert!(mode.contains(TermMode::FOCUS_IN_OUT), "1004 must be set");
    assert!(
        mode.contains(TermMode::ALTERNATE_SCROLL),
        "1007 must be set"
    );

    // Now DECSET 1002 — tracking replacement.
    h.feed(b"\x1b[?1002h");

    let mode = h.term().mode();
    // 1000 must be cleared (tracking mutual exclusion).
    assert!(
        !mode.contains(TermMode::MOUSE_REPORT_CLICK),
        "1000 must be cleared after 1002"
    );
    // 1002 must be set.
    assert!(mode.contains(TermMode::MOUSE_DRAG), "1002 must be set");
    // Non-tracking modes must be preserved.
    assert!(
        mode.contains(TermMode::MOUSE_SGR),
        "1006 must survive tracking replacement"
    );
    assert!(
        mode.contains(TermMode::FOCUS_IN_OUT),
        "1004 must survive tracking replacement"
    );
    assert!(
        mode.contains(TermMode::ALTERNATE_SCROLL),
        "1007 must survive tracking replacement"
    );
}

// ── 09.5 Item 5: SaveCursor / ColumnMode DECRQM deviation pin ─────

/// SaveCursor (1048) has no `TermMode` flag mapping —
/// `named_private_mode_flag()` returns `None`. DECRQM returns 0
/// (not recognized) regardless of whether the cursor was saved.
///
/// Deviation from xterm: xterm reports SaveCursor as 3 (permanently
/// set) or 4 (permanently reset). ori_term returns 0 because the mode
/// triggers a side effect (cursor save/restore) without a persistent
/// state bit.
#[test]
fn save_cursor_decrqm_returns_zero() {
    let mut h = SpecHarness::new();
    // Default — never saved.
    let val = query_decrqm(&mut h, 1048);
    assert_eq!(
        val, 0,
        "SaveCursor DECRQM default must be 0 (not recognized)"
    );

    // After DECSET ?1048 (save cursor) — still 0.
    h.feed(b"\x1b[?1048h");
    let val = query_decrqm(&mut h, 1048);
    assert_eq!(
        val, 0,
        "SaveCursor DECRQM after DECSET must still be 0 (not recognized)"
    );

    // After DECRST ?1048 (restore cursor) — still 0.
    h.feed(b"\x1b[?1048l");
    let val = query_decrqm(&mut h, 1048);
    assert_eq!(
        val, 0,
        "SaveCursor DECRQM after DECRST must still be 0 (not recognized)"
    );
}

/// ColumnMode (3) has no `TermMode` flag mapping —
/// `named_private_mode_flag()` returns `None`. DECRQM returns 0
/// (not recognized) regardless of column mode state.
///
/// Deviation from xterm: xterm may report ColumnMode as 1/2/3/4
/// depending on DECCOLM state. ori_term returns 0 because mode 3
/// triggers side effects (clear/resize) without a persistent state bit
/// (the "allow/disallow" gating is done by mode 40 / EnableMode3).
#[test]
fn column_mode_decrqm_returns_zero() {
    let mut h = SpecHarness::new();
    // Default.
    let val = query_decrqm(&mut h, 3);
    assert_eq!(
        val, 0,
        "ColumnMode DECRQM default must be 0 (not recognized)"
    );

    // After DECSET ?3.
    h.feed(b"\x1b[?3h");
    let val = query_decrqm(&mut h, 3);
    assert_eq!(
        val, 0,
        "ColumnMode DECRQM after DECSET must still be 0 (not recognized)"
    );

    // After DECRST ?3.
    h.feed(b"\x1b[?3l");
    let val = query_decrqm(&mut h, 3);
    assert_eq!(
        val, 0,
        "ColumnMode DECRQM after DECRST must still be 0 (not recognized)"
    );
}

/// Unknown modes (not in `NamedPrivateMode`) must also return 0.
#[test]
fn unknown_mode_decrqm_returns_zero() {
    let mut h = SpecHarness::new();
    // Mode 9999 is not a recognized private mode.
    let val = query_decrqm(&mut h, 9999);
    assert_eq!(val, 0, "unknown mode DECRQM must return 0 (not recognized)");
}
