//! Tests for the mouse-event encoder.
//!
//! Catalog rows: MOUSE-X10, MOUSE-VT200, MOUSE-BTN-EVENT, MOUSE-ANY-EVENT,
//! MOUSE-SGR, MOUSE-URXVT, MOUSE-UTF8, MOUSE-SGR-PIXEL. All routed through
//! `encode_mouse_event` dispatch + `Term::handle_mouse_input` apex per
//! Decision 10 Option A; pin tests under `term_apex` cover the kind=MouseEvent
//! preservation across the Effect → mux → pump → pane.write_input boundary
//! chain for every encoder branch.

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

#[test]
fn encode_mouse_event_sgr_pixel_mode_falls_back_to_sgr_when_coords_missing() {
    // Pre-§16.2.B callers or cursor-outside-grid: px/py are None.
    // Even with MOUSE_SGR_PIXEL active, the encoder falls through to
    // SGR (cell coords) so wiring breakage degrades gracefully.
    let ev = event(MouseButton::Left, MouseEventKind::Press, 5, 10);
    let mode = TermMode::MOUSE_REPORT_CLICK | TermMode::MOUSE_SGR | TermMode::MOUSE_SGR_PIXEL;
    let report = encode_mouse_event(&ev, mode);
    // SGR cell encoding (col+1=6, line+1=11), NOT pixel.
    assert_eq!(report.as_bytes(), b"\x1b[<0;6;11M");
}

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
