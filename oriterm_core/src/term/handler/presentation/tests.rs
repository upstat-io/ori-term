//! No-op contract pins for DEC presentation / column / index handlers
//! added in §09A.4. Each test feeds the full escape sequence through
//! the VTE dispatch chain and asserts that grid state and cursor are
//! unchanged — proving the dispatch route reaches the stub helper
//! without panic and without unintended side effects.
//!
//! §09A.7 (DECIC / DECDC / DECBI / DECFI), §09A.8 (CSI-path
//! presentation queries), and §09A.9 (DCS-path DECRQSS / DECRSPS)
//! REPLACE these tests with real semantic pins (column shift, index
//! scroll, PSR reply format, conformance-level mutation).

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
/// at (0, 0). Used after every presentation-op stub to prove no-op
/// semantics.
fn assert_seed_intact(t: &Term<VoidEffectSink>) {
    let row = &t.grid()[crate::index::Line(0)];
    assert_eq!(row[Column(0)].ch, 'A');
    assert_eq!(row[Column(1)].ch, 'B');
    assert_eq!(row[Column(2)].ch, 'C');
    assert_eq!(t.grid().cursor().line(), 0);
    assert_eq!(t.grid().cursor().col().0, 0);
}

#[test]
fn decrqpsr_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1$w"); // DECRQPSR mode=1 (cursor info)
    assert_seed_intact(&t);
}

#[test]
fn decrqupss_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[&u"); // DECRQUPSS
    assert_seed_intact(&t);
}

#[test]
fn decrqde_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[\"v"); // DECRQDE
    assert_seed_intact(&t);
}

#[test]
fn decscl_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[64;1\"p"); // DECSCL level=64 (VT400) c1=1 (7-bit)
    assert_seed_intact(&t);
}

#[test]
fn decsca_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[1\"q"); // DECSCA protected=1
    assert_seed_intact(&t);
}

#[test]
fn decsasd_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[0$}"); // DECSASD target=0 (main display)
    assert_seed_intact(&t);
}

#[test]
fn decssdt_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[0$~"); // DECSSDT line_type=0 (none)
    assert_seed_intact(&t);
}

#[test]
fn decic_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[3'}"); // DECIC count=3
    assert_seed_intact(&t);
}

#[test]
fn decdc_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b[3'~"); // DECDC count=3
    assert_seed_intact(&t);
}

#[test]
fn decbi_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b6"); // DECBI (ESC 6)
    assert_seed_intact(&t);
}

#[test]
fn decfi_stub_is_noop() {
    let mut t = term_24x80();
    seed_abc(&mut t);
    feed(&mut t, b"\x1b9"); // DECFI (ESC 9)
    assert_seed_intact(&t);
}
