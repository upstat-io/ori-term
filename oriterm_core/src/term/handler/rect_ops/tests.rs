//! Contract pins for DEC rectangular-area handlers.
//!
//! §09A.5 replaced the XTCHECKSUM and DECRQCRA stub pins with real
//! semantic pins (flag mutation, reply shape, negate-on behavior).
//! The remaining §09A.6/§09A.8 stubs keep their no-op pins so their
//! dispatch routes remain exercised without panic until those
//! subsections replace the bodies.

use crate::effect::{
    Effect, EffectSink, PtyEffect, PtyWriteKind, QueueingEffectSink, VoidEffectSink,
};
use crate::index::Column;
use crate::term::Term;
use crate::theme::Theme;

use super::super::test_helpers::feed;
use super::CS_ATTRIBS;

fn term_24x80() -> Term<VoidEffectSink> {
    Term::new(24, 80, 0, Theme::default(), VoidEffectSink)
}

fn term_1x1() -> Term<QueueingEffectSink> {
    Term::new(1, 1, 0, Theme::default(), QueueingEffectSink::new())
}

/// Drain exactly one `PtyEffect::Write` from `t.effect_sink()` and
/// return its payload. Panics if zero or more than one effect was
/// emitted — the DECRQCRA reply must be exactly one synchronous write.
fn drain_single_pty_write(t: &Term<QueueingEffectSink>) -> (Vec<u8>, PtyWriteKind) {
    let mut effects = Vec::new();
    t.effect_sink().drain_into(&mut effects);
    assert_eq!(
        effects.len(),
        1,
        "expected exactly one effect, got {}: {effects:?}",
        effects.len()
    );
    match effects.pop().unwrap() {
        Effect::Pty(PtyEffect::Write { bytes, kind }) => (bytes, kind),
        other => panic!("expected PtyEffect::Write, got {other:?}"),
    }
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

/// XTCHECKSUM `csPOSITIVE` (bit 1) suppresses the final negation.
/// With a single drawn `'A'` (0x41): default yields `-0x41 → 0xFFBF`;
/// `csPOSITIVE` yields `0x0041`. Differ by exactly the negation step,
/// not by content.
#[test]
fn xtchecksum_cs_positive_suppresses_negate() {
    let mut t = term_1x1();
    feed(&mut t, b"A");
    feed(&mut t, b"\x1b[1;1;1;1;1;1*y");
    let (bytes, kind) = drain_single_pty_write(&t);
    assert_eq!(kind, PtyWriteKind::ChecksumReport);
    assert_eq!(bytes, b"\x1bP1!~FFBF\x1b\\");

    feed(&mut t, b"\x1b[1#y"); // csPOSITIVE
    feed(&mut t, b"\x1b[2;1;1;1;1;1*y");
    let (bytes, _) = drain_single_pty_write(&t);
    assert_eq!(bytes, b"\x1bP2!~0041\x1b\\");
}

/// Canonical DECRQCRA reply: 1×1 grid containing ASCII `'A'` (0x41),
/// default flags. xterm-compatible sum-then-negate yields 0xFFBF.
/// Pinned verbatim against `plans/spec-conformance/section-09a-*.md`
/// "Concrete reply example".
#[test]
fn decrqcra_a_char_matches_xterm_ffbf() {
    let mut t = term_1x1();
    feed(&mut t, b"A");
    feed(&mut t, b"\x1b[3;1;1;1;1;1*y"); // id=3 page=1 rect (1,1)-(1,1)
    let (bytes, kind) = drain_single_pty_write(&t);
    assert_eq!(kind, PtyWriteKind::ChecksumReport);
    assert_eq!(bytes, b"\x1bP3!~FFBF\x1b\\");
}

/// Every xterm video-attribute constant pins to its exact value from
/// `screen.c:3221-3234`. Each SGR adds its tag to `'A'` (0x41) and
/// negates. Matrix pin against typos in ATTR_* constants.
#[test]
fn decrqcra_attribute_constants_match_xterm() {
    // (sgr, xterm attr tag, expected base-0 hex after negate(-(0x41+tag)))
    // The expected values are derived arithmetically, not magic numbers.
    let matrix: &[(&[u8], i32)] = &[
        (b"\x1b[1mA", 0x80), // BOLD
        (b"\x1b[4mA", 0x10), // UNDERLINE
        (b"\x1b[5mA", 0x40), // BLINK
        (b"\x1b[7mA", 0x20), // INVERSE
        (b"\x1b[8mA", 0x08), // HIDDEN (INVISIBLE)
    ];

    for (sgr, tag) in matrix {
        let mut t = term_1x1();
        feed(&mut t, sgr);
        feed(&mut t, b"\x1b[0m"); // clear SGR for cursor template
        feed(&mut t, b"\x1b[1;1;1;1;1;1*y");
        let (bytes, _) = drain_single_pty_write(&t);

        let total: i32 = 0x41 + tag;
        let checksum: u16 = (total.wrapping_neg() & 0xFFFF) as u16;
        let expected = format!("\x1bP1!~{checksum:04X}\x1b\\");
        assert_eq!(
            bytes,
            expected.into_bytes(),
            "SGR {sgr:?}: expected xterm ATTR tag {tag:#04x}"
        );
    }
}

/// Empty 1×1 grid → undrawn cell → skipped in default mode → checksum
/// 0. xterm `screen.c:3178-3180`: non-CHARDRAWN cells `continue` when
/// neither `csNOTRIM` nor `csDRAWN` is set.
#[test]
fn decrqcra_undrawn_cell_skipped_by_default() {
    let mut t = term_1x1();
    feed(&mut t, b"\x1b[1;1;1;1;1;1*y");
    let (bytes, _) = drain_single_pty_write(&t);
    assert_eq!(bytes, b"\x1bP1!~0000\x1b\\");
}

/// `csDRAWN` (bit 3) and `csNOTRIM` (bit 2) both promote undrawn cells
/// to `' '`. With `csNOTRIM | csDRAWN` (12), a 1×3 grid containing just
/// `'A'` folds the two trailing undrawn-as-space cells into `total`
/// without trimming: `0x41 + 0x20 + 0x20 = 0x81 → negate → 0xFF7F`.
#[test]
fn decrqcra_csnotrim_csdrawn_promotes_undrawn_to_space() {
    let mut t = Term::new(1, 3, 0, Theme::default(), QueueingEffectSink::new());
    feed(&mut t, b"A");
    feed(&mut t, b"\x1b[12#y"); // csNOTRIM | csDRAWN
    feed(&mut t, b"\x1b[1;1;1;1;1;3*y");
    let (bytes, _) = drain_single_pty_write(&t);
    assert_eq!(bytes, b"\x1bP1!~FF7F\x1b\\");
}

/// Direct call into `compute_rect_checksum` for combining marks —
/// exercises xterm's wide-char fold at `screen.c:3245-3251` without
/// depending on VTE combining-mark parser plumbing. When `csNOTRIM` is
/// set, the final checksum is `total` (which includes combining marks).
#[test]
fn compute_rect_checksum_folds_combining_marks_in_notrim_mode() {
    use crate::cell::{Cell, CellExtra, CellFlags};
    use std::sync::Arc;
    let mut t = Term::new(1, 1, 0, Theme::default(), VoidEffectSink);
    // Install a cell with 'a' + combining acute accent (U+0301) at (0,0).
    // DRAWN set so the checksum treats this as an application-written
    // cell per xterm CHARDRAWN semantics (BUG-08-17).
    let mut extra = CellExtra::new();
    extra.zerowidth.push('\u{0301}');
    let cell = Cell {
        ch: 'a',
        flags: CellFlags::DRAWN,
        extra: Some(Arc::new(extra)),
        ..Cell::default()
    };
    t.grid_mut()[crate::index::Line(0)][Column(0)] = cell;

    // csNOTRIM | csBYTE unset → fold combining, total (not trimmed).
    t.xtchecksum_impl(0x04); // csNOTRIM
    // total = 0x61 ('a') + 0x301 = 0x362. negate → -0x362 = 0xFC9E.
    assert_eq!(t.compute_rect_checksum(1, 1, 1, 1), 0xFC9E);

    // Default flags → trim takes over; combining marks are absorbed
    // into `total` per xterm but the returned value comes from
    // `trimmed` (just the base char) — xterm's FIXME at screen.c:3244.
    t.xtchecksum_impl(0);
    // trimmed = 0x61; negate → -0x61 = 0xFF9F.
    assert_eq!(t.compute_rect_checksum(1, 1, 1, 1), 0xFF9F);
}

/// Trim state (`first` / `embedded`) must persist across row boundaries
/// per xterm `screen.c:3166-3257`. Under `csDRAWN` (bit 3) mode, every
/// undrawn cell is substituted with `' '` and `trim` stays active, so
/// the `first` flag is the sole discriminator for whether each row's
/// leading synthetic-space cell pushes into `trimmed` or gets dropped.
///
/// On a pristine 2×3 grid:
///   - Hoisted (correct, xterm parity): row 0 col 0 has `first=true`,
///     contributes `' '` to `trimmed`. End-of-row reset sets
///     `first=false` (matching `screen.c:3256`). Row 1 col 0 has
///     `first=false` → synthetic space drops. Total `trimmed = 0x20`,
///     negate → `0xFFE0`.
///   - Buggy (per-row reset): each row's col 0 has `first=true` again,
///     contributing `' '` twice. Total `trimmed = 0x40`, negate →
///     `0xFFC0`.
///
/// The test asserts the hoisted value (`0xFFE0`) so any regression to
/// per-row reset trips it. Regression pin for round-1 F1 + round-2 F2.
#[test]
fn compute_rect_checksum_trim_state_crosses_rows() {
    let mut t = Term::new(2, 3, 0, Theme::default(), VoidEffectSink);
    // All cells pristine (undrawn). csDRAWN (bit 3) substitutes every
    // undrawn cell with ' '; trim still active, so only `first` gates
    // whether the synthetic space pushes into trimmed.
    t.xtchecksum_impl(0x08); // csDRAWN without csNOTRIM
    assert_eq!(t.compute_rect_checksum(1, 1, 2, 3), 0xFFE0);
}

/// Wide-char spacer cells (xterm's DRAWX_MASK analog) push into
/// `trimmed` even when `ch == ' '`. Regression pin for round-1 F2.
/// Construct a 1×2 grid with `'A'` at col 0 + WIDE_CHAR_SPACER at
/// col 1 (ch=' ', flags=WIDE_CHAR_SPACER); confirm the spacer is
/// NOT trimmed as a trailing blank.
#[test]
fn compute_rect_checksum_wide_char_spacer_not_trimmed() {
    use crate::cell::{Cell, CellFlags};
    let mut t = Term::new(1, 2, 0, Theme::default(), VoidEffectSink);
    // Both cells are application-written: 'A' with DRAWN, and a
    // wide-char spacer (structurally drawn) with DRAWN (BUG-08-17).
    t.grid_mut()[crate::index::Line(0)][Column(0)] = Cell {
        ch: 'A',
        flags: CellFlags::DRAWN,
        ..Cell::default()
    };
    t.grid_mut()[crate::index::Line(0)][Column(1)] = Cell {
        ch: ' ',
        flags: CellFlags::WIDE_CHAR_SPACER | CellFlags::DRAWN,
        ..Cell::default()
    };
    // Exclude attribs so the spacer contributes only ' ' (0x20), not
    // a SGR-tagged constant. `drawn` pushes it into trimmed:
    // 0x41 + 0x20 = 0x61; negate → -0x61 = 0xFF9F.
    t.xtchecksum_impl(CS_ATTRIBS);
    assert_eq!(t.compute_rect_checksum(1, 1, 1, 2), 0xFF9F);
}

/// DECRQCRA emits exactly one `PtyEffect::Write { kind: ChecksumReport }`
/// — no `HostRequest`, no pipeline round-trip. Negative pin for the
/// synchronous-emission design decision in §09A.5.
#[test]
fn decrqcra_emits_synchronous_pty_write_only() {
    let mut t = term_1x1();
    feed(&mut t, b"\x1b[1;1;1;1;1;1*y");
    let (_bytes, kind) = drain_single_pty_write(&t);
    assert_eq!(kind, PtyWriteKind::ChecksumReport);
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
