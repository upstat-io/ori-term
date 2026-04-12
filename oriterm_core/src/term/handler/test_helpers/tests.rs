//! Sibling tests for the `test_helpers` module.
//!
//! Pin the load-bearing semantics of `RecordingListener` and the
//! `feed` helper so a future refactor that accidentally broke
//! `RecordingListener::send_event` (or `feed`'s processor wiring)
//! is caught here BEFORE the 21+ direct-VTE cap-xcheck tests in
//! `tack_cap_xcheck/` start silently passing on a broken recorder.

use super::{feed, term_with_recorder};

#[test]
fn recording_listener_captures_title_event() {
    // SEMANTIC PIN — feed `\x1b]2;hello\x07` (OSC 2 set window
    // title to "hello") through the VTE parser into a
    // `Term<RecordingListener>` and assert the recorder captured
    // an `Event::Title("hello")` entry. If the recorder stops
    // forwarding events, every Section 06.5 test silently passes
    // because no event would ever land — this in-isolation sanity
    // pin guards against that regression.
    let (mut term, listener) = term_with_recorder();
    feed(&mut term, b"\x1b]2;hello\x07");
    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("Title") && e.contains("hello")),
        "expected RecordingListener to capture an Event::Title(\"hello\"), \
         got events: {events:?}",
    );
}

#[test]
fn recording_listener_starts_with_empty_event_log() {
    // SSOT pin: a freshly constructed RecordingListener has zero
    // recorded events. Pins that `RecordingListener::new()` does
    // not accidentally seed a synthetic event during construction
    // (a regression that would skew event-count assertions in
    // every consumer test).
    let (_term, listener) = term_with_recorder();
    assert!(listener.events().is_empty());
}

#[test]
fn feed_advances_processor_with_printable_bytes() {
    // SEMANTIC PIN — `feed` correctly advances a fresh `Processor`
    // and routes printable bytes into the handler's `input`
    // method. A regression that swapped `feed` to use a stale or
    // unconfigured processor would surface here as the cells
    // remaining empty after the call.
    let (mut term, _listener) = term_with_recorder();
    feed(&mut term, b"hi");
    let grid = term.grid();
    assert_eq!(grid[crate::index::Line(0)][crate::index::Column(0)].ch, 'h');
    assert_eq!(grid[crate::index::Line(0)][crate::index::Column(1)].ch, 'i');
}
