//! Tests for the mouse-event encoder.
//!
//! Catalog rows: MOUSE-X10, MOUSE-VT200, MOUSE-BTN-EVENT, MOUSE-ANY-EVENT,
//! MOUSE-SGR, MOUSE-URXVT, MOUSE-UTF8, MOUSE-SGR-PIXEL, MOUSE-FOCUS. All
//! routed through Decision 10 Option A apex (`Term::handle_mouse_input` or
//! `Term::handle_focus_event` → `Effect::Pty(PtyEffect::Write { kind, .. })`
//! → effect_router → MuxEvent::PtyWrite → pump → pane.write_input).

use crate::TermMode;

use super::{
    MouseButton, MouseEvent, MouseEventKind, MouseModifiers, apply_modifiers, button_code,
    encode_mouse_event, encode_normal, encode_sgr, encode_sgr_pixel, encode_utf8,
};

fn event(button: MouseButton, kind: MouseEventKind, col: usize, line: usize) -> MouseEvent {
    MouseEvent {
        button,
        kind,
        col,
        line,
        mods: MouseModifiers::default(),
        px: None,
        py: None,
    }
}

#[test]
fn button_code_left_press_is_zero() {
    assert_eq!(button_code(MouseButton::Left, MouseEventKind::Press), 0);
}

#[test]
fn button_code_middle_press_is_one() {
    assert_eq!(button_code(MouseButton::Middle, MouseEventKind::Press), 1);
}

#[test]
fn button_code_right_press_is_two() {
    assert_eq!(button_code(MouseButton::Right, MouseEventKind::Press), 2);
}

#[test]
fn button_code_scroll_up_is_64() {
    assert_eq!(
        button_code(MouseButton::ScrollUp, MouseEventKind::Press),
        64
    );
}

#[test]
fn button_code_motion_adds_32() {
    assert_eq!(button_code(MouseButton::Left, MouseEventKind::Motion), 32);
    assert_eq!(button_code(MouseButton::Middle, MouseEventKind::Motion), 33);
}

#[test]
fn apply_modifiers_shift_alt_ctrl() {
    let mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    assert_eq!(apply_modifiers(0, mods), 28);
}

#[test]
fn encode_sgr_press_emits_uppercase_m_suffix() {
    let mut buf = [0u8; 32];
    let n = encode_sgr(&mut buf, 0, 10, 20, true);
    assert_eq!(&buf[..n], b"\x1b[<0;11;21M");
}

#[test]
fn encode_sgr_release_emits_lowercase_m_suffix() {
    let mut buf = [0u8; 32];
    let n = encode_sgr(&mut buf, 0, 10, 20, false);
    assert_eq!(&buf[..n], b"\x1b[<0;11;21m");
}

#[test]
fn encode_normal_basic_press() {
    let mut buf = [0u8; 32];
    let n = encode_normal(&mut buf, 0, 10, 20);
    assert_eq!(&buf[..n], &[0x1b, b'[', b'M', 32, 32 + 1 + 10, 32 + 1 + 20]);
}

#[test]
fn encode_normal_overflow_returns_zero() {
    let mut buf = [0u8; 32];
    assert_eq!(encode_normal(&mut buf, 0, 223, 0), 0);
    assert_eq!(encode_normal(&mut buf, 0, 0, 223), 0);
}

#[test]
fn encode_utf8_basic_press() {
    let mut buf = [0u8; 32];
    let n = encode_utf8(&mut buf, 0, 10, 20);
    assert_eq!(&buf[..n], &[0x1b, b'[', b'M', 32, 32 + 1 + 10, 32 + 1 + 20]);
}

#[test]
fn encode_mouse_event_sgr_mode_routes_through_sgr() {
    let ev = event(MouseButton::Left, MouseEventKind::Press, 10, 20);
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR;
    let report = encode_mouse_event(&ev, mode);
    assert_eq!(report.as_bytes(), b"\x1b[<0;11;21M");
}

#[test]
fn encode_mouse_event_x10_mode_drops_release() {
    let ev = event(MouseButton::Left, MouseEventKind::Release, 5, 5);
    let mode = TermMode::MOUSE_X10;
    let report = encode_mouse_event(&ev, mode);
    assert!(report.as_bytes().is_empty());
}

#[test]
fn encode_mouse_event_x10_mode_strips_modifiers() {
    let mut ev = event(MouseButton::Left, MouseEventKind::Press, 5, 5);
    ev.mods = MouseModifiers {
        shift: true,
        alt: true,
        ctrl: true,
    };
    let mode = TermMode::MOUSE_X10;
    let report = encode_mouse_event(&ev, mode);
    let expected = &[0x1b, b'[', b'M', 32, 32 + 1 + 5, 32 + 1 + 5];
    assert_eq!(report.as_bytes(), expected);
}

#[test]
fn encode_mouse_event_normal_release_uses_code_3() {
    let ev = event(MouseButton::Left, MouseEventKind::Release, 5, 5);
    let mode = TermMode::MOUSE_REPORT_CLICK;
    let report = encode_mouse_event(&ev, mode);
    let expected = &[0x1b, b'[', b'M', 32 + 3, 32 + 1 + 5, 32 + 1 + 5];
    assert_eq!(report.as_bytes(), expected);
}

