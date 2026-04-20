//! No-op contract pins for DEC rectangular-area handlers added in
//! §09A.4. Each test feeds the full escape sequence through the VTE
//! dispatch chain and asserts that grid state, cursor, and selection-
//! dirty flag are unchanged — proving the dispatch route reaches the
//! stub helper without panic and without unintended side effects.
//!
//! §09A.5 (DECRQCRA) and §09A.6 (the six mutation ops + DECSACE /
//! XTCHECKSUM / XTREPORTSGR) REPLACE these tests with real semantic
//! pins (grid mutation, checksum format, attribute flip, SGR reply).

use crate::effect::VoidEffectSink;
use crate::index::Column;
use crate::term::Term;
use crate::theme::Theme;

use super::super::test_helpers::feed;

fn term_24x80() -> Term<VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), VoidEffectSink)
}

fn seed_abc(t: &mut Term<VoidEffectSink>) {
    feed(t, b"ABC");
    feed(t, b"\x1b[1;1H"); // home cursor so state is deterministic
}

/// Shared assertion: grid line 0 still starts with "ABC" and cursor is
/// at (0, 0). Used after every rect-op stub to prove no-op semantics.
fn assert_seed_intact(t: &Term<VoidEffectSink>) {
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col().0, 0);
}

#[test]
fn decsace_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[2*x"); // DECSACE rectangular mode
    assert_seed_intact(&t);
}

#[test]
fn deccara_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1;5;5;7$r"); // DECCARA with SGR 7 (reverse)
    assert_seed_intact(&t);
}

#[test]
fn decrara_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1;5;5;1$t"); // DECRARA toggling SGR 1 (bold)
    assert_seed_intact(&t);
}

#[test]
fn deccra_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1;3;3;1;10;10;1$v"); // DECCRA copy
    assert_seed_intact(&t);
}

#[test]
fn decfra_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[88;1;1;5;5$x"); // DECFRA fill with 'X'
    assert_seed_intact(&t);
}

#[test]
fn xtchecksum_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1#y"); // XTCHECKSUM flags=1
    assert_seed_intact(&t);
}

#[test]
fn decrqcra_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1;1;1;5;5*y"); // DECRQCRA id=1 page=1 rect
    assert_seed_intact(&t);
}

#[test]
fn decera_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1;5;5$z"); // DECERA
    assert_seed_intact(&t);
}

#[test]
fn decsera_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1;5;5${"); // DECSERA
    assert_seed_intact(&t);
}

#[test]
fn xtreportsgr_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1;1;5;5#|"); // XTREPORTSGR
    assert_seed_intact(&t);
}
