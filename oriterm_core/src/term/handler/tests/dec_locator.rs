//! End-to-end DEC Locator pin tests — byte stream → vte parser →
//! Handler trait → DecLocatorState mutation + DECLRP apex emission.
//!
//! Catalog rows: MOUSE-DECEFR, MOUSE-DECELR, MOUSE-DECSLE, MOUSE-DECRQLP,
//! MOUSE-DECLRP-REPLY. State-rung + apex-emission pin tests.

use crate::encode::mouse::{MouseButton, MouseEvent, MouseEventKind, MouseModifiers};
use crate::term::Term;
use crate::term::dec_locator::{
    LocatorEventMask, LocatorPosition, LocatorRect, LocatorReportingMode,
};
use crate::theme::Theme;

use super::super::test_helpers::{feed, term_with_recorder};

fn term() -> Term<crate::effect::VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), crate::effect::VoidEffectSink)
}

/// Build a cell-coord press/release/motion event. `physical_px` defaults
/// to `None`; tests that exercise DECELR Pu=1 set it explicitly.
fn cell_event(button: MouseButton, kind: MouseEventKind, col: usize, line: usize) -> MouseEvent {
    MouseEvent::cell(button, kind, col, line, MouseModifiers::default())
}

// ── DECELR (CSI Ps;Pu ' z) ──────────────────────────────────────────

#[test]
fn decelr_ps1_continuous_cells_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;0'z");
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
    assert!(!t.dec_locator().pixel_unit());
}

#[test]
fn decelr_ps2_oneshot_pixels_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[2;1'z");
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::OneShot)
    );
    assert!(t.dec_locator().pixel_unit());
}

#[test]
fn decelr_ps0_disables_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;0'z"); // enable continuous
    assert!(t.dec_locator().reporting().is_some());
    feed(&mut t, b"\x1b[0;0'z"); // disable
    assert_eq!(t.dec_locator().reporting(), None);
}

// ── DECSLE (CSI Pm ' {) ─────────────────────────────────────────────

#[test]
fn decsle_pm1_sets_button_down_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1'{");
    assert_eq!(t.dec_locator().event_mask(), LocatorEventMask::BUTTON_DOWN);
}

#[test]
fn decsle_pm_list_combines_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;3'{");
    assert_eq!(
        t.dec_locator().event_mask(),
        LocatorEventMask::BUTTON_DOWN | LocatorEventMask::BUTTON_UP
    );
}

// ── DECEFR (CSI Pt;Pl;Pb;Pr ' w) ────────────────────────────────────

#[test]
fn decefr_stores_rectangle_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;10;15;20'w");
    assert_eq!(
        t.dec_locator().filter_rect(),
        Some(LocatorRect {
            top: 5,
            left: 10,
            bottom: 15,
            right: 20,
        })
    );
}

#[test]
fn decefr_all_zeros_clears_rectangle_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[5;10;15;20'w");
    assert!(t.dec_locator().filter_rect().is_some());
    feed(&mut t, b"\x1b[0;0;0;0'w");
    assert_eq!(t.dec_locator().filter_rect(), None);
}

// ── DECRQLP (CSI Ps ' |) ────────────────────────────────────────────

#[test]
fn decrqlp_in_oneshot_auto_clears_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[2;0'z"); // one-shot
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::OneShot)
    );
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    assert_eq!(
        t.dec_locator().reporting(),
        None,
        "OneShot must auto-clear after DECRQLP per xterm spec"
    );
}

#[test]
fn decrqlp_in_continuous_does_not_clear_via_csi() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;0'z"); // continuous
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
}

// ── Cross-state independence (independent of DECSET 1001) ───────────

// ── DECLRP apex-emission pin tests (§16.1.C item 1) ─────────────────