// --- SGR-Pixel (mode 1016) encoder tests (§16.2.C) ---

#[test]
fn encode_sgr_pixel_press_emits_uppercase_m_suffix() {
    let mut buf = [0u8; 32];
    let n = encode_sgr_pixel(&mut buf, 0, 320, 480, true);
    // px+1, py+1 → 321, 481 per xterm 1-indexing.
    assert_eq!(&buf[..n], b"\x1b[<0;321;481M");
}

#[test]
fn encode_sgr_pixel_release_emits_lowercase_m_suffix() {
    let mut buf = [0u8; 32];
    let n = encode_sgr_pixel(&mut buf, 0, 320, 480, false);
    assert_eq!(&buf[..n], b"\x1b[<0;321;481m");
}

#[test]
fn encode_sgr_pixel_zero_coords_emit_1_1() {
    let mut buf = [0u8; 32];
    let n = encode_sgr_pixel(&mut buf, 0, 0, 0, true);
    assert_eq!(&buf[..n], b"\x1b[<0;1;1M");
}

#[test]
fn encode_mouse_event_sgr_pixel_mode_routes_through_sgr_pixel_when_coords_present() {
    let ev = MouseEvent {
        button: MouseButton::Left,
        kind: MouseEventKind::Press,
        col: 5,
        line: 10,
        mods: MouseModifiers::default(),
        px: Some(80),
        py: Some(160),
    };
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR | TermMode::MOUSE_SGR_PIXEL;
    let report = encode_mouse_event(&ev, mode);
    // SGR-Pixel takes precedence over SGR per xterm: pixel coords, not cell.
    assert_eq!(report.as_bytes(), b"\x1b[<0;81;161M");
}

// The historical `encode_mouse_event_sgr_pixel_mode_falls_back_to_sgr_when_coords_missing`
// test was removed. It pinned an SGR cell-coord fallback under MOUSE_SGR_PIXEL
// that silently corrupts 1016-aware clients (notcurses pixelmouse_click at
// in.c:556-586 divides the parsed SGR payload by cell_pixel dimensions when
// 1016 is active; cell coords substituted into the SGR envelope would land
// at cell `(col/cellpxx, line/cellpxy)` instead of `(col, line)`). The
// drop-on-None semantic in `encode_mouse_event` + App-side clamping in
// `mouse_pixel_coords` replaces the fallback. Coverage now lives in the
// sgr_pixel_drop module below.

#[test]
fn encode_mouse_event_sgr_pixel_alone_works_without_sgr() {
    // MOUSE_SGR_PIXEL active without MOUSE_SGR: still routes via
    // SGR-Pixel path (xterm allows mode 1016 standalone).
    let ev = MouseEvent {
        button: MouseButton::Left,
        kind: MouseEventKind::Press,
        col: 5,
        line: 10,
        mods: MouseModifiers::default(),
        px: Some(80),
        py: Some(160),
    };
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR_PIXEL;
    let report = encode_mouse_event(&ev, mode);
    assert_eq!(report.as_bytes(), b"\x1b[<0;81;161M");
}

#[test]
fn encode_mouse_event_sgr_takes_precedence_over_urxvt() {
    let ev = event(MouseButton::Left, MouseEventKind::Press, 10, 20);
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR | TermMode::MOUSE_URXVT;
    let report = encode_mouse_event(&ev, mode);
    assert!(report.as_bytes().starts_with(b"\x1b[<"));
}

// --- §16.6 modifier matrix (protocols × modifier-sets) ---

