//! Section 06.5 direct-VTE cap xcheck — focus event markers.
//!
//! `XF` is a bool cap advertising focus-event support. `kxIN` and
//! `kxOUT` are OUTBOUND focus-in / focus-out byte sequences
//! emitted by `oriterm/src/app/event_loop_helpers/mod.rs:153
//! send_focus_event` in response to winit `WindowEvent::Focused`.
//!
//! kxIN/kxOUT are GENUINELY cross-crate (the winit dependency
//! anchors the emission site in `oriterm`, not `oriterm_core`).
//! This module carries documentation-only stub tests for those
//! two caps; the real assertion lives in
//! `oriterm/src/app/event_loop_helpers/tests.rs::focus_event_tests`.
//! The stubs exist so the registry sweep does not flag the caps
//! as missing — Section 06.5's `NON_TACK_CAP_XCHECK_CAPS` and
//! this submodule's `REGISTERED` slice MUST stay in sync.

use super::assert_cap_declared;

pub(super) const REGISTERED: &[&str] = &["XF", "kxIN", "kxOUT"];

#[test]
fn tack_cap_xcheck_xf_bool_declared() {
    // XF is a pure bool cap with no escape sequence — it's a
    // terminfo advertisement that the terminal sends focus events
    // when the application enables them via DECSET 1004. The
    // cap-declaration check is the entire test surface for bool
    // caps.
    assert_cap_declared("XF");
}

#[test]
fn tack_cap_xcheck_kxin_declared_real_test_in_oriterm() {
    // STUB — kxIN is OUTBOUND bytes (`\E[I`) emitted by
    // `oriterm/src/app/event_loop_helpers/mod.rs:153 send_focus_event`
    // when winit reports `WindowEvent::Focused(true)` AND the
    // terminal has `TermMode::FOCUS_IN_OUT` set. The byte
    // emission requires a winit focus event path that lives in
    // the `oriterm` crate, NOT `oriterm_core`, so the real
    // assertion lives in
    // `oriterm/src/app/event_loop_helpers/tests.rs`.
    //
    // This stub exists so the registry sweep does not flag kxIN
    // as missing; the cap-declaration check is the only thing
    // testable from inside `oriterm_core`.
    assert_cap_declared("kxIN");
}

#[test]
fn tack_cap_xcheck_kxout_declared_real_test_in_oriterm() {
    // STUB — same rationale as kxIN. Real test lives in
    // `oriterm/src/app/event_loop_helpers/tests.rs`. kxOUT is
    // emitted as `\E[O` when winit reports
    // `WindowEvent::Focused(false)`.
    assert_cap_declared("kxOUT");
}
