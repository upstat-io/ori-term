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

/// Pins: DECRSPS Ps=1 (DECCIR cursor-info restore) emits ZERO PTY writes —
/// per xterm ctlseqs the host infers completion from absence-of-error, so
/// the stub must stay silent even though the payload is accepted.
/// Anchor: catalog row `DECPRES-DECRSPS` (Ps=1 silent-accept).
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

/// Pins: DECRSPS Ps=2 (DECTABSR tab-stop restore) also silently accepts —
/// matrix companion to the Ps=1 pin covering the second recognized Ps so a
/// future regression that routes only one Ps to the silent path is caught.
/// Anchor: catalog row `DECPRES-DECRSPS` (Ps=2 silent-accept).
#[test]
fn decrsps_ps2_emits_no_pty_write() {
    let mut h = SpecHarness::with_size(24, 80);
    // DCS 2 $ t <payload> ST — Ps=2 DECTABSR tab-stop restore.
    h.feed(b"\x1bP2$t1/9/17/25\x1b\\");
    assert_eq!(pty_writes(&h).count(), 0);
}

/// Regression guard: a DECRSPS Ps=1 payload that a full implementation would
/// parse as "move cursor" must NOT mutate cursor row or column — the stub
/// logs and drops. Guards against accidental cursor-mutation if someone
/// wires the parse path before completing the full state restoration.
/// Anchor: catalog row `DECPRES-DECRSPS` (parse-stub cursor-immutable).
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

/// Pins: DECRSPS with Ps present but empty Pt is accepted silently — the
/// parser must not panic or reply on the degenerate form.
/// Anchor: catalog row `DECPRES-DECRSPS` (empty-Pt degenerate form).
#[test]
fn decrsps_empty_payload_is_accepted() {
    let mut h = SpecHarness::with_size(24, 80);
    // Degenerate form — Ps present, Pt empty.
    h.feed(b"\x1bP1$t\x1b\\");
    assert_eq!(pty_writes(&h).count(), 0);
}

/// Pins: DECRSPS with omitted Ps defaults to 0 inside the dispatcher and
/// the stub still silently accepts — proves the default-parameter path
/// reaches the same silent-drop handler as explicit Ps=1/Ps=2.
/// Anchor: catalog row `DECPRES-DECRSPS` (default-Ps branch).
#[test]
fn decrsps_default_ps_is_accepted() {
    let mut h = SpecHarness::with_size(24, 80);
    // No Ps present — defaults to 0 inside the dispatcher; stub logs and
    // ignores.
    h.feed(b"\x1bP$tdata\x1b\\");
    assert_eq!(pty_writes(&h).count(), 0);
}
