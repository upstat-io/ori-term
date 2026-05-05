//! OSC 52 clipboard spec_chain coverage (Section 10.2).
//!
//! Drives `OSC 52 ; Pc ; <payload> ST|BEL` through the high-level VTE
//! `Processor` and asserts the correct apex effect:
//!
//! - `Pc` ∈ {`c`, `s`, `p`} + base64 payload → `Effect::Host(HostEffect::
//! ClipboardStore { selection, data })` (fire-and-forget store).
//! - `Pc` ∈ {`c`, `s`, `p`} + `?` payload → `Effect::HostRequest(HostRequest::
//! ClipboardLoad { selection, clipboard_char, terminator, reply })` (load
//! request carrying a `ResponseToken` the consumer fulfills).
//! - Unknown `Pc` (e.g. `q`) and invalid base64 are silently dropped; no
//! effect is emitted.
//!
//! The `ResponseToken` round-trip to `PtyEffect::Write` is verified in
//! `oriterm_mux/src/pane/io_thread/response_poll/tests.rs` (`osc52_register_
//! poll_roundtrip` + siblings) — out of scope for spec_chain, which stops at
//! the `HostRequest` apex per §10.2 scope clarification G.
//!
//! Dispatch: `crates/vte/src/ansi/dispatch/osc.rs` `b"52"` arm.
//! Handler: `oriterm_core/src/term/handler/osc.rs::osc_clipboard_{store,load}`.
//!
//! Catalog rows: OSC-52-STORE, OSC-52-LOAD

use oriterm_core::effect::{ClipboardSelection, Effect, HostEffect, HostRequest};
use oriterm_test_support::spec_chain::SpecHarness;

/// Locate the single `ClipboardStore` effect on the transcript and
/// return `(selection, data)`. Panics if there is not exactly one.
fn expect_clipboard_store(harness: &SpecHarness) -> (ClipboardSelection, String) {
    let effects = &harness.outcome().effects_emitted;
    let mut found = None;
    for eff in effects {
        if let Effect::Host(HostEffect::ClipboardStore { selection, data }) = eff {
            assert!(found.is_none(), "more than one ClipboardStore emitted");
            found = Some((*selection, data.clone()));
        }
    }
    found.unwrap_or_else(|| panic!("expected one ClipboardStore; got {effects:?}"))
}

/// Locate the single `ClipboardLoad` request on the transcript and return
/// its non-token fields. Panics if there is not exactly one.
fn expect_clipboard_load(harness: &SpecHarness) -> (ClipboardSelection, u8, String) {
    let effects = &harness.outcome().effects_emitted;
    let mut found = None;
    for eff in effects {
        if let Effect::HostRequest(HostRequest::ClipboardLoad {
            selection,
            clipboard_char,
            terminator,
            ..
        }) = eff
        {
            assert!(found.is_none(), "more than one ClipboardLoad emitted");
            found = Some((*selection, *clipboard_char, terminator.clone()));
        }
    }
    found.unwrap_or_else(|| panic!("expected one ClipboardLoad; got {effects:?}"))
}

/// Assert no `ClipboardStore` and no `ClipboardLoad` was emitted.
fn assert_no_clipboard_effect(harness: &SpecHarness) {
    for eff in &harness.outcome().effects_emitted {
        match eff {
            Effect::Host(HostEffect::ClipboardStore { .. }) => {
                panic!("unexpected ClipboardStore on transcript: {eff:?}");
            }
            Effect::HostRequest(HostRequest::ClipboardLoad { .. }) => {
                panic!("unexpected ClipboardLoad on transcript: {eff:?}");
            }
            _ => {}
        }
    }
}

// ── Store path (OSC 52 ; Pc ; <base64> ST) ──────────────────────────

/// `OSC 52 ; c ; base64("Hello") ST` emits `HostEffect::ClipboardStore`
/// against `ClipboardSelection::Clipboard` with the decoded UTF-8 payload
/// and NOT a `HostRequest::ClipboardLoad`.
#[test]
fn osc52_store_clipboard_c() {
    let mut harness = SpecHarness::new();
    // base64("Hello") = "SGVsbG8="
    harness.feed(b"\x1b]52;c;SGVsbG8=\x1b\\");

    let (selection, data) = expect_clipboard_store(&harness);
    assert_eq!(selection, ClipboardSelection::Clipboard);
    assert_eq!(data, "Hello");

    // No ClipboardLoad — this is a store, not a query.
    for eff in &harness.outcome().effects_emitted {
        if let Effect::HostRequest(HostRequest::ClipboardLoad { .. }) = eff {
            panic!("store path must not emit ClipboardLoad: {eff:?}");
        }
    }
}