/// DECRQLP when locator is disabled (default) emits DECLRP with Pe=0
/// ("locator unavailable") as a ONE-parameter reply `CSI 0 & w`
/// (`a_nparam = 1` per xterm `button.c:857-861`), via Effect::Pty with
/// kind = PtyWriteKind::MouseEvent.
///
/// Regression: BUG-08-058 — the prior 5-parameter `\x1b[0;0;1;1;1&w`
/// placeholder form contradicted xterm; the unavailable reply carries
/// a single `0` parameter.
#[test]
fn decrqlp_disabled_emits_declrp_pe0_via_effect_pty_mouse_event() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1'|"); // DECRQLP Ps=1
    let events = listener.events();
    let expected = "PtyWrite(\x1b[0&w)".to_string();
    assert!(
        events.iter().any(|e| *e == expected),
        "expected DECLRP Pe=0 one-param reply, got events: {events:?}"
    );
}

/// DECRQLP when locator is Continuous-enabled, after an observed click,
/// emits DECLRP with Pe=1 ("request response") + the REAL observed
/// coords. Locator stays Continuous (no auto-clear).
///
/// Regression: BUG-08-058 — the prior reply hardcoded placeholder
/// `Pr=1 Pc=1`; it now reports the click position observed by
/// `handle_mouse_input` Step A.
#[test]
fn decrqlp_continuous_emits_declrp_pe1_no_auto_clear() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // DECELR Ps=1 (continuous, cells)
    // Left press at (col=10, line=20) → Pm=4 (left, bit 2), Pr=21, Pc=11.
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        10,
        20,
    ));
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    let events = listener.events();
    let expected = "PtyWrite(\x1b[1;4;21;11;1&w)".to_string();
    assert!(
        events.iter().any(|e| *e == expected),
        "expected DECLRP Pe=1 reply with real coords, got events: {events:?}"
    );
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous),
        "Continuous reporting persists across DECRQLP per xterm spec"
    );
}

/// DECRQLP when locator is OneShot-enabled, after an observed click,
/// emits DECLRP with Pe=1 + real coords AND auto-clears reporting →
/// None per xterm spec.
///
/// Regression: BUG-08-058 — reply now carries the observed click
/// position, not the placeholder `Pr=1 Pc=1`.
#[test]
fn decrqlp_oneshot_emits_declrp_pe1_and_auto_clears() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[2;0'z"); // DECELR Ps=2 (one-shot, cells)
    // Left press at (col=10, line=20) → Pm=4, Pr=21, Pc=11.
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        10,
        20,
    ));
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    let events = listener.events();
    let expected = "PtyWrite(\x1b[1;4;21;11;1&w)".to_string();
    assert!(
        events.iter().any(|e| *e == expected),
        "expected DECLRP Pe=1 reply with real coords, got events: {events:?}"
    );
    assert_eq!(
        t.dec_locator().reporting(),
        None,
        "OneShot reporting must auto-clear to None after DECRQLP reply"
    );
}

/// Second DECRQLP after OneShot auto-clear emits Pe=0 (locator now
/// disabled by the auto-clear) — pins the OneShot semantic end-to-end.
///
/// Regression: BUG-08-058 — the Pe=1 reply carries observed coords;
/// the post-clear Pe=0 reply is the ONE-param `\x1b[0&w` form.
#[test]
fn second_decrqlp_after_oneshot_clear_emits_pe0() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[2;0'z"); // OneShot
    // Left press at (col=10, line=20) → Pm=4, Pr=21, Pc=11.
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        10,
        20,
    ));
    feed(&mut t, b"\x1b[1'|"); // DECRQLP (auto-clears)
    feed(&mut t, b"\x1b[1'|"); // DECRQLP again
    let events = listener.events();
    let pe1 = "PtyWrite(\x1b[1;4;21;11;1&w)".to_string();
    let pe0 = "PtyWrite(\x1b[0&w)".to_string();
    let pe1_count = events.iter().filter(|e| **e == pe1).count();
    let pe0_count = events.iter().filter(|e| **e == pe0).count();
    assert_eq!(pe1_count, 1, "exactly one Pe=1 reply (the OneShot)");
    assert_eq!(pe0_count, 1, "second DECRQLP emits Pe=0 (locator cleared)");
}