/// Parameterized 64-pilot matrix: 8 protocols × 8 modifier-sets.
/// Asserts the encoded button Cb byte matches base_code + mouse-Cb
/// additive bits (Shift=+4, Alt=+8, Ctrl=+16) for every cell. X10
/// (mode 9) is the negative-pin row: modifiers MUST be stripped.
///
/// Self-verifying matrix completeness pin: count assertion asserts
/// 8 × 8 = 64 cells visited so a future regression that silently
/// drops a row/column fires immediately.
#[test]
fn modifier_matrix_64_cells_protocols_x_modifier_sets() {
    let modifier_sets: [(MouseModifiers, u8); 8] = [
        (
            MouseModifiers {
                shift: false,
                alt: false,
                ctrl: false,
            },
            0,
        ),
        (
            MouseModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            },
            4,
        ),
        (
            MouseModifiers {
                shift: false,
                alt: true,
                ctrl: false,
            },
            8,
        ),
        (
            MouseModifiers {
                shift: false,
                alt: false,
                ctrl: true,
            },
            16,
        ),
        (
            MouseModifiers {
                shift: true,
                alt: true,
                ctrl: false,
            },
            12,
        ),
        (
            MouseModifiers {
                shift: true,
                alt: false,
                ctrl: true,
            },
            20,
        ),
        (
            MouseModifiers {
                shift: false,
                alt: true,
                ctrl: true,
            },
            24,
        ),
        (
            MouseModifiers {
                shift: true,
                alt: true,
                ctrl: true,
            },
            28,
        ),
    ];

    let protocols: [(TermMode, &'static str); 8] = [
        (TermMode::MOUSE_X10, "X10"),
        (TermMode::MOUSE_REPORT_CLICK, "Normal/1000"),
        (TermMode::MOUSE_DRAG, "BtnEvent/1002"),
        (TermMode::MOUSE_MOTION, "AnyEvent/1003"),
        (
            TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_UTF8,
            "UTF-8/1005",
        ),
        (
            TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR,
            "SGR/1006",
        ),
        (
            TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_URXVT,
            "URXVT/1015",
        ),
        (
            TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR | TermMode::MOUSE_SGR_PIXEL,
            "SGR-Pixel/1016",
        ),
    ];

    let mut cells_visited = 0;
    for (proto_mode, proto_name) in protocols {
        for (mods, expected_modifier_bits) in modifier_sets {
            let mut ev = event(MouseButton::Left, MouseEventKind::Press, 5, 5);
            ev.mods = mods;
            // SGR-Pixel needs px/py populated to route through SGR-Pixel branch.
            if proto_mode.contains(TermMode::MOUSE_SGR_PIXEL) {
                ev.px = Some(80);
                ev.py = Some(160);
            }
            let report = encode_mouse_event(&ev, proto_mode);
            let bytes = report.as_bytes();

            // X10 strips modifiers AND is press-only (release would be 0 bytes).
            // For Press events X10 emits Normal-format byte stream but with
            // modifier bits stripped to 0.
            let effective_mod_bits = if proto_mode.contains(TermMode::MOUSE_X10) {
                0
            } else {
                expected_modifier_bits
            };

            assert!(
                !bytes.is_empty(),
                "{proto_name} mods={mods:?}: empty buffer (encoder failed)"
            );

            // Extract the Cb byte per protocol's wire shape:
            // - SGR/SGR-Pixel: parse the decimal after CSI <
            // - URXVT: parse decimal after CSI
            // - UTF-8 / Normal / X10: byte 4 (after \x1b[M is byte 3, btn is byte 4 = 32+Cb)
            let cb = extract_cb(bytes, proto_mode);
            let expected_cb = effective_mod_bits; // base_code = 0 for Left button + Press
            assert_eq!(
                cb, expected_cb,
                "{proto_name} mods={mods:?}: Cb={cb}, expected {expected_cb}"
            );

            cells_visited += 1;
        }
    }
    assert_eq!(
        cells_visited, 64,
        "matrix completeness — all 64 cells visited"
    );
}

/// Extract the Cb byte (button code with modifier bits) from an
/// encoded mouse-event byte stream per the protocol's wire shape.
fn extract_cb(bytes: &[u8], mode: TermMode) -> u8 {
    let s = std::str::from_utf8(bytes).expect("ASCII bytes");
    if mode.contains(TermMode::MOUSE_SGR_PIXEL) || mode.contains(TermMode::MOUSE_SGR) {
        // \x1b[<{code};...
        let rest = s.strip_prefix("\x1b[<").expect("SGR prefix");
        let semi = rest.find(';').expect("SGR semicolon");
        rest[..semi].parse().expect("SGR code")
    } else if mode.contains(TermMode::MOUSE_URXVT) {
        // \x1b[{cb};... where cb = 32 + code
        let rest = s.strip_prefix("\x1b[").expect("URXVT prefix");
        let semi = rest.find(';').expect("URXVT semicolon");
        let cb_plus_32: u8 = rest[..semi].parse().expect("URXVT cb");
        cb_plus_32 - 32
    } else {
        // UTF-8 / Normal / X10: \x1b[M{btn}{col}{line} where btn = 32 + code
        bytes[3] - 32
    }
}

// --- Term::handle_mouse_input apex tests (Decision 10 Option A §16.2.0.B) ---

mod term_apex {
    use std::sync::{Arc, Mutex};

    use vte::ansi::Processor;

    use crate::TermMode;
    use crate::effect::sink::EffectSink;
    use crate::effect::{Effect, PtyEffect, PtyWriteKind};
    use crate::term::Term;
    use crate::theme::Theme;

    use super::*;

    /// Records every Effect::Pty payload with its kind for apex
    /// verification of Term::handle_mouse_input.
    #[derive(Clone, Default)]
    struct PtyRecorder {
        emissions: Arc<Mutex<Vec<(PtyWriteKind, Vec<u8>)>>>,
    }

    impl PtyRecorder {
        fn new() -> Self {
            Self::default()
        }

        fn emissions(&self) -> Vec<(PtyWriteKind, Vec<u8>)> {
            self.emissions.lock().expect("lock poisoned").clone()
        }
    }

    impl EffectSink for PtyRecorder {
        fn push(&self, effect: Effect) {
            if let Effect::Pty(PtyEffect::Write { bytes, kind }) = effect {
                self.emissions
                    .lock()
                    .expect("lock poisoned")
                    .push((kind, bytes));
            }
        }

        fn drain_into(&self, _out: &mut Vec<Effect>) {}
    }

    fn term_with_recorder() -> (Term<PtyRecorder>, PtyRecorder) {
        let recorder = PtyRecorder::new();
        let term = Term::new(24, 80, 100, Theme::default(), recorder.clone());
        (term, recorder)
    }

