//! Section 06.5 direct-VTE cap xcheck — OSC 52 clipboard.
//!
//! `Ms` is the OSC 52 clipboard cap. `ori_term` routes both store
//! and load via `oriterm_core/src/term/handler/osc.rs:114
//! osc_clipboard_store` and `osc.rs:145 osc_clipboard_load`.
//! Store fires `Effect::Host(HostEffect::ClipboardStore { .. })`
//! with the decoded payload; load fires
//! `Effect::HostRequest(HostRequest::ClipboardLoad { .. })`
//! with a response token the upstream consumer fulfills.

use crate::effect::{ClipboardSelection, Effect, EffectSink, HostEffect, HostRequest};

use super::super::test_helpers::{feed, term_with_effect_sink};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["Ms"];

#[test]
fn tack_cap_xcheck_ms_cap_value_matches() {
    // fix: pin the literal Ms declaration so a
    // terminfo edit that changes the OSC 52 cap value triggers
    // a test failure BEFORE the round-trip tests below.
    assert_cap_value_matches("Ms", "\\E]52;%p1%s;%p2%s\\007");
}

#[test]
fn tack_cap_xcheck_ms_osc_52_store_fires_clipboard_store_effect() {
    assert_cap_declared("Ms");
    let mut term = term_with_effect_sink();
    // OSC 52 ; c ; aGVsbG8= BEL — store "hello" (base64) into the
    // OS clipboard. The decoded payload should appear in the
    // ClipboardStore effect.
    feed(&mut term, b"\x1b]52;c;aGVsbG8=\x07");

    let mut effects = Vec::new();
    term.effect_sink().drain_into(&mut effects);

    let found = effects.iter().any(|e| {
        matches!(
            e,
            Effect::Host(HostEffect::ClipboardStore {
                selection: ClipboardSelection::Clipboard,
                data,
            }) if data == "hello"
        )
    });
    assert!(
        found,
        "OSC 52 store must fire Effect::Host(HostEffect::ClipboardStore \
         {{ selection: Clipboard, data: \"hello\" }}); got {effects:?}",
    );
}

#[test]
fn tack_cap_xcheck_ms_osc_52_load_fires_clipboard_load_effect() {
    let mut term = term_with_effect_sink();
    // OSC 52 ; c ; ? BEL — query clipboard. The handler must fire
    // Effect::HostRequest(HostRequest::ClipboardLoad { .. }) with
    // a response token the consumer fulfills.
    feed(&mut term, b"\x1b]52;c;?\x07");

    let mut effects = Vec::new();
    term.effect_sink().drain_into(&mut effects);

    let found = effects.iter().any(|e| {
        matches!(
            e,
            Effect::HostRequest(HostRequest::ClipboardLoad {
                selection: ClipboardSelection::Clipboard,
                clipboard_char: b'c',
                ..
            })
        )
    });
    assert!(
        found,
        "OSC 52 query must fire Effect::HostRequest(HostRequest::ClipboardLoad \
         {{ selection: Clipboard, clipboard_char: b'c', .. }}); got {effects:?}",
    );
}

#[test]
fn tack_cap_xcheck_ms_osc_52_invalid_base64_does_not_panic() {
    // NEGATIVE PIN — invalid base64 must be a no-op (the handler
    // logs a debug warning and returns), NOT a panic. Catches a
    // regression where a malformed clipboard payload would
    // crash the terminal.
    let mut term = term_with_effect_sink();
    feed(&mut term, b"\x1b]52;c;not_valid_base64!\x07");

    let mut effects = Vec::new();
    term.effect_sink().drain_into(&mut effects);

    // No ClipboardStore effect for invalid input — the handler
    // dropped the request silently after logging.
    let has_store = effects
        .iter()
        .any(|e| matches!(e, Effect::Host(HostEffect::ClipboardStore { .. })));
    assert!(
        !has_store,
        "invalid base64 must not produce a ClipboardStore effect; \
         got {effects:?}",
    );
}
