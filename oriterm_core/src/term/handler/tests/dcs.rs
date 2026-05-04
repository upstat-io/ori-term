use crate::term::{Term, TermMode};
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_recorder};

/// Create a Term with VoidEffectSink (when effects don't matter).
fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

// --- Kitty keyboard protocol tests ---

#[test]
fn push_keyboard_mode_1() {
    let (mut t, _listener) = term_with_recorder();
    // CSI > 1 u — push mode with DISAMBIGUATE_ESC_CODES.
    feed(&mut t, b"\x1b[>1u");

    assert_eq!(t.keyboard_mode_stack().len(), 1);
    assert!(
        t.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES),
        "push_keyboard_mode(1) should set DISAMBIGUATE_ESC_CODES"
    );
}

#[test]
fn push_keyboard_mode_3() {
    let (mut t, _listener) = term_with_recorder();
    // Mode 3 = DISAMBIGUATE_ESC_CODES | REPORT_EVENT_TYPES.
    feed(&mut t, b"\x1b[>3u");

    assert_eq!(t.keyboard_mode_stack().len(), 1);
    assert!(t.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
    assert!(t.mode().contains(TermMode::REPORT_EVENT_TYPES));
}

#[test]
fn pop_keyboard_mode() {
    let (mut t, _listener) = term_with_recorder();
    // Push two modes.
    feed(&mut t, b"\x1b[>1u");
    feed(&mut t, b"\x1b[>3u");
    assert_eq!(t.keyboard_mode_stack().len(), 2);

    // Pop one.
    feed(&mut t, b"\x1b[<u");
    assert_eq!(t.keyboard_mode_stack().len(), 1);
    // Active mode should be the remaining mode (1 = DISAMBIGUATE_ESC_CODES).
    assert!(t.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
    assert!(
        !t.mode().contains(TermMode::REPORT_EVENT_TYPES),
        "after pop, only first pushed mode should remain active"
    );
}

#[test]
fn pop_all_keyboard_modes() {
    let (mut t, _listener) = term_with_recorder();
    feed(&mut t, b"\x1b[>1u");
    feed(&mut t, b"\x1b[>3u");

    // Pop both (pop 2).
    feed(&mut t, b"\x1b[<2u");
    assert!(t.keyboard_mode_stack().is_empty());
    assert!(
        !t.mode().intersects(TermMode::KITTY_KEYBOARD_PROTOCOL),
        "all kitty flags should be cleared after popping all modes"
    );
}

#[test]
fn query_keyboard_mode_responds_with_current() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[>1u");
    // CSI ? u — query.
    feed(&mut t, b"\x1b[?u");

    let events = listener.events();
    let pty_writes: Vec<_> = events.iter().filter(|e| e.contains("PtyWrite")).collect();
    assert!(
        pty_writes.iter().any(|w| w.contains("[?1u")),
        "report should contain current mode bits: {pty_writes:?}"
    );
}

/// Regression: review round-6 F1 — `CSI ? u` must report the
/// live `TermMode::KITTY_KEYBOARD_PROTOCOL` bits, not the stack top.
/// Set-only modes enabled via `CSI = Ps u` do not enter the stack; a
/// stack-top-derived report would incorrectly reply `?0u` while the
/// bits are actually live.
/// See: bug-tracker/plans/completed/00-overview.md §2.5 review round 6.
#[test]
fn query_keyboard_mode_set_only_via_csi_equals_u_reports_live_bits() {
    let (mut t, listener) = term_with_recorder();
    // Set mode via `CSI = 1 u` (Replace, no push) — bits live in
    // `self.mode` only; stack remains empty.
    feed(&mut t, b"\x1b[=1u");
    assert!(t.mode().contains(TermMode::DISAMBIGUATE_ESC_CODES));
    assert!(
        t.keyboard_mode_stack().is_empty(),
        "set-only path must not touch the stack"
    );

    feed(&mut t, b"\x1b[?u");

    let events = listener.events();
    let pty_writes: Vec<_> = events.iter().filter(|e| e.contains("PtyWrite")).collect();
    assert!(
        pty_writes.iter().any(|w| w.contains("[?1u")),
        "query reply must reflect live bits (1), not stack top (0): {pty_writes:?}"
    );
}

#[test]
fn pop_from_empty_stack_is_noop() {
    let mut t = term();
    // Pop from empty — should not panic.
    feed(&mut t, b"\x1b[<u");
    assert!(t.keyboard_mode_stack().is_empty());
}

// --- DECRQSS tests (DCS $ q... ST) ---

#[test]
fn decrqss_decscl_reports_vt400() {
    let (mut t, listener) = term_with_recorder();
    // DCS $ q " p ST — request conformance level.
    feed(&mut t, b"\x1bP$q\"p\x1b\\");

    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e == "PtyWrite(\x1bP1$r64;1\"p\x1b\\)"),
        "DECRQSS DECSCL should report VT400 level: {events:?}"
    );
}

