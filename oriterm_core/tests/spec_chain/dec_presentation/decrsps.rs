//! DECRSPS (DCS Ps $ t Pt ST) spec-chain scenarios.
//!
//! Catalog row: `DECPRES-DECRSPS`
//! Apex: StateSnapshot (parse-and-acknowledge stub — full state restoration
//! not implemented)
//!
//! §09A.9 provides a parse-and-acknowledge stub: the dispatcher routes the
//! sequence to `Term::status_decrsps`, which logs the arrival without
//! mutating terminal state and without emitting a reply. DECRSPS has no
//! acknowledgement per xterm ctlseqs.txt — the host infers completion from
//! the absence of an error response.

use oriterm_core::index::{Column, Line};
use oriterm_test_support::spec_chain::{SpecHarness, pty_writes};

#[test]
fn decrsps_ps1_emits_no_pty_write() {
    let mut h = SpecHarness::with_size(24, 80);
    // DCS 1 $ t <payload> ST — Ps=1 DECCIR cursor-info restore.
    h.feed(b"\x1bP1$tsome-payload\x1b\\");
    let writes: Vec<_> = pty_writes(&h).collect();
    assert!(
        writes.is_empty(),
        "DECRSPS stub must not emit any PTY reply: {:?}",
        writes
    );
}

#[test]
fn decrsps_ps2_emits_no_pty_write() {
    let mut h = SpecHarness::with_size(24, 80);
    // DCS 2 $ t <payload> ST — Ps=2 DECTABSR tab-stop restore.
    h.feed(b"\x1bP2$t1/9/17/25\x1b\\");
    assert_eq!(pty_writes(&h).count(), 0);
}

#[test]
fn decrsps_does_not_mutate_cursor_position() {
    let mut h = SpecHarness::with_size(24, 80);
    // Plant the cursor at (line 5, col 10) 1-based (= Line(4), Column(9)).
    h.feed(b"\x1b[5;10H");
    let before_line = h.term().grid().cursor().line();
    let before_col = h.term().grid().cursor().col();
    // Feed a DECRSPS Ps=1 payload that, if wired, would be expected to move
    // the cursor. The stub must keep the cursor exactly where it was.
    h.feed(b"\x1bP1$t1;1;1;@;@;@;0;0;@;BB\x1b\\");
    assert_eq!(h.term().grid().cursor().line(), before_line);
    assert_eq!(h.term().grid().cursor().col(), before_col);
    assert_eq!(before_line, Line(4).0 as usize);
    assert_eq!(before_col, Column(9));
}

#[test]
fn decrsps_empty_payload_is_accepted() {
    let mut h = SpecHarness::with_size(24, 80);
    // Degenerate form — Ps present, Pt empty.
    h.feed(b"\x1bP1$t\x1b\\");
    assert_eq!(pty_writes(&h).count(), 0);
}

#[test]
fn decrsps_default_ps_is_accepted() {
    let mut h = SpecHarness::with_size(24, 80);
    // No Ps present — defaults to 0 inside the dispatcher; stub logs and
    // ignores.
    h.feed(b"\x1bP$tdata\x1b\\");
    assert_eq!(pty_writes(&h).count(), 0);
}
