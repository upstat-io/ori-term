//! Section 06.5 direct-VTE cap xcheck — status line via OSC title.
//!
//! The `hs`/`dsl`/`tsl`/`fsl` caps are title-backed via
//! `oriterm_core/src/term/handler/osc.rs:22 osc_set_title`. The
//! contract is:
//!   - `tsl=\E]2;` opens an OSC 2 (set title) sequence.
//!   - The title payload is whatever bytes follow.
//!   - `fsl=^G` (BEL, 0x07) terminates the OSC sequence.
//!   - `dsl=\E]2;\007` writes an empty title (clearing the
//!     status line).
//!   - `hs` is a pure-bool advertisement (no escape sequence).

use super::super::test_helpers::{feed, term_with_recorder};
use super::{assert_cap_declared, assert_cap_value_matches};

pub(super) const REGISTERED: &[&str] = &["hs", "dsl", "fsl", "tsl"];

#[test]
fn tack_cap_xcheck_tsl_fsl_round_trip() {
    assert_cap_declared("tsl");
    assert_cap_declared("fsl");
    let (mut term, listener) = term_with_recorder();
    // tsl + payload + fsl == OSC 2 set window title to "test status line"
    feed(&mut term, b"\x1b]2;test status line\x07");
    let events = listener.events();
    assert!(
        events
            .iter()
            .any(|e| e.contains("Title") && e.contains("test status line")),
        "expected Event::Title(\"test status line\") to fire; got {events:?}",
    );
    assert_eq!(
        term.title(),
        "test status line",
        "term.title() must reflect the OSC 2 payload"
    );
}

#[test]
fn tack_cap_xcheck_dsl_clears_title() {
    let (mut term, _l) = term_with_recorder();
    // First set a non-empty title.
    feed(&mut term, b"\x1b]2;initial\x07");
    assert_eq!(term.title(), "initial");
    // Then send the dsl sequence (OSC 2 with empty payload).
    feed(&mut term, b"\x1b]2;\x07");
    assert!(
        term.title().is_empty(),
        "OSC 2 with empty payload (the dsl form) must clear the \
         title; got {:?}",
        term.title(),
    );
}

#[test]
fn tack_cap_xcheck_hs_bool_declared() {
    // hs is a pure-bool advertisement — no escape sequence to feed.
    assert_cap_declared("hs");
}

#[test]
fn tack_cap_xcheck_status_line_cap_values_match_terminfo() {
    // Pin the literal cap declarations so a future terminfo edit
    // that changes the title-backed contract fires here BEFORE
    // the event-firing tests above.
    assert_cap_value_matches("tsl", "\\E]2;");
    assert_cap_value_matches("fsl", "^G");
    assert_cap_value_matches("dsl", "\\E]2;\\007");
}