    /// Drive mode flags onto a fresh `Term` via VTE-fed DECSET bytes.
    /// Production `Term::mode` mutation routes through the VTE handler
    /// — tests exercise the same path so no private setter shortcut is
    /// needed.
    fn apply_decset(term: &mut Term<PtyRecorder>, bytes: &[u8]) {
        let mut proc: Processor = Processor::new();
        proc.advance(term, bytes);
    }

    fn ev(button: MouseButton, kind: MouseEventKind, col: usize, line: usize) -> MouseEvent {
        MouseEvent {
            button,
            kind,
            col,
            line,
            mods: MouseModifiers::default(),
            px: None,
            py: None,
        }
    }

    /// Pin: Term::handle_mouse_input emits Effect::Pty(Write { kind:
    /// PtyWriteKind::MouseEvent, .. }) with bytes equal to
    /// encode_mouse_event(&event, term.mode). Decision 10 Option A
    /// apex contract — kind discriminator survives Effect → mux →
    /// pump → pane.write_input boundary chain.
    #[test]
    fn handle_mouse_input_emits_effect_pty_with_mouse_event_kind_for_sgr_mode() {
        let (mut term, recorder) = term_with_recorder();
        // DECSET 1000 (MOUSE_REPORT_CLICK) + DECSET 1006 (MOUSE_SGR).
        apply_decset(&mut term, b"\x1b[?1000h\x1b[?1006h");
        let event = ev(MouseButton::Left, MouseEventKind::Press, 10, 20);
        term.handle_mouse_input(&event);

        let emissions = recorder.emissions();
        assert_eq!(emissions.len(), 1, "exactly one PTY emission expected");
        let (kind, bytes) = &emissions[0];
        assert_eq!(*kind, PtyWriteKind::MouseEvent, "kind discriminator");
        let expected = encode_mouse_event(&event, term.mode());
        assert_eq!(bytes.as_slice(), expected.as_bytes(), "byte content");
    }

    /// Pin: matrix over four mouse-encoding modes (SGR / URXVT / UTF-8 /
    /// Normal) confirms every encoder path emits via the same apex with
    /// kind == MouseEvent. Catches a future regression where one of the
    /// encoder branches is migrated to a different apex (or loses the
    /// kind discriminator).
    #[test]
    fn handle_mouse_input_carries_mouse_event_kind_across_every_encoder() {
        let setups: [&[u8]; 4] = [
            b"\x1b[?1000h\x1b[?1006h", // SGR
            b"\x1b[?1000h\x1b[?1015h", // URXVT
            b"\x1b[?1000h\x1b[?1005h", // UTF-8
            b"\x1b[?1000h",            // Normal (X10-like, click reporting)
        ];
        let event = ev(MouseButton::Right, MouseEventKind::Press, 5, 5);
        for setup in setups {
            let (mut term, recorder) = term_with_recorder();
            apply_decset(&mut term, setup);
            term.handle_mouse_input(&event);
            let emissions = recorder.emissions();
            assert_eq!(emissions.len(), 1, "setup {setup:?} emission count");
            assert_eq!(
                emissions[0].0,
                PtyWriteKind::MouseEvent,
                "setup {setup:?} kind"
            );
        }
    }

    /// When no mouse-encoding mode is active, Term::handle_mouse_input
    /// must be a no-op. Catches a regression where the apex push fires
    /// unconditionally (downstream observers would have to filter
    /// empty PTY writes — defeating the early-exit guard).
    #[test]
    fn handle_mouse_input_with_no_encoding_mode_emits_nothing() {
        let (term, recorder) = term_with_recorder();
        // Default mode has no MOUSE_REPORT_* flags — Term's
        // defense-in-depth gate suppresses the push.
        assert!(!term.mode().intersects(TermMode::ANY_MOUSE));
        let event = ev(MouseButton::Left, MouseEventKind::Press, 1, 1);
        term.handle_mouse_input(&event);
        assert!(
            recorder.emissions().is_empty(),
            "no-mode-active must not emit"
        );
    }

    /// Pin: SGR-Pixel mode 1016 + pixel coords present → apex emits
    /// pixel-encoded bytes through the Decision 10 Effect path.
    /// Confirms the §16.2.C encoder migration + §16.2.B cell-metric
    /// plumbing land together correctly.
    #[test]
    fn handle_mouse_input_sgr_pixel_mode_emits_pixel_bytes_via_apex() {
        let (mut term, recorder) = term_with_recorder();
        // DECSET 1000 (MOUSE_REPORT_CLICK) + DECSET 1016 (MOUSE_SGR_PIXEL).
        apply_decset(&mut term, b"\x1b[?1000h\x1b[?1016h");
        let event = MouseEvent {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 5,
            line: 10,
            mods: MouseModifiers::default(),
            px: Some(80),
            py: Some(160),
        };
        term.handle_mouse_input(&event);
        let emissions = recorder.emissions();
        assert_eq!(emissions.len(), 1, "exactly one PTY emission");
        let (kind, bytes) = &emissions[0];
        assert_eq!(*kind, PtyWriteKind::MouseEvent, "kind discriminator");
        // px+1=81, py+1=161 per xterm 1-indexed pixel coords.
        assert_eq!(bytes.as_slice(), b"\x1b[<0;81;161M", "SGR-Pixel bytes");
    }

    // --- Focus event apex tests (§16.7) ---