/// DECRQLP with Ps other than 0/1 is silently dropped per xterm spec.
#[test]
fn decrqlp_invalid_ps_emits_nothing() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // enable continuous
    feed(&mut t, b"\x1b[99'|"); // invalid Ps=99
    let events = listener.events();
    let declrp_emitted = events.iter().any(|e| e.contains("&w"));
    assert!(
        !declrp_emitted,
        "DECRQLP with Ps != 0|1 must emit no reply, got: {events:?}"
    );
}

#[test]
fn dec_locator_independent_of_mode_1001() {
    let mut t = term();
    feed(&mut t, b"\x1b[?1001h"); // enable highlight tracking (mode 1001)
    // Mode 1001 must NOT enable DEC Locator reporting.
    assert_eq!(t.dec_locator().reporting(), None);

    feed(&mut t, b"\x1b[1;0'z"); // enable DEC Locator (continuous, cells)
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
    // Mode 1001 still set; DEC Locator independently active.
    assert!(t.mode().contains(crate::term::TermMode::MOUSE_HIGHLIGHT));

    feed(&mut t, b"\x1b[?1001l"); // disable highlight tracking
    // DEC Locator state unaffected.
    assert_eq!(
        t.dec_locator().reporting(),
        Some(LocatorReportingMode::Continuous)
    );
}

// ── DECRQLP POLL-path coords ─────────────────────────────────────
//
// These pin the observed-position → DECLRP-reply contract: a click /
// motion / wheel observed by `handle_mouse_input` Step A while DECELR
// reporting is active is reported back by a subsequent DECRQLP with the
// REAL coords + Pm button mask, NOT the pre-fix placeholder `Pr=Pc=1`.

/// Extract the parameters of the last DECLRP reply
/// (`CSI Pe ; Pm ; Pr ; Pc ; Pp & w`, or the one-param `CSI 0 & w`
/// unavailable form) from the recorder events. Returns the param list
/// (e.g. `[1, 4, 31, 51, 1]` or `[0]`).
fn last_declrp_params(events: &[String]) -> Vec<u16> {
    let reply = events
        .iter()
        .rev()
        .find_map(|e| {
            e.strip_prefix("PtyWrite(\x1b[")
                .and_then(|s| s.strip_suffix("&w)"))
        })
        .expect("no DECLRP reply found in recorder events");
    reply
        .split(';')
        .map(|p| p.parse().expect("DECLRP param is not a u16"))
        .collect()
}

/// Exact failing case from the repro: DECELR Ps=1 Pu=0 + Left press at
/// (col=50, line=30) + DECRQLP → `CSI 1 ; 4 ; 31 ; 51 ; 1 & w`
/// (Pe=1, Pm=4 [left, bit 2 per xterm button.c:944-948 swap], Pr=31,
/// Pc=51 [0-indexed input +1], Pp=1).
#[test]
fn dec_locator_decrqlp_reports_actual_click_position() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // DECELR Ps=1 Pu=0 (continuous, cells)
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        50,
        30,
    ));
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    assert_eq!(
        listener.events().iter().rev().find(|e| e.contains("&w")),
        Some(&"PtyWrite(\x1b[1;4;31;51;1&w)".to_string()),
        "DECLRP must carry the observed click position",
    );
}

/// DECELR Ps=1 + DECRQLP with no preceding click/motion → Pe=0
/// locator-unavailable (one-param `\x1b[0&w`); the default
/// `LocatorPosition::Unavailable` is reported, NOT a placeholder.
#[test]
fn dec_locator_decrqlp_emits_pe0_when_no_prior_observation() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // DECELR Ps=1 (reporting ON, but no observation)
    feed(&mut t, b"\x1b[1'|"); // DECRQLP
    assert_eq!(last_declrp_params(&listener.events()), vec![0]);
}

