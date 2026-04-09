//! Section 06.5 direct-VTE cap xcheck — OSC 52 clipboard.
//!
//! `Ms` is the OSC 52 clipboard cap. `ori_term` routes both store
//! and load via `oriterm_core/src/term/handler/osc.rs:114
//! osc_clipboard_store` and `osc.rs:145 osc_clipboard_load`.
//! Store fires `Event::ClipboardStore(ClipboardType, String)`
//! with the decoded payload; load fires `Event::ClipboardLoad`
//! with a closure that the upstream listener invokes to format
//! the response.

use super::super::test_helpers::{feed, term_with_recorder};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["Ms"];

#[test]
fn tack_cap_xcheck_ms_cap_value_matches() {
    // TPR-06-001 fix: pin the literal Ms declaration so a
    // terminfo edit that changes the OSC 52 cap value triggers
    // a test failure BEFORE the round-trip tests below.
    assert_cap_value_matches("Ms", "\\E]52;%p1%s;%p2%s\\007");
}

#[test]
fn tack_cap_xcheck_ms_osc_52_store_fires_clipboard_store_event() {
    assert_cap_declared("Ms");
    let (mut term, listener) = term_with_recorder();
    // OSC 52 ; c ; aGVsbG8= BEL — store "hello" (base64) into the
    // OS clipboard. The decoded payload should appear in the
    // ClipboardStore event.
    feed(&mut term, b"\x1b]52;c;aGVsbG8=\x07");
    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("ClipboardStore") && e.contains("hello")),
        "OSC 52 store must fire Event::ClipboardStore(.., \"hello\"); \
         got {events:?}",
    );
}

#[test]
fn tack_cap_xcheck_ms_osc_52_load_fires_clipboard_load_event() {
    let (mut term, listener) = term_with_recorder();
    // OSC 52 ; c ; ? BEL — query clipboard. The handler must
    // fire Event::ClipboardLoad with a response-formatter closure.
    feed(&mut term, b"\x1b]52;c;?\x07");
    let events = listener.events();
    assert!(
        events.iter().any(|e| e.contains("ClipboardLoad")),
        "OSC 52 query must fire Event::ClipboardLoad; got {events:?}",
    );
}

#[test]
fn tack_cap_xcheck_ms_osc_52_invalid_base64_does_not_panic() {
    // NEGATIVE PIN — invalid base64 must be a no-op (the handler
    // logs a debug warning and returns), NOT a panic. Catches a
    // regression where a malformed clipboard payload would
    // crash the terminal.
    let (mut term, listener) = term_with_recorder();
    feed(&mut term, b"\x1b]52;c;not_valid_base64!\x07");
    // No ClipboardStore event for invalid input — the handler
    // dropped the request silently after logging.
    let events = listener.events();
    assert!(
        !events.iter().any(|e| e.contains("ClipboardStore")),
        "invalid base64 must not produce a ClipboardStore event; \
         got {events:?}",
    );
}
