//! Tests for `HostRequest` / `ResponseToken` / reply formatters.

use super::{AlreadyFulfilled, ResponseToken, format_clipboard_reply, format_color_reply};
use crate::color::Rgb;

#[test]
fn response_token_fulfill_and_take() {
    let token: ResponseToken<String> = ResponseToken::new();
    assert!(!token.is_fulfilled());
    assert!(token.take().is_none());

    token.fulfill("hello".to_owned()).unwrap();
    assert!(token.is_fulfilled());

    let value = token.take();
    assert_eq!(value.as_deref(), Some("hello"));
    assert!(!token.is_fulfilled());
}

#[test]
fn response_token_default_is_unfulfilled() {
    let token: ResponseToken<u32> = ResponseToken::default();
    assert!(!token.is_fulfilled());
}

/// Blind-spot §12: duplicate `fulfill` is a routing bug — returns `Err` and
/// preserves the first-write value rather than silently overwriting.
#[test]
fn response_token_rejects_double_fulfillment() {
    let token: ResponseToken<String> = ResponseToken::new();
    assert_eq!(token.fulfill("first".to_owned()), Ok(()));
    assert_eq!(
        token.fulfill("second".to_owned()),
        Err(AlreadyFulfilled),
        "second fulfill must return Err(AlreadyFulfilled)"
    );
    // First value wins.
    assert_eq!(token.take(), Some("first".to_owned()));
}

#[test]
fn response_token_fulfill_succeeds_once() {
    let token: ResponseToken<u32> = ResponseToken::new();
    assert_eq!(token.fulfill(42), Ok(()));
    // Duplicate rejected (slot is `Fulfilled`).
    assert_eq!(token.fulfill(100), Err(AlreadyFulfilled));
    // take() retrieves the first-write value and transitions the slot to
    // `Consumed` — single-assignment per the token's lifetime, NOT per
    // intervening take.
    assert_eq!(token.take(), Some(42));
    assert!(!token.is_fulfilled(), "Consumed is not Fulfilled");
}

/// Regression: post-take fulfill must reject so a
/// routing bug that double-fulfills after the IO thread already drained
/// the reply surfaces loudly instead of silently overwriting the
/// already-consumed slot.
#[test]
fn response_token_rejects_fulfill_after_take() {
    let token: ResponseToken<u32> = ResponseToken::new();
    assert_eq!(token.fulfill(42), Ok(()));
    assert_eq!(token.take(), Some(42));
    // The slot is now `Consumed`. A second fulfill MUST return
    // AlreadyFulfilled — the token's single-assignment guarantee
    // covers its entire lifetime, not just "until the first take".
    assert_eq!(
        token.fulfill(7),
        Err(AlreadyFulfilled),
        "fulfill after take must reject — single-assignment for the token's lifetime"
    );
    // The slot stays `Consumed` — no value is buffered for a future take.
    assert_eq!(token.take(), None);
}

/// Companion: take() on an unfulfilled token does NOT consume the slot.
/// A legitimate later fulfill() must still succeed (the IO thread polls
/// every tick on a not-yet-fulfilled token; consuming the slot on the
/// poll's empty take would prematurely lock out the consumer).
#[test]
fn response_token_take_on_pending_does_not_consume_slot() {
    let token: ResponseToken<u32> = ResponseToken::new();
    // Empty take() — slot was Pending, returns None.
    assert_eq!(token.take(), None);
    // The legitimate later fulfill MUST still succeed — Pending state
    // survived the empty take.
    assert_eq!(token.fulfill(11), Ok(()));
    assert_eq!(token.take(), Some(11));
}

#[test]
fn response_token_consumer_strong_count_reports_arc_clones() {
    let token: ResponseToken<String> = ResponseToken::new();
    assert_eq!(token.consumer_strong_count(), 1);
    let clone_a = token.clone();
    assert_eq!(token.consumer_strong_count(), 2);
    let clone_b = clone_a.clone();
    assert_eq!(token.consumer_strong_count(), 3);
    drop(clone_b);
    assert_eq!(token.consumer_strong_count(), 2);
    drop(clone_a);
    assert_eq!(token.consumer_strong_count(), 1);
}

#[test]
fn format_clipboard_reply_matches_osc52() {
    let bytes = format_clipboard_reply("hi", b'c', "\x1b\\");
    assert_eq!(bytes, b"\x1b]52;c;aGk=\x1b\\");
}

#[test]
fn format_clipboard_reply_with_bel_terminator() {
    let bytes = format_clipboard_reply("hi", b'p', "\x07");
    assert_eq!(bytes, b"\x1b]52;p;aGk=\x07");
}

#[test]
fn format_color_reply_matches_xparsecolor_doubled_nibble() {
    let bytes = format_color_reply(
        Rgb {
            r: 0xff,
            g: 0,
            b: 0x80,
        },
        "10",
        "\x1b\\",
    );
    assert_eq!(bytes, b"\x1b]10;rgb:ffff/0000/8080\x1b\\");
}

#[test]
fn already_fulfilled_implements_error_trait() {
    let err = AlreadyFulfilled;
    // Display impl carries a diagnostic message.
    let msg = format!("{err}");
    assert!(msg.contains("already fulfilled"));
    // Error trait marker — verify the dyn cast compiles.
    let _: &dyn std::error::Error = &err;
}

/// Effect-cutover §01.4 documentation pin: the `ResponseToken<T>`
/// doc comment MUST explicitly state that the token is process-local
/// (cannot cross IPC) AND name the bug-tracker artifact tracking the
/// daemon-mode follow-up. This pin keeps the `00-overview.md` Path B
/// rationale anchored in source — if a future change strips the
/// process-locality warning from the doc comment, this test surfaces
/// the regression at compile/test time.
#[test]
fn host_request_process_locality_is_documented() {
    let source = include_str!("mod.rs");

    assert!(
        source.contains("Process-local — cannot cross IPC"),
        "ResponseToken<T> doc comment must include the `Process-local — cannot cross IPC` warning"
    );
    assert!(
        source.contains("BUG-11-011"),
        "ResponseToken<T> doc comment must cite the daemon-mode follow-up artifact (BUG-11-011)"
    );
    assert!(
        source.contains("§01.4"),
        "ResponseToken<T> doc comment must point at §01.4 for the deferral rationale"
    );
}
