//! Sibling tests for `pty_responder/mod.rs`.
//!
//! Each test drives a `Term<PtyResponder>` directly via a private VTE
//! processor — no `PtySession`, no real PTY, no tack. The responder is
//! cloned so one handle moves into `Term::new` and the other stays
//! inspectable.

use std::sync::Arc;

use oriterm_core::effect::LegacyEventSink;
use oriterm_core::event::ClipboardType;
use oriterm_core::{Event, EventListener, Term, Theme};
use vte::ansi::Processor;

use super::PtyResponder;

/// Build a 24x80 `Term` driven by a fresh `PtyResponder`, returning both
/// the term and a clone of the responder that tests can inspect.
fn term_with_responder() -> (Term<LegacyEventSink<PtyResponder>>, PtyResponder) {
    let responder = PtyResponder::new();
    let term = Term::new(
        24,
        80,
        0,
        Theme::default(),
        LegacyEventSink::new(responder.clone()),
    );
    (term, responder)
}

fn feed(term: &mut Term<LegacyEventSink<PtyResponder>>, bytes: &[u8]) {
    let mut processor: Processor = Processor::new();
    processor.advance(term, bytes);
}

#[test]
fn pty_responder_captures_color_request() {
    // OSC 10 ? ST → Term fires ColorRequest(10, formatter); the responder
    // calls formatter(TEST_COLOR) and pushes the result into osc_responses.
    // The canonical `0xabcdef` test color renders as `rgb:abab/cdcd/efef`
    // using the same ESC]...ST terminator the query carried.
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
        "ColorRequest must NOT populate the PtyWrite queue"
    );
    assert!(
        responder.take_clipboard_stores().is_empty(),
        "ColorRequest must NOT populate the clipboard-store queue"
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
    // OSC 52;c;? ST → Term fires ClipboardLoad(Clipboard, formatter); the
    // responder calls formatter(TEST_CLIPBOARD_TEXT) and pushes the base64
    // response into osc_responses. Base64("ori-term-clipboard-stub") is
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
    // OSC 52;c;<base64> ST → Term decodes and fires ClipboardStore.
    // Base64("store-me") = "c3RvcmUtbWU=".
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

#[test]
fn pty_responder_still_captures_pty_write() {
    // Regression pin for the existing PtyWrite path: the 06.0.c extension
    // must NOT break DA/DSR handling. DA1 (`ESC [ c`) fires a PtyWrite
    // whose contents match ori_term's canonical DA1 response.
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
fn pty_responder_ignores_non_round_trip_events() {
    // Negative pin locking in the `_ => {}` catch-all: non-round-trip
    // event variants must not populate any queue. Future extensions that
    // add a new Event variant cannot silently leak into these queues
    // without this test failing.
    let responder = PtyResponder::new();
    responder.send_event(Event::Title("probe".to_string()));
    responder.send_event(Event::ResetTitle);
    responder.send_event(Event::IconName("probe".to_string()));
    responder.send_event(Event::ResetIconName);
    responder.send_event(Event::Wakeup);
    responder.send_event(Event::Bell);
    responder.send_event(Event::Cwd("/tmp".to_string()));
    responder.send_event(Event::CursorBlinkingChange);
    responder.send_event(Event::MouseCursorDirty);
    responder.send_event(Event::ChildExit(0));

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
    responder.send_event(Event::PtyWrite("\x1b[?62;c".to_string()));
    responder.send_event(Event::ColorRequest(
        10,
        Arc::new(|rgb| format!("color:{:02x}{:02x}{:02x}", rgb.r, rgb.g, rgb.b)),
    ));
    responder.send_event(Event::ClipboardLoad(
        ClipboardType::Selection,
        Arc::new(|text| format!("load:{text}")),
    ));
    responder.send_event(Event::ClipboardStore(
        ClipboardType::Clipboard,
        "hello".to_string(),
    ));

    assert_eq!(responder.take_responses(), vec!["\x1b[?62;c".to_string()]);
    assert_eq!(
        responder.take_osc_responses(),
        vec![
            "color:abcdef".to_string(),
            "load:ori-term-clipboard-stub".to_string(),
        ],
        "ColorRequest and ClipboardLoad must push into osc_responses in arrival order"
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

    responder.send_event(Event::PtyWrite("via-original".to_string()));

    assert_eq!(
        inspector.take_responses(),
        vec!["via-original".to_string()],
        "clone must observe writes made through the original handle"
    );
    // Original's queue is now drained via the clone — both views share state.
    assert!(responder.take_responses().is_empty());
}