/// `Pc = s` routes to `ClipboardSelection::Select` (NOT `Selection` — the
/// enum variant is spelled `Select` at `oriterm_core/src/effect/families/
/// host.rs:116`).
#[test]
fn osc52_store_clipboard_s() {
    let mut harness = SpecHarness::new();
    // base64("selected") = "c2VsZWN0ZWQ="
    harness.feed(b"\x1b]52;s;c2VsZWN0ZWQ=\x1b\\");

    let (selection, data) = expect_clipboard_store(&harness);
    assert_eq!(selection, ClipboardSelection::Select);
    assert_eq!(data, "selected");
}

/// `Pc = p` routes to `ClipboardSelection::Primary`.
#[test]
fn osc52_store_clipboard_p() {
    let mut harness = SpecHarness::new();
    // base64("primary") = "cHJpbWFyeQ=="
    harness.feed(b"\x1b]52;p;cHJpbWFyeQ==\x1b\\");

    let (selection, data) = expect_clipboard_store(&harness);
    assert_eq!(selection, ClipboardSelection::Primary);
    assert_eq!(data, "primary");
}

/// Regression guard: `Pc = q` has no `ClipboardSelection` variant, so the
/// handler silently drops the payload without emitting any effect.
///
/// `ClipboardSelection` at `oriterm_core/src/effect/families/host.rs:110-117`
/// has only `Clipboard`, `Primary`, `Select` — `q` is unsupported. This
/// rejects the §10.2 success-criteria wording that mentioned `q` as a
/// supported character.
#[test]
fn osc52_store_clipboard_q_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]52;q;SGVsbG8=\x1b\\");

    assert_no_clipboard_effect(&harness);
}

/// Regression guard: the store path rejects invalid base64 silently.
///
/// `osc_clipboard_store` (`oriterm_core/src/term/handler/osc.rs:115-121`)
/// uses `Base64.decode` and returns without emitting when decoding fails.
/// No `HostEffect::ClipboardStore` reaches the transcript.
#[test]
fn osc52_store_invalid_base64_dropped() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]52;c;!!!invalid-base64!!!\x1b\\");

    assert_no_clipboard_effect(&harness);
}

// ── Load path (OSC 52 ; Pc ; ? ST) ──────────────────────────────────

/// `OSC 52 ; c ; ? ST` emits `HostRequest::ClipboardLoad` with the
/// `ClipboardSelection`, raw clipboard character, and ST terminator
/// preserved for the reply.
#[test]
fn osc52_load_request_fires_hostrequest() {
    let mut harness = SpecHarness::new();
    harness.feed(b"\x1b]52;c;?\x1b\\");

    let (selection, clipboard_char, terminator) = expect_clipboard_load(&harness);
    assert_eq!(selection, ClipboardSelection::Clipboard);
    assert_eq!(clipboard_char, b'c');
    assert_eq!(terminator, "\x1b\\");

    // No ClipboardStore — the `?` payload is a query, not a store.
    for eff in &harness.outcome().effects_emitted {
        if let Effect::Host(HostEffect::ClipboardStore { .. }) = eff {
            panic!("load path must not emit ClipboardStore: {eff:?}");
        }
    }
}

/// Matrix pin: load with `Pc = s` and `Pc = p` carries the correct
/// `ClipboardSelection` variant so a future dispatch refactor cannot
/// silently collapse the three selection targets.
#[test]
fn osc52_load_with_s_and_p_selections() {
    // `s` selection, BEL-terminated to pin the BEL reply terminator path.
    {
        let mut harness = SpecHarness::new();
        harness.feed(b"\x1b]52;s;?\x07");
        let (selection, clipboard_char, terminator) = expect_clipboard_load(&harness);
        assert_eq!(selection, ClipboardSelection::Select);
        assert_eq!(clipboard_char, b's');
        assert_eq!(terminator, "\x07");
    }

    // `p` selection, ST-terminated.
    {
        let mut harness = SpecHarness::new();
        harness.feed(b"\x1b]52;p;?\x1b\\");
        let (selection, clipboard_char, terminator) = expect_clipboard_load(&harness);
        assert_eq!(selection, ClipboardSelection::Primary);
        assert_eq!(clipboard_char, b'p');
        assert_eq!(terminator, "\x1b\\");
    }
}