#[test]
fn decrqss_decstbm_reports_scroll_region() {
    let (mut t, listener) = term_with_recorder();
    // Set scroll region to lines 5-15 (1-based).
    feed(&mut t, b"\x1b[5;15r");
    // DCS $ q r ST — request scroll region.
    feed(&mut t, b"\x1bP$qr\x1b\\");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1bP1$r5;15r\x1b\\)"),
        "DECRQSS DECSTBM should report 5;15: {events:?}"
    );
}

#[test]
fn decrqss_sgr_reports_default_rendition() {
    let (mut t, listener) = term_with_recorder();
    // DCS $ q m ST — request SGR status.
    feed(&mut t, b"\x1bP$qm\x1b\\");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1bP1$r0m\x1b\\)"),
        "DECRQSS SGR should report reset (0m) at default: {events:?}"
    );
}

#[test]
fn decrqss_sgr_reports_bold() {
    let (mut t, listener) = term_with_recorder();
    // Enable bold via SGR 1.
    feed(&mut t, b"\x1b[1m");
    // DCS $ q m ST — request SGR status.
    feed(&mut t, b"\x1bP$qm\x1b\\");

    let events = listener.events();
    let sgr_resp = events
        .iter()
        .find(|e| e.starts_with("PtyWrite(\x1bP1$r") && e.ends_with("m\x1b\\)"))
        .expect("DECRQSS SGR response should be emitted");
    assert!(
        sgr_resp.contains(";1") || sgr_resp.contains("r1"),
        "DECRQSS SGR should include bold (1): {sgr_resp}"
    );
}

#[test]
fn decrqss_unknown_reports_invalid() {
    let (mut t, listener) = term_with_recorder();
    // DCS $ q Z ST — unknown query.
    feed(&mut t, b"\x1bP$qZ\x1b\\");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1bP0$r\x1b\\)"),
        "DECRQSS unknown query should report invalid (0$r): {events:?}"
    );
}

#[test]
fn decrqss_decscusr_reports_default_blink_block() {
    let (mut t, listener) = term_with_recorder();
    // DCS $ q <SP> q ST — request cursor style.
    feed(&mut t, b"\x1bP$q q\x1b\\");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1bP1$r1 q\x1b\\)"),
        "DECRQSS DECSCUSR at default should be blink block (Ps=1): {events:?}"
    );
}

#[test]
fn decrqss_decscusr_reports_steady_bar_after_decscusr_6() {
    let (mut t, listener) = term_with_recorder();
    // DECSCUSR 6 = steady bar.
    feed(&mut t, b"\x1b[6 q");
    feed(&mut t, b"\x1bP$q q\x1b\\");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1bP1$r6 q\x1b\\)"),
        "DECRQSS DECSCUSR after `CSI 6 SP q` should be Ps=6: {events:?}"
    );
}

#[test]
fn decrqss_decsca_reports_unprotected_at_default() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1bP$q\"q\x1b\\");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1bP1$r2\"q\x1b\\)"),
        "DECRQSS DECSCA at default should report unprotected (Ps=2): {events:?}"
    );
}

#[test]
fn decrqss_decsca_reports_protected_after_decsca_1() {
    let (mut t, listener) = term_with_recorder();
    // DECSCA 1 = protected.
    feed(&mut t, b"\x1b[1\"q");
    feed(&mut t, b"\x1bP$q\"q\x1b\\");

    let events = listener.events();
    assert!(
        events.iter().any(|e| e == "PtyWrite(\x1bP1$r1\"q\x1b\\)"),
        "DECRQSS DECSCA after `CSI 1 \" q` should report Ps=1: {events:?}"
    );
}

// --- DECRSPS tests (DCS Ps $ t... ST) ---

#[test]
fn decrsps_ps1_is_accepted_without_reply_or_state_change() {
    let (mut t, listener) = term_with_recorder();
    let cursor_shape_before = t.cursor_shape();
    // DCS 1 $ t... ST — Ps=1 DECCIR cursor-info restore.
    feed(&mut t, b"\x1bP1$tdata\x1b\\");

    // Stub: no reply emitted (DECRSPS has no acknowledgement per xterm spec),
    // and no presentation state mutated.
    let events = listener.events();
    let pty_writes: Vec<_> = events.iter().filter(|e| e.contains("PtyWrite")).collect();
    assert!(
        pty_writes.is_empty(),
        "DECRSPS stub should emit no PTY write: {pty_writes:?}"
    );
    assert_eq!(t.cursor_shape(), cursor_shape_before);
}

#[test]
fn decrsps_ps2_is_accepted_without_reply_or_state_change() {
    let (mut t, listener) = term_with_recorder();
    // DCS 2 $ t... ST — Ps=2 DECTABSR tab-stop restore.
    feed(&mut t, b"\x1bP2$t1/9/17\x1b\\");

    let events = listener.events();
    let pty_writes: Vec<_> = events.iter().filter(|e| e.contains("PtyWrite")).collect();
    assert!(
        pty_writes.is_empty(),
        "DECRSPS Ps=2 stub should emit no PTY write: {pty_writes:?}"
    );
}
