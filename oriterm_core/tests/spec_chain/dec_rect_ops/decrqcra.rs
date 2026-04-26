//! DECRQCRA (CSI Pi;Pg;Pt;Pl;Pb;Pr * y) spec-chain scenarios.
//!
//! Catalog row: `DECRECT-DECRQCRA`
//! Apex: EffectPtyWrite (synchronous — not HostRequest)
//!
//! xterm sum-then-negate algorithm pinned against
//! `~/projects/reference_repos/console_repos/xterm/screen.c:3136`.
//! Reply shape: `DCS Pi ! ~ XXXX ST` (4-digit uppercase hex).

use oriterm_core::effect::{Effect, PtyEffect, PtyWriteKind};
use oriterm_test_support::spec_chain::{SpecHarness, last_pty_write};

/// Canonical DECRQCRA reply over a known-content rectangle.
///
/// A 5×5 grid filled with ASCII `'X'` (0x58) yields
/// `total = 25 * 0x58 = 0x898 = 2200`; default flags negate →
/// `-2200 = 0xFFFFF768` (i32), masked to 16 bits = `0xF768`.
/// Reply format: `DCS Pi ! ~ XXXX ST` with `Pi = 1`.
#[test]
fn decrqcra_5x5_x_fill_matches_xterm_sum_then_negate() {
    let mut h = SpecHarness::with_size(5, 5);
    h.feed(b"XXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX");
    h.feed(b"\x1b[1;1;1;1;5;5*y");

    let (bytes, kind) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(kind, PtyWriteKind::ChecksumReport);
    assert_eq!(bytes, b"\x1bP1!~F768\x1b\\");
}

/// Different `Pi` values round-trip verbatim into the DCS reply prefix.
/// Pins the `format!("\x1bP{id}!~...")` shape against an ID-swap bug.
/// Empty grid → all cells undrawn → checksum `0000` per xterm default
/// mode (`screen.c:3178-3180`).
#[test]
fn decrqcra_id_passthrough_into_reply_prefix() {
    for id in [0u16, 1, 7, 99, 42] {
        let mut h = SpecHarness::with_size(1, 1);
        h.feed(format!("\x1b[{id};1;1;1;1;1*y").as_bytes());
        let (bytes, _kind) = last_pty_write(&h)
            .expect("expected at least one PtyEffect::Write in the harness outcome");
        let expected = format!("\x1bP{id}!~0000\x1b\\");
        assert_eq!(
            bytes,
            expected.into_bytes(),
            "id={id}: reply prefix must echo Pi"
        );
    }
}

/// Out-of-range coordinates clamp to the physical grid (xterm semantics).
/// A request for (1,1)-(999,999) on a 5×5 grid must produce the same
/// checksum as the valid-bounds (1,1)-(5,5) equivalent.
#[test]
fn decrqcra_coordinate_clamping_to_grid_bounds() {
    let mut ref_h = SpecHarness::with_size(5, 5);
    ref_h.feed(b"XXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX");
    ref_h.feed(b"\x1b[1;1;1;1;5;5*y");
    let (ref_bytes, _) = last_pty_write(&ref_h)
        .expect("expected at least one PtyEffect::Write in the harness outcome");

    let mut oob_h = SpecHarness::with_size(5, 5);
    oob_h.feed(b"XXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX");
    oob_h.feed(b"\x1b[1;1;1;1;999;999*y");
    let (oob_bytes, _) = last_pty_write(&oob_h)
        .expect("expected at least one PtyEffect::Write in the harness outcome");

    assert_eq!(
        ref_bytes, oob_bytes,
        "out-of-range rectangle must clamp — checksum identical"
    );
}

/// Zero-area rectangle (top > bot OR left > right after clamping) is
/// a silent no-op: the checksum is 0 per xterm. Negated 0 is still 0.
#[test]
fn decrqcra_zero_area_rectangle_is_noop() {
    let mut h = SpecHarness::with_size(5, 5);
    h.feed(b"XXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX\r\nXXXXX");
    // top=5, bot=1 → after 1-based→0-based top0=4, bot0=0, top0 > bot0.
    h.feed(b"\x1b[1;1;5;1;1;5*y");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP1!~0000\x1b\\");
}

