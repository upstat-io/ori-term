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
    // Duplicate rejected.
    assert_eq!(token.fulfill(100), Err(AlreadyFulfilled));
    // take() retrieves the first-write value; afterwards fulfill is allowed
    // again because the slot is empty.
    assert_eq!(token.take(), Some(42));
    assert_eq!(token.fulfill(7), Ok(()));
    assert_eq!(token.take(), Some(7));
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