/// Press then release the same button: Pm clears to 0 (no buttons
/// held) but Pr/Pc still reflect the release-event position.
#[test]
fn dec_locator_decrqlp_after_button_release_carries_release_pm() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z");
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        50,
        30,
    ));
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Release,
        50,
        30,
    ));
    feed(&mut t, b"\x1b[1'|");
    let p = last_declrp_params(&listener.events());
    assert_eq!(p[0], 1, "Pe=1 request response");
    assert_eq!(p[1], 0, "Pm=0 after release (no buttons held)");
    assert_eq!((p[2], p[3]), (31, 51), "Pr/Pc reflect release position");
}

/// Left + Right held → Pm = 4 (left, bit 2) | 1 (right, bit 0) = 5.
#[test]
fn dec_locator_decrqlp_with_multiple_buttons() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z");
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        10,
        10,
    ));
    t.handle_mouse_input(&cell_event(
        MouseButton::Right,
        MouseEventKind::Press,
        10,
        10,
    ));
    feed(&mut t, b"\x1b[1'|");
    assert_eq!(
        last_declrp_params(&listener.events())[1],
        5,
        "Pm = left|right = 5"
    );
}

/// Parameterized Pm-per-button matrix per xterm button.c:944-948 swap:
/// Left→4 (bit 2), Middle→2 (bit 1), Right→1 (bit 0). Self-verifying
/// count assertion proves every cell ran.
#[test]
fn dec_locator_decrqlp_pm_bit_mapping_per_button() {
    let cases = [
        (MouseButton::Left, 4u16),
        (MouseButton::Middle, 2),
        (MouseButton::Right, 1),
    ];
    let mut count = 0;
    for (button, expected_pm) in cases {
        let (mut t, listener) = term_with_recorder();
        feed(&mut t, b"\x1b[1;0'z");
        t.handle_mouse_input(&cell_event(button, MouseEventKind::Press, 10, 10));
        feed(&mut t, b"\x1b[1'|");
        assert_eq!(
            last_declrp_params(&listener.events())[1],
            expected_pm,
            "Pm for {button:?} press",
        );
        count += 1;
    }
    assert_eq!(count, cases.len(), "every Pm-mapping cell must run");
}

/// Motion events update the observed position (mode-1003-style motion
/// reaches Step A too). DECELR Ps=1 + Motion at (50, 30) + DECRQLP →
/// Pr=31 Pc=51.
#[test]
fn dec_locator_decrqlp_after_motion_updates_position() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z");
    t.handle_mouse_input(&cell_event(
        MouseButton::None,
        MouseEventKind::Motion,
        50,
        30,
    ));
    feed(&mut t, b"\x1b[1'|");
    let p = last_declrp_params(&listener.events());
    assert_eq!((p[2], p[3]), (31, 51), "motion updates Pr/Pc");
}

/// DECELR Pu=1 reports DEVICE physical pixels (Pr/Pc from `physical_px`,
/// +1). Rejection guard: the same event under Pu=0 reports cell coords.
#[test]
fn dec_locator_decrqlp_pu1_pixel_unit_reports_pixels() {
    let pixel_event = MouseEvent {
        button: MouseButton::Left,
        kind: MouseEventKind::Press,
        col: 50,
        line: 30,
        mods: MouseModifiers::default(),
        px: None,
        py: None,
        physical_px: Some((400, 240)),
        in_grid: true,
    };
    // Pu=1 → pixel coords: Pc = 400+1 = 401, Pr = 240+1 = 241.
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;1'z"); // DECELR Ps=1 Pu=1 (pixels)
    t.handle_mouse_input(&pixel_event);
    feed(&mut t, b"\x1b[1'|");
    let p = last_declrp_params(&listener.events());
    assert_eq!((p[2], p[3]), (241, 401), "Pu=1 reports physical pixels");

    // Rejection guard: Pu=0, same event → cell coords (Pr=31, Pc=51).
    let (mut t0, listener0) = term_with_recorder();
    feed(&mut t0, b"\x1b[1;0'z"); // DECELR Ps=1 Pu=0 (cells)
    t0.handle_mouse_input(&pixel_event);
    feed(&mut t0, b"\x1b[1'|");
    let p0 = last_declrp_params(&listener0.events());
    assert_eq!((p0[2], p0[3]), (31, 51), "Pu=0 reports cell coords");
}