/// `csATTRIBS` (bit 1, value 2) controls whether SGR attributes fold
/// into the sum. When unset (default), a bold cell's checksum differs
/// from a plain cell. When set, the checksums match.
#[test]
fn decrqcra_csattribs_flag_toggles_attribute_folding() {
    // Default flags, bold 'A': attrs included → checksum picks up bold tag.
    let mut bold_h = SpecHarness::with_size(1, 1);
    bold_h.feed(b"\x1b[1mA\x1b[m");
    bold_h.feed(b"\x1b[1;1;1;1;1;1*y");
    let (bold_default, _) = last_pty_write(&bold_h)
        .expect("expected at least one PtyEffect::Write in the harness outcome");

    // Flags=2 (csATTRIBS set): attrs excluded → checksum == plain 'A'.
    let mut excl_h = SpecHarness::with_size(1, 1);
    excl_h.feed(b"\x1b[1mA\x1b[m");
    excl_h.feed(b"\x1b[2#y");
    excl_h.feed(b"\x1b[1;1;1;1;1;1*y");
    let (bold_excl, _) = last_pty_write(&excl_h)
        .expect("expected at least one PtyEffect::Write in the harness outcome");

    let mut plain_h = SpecHarness::with_size(1, 1);
    plain_h.feed(b"A");
    plain_h.feed(b"\x1b[1;1;1;1;1;1*y");
    let (plain, _) = last_pty_write(&plain_h)
        .expect("expected at least one PtyEffect::Write in the harness outcome");

    assert_ne!(
        bold_default, plain,
        "default (attrs included): bold 'A' ≠ plain 'A'"
    );
    assert_eq!(
        bold_excl, plain,
        "csATTRIBS set: bold 'A' == plain 'A' (attrs excluded)"
    );
}

/// Regression: BUG-08-017. Application-written plain spaces with
/// default SGR must count as drawn cells in the DECRQCRA checksum
/// (xterm CHARDRAWN parity). On a 1×3 grid writing "A B":
///   trimmed = 'A'(0x41) + ' '(0x20) + 'B'(0x42) = 0xA3
///   negate  = -0xA3 = 0xFF5D
/// Pre-fix returned 0xFF7D (middle space skipped as "pristine").
#[test]
fn decrqcra_explicit_spaces_match_xterm_ff5d() {
    let mut h = SpecHarness::with_size(1, 3);
    h.feed(b"A B");
    h.feed(b"\x1b[1;1;1;1;1;3*y");
    let (bytes, kind) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(kind, PtyWriteKind::ChecksumReport);
    assert_eq!(
        bytes, b"\x1bP1!~FF5D\x1b\\",
        "plain-space write must contribute to checksum — BUG-08-017 repro"
    );
}

/// Negative pin: pristine cells (never written) MUST NOT contribute.
/// A 3×3 never-written grid stays at checksum 0x0000. If DRAWN were
/// accidentally set on default cells, this would return -(9 × 0x20)
/// = -0x120 = 0xFEE0.
#[test]
fn decrqcra_pristine_grid_is_zero() {
    let mut h = SpecHarness::with_size(3, 3);
    h.feed(b"\x1b[1;1;1;1;3;3*y");
    let (bytes, _) =
        last_pty_write(&h).expect("expected at least one PtyEffect::Write in the harness outcome");
    assert_eq!(bytes, b"\x1bP1!~0000\x1b\\");
}

/// Negative pin: DECRQCRA emits EXACTLY ONE synchronous PTY write and
/// ZERO `HostRequest` effects. Guards the architectural decision to
/// emit the reply synchronously via `PtyEffect::Write` rather than
/// routing through the async `HostRequest` pipeline.
#[test]
fn decrqcra_emits_one_pty_write_zero_host_request() {
    let mut h = SpecHarness::with_size(3, 3);
    h.feed(b"ABC");
    // Pre-count other effects (prior pipelines may have emitted some).
    let before = h.outcome().effects_emitted.len();
    h.feed(b"\x1b[1;1;1;1;3;3*y");
    let after = h.outcome().effects_emitted.len();

    let new_effects: Vec<&Effect> = h.outcome().effects_emitted[before..after].iter().collect();
    let checksum_writes = new_effects
        .iter()
        .filter(|eff| {
            matches!(
                eff,
                Effect::Pty(PtyEffect::Write {
                    kind: PtyWriteKind::ChecksumReport,
                    ..
                })
            )
        })
        .count();
    let host_requests = new_effects
        .iter()
        .filter(|eff| matches!(eff, Effect::HostRequest(_)))
        .count();

    assert_eq!(
        checksum_writes, 1,
        "DECRQCRA must emit exactly one ChecksumReport PTY write"
    );
    assert_eq!(
        host_requests, 0,
        "DECRQCRA must NOT route through HostRequest (synchronous by design)"
    );
}