    /// Pin: Term::handle_focus_event emits Effect::Pty(Write {
    /// kind: PtyWriteKind::FocusEvent, bytes: b"\x1b[I" | b"\x1b[O" })
    /// when FOCUS_IN_OUT mode is active. Same Decision 10 Option A
    /// apex contract as mouse events.
    #[test]
    fn handle_focus_event_emits_effect_pty_with_focus_event_kind() {
        let (mut term, recorder) = term_with_recorder();
        apply_decset(&mut term, b"\x1b[?1004h"); // DECSET 1004 = FOCUS_IN_OUT
        term.handle_focus_event(true);
        term.handle_focus_event(false);
        let emissions = recorder.emissions();
        assert_eq!(emissions.len(), 2, "two PTY emissions expected");
        assert_eq!(emissions[0].0, PtyWriteKind::FocusEvent, "focus-in kind");
        assert_eq!(emissions[0].1, b"\x1b[I", "focus-in bytes");
        assert_eq!(emissions[1].0, PtyWriteKind::FocusEvent, "focus-out kind");
        assert_eq!(emissions[1].1, b"\x1b[O", "focus-out bytes");
    }

    /// Defense-in-depth gate: when FOCUS_IN_OUT not enabled, no
    /// emission. Catches a regression where the apex push fires
    /// unconditionally.
    #[test]
    fn handle_focus_event_with_mode_disabled_emits_nothing() {
        let (term, recorder) = term_with_recorder();
        // Default mode lacks FOCUS_IN_OUT.
        assert!(!term.mode().contains(TermMode::FOCUS_IN_OUT));
        term.handle_focus_event(true);
        term.handle_focus_event(false);
        assert!(
            recorder.emissions().is_empty(),
            "no FOCUS_IN_OUT must not emit"
        );
    }

    // --- §16.8 mode-interaction pilots ---

    /// Pilot (a): focus + mouse interaction. Enable 1004 + 1006
    /// simultaneously; emit focus-in then click. Apex captures BOTH
    /// `\x1b[I` AND SGR click bytes, each with its own kind
    /// discriminator (FocusEvent + MouseEvent respectively).
    #[test]
    fn focus_plus_sgr_mouse_both_emit_via_apex_with_distinct_kinds() {
        let (mut term, recorder) = term_with_recorder();
        // DECSET 1000 (mouse click) + 1006 (SGR) + 1004 (focus).
        apply_decset(&mut term, b"\x1b[?1000h\x1b[?1006h\x1b[?1004h");
        term.handle_focus_event(true);
        let click = MouseEvent {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 5,
            line: 5,
            mods: MouseModifiers::default(),
            px: None,
            py: None,
        };
        term.handle_mouse_input(&click);
        let emissions = recorder.emissions();
        assert_eq!(emissions.len(), 2, "two emissions expected (focus + click)");
        assert_eq!(emissions[0].0, PtyWriteKind::FocusEvent);
        assert_eq!(emissions[0].1, b"\x1b[I");
        assert_eq!(emissions[1].0, PtyWriteKind::MouseEvent);
        assert_eq!(emissions[1].1, b"\x1b[<0;6;6M");
    }

    /// Pilot (c): encoding-precedence. With 1006 + 1015 + 1016 all
    /// enabled and px/py present, SGR-Pixel wins (highest priority).
    /// Lower-priority encoders MUST NOT also emit (single emission).
    /// Pin per `encode_mouse_event` dispatch order in `mod.rs`.
    #[test]
    fn encoding_precedence_sgr_pixel_wins_over_sgr_and_urxvt() {
        let (mut term, recorder) = term_with_recorder();
        // DECSET 1000 + 1006 + 1015 + 1016 — all four encoding modes.
        apply_decset(&mut term, b"\x1b[?1000h\x1b[?1006h\x1b[?1015h\x1b[?1016h");
        let click = MouseEvent {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 5,
            line: 10,
            mods: MouseModifiers::default(),
            px: Some(80),
            py: Some(160),
        };
        term.handle_mouse_input(&click);
        let emissions = recorder.emissions();
        assert_eq!(emissions.len(), 1, "single emission — SGR-Pixel wins");
        // SGR-Pixel format: pixel coords (81, 161), NOT cell coords.
        assert_eq!(emissions[0].1, b"\x1b[<0;81;161M");
    }

    /// Pilot (b): sync output (mode 2026) × mouse orthogonality. Enable
    /// 2026 BSU + 1000 + 1006; emit click; assert mouse byte stream
    /// fires in-line (NOT deferred until ESU). Sync gates renderable
    /// output (frame publication), not PTY input (mouse encoding) —
    /// the two subsystems are orthogonal per §06 design.
    #[test]
    fn sync_output_does_not_defer_mouse_emission() {
        let (mut term, recorder) = term_with_recorder();
        // DECSET 2026 (sync output BSU) + 1000 + 1006.
        apply_decset(&mut term, b"\x1b[?2026h\x1b[?1000h\x1b[?1006h");
        let click = MouseEvent {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 5,
            line: 5,
            mods: MouseModifiers::default(),
            px: None,
            py: None,
        };
        term.handle_mouse_input(&click);
        let emissions = recorder.emissions();
        assert_eq!(
            emissions.len(),
            1,
            "mouse emission fires DURING sync — not deferred until ESU"
        );
        assert_eq!(emissions[0].0, PtyWriteKind::MouseEvent);
        assert_eq!(emissions[0].1, b"\x1b[<0;6;6M");
    }