/// Wheel scroll observed while DECELR is active captures the cursor
/// position (wheel doesn't change Pm — `button_mask_for_event` returns
/// the prior mask for wheel). Cursor at (col=30, line=50) → Pr=51 Pc=31.
#[test]
fn dec_locator_wheel_updates_position() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z");
    t.handle_mouse_input(&cell_event(
        MouseButton::ScrollUp,
        MouseEventKind::Press,
        30,
        50,
    ));
    feed(&mut t, b"\x1b[1'|");
    let p = last_declrp_params(&listener.events());
    assert_eq!((p[2], p[3]), (51, 31), "wheel captures cursor position");
    assert_eq!(p[1], 0, "wheel does not set a Pm button bit");
}

/// Out-of-grid cursor (`in_grid = false`) → `LocatorPosition::Unavailable`
/// → DECRQLP Pe=0 (one-param) per xterm button.c:857-861, even though
/// reporting is active.
#[test]
fn dec_locator_decrqlp_out_of_grid_click_emits_pe0() {
    let out_of_grid = MouseEvent {
        button: MouseButton::Left,
        kind: MouseEventKind::Press,
        col: 50,
        line: 30,
        mods: MouseModifiers::default(),
        px: None,
        py: None,
        physical_px: None,
        in_grid: false,
    };
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z");
    t.handle_mouse_input(&out_of_grid);
    feed(&mut t, b"\x1b[1'|");
    assert_eq!(last_declrp_params(&listener.events()), vec![0]);
}

/// Behavioral guard: Step A observes a position even when NO mouse-tracking
/// mode is active (ANY_MOUSE empty) — the observation gate is Term's OWN
/// `dec_locator.reporting()`, independent of the encoder gate.
#[test]
fn dec_locator_observes_position_without_any_mouse_mode() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // DECELR only — no DECSET 1000/1002/1003
    assert!(
        !t.mode().intersects(crate::term::TermMode::ANY_MOUSE),
        "no mouse-tracking mode active",
    );
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        20,
        20,
    ));
    feed(&mut t, b"\x1b[1'|");
    let p = last_declrp_params(&listener.events());
    assert_eq!((p[2], p[3]), (21, 21), "observation runs without ANY_MOUSE");
}

/// Regression: BUG-08-058 — re-enabling DEC Locator after a disable must
/// NOT report the stale pre-disable position. DECELR resets the observed
/// position to `Unavailable`; a DECRQLP before a fresh event emits Pe=0.
#[test]
fn dec_locator_decelr_reenable_clears_stale_position() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z"); // DECELR Ps=1
    t.handle_mouse_input(&cell_event(MouseButton::Left, MouseEventKind::Press, 12, 8));
    feed(&mut t, b"\x1b[0;0'z"); // disable
    feed(&mut t, b"\x1b[1;0'z"); // re-enable (resets position)
    feed(&mut t, b"\x1b[1'|"); // DECRQLP — no fresh observation yet
    assert_eq!(
        last_declrp_params(&listener.events()),
        vec![0],
        "re-enabled locator with no fresh event must emit Pe=0, not stale coords",
    );
}

