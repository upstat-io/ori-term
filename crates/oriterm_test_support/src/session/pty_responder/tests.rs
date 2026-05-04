//! Sibling tests for `pty_responder/mod.rs`.
//!
//! Each test drives a `Term<PtyResponder>` directly via a private VTE
//! processor — no `PtySession`, no real PTY, no tack. The responder is
//! cloned so one handle moves into `Term::new` and the other stays
//! inspectable.

use oriterm_core::effect::sink::EffectSink;
use oriterm_core::effect::{
    ClipboardSelection, Effect, HostEffect, HostRequest, PtyEffect, PtyWriteKind, ResponseToken,
    format_clipboard_reply,
};
use oriterm_core::event::ClipboardType;
use oriterm_core::{Term, Theme};
use vte::ansi::Processor;

use super::PtyResponder;

/// Build a 24x80 `Term` driven by a fresh `PtyResponder`, returning both
/// the term and a clone of the responder that tests can inspect.
fn term_with_responder() -> (Term<PtyResponder>, PtyResponder) {
    let responder = PtyResponder::new();
    let term = Term::new(24, 80, 0, Theme::default(), responder.clone());
    (term, responder)
}

fn feed(term: &mut Term<PtyResponder>, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

#[test]
fn pty_responder_captures_color_request() {
    // OSC 10 ? ST → Term emits `Effect::HostRequest(HostRequest::ColorQuery)`;
    // the responder formats the canonical reply for the pinned test color
    // (`0xabcdef` → `rgb:abab/cdcd/efef`) using the same ESC]…ST terminator
    // the query carried.
    let (mut term, responder) = term_with_responder();
    feed(&mut term, b"\x1b]10;?\x1b\\");

    let osc = responder.take_osc_responses();
    assert_eq!(
        osc,
        vec!["\x1b]10;rgb:abab/cdcd/efef\x1b\\".to_string()],
        "OSC 10 query must round-trip the pinned test color"
    );
    assert!(
        responder.take_responses().is_empty(),
        "ColorQuery must NOT populate the PtyWrite queue"
    );
    assert!(
        responder.take_clipboard_stores().is_empty(),
        "ColorQuery must NOT populate the clipboard-store queue"
    );
}

#[test]
fn pty_responder_captures_color_request_bel_terminated() {
    // Terminator parity: the formatter captures the query's terminator,
    // so a BEL-terminated OSC 10 query must produce a BEL-terminated
    // response.
    let (mut term, responder) = term_with_responder();
    feed(&mut term, b"\x1b]10;?\x07");

    let osc = responder.take_osc_responses();
    assert_eq!(
        osc,
        vec!["\x1b]10;rgb:abab/cdcd/efef\x07".to_string()],
        "BEL-terminated OSC 10 query must produce a BEL-terminated response"
    );
}

#[test]
fn pty_responder_captures_clipboard_load() {
    // OSC 52;c;? ST → Term emits `Effect::HostRequest(HostRequest::ClipboardLoad)`;
    // the responder formats the base64-encoded reply for the pinned
    // clipboard stub. Base64("ori-term-clipboard-stub") is
    // "b3JpLXRlcm0tY2xpcGJvYXJkLXN0dWI=".
    let (mut term, responder) = term_with_responder();
    feed(&mut term, b"\x1b]52;c;?\x1b\\");

    let osc = responder.take_osc_responses();
    assert_eq!(
        osc,
        vec!["\x1b]52;c;b3JpLXRlcm0tY2xpcGJvYXJkLXN0dWI=\x1b\\".to_string()],
        "OSC 52 load must round-trip the pinned clipboard stub as base64"
    );
    assert!(
        responder.take_clipboard_stores().is_empty(),
        "ClipboardLoad must NOT populate the clipboard-store queue"
    );
}

#[test]
fn pty_responder_captures_clipboard_store() {
    // OSC 52;c;<base64> ST → Term decodes and emits
    // `Effect::Host(HostEffect::ClipboardStore)`. Base64("store-me") =
    // "c3RvcmUtbWU=".
    let (mut term, responder) = term_with_responder();
    feed(&mut term, b"\x1b]52;c;c3RvcmUtbWU=\x1b\\");

    let stores = responder.take_clipboard_stores();
    assert_eq!(
        stores,
        vec![(ClipboardType::Clipboard, "store-me".to_string())],
        "OSC 52 store must decode base64 and capture the (type, text) pair"
    );
    assert!(
        responder.take_osc_responses().is_empty(),
        "ClipboardStore must NOT populate the osc_responses queue"
    );
}

/// §10.2 blind-spot #6 remediation — embedded-backend SSOT pin.
///
/// The daemon-path OSC 52 round-trip is pinned in
/// `oriterm_mux/src/pane/io_thread/response_poll/tests.rs::osc52_register_
/// poll_roundtrip`; this sibling test pins the embedded-backend path so
/// `ResponseToken` fulfillment for OSC 52 cannot silently diverge between
/// the two consumers. The responder's reply MUST come from the canonical
/// `format_clipboard_reply` helper — byte-equality against the helper's
/// output rejects any ad-hoc reply formatting.
///
/// Matrix: all three `ClipboardSelection` variants (`c`, `s`, `p`) plus
/// both terminators (ST, BEL) — `q` has no variant and is exercised as a
/// drop in `oriterm_core/tests/spec_chain/osc/clipboard.rs::
/// osc52_store_clipboard_q_dropped`.
#[test]
fn osc52_embedded_backend_fulfills_via_session_pty_responder() {
    const TEST_CLIPBOARD_TEXT: &str = "ori-term-clipboard-stub";

    for (bytes, clipboard_char, terminator) in [
        (&b"\x1b]52;c;?\x1b\\"[..], b'c', "\x1b\\"),
        (&b"\x1b]52;s;?\x1b\\"[..], b's', "\x1b\\"),
        (&b"\x1b]52;p;?\x1b\\"[..], b'p', "\x1b\\"),
        (&b"\x1b]52;c;?\x07"[..], b'c', "\x07"),
    ] {
        let (mut term, responder) = term_with_responder();
        feed(&mut term, bytes);

        let osc = responder.take_osc_responses();
        let expected = String::from_utf8(format_clipboard_reply(
            TEST_CLIPBOARD_TEXT,
            clipboard_char,
            terminator,
        ))
        .expect("format_clipboard_reply output is ASCII");
        assert_eq!(
            osc,
            vec![expected.clone()],
            "OSC 52 load (Pc={}, terminator={:?}) must round-trip via \
             format_clipboard_reply — no ad-hoc formatting at the responder \
             call site. got {osc:?}, want [{expected:?}]",
            clipboard_char as char,
            terminator,
        );
        assert!(
            responder.take_responses().is_empty(),
            "ClipboardLoad must NOT populate the PtyWrite queue"
        );
        assert!(
            responder.take_clipboard_stores().is_empty(),
            "ClipboardLoad must NOT populate the clipboard-store queue"
        );
    }
}

#[test]
fn pty_responder_still_captures_pty_write() {
    // Regression pin for the existing PtyWrite path: the effect-cutover
    // migration must NOT break DA/DSR handling. DA1 (`ESC [ c`) emits a
    // PtyEffect::Write whose contents match ori_term's canonical DA1
    // response.
    let (mut term, responder) = term_with_responder();
    feed(&mut term, b"\x1b[c");

    let writes = responder.take_responses();
    assert_eq!(
        writes.len(),
        1,
        "DA1 must populate exactly one PtyWrite entry"
    );
    assert!(
        writes[0].starts_with("\x1b[?"),
        "DA1 response must start with CSI ? — got {:?}",
        writes[0]
    );
    assert!(
        responder.take_osc_responses().is_empty(),
        "PtyWrite must NOT populate the osc_responses queue"
    );
}

#[test]
fn pty_responder_ignores_non_round_trip_effects() {
 // Regression guard locking in the `_ => {}` catch-all: non-round-trip
    // effect variants must not populate any queue. Future extensions that
    // add a new effect variant cannot silently leak into these queues
    // without this test failing.
    let responder = PtyResponder::new();
    responder.push(Effect::Host(HostEffect::TitleSet {
        value: Some("probe".to_string()),
    }));
    responder.push(Effect::Host(HostEffect::IconNameSet {
        value: Some("probe".to_string()),
    }));
    responder.push(Effect::Host(HostEffect::Bell));
    responder.push(Effect::Host(HostEffect::CwdSet {
        cwd: "/tmp".to_string(),
    }));
    responder.push(Effect::Host(HostEffect::ChildExit { code: 0 }));

    assert!(responder.take_responses().is_empty());
    assert!(responder.take_osc_responses().is_empty());
    assert!(responder.take_clipboard_stores().is_empty());
}

#[test]
fn pty_responder_direct_dispatch_populates_queues() {
    // Direct-dispatch sanity check that does NOT go through Term — this
    // locks in the `handle_*` private methods independently of VTE
    // parsing so a future osc.rs refactor cannot mask a responder-side
    // regression.
    let responder = PtyResponder::new();
    responder.push(Effect::Pty(PtyEffect::Write {
        bytes: b"\x1b[?62;c".to_vec(),
        kind: PtyWriteKind::DeviceAttribute,
    }));
    responder.push(Effect::HostRequest(HostRequest::ColorQuery {
        prefix: "10".into(),
        index: 0,
        terminator: "\x1b\\".into(),
        reply: ResponseToken::new(),
    }));
    responder.push(Effect::HostRequest(HostRequest::ClipboardLoad {
        selection: ClipboardSelection::Select,
        clipboard_char: b's',
        terminator: "\x1b\\".into(),
        reply: ResponseToken::new(),
    }));
    responder.push(Effect::Host(HostEffect::ClipboardStore {
        selection: ClipboardSelection::Clipboard,
        data: "hello".to_string(),
    }));

    assert_eq!(responder.take_responses(), vec!["\x1b[?62;c".to_string()]);
    assert_eq!(
        responder.take_osc_responses(),
        vec![
            "\x1b]10;rgb:abab/cdcd/efef\x1b\\".to_string(),
            "\x1b]52;s;b3JpLXRlcm0tY2xpcGJvYXJkLXN0dWI=\x1b\\".to_string(),
        ],
        "ColorQuery and ClipboardLoad must push into osc_responses in arrival order"
    );
    assert_eq!(
        responder.take_clipboard_stores(),
        vec![(ClipboardType::Clipboard, "hello".to_string())],
    );
}

#[test]
fn pty_responder_clone_shares_queues() {
    // SSOT pin: cloning a responder must produce a handle that sees the
    // same queues — this is the test-harness pattern (one clone moves
    // into Term, the other stays inspectable). A future change that
    // accidentally replaces `Arc<Mutex<_>>` with an owned buffer would
    // fire this test.
    let responder = PtyResponder::new();
    let inspector = responder.clone();

    responder.push(Effect::Pty(PtyEffect::Write {
        bytes: b"via-original".to_vec(),
        kind: PtyWriteKind::Other,
    }));

    assert_eq!(
        inspector.take_responses(),
        vec!["via-original".to_string()],
        "clone must observe writes made through the original handle"
    );
    // Original's queue is now drained via the clone — both views share state.
    assert!(responder.take_responses().is_empty());
}