    /// Mouse-encoding modes (1005/1006/1015/1016) are mutually
    /// exclusive per xterm spec — `set_mouse_encoding` clears the
    /// others on DECSET. Pin: enabling 1006 then 1015 leaves only
    /// URXVT active; the URXVT byte stream wins.
    #[test]
    fn mouse_encoding_modes_are_mutually_exclusive_last_wins() {
        let (mut term, recorder) = term_with_recorder();
        // DECSET 1000 (click reporting) + 1006 (SGR) + 1015 (URXVT).
        // 1015 setting clears 1006 per mutual-exclusion contract.
        apply_decset(&mut term, b"\x1b[?1000h\x1b[?1006h\x1b[?1015h");
        let click = MouseEvent {
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            col: 5,
            line: 10,
            mods: MouseModifiers::default(),
            px: None,
            py: None,
        };
        term.handle_mouse_input(&click);
        let emissions = recorder.emissions();
        assert_eq!(emissions.len(), 1, "single emission — URXVT wins");
        // URXVT: \x1b[{32+0};{5+1};{10+1}M = \x1b[32;6;11M
        assert_eq!(emissions[0].1, b"\x1b[32;6;11M");
    }

    /// X10 mode + Release event encodes to empty buffer (X10 reports
    /// presses only). Term::handle_mouse_input must suppress the
    /// Effect push, not emit an empty PtyEffect::Write.
    #[test]
    fn handle_mouse_input_x10_release_suppresses_emission() {
        let (mut term, recorder) = term_with_recorder();
        // DECSET 9 (MOUSE_X10).
        apply_decset(&mut term, b"\x1b[?9h");
        let event = ev(MouseButton::Left, MouseEventKind::Release, 1, 1);
        term.handle_mouse_input(&event);
        assert!(recorder.emissions().is_empty(), "X10 release must not emit");
    }
}

/// SGR-Pixel drop-on-None matrix: `encode_mouse_event` returns an empty
/// buffer (drops the event) when `MOUSE_SGR_PIXEL` is the active encoding
/// flag and `px / py` are `None`. The encoder NEVER falls through to
/// SGR / URXVT / UTF-8 / X10 from the 1016 branch — 1016-aware clients
/// (notcurses `pixelmouse_click` at in.c:556-586) divide the parsed
/// payload by cell_pixel dimensions, so any other byte shape would
/// silently corrupt the client-side cell mapping.
///
/// See: bug-tracker/plans/BUG-08-057/
#[cfg(test)]
mod sgr_pixel_drop {
    use crate::TermMode;

    use super::{MouseButton, MouseEvent, MouseEventKind, MouseModifiers, encode_mouse_event};

    fn ev_no_pixels(button: MouseButton, kind: MouseEventKind) -> MouseEvent {
        MouseEvent {
            button,
            kind,
            col: 10,
            line: 20,
            mods: MouseModifiers::default(),
            px: None,
            py: None,
        }
    }

    fn ev_with_pixels(button: MouseButton, kind: MouseEventKind) -> MouseEvent {
        MouseEvent {
            button,
            kind,
            col: 10,
            line: 20,
            mods: MouseModifiers::default(),
            px: Some(80),
            py: Some(160),
        }
    }

    // -- §03 t01: load-bearing failing case --

    /// 1016 active + Press Left + px/py = None → emit empty buffer
    /// (drop the event). Pre-fix emitted X10 `\x1b[M ` bytes.
    #[test]
    fn t01_sgr_pixel_only_without_pixel_coords_drops_event() {
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(
            &ev_no_pixels(MouseButton::Left, MouseEventKind::Press),
            mode,
        );
        assert_eq!(
            report.as_bytes(),
            b"",
            "1016 active + None pixel coords must drop the event (empty buffer); \
             NEVER fall through to X10 / SGR / URXVT / UTF-8"
        );
    }

    /// 1016 active + Press Left + Some pixel coords → SGR-Pixel bytes
    /// (regression guard for existing happy path).
    #[test]
    fn t02_sgr_pixel_with_pixel_coords_still_emits_sgr_pixel() {
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(
            &ev_with_pixels(MouseButton::Left, MouseEventKind::Press),
            mode,
        );
        assert_eq!(report.as_bytes(), b"\x1b[<0;81;161M");
    }

    /// 1016 active + Release Left + px/py = None → drop (lowercase m
    /// would be the SGR-Pixel release suffix, but no bytes emit).
    #[test]
    fn t03_sgr_pixel_only_release_without_pixel_coords_drops_event() {
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(
            &ev_no_pixels(MouseButton::Left, MouseEventKind::Release),
            mode,
        );
        assert_eq!(report.as_bytes(), b"");
    }

    /// 1016 active + Motion + px/py = None → drop. Mode 1003 path.
    #[test]
    fn t04_sgr_pixel_motion_without_pixel_coords_drops_event() {
        let mode = TermMode::MOUSE_MOTION | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(
            &ev_no_pixels(MouseButton::None, MouseEventKind::Motion),
            mode,
        );
        assert_eq!(report.as_bytes(), b"");
    }