/// Regression: BUG-08-058 — `observe_locator_input` (Step A only) MUST
/// NOT encode a mouse report even when a mouse-tracking mode is active.
/// The App routes here on the shift-to-select bypass (ANY_MOUSE set but
/// suppressed); routing through `handle_mouse_input` instead would fire
/// the encoder and emit a spurious report. Pins observe-only never emits.
#[test]
fn observe_locator_input_does_not_encode_even_with_any_mouse() {
    let (mut t, listener) = term_with_recorder();
    // DECSET 1000 (mouse click reporting) + DECELR Ps=1 (locator) both on.
    feed(&mut t, b"\x1b[?1000h\x1b[1;0'z");
    assert!(t.mode().intersects(crate::term::TermMode::ANY_MOUSE));
    // Observe-only dispatch: records position, never encodes.
    t.observe_locator_input(&cell_event(MouseButton::Left, MouseEventKind::Press, 12, 8));
    assert!(
        listener.events().iter().all(|e| !e.contains("PtyWrite")),
        "observe_locator_input must NOT emit a mouse report, got: {:?}",
        listener.events(),
    );
    // The position WAS recorded — a subsequent DECRQLP reports it.
    feed(&mut t, b"\x1b[1'|");
    let p = last_declrp_params(&listener.events());
    assert_eq!(
        (p[2], p[3]),
        (9, 13),
        "observed Pr/Pc via observe-only path"
    );
}

/// Behavioral guard: locator position state lives on `DecLocatorState`
/// (not bare `Term` fields) and is written by Step A / read by DECRQLP.
#[test]
fn dec_locator_position_state_lives_on_dec_locator() {
    let mut t = term();
    feed(&mut t, b"\x1b[1;0'z");
    assert_eq!(
        t.dec_locator().position(),
        LocatorPosition::Unavailable,
        "default position is Unavailable",
    );
    t.handle_mouse_input(&cell_event(MouseButton::Left, MouseEventKind::Press, 7, 9));
    assert_eq!(
        t.dec_locator().position(),
        LocatorPosition::Known {
            cell: (7, 9),
            pixel: (0, 0),
            buttons: 4,
        },
        "Step A writes Known to DecLocatorState::position",
    );
}

// ── Rejection guards ──────────────────────────────────────────────

/// Rejection guard: a DECLRP after a real click must NOT carry the broken
/// placeholder `Pr=1 Pc=1` — would fail against pre-fix hardcoded coords.
#[test]
fn dec_locator_decrqlp_reply_is_not_placeholder_after_click() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z");
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        10,
        10,
    ));
    feed(&mut t, b"\x1b[1'|");
    let p = last_declrp_params(&listener.events());
    assert_ne!(p[2], 1, "Pr must not be the placeholder 1");
    assert_ne!(p[3], 1, "Pc must not be the placeholder 1");
}

/// Rejection guard: reject the inverted Pm mapping. Left sets bit 2
/// (value 4) and NOT bit 0; Right sets bit 0 (value 1) and NOT bit 2.
#[test]
fn dec_locator_decrqlp_rejects_inverted_pm_mapping() {
    let (mut t, listener) = term_with_recorder();
    feed(&mut t, b"\x1b[1;0'z");
    t.handle_mouse_input(&cell_event(
        MouseButton::Left,
        MouseEventKind::Press,
        10,
        10,
    ));
    feed(&mut t, b"\x1b[1'|");
    let pm_left = last_declrp_params(&listener.events())[1];
    assert_eq!(pm_left & 0b100, 0b100, "Left sets bit 2");
    assert_eq!(
        pm_left & 0b001,
        0,
        "Left does NOT set bit 0 (inverted mapping)"
    );

    let (mut t2, listener2) = term_with_recorder();
    feed(&mut t2, b"\x1b[1;0'z");
    t2.handle_mouse_input(&cell_event(
        MouseButton::Right,
        MouseEventKind::Press,
        10,
        10,
    ));
    feed(&mut t2, b"\x1b[1'|");
    let pm_right = last_declrp_params(&listener2.events())[1];
    assert_eq!(pm_right & 0b001, 0b001, "Right sets bit 0");
    assert_eq!(
        pm_right & 0b100,
        0,
        "Right does NOT set bit 2 (inverted mapping)"
    );
}