    /// 1016 active + Drag Motion (button held) + px/py = None → drop.
    /// Mode 1002 path.
    #[test]
    fn t05_sgr_pixel_drag_motion_without_pixel_coords_drops_event() {
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(
            &ev_no_pixels(MouseButton::Left, MouseEventKind::Motion),
            mode,
        );
        assert_eq!(report.as_bytes(), b"");
    }

    // -- §03 t06 / t07: mixed px/py states --

    /// 1016 active + only px Some, py None → drop. Defends against
    /// future regression where one of the two coords being present is
    /// accidentally treated as "valid enough."
    #[test]
    fn t06_sgr_pixel_with_only_px_some_drops_event() {
        let mut ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        ev.px = Some(80);
        ev.py = None;
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"");
    }

    /// 1016 active + only py Some, px None → drop. Mirror of t06.
    #[test]
    fn t07_sgr_pixel_with_only_py_some_drops_event() {
        let mut ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        ev.px = None;
        ev.py = Some(160);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"");
    }

    // -- §03 t08 / t09: wheel button coverage --

    /// 1016 active + ScrollUp + px/py = None → drop. The wheel-Tier-1
    /// path was the most likely real-world trigger of the bug per §01.
    #[test]
    fn t08_sgr_pixel_scrollup_without_pixel_coords_drops_event() {
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(
            &ev_no_pixels(MouseButton::ScrollUp, MouseEventKind::Press),
            mode,
        );
        assert_eq!(report.as_bytes(), b"");
    }

    /// 1016 active + ScrollDown + px/py = None → drop. Mirror of t08.
    #[test]
    fn t09_sgr_pixel_scrolldown_without_pixel_coords_drops_event() {
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(
            &ev_no_pixels(MouseButton::ScrollDown, MouseEventKind::Press),
            mode,
        );
        assert_eq!(report.as_bytes(), b"");
    }

    // -- §03 t10 / t11: modifier propagation --

    /// 1016 active + Shift|Ctrl + Some pixel coords → button code
    /// `0 + 4 + 16 = 20` in the SGR-Pixel envelope. Regression guard:
    /// modifier propagation not perturbed by the new branch.
    #[test]
    fn t10_sgr_pixel_drop_carries_through_modifiers_with_present_coords() {
        let mut ev = ev_with_pixels(MouseButton::Left, MouseEventKind::Press);
        ev.mods = MouseModifiers {
            shift: true,
            alt: false,
            ctrl: true,
        };
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"\x1b[<20;81;161M");
    }

    /// 1016 active + Shift|Ctrl + None pixel coords → drop (modifiers
    /// don't rescue missing pixel coords).
    #[test]
    fn t11_sgr_pixel_drop_with_modifiers_without_pixel_coords_drops_event() {
        let mut ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        ev.mods = MouseModifiers {
            shift: true,
            alt: false,
            ctrl: true,
        };
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"");
    }

    // -- §03 t12–t18: cross-encoding regression matrix --

    /// SGR (1006) + drag is unchanged by the fix.
    #[test]
    fn t12_sgr_with_drag_present_unchanged() {
        let ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"\x1b[<0;11;21M");
    }

    /// URXVT (1015) + drag is unchanged.
    #[test]
    fn t15_urxvt_unchanged() {
        let ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_URXVT;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"\x1b[32;11;21M");
    }

    /// UTF-8 (1005) + drag is unchanged.
    #[test]
    fn t16_utf8_unchanged() {
        let ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_UTF8;
        let report = encode_mouse_event(&ev, mode);
        // UTF-8 encoding: \x1b[M + button (32) + col+33 (43) + line+53 (53)
        assert_eq!(report.as_bytes(), &[0x1b, b'[', b'M', 32, 43, 53]);
    }

    /// 1002 alone (no extended encoding flag) → X10 envelope is the
    /// LEGITIMATE legacy path. Fix must NOT change this.
    #[test]
    fn t17_no_extended_encoding_uses_x10_unchanged() {
        let ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        let mode = TermMode::MOUSE_DRAG;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), &[0x1b, b'[', b'M', 32, 43, 53]);
    }

    // -- §03 t19 / t21 / t22: additional regression guards --

    /// Press then Release distinction preserved through the 1016 branch.
    #[test]
    fn t19_sgr_pixel_press_release_distinction_preserved() {
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let press = encode_mouse_event(
            &ev_with_pixels(MouseButton::Left, MouseEventKind::Press),
            mode,
        );
        let release = encode_mouse_event(
            &ev_with_pixels(MouseButton::Left, MouseEventKind::Release),
            mode,
        );
        assert_eq!(press.as_bytes(), b"\x1b[<0;81;161M");
        assert_eq!(release.as_bytes(), b"\x1b[<0;81;161m");
    }

    /// SGR (1006) mode ignores px/py (cell coords always). Regression
    /// guard: fix must not accidentally make 1006 consume px/py.
    #[test]
    fn t20_sgr_only_mode_ignores_px_py_when_present() {
        let ev = ev_with_pixels(MouseButton::Left, MouseEventKind::Press);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"\x1b[<0;11;21M");
    }

    /// 1016 + Some(0), Some(0) → `1;1` (1-indexed origin).
    #[test]
    fn t21_sgr_pixel_at_origin_emits_origin_pixels() {
        let mut ev = ev_with_pixels(MouseButton::Left, MouseEventKind::Press);
        ev.px = Some(0);
        ev.py = Some(0);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"\x1b[<0;1;1M");
    }

    /// 1016 + large logical-pixel values (HD-class display) emits
    /// correct offset without overflow.
    #[test]
    fn t22_sgr_pixel_at_large_coords_emits_correct_offset() {
        let mut ev = ev_with_pixels(MouseButton::Left, MouseEventKind::Press);
        ev.px = Some(1920);
        ev.py = Some(1080);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(&ev, mode);
        assert_eq!(report.as_bytes(), b"\x1b[<0;1921;1081M");
    }

    // -- §03 t25: self-verifying-matrix --

    /// Matrix loop: 6 buttons × 3 kinds × 8 modifier sets × 11 mode
    /// cases = 1584 cells. For each cell, 1016 active + px/py = None
    /// → emissions empty. The `count` assertion proves no cell was
    /// silently skipped by the loop body.
    #[test]
    fn t25_pin_sgr_pixel_active_with_none_coords_never_emits_any_bytes() {
        let buttons = [
            MouseButton::Left,
            MouseButton::Middle,
            MouseButton::Right,
            MouseButton::ScrollUp,
            MouseButton::ScrollDown,
            MouseButton::None,
        ];
        let kinds = [
            MouseEventKind::Press,
            MouseEventKind::Release,
            MouseEventKind::Motion,
        ];
        let mod_sets = [
            MouseModifiers {
                shift: false,
                alt: false,
                ctrl: false,
            },
            MouseModifiers {
                shift: true,
                alt: false,
                ctrl: false,
            },
            MouseModifiers {
                shift: false,
                alt: true,
                ctrl: false,
            },
            MouseModifiers {
                shift: false,
                alt: false,
                ctrl: true,
            },
            MouseModifiers {
                shift: true,
                alt: true,
                ctrl: false,
            },
            MouseModifiers {
                shift: true,
                alt: false,
                ctrl: true,
            },
            MouseModifiers {
                shift: false,
                alt: true,
                ctrl: true,
            },
            MouseModifiers {
                shift: true,
                alt: true,
                ctrl: true,
            },
        ];
        let base = TermMode::MOUSE_SGR_PIXEL;
        let mode_cases: [TermMode; 11] = [
            TermMode::MOUSE_DRAG | base,
            TermMode::MOUSE_REPORT_CLICK | base,
            TermMode::MOUSE_MOTION | base,
            TermMode::MOUSE_X10 | base,
            TermMode::MOUSE_DRAG | base | TermMode::SYNC_UPDATE,
            TermMode::MOUSE_DRAG | base | TermMode::MOUSE_HIGHLIGHT,
            TermMode::MOUSE_DRAG | base | TermMode::FOCUS_IN_OUT,
            TermMode::MOUSE_DRAG | base | TermMode::ALT_SCREEN,
            TermMode::MOUSE_DRAG | base | TermMode::MOUSE_SGR,
            TermMode::MOUSE_DRAG | base | TermMode::MOUSE_URXVT,
            TermMode::MOUSE_DRAG | base | TermMode::MOUSE_UTF8,
        ];

        let mut count: usize = 0;
        for button in buttons {
            for kind in kinds {
                for &mods in &mod_sets {
                    for mode in mode_cases {
                        let mut ev = ev_no_pixels(button, kind);
                        ev.mods = mods;
                        let report = encode_mouse_event(&ev, mode);
                        assert_eq!(
                            report.as_bytes(),
                            b"",
                            "cell button={:?} kind={:?} mods={:?} mode={:?} \
                             must emit empty buffer when 1016 + px/py=None",
                            button,
                            kind,
                            mods,
                            mode,
                        );
                        count += 1;
                    }
                }
            }
        }
        assert_eq!(
            count,
            buttons.len() * kinds.len() * mod_sets.len() * mode_cases.len(),
            "matrix completeness — every cell must be visited (no silent skip)"
        );
        assert_eq!(count, 1584, "expected 6 * 3 * 8 * 11 = 1584 cells");
    }

    /// Under any 1016-active mode, the encoder NEVER emits X10
    /// envelope bytes (`\x1b[M`) NOR SGR cell-coord bytes
    /// masquerading as pixels.
    #[test]
    fn t26_negative_pin_sgr_pixel_active_x10_envelope_banned() {
        let ev = ev_no_pixels(MouseButton::Left, MouseEventKind::Press);
        let mode = TermMode::MOUSE_DRAG | TermMode::MOUSE_SGR_PIXEL;
        let report = encode_mouse_event(&ev, mode);
        let bytes = report.as_bytes();
        // Drop-on-None is empty. If non-empty in some future regression,
        // it MUST NOT be X10 (`\x1b[M`) nor SGR with cell coords.
        assert!(
            !bytes.windows(3).any(|w| w == b"\x1b[M"),
            "1016 active path must never emit X10 envelope: got {bytes:?}"
        );
    }
}
