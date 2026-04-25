---
section: "03"
title: "Wire-protocol bit-position + roundtrip pins (F-07 + F-19)"
status: not-started
reviewed: false
goal: "Pin CellFlags bit positions in `cell/tests.rs`, pin wire-protocol roundtrip in `protocol/tests.rs` using `from_bits` (not `from_bits_truncate`), and pin the `from_snapshot` conversion path so a cross-version mux snapshot cannot silently drop OVERLINE / SUPERSCRIPT / SUBSCRIPT."
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "03.1"
    title: "Bit-position pin in cell/tests.rs"
    status: not-started
  - id: "03.2"
    title: "Wire-protocol roundtrip pin in protocol/tests.rs"
    status: not-started
  - id: "03.3"
    title: "from_snapshot conversion pin"
    status: not-started
  - id: "03.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "03.N"
    title: "Build & Verify"
    status: not-started
---

# Section 03: Wire-protocol bit-position + roundtrip pins

**Goal:** Lock the wire-protocol invariant for `CellFlags` so OVERLINE
(1<<16), SUPERSCRIPT (1<<17), and SUBSCRIPT (1<<18) (and any future flag)
cannot be silently dropped during cross-version mux snapshot replay.
Three pins together: (a) bit positions are asserted in `cell/tests.rs`,
(b) wire-protocol roundtrip uses `CellFlags::from_bits` (not
`from_bits_truncate`) and exercises every SGR-mapped bit, (c) the
`from_snapshot` conversion path in `oriterm/src/gpu/extract/from_snapshot/mod.rs`
is exercised end-to-end.

**Production code path:** `oriterm_mux::server::snapshot` → wire encode
(`oriterm_mux::protocol::snapshot::WireCellFlags`) → wire decode →
`oriterm/src/gpu/extract/from_snapshot::convert_cell`. Any cross-version
mismatch (older client receiving a newer server's snapshot) silently
discards unknown flag bits via `from_bits_truncate`.

**Observable change:** None at runtime under matched-version mux
deployment. Behavior change only if a mismatched-version snapshot is
replayed: today the unknown bits are silently dropped; after Section 03
the test pin documents and locks the bit positions, and the roundtrip
test makes any future widening-without-coordination loud.

**Context:** The Phase 5 hygiene report flagged this as paired DRIFT —
F-07 (missing-wire-protocol-pin) and F-19 (no-cellflags-exhaustiveness-test).
The existing `INTERNAL_CELL_STATE` exhaustiveness test in
`oriterm_core/src/cell/tests.rs:114-133` pins the *set* of flags but does
not pin *bit positions*. Reordering the bitflags definition would not
fail any test today, but would silently break cross-version mux
compatibility.

`oriterm_mux/src/protocol/snapshot.rs:45,60` uses
`CellFlags::from_bits_truncate(wire.flags)` on decode. With OVERLINE /
SUPERSCRIPT / SUBSCRIPT added in BUG-06-014, an older client decoding a
newer server's snapshot drops these flags without diagnostic. This is
the silent-drop fingerprint.

**Reference implementations:**
- **WezTerm** `wezterm-mux-server/src/protocol.rs`: every wire-format
  enum has explicit roundtrip tests with `from_repr` (exhaustive) rather
  than `try_from` (silently rejecting). Same defensive pattern.
- **Alacritty** does not have a remote-mux protocol but applies the same
  rigor in `alacritty_terminal/src/term/cell.rs` exhaustiveness tests.

**Depends on:** None.

---

## 03.1 Bit-position pin in cell/tests.rs

**File(s):** `oriterm_core/src/cell/tests.rs`.

**Context:** A single test asserts the literal bit position of every
SGR-mapped flag. The doc comment explains why: cross-version mux
compatibility requires positions to be stable.

- [ ] Add the pin test (sibling-tests pattern, no `mod tests {}` wrapper):
  ```rust
  /// Wire-protocol invariant: CellFlags bit positions are part of the
  /// cross-version mux protocol contract. `oriterm_mux::protocol::snapshot`
  /// transmits `cell.flags.bits()` as a `u32` and decodes via
  /// `CellFlags::from_bits(...)` on the receiver (see Section 03.2 of
  /// `plans/gpu-prepare-html-algorithmic-dry/section-03-wire-protocol-pins.md`).
  ///
  /// If a bit position is reordered, an older client decoding a newer
  /// server's snapshot silently misinterprets flags. This test pins
  /// every position; any change here MUST be paired with a wire-protocol
  /// version bump in `oriterm_mux::protocol`.
  #[test]
  fn cell_flags_bit_positions_pin_wire_protocol() {
      use super::CellFlags;
      assert_eq!(CellFlags::INVERSE.bits(),                  1 <<  0);
      assert_eq!(CellFlags::BOLD.bits(),                     1 <<  1);
      assert_eq!(CellFlags::ITALIC.bits(),                   1 <<  2);
      // … (every flag, in declaration order, with explicit literal)
      assert_eq!(CellFlags::OVERLINE.bits(),                 1 << 16);
      assert_eq!(CellFlags::SUPERSCRIPT.bits(),              1 << 17);
      assert_eq!(CellFlags::SUBSCRIPT.bits(),                1 << 18);
  }
  ```
- [ ] Walk `oriterm_core/src/cell/mod.rs:18-83` and copy every flag's
      bit position into the assertion list. Do NOT compute the expected
      value from the flag itself — use a literal so a reorder is caught.
- [ ] Verify the test fails when a flag is mutated for testing purposes:
      temporarily reorder OVERLINE/SUPERSCRIPT in `cell/mod.rs`, confirm
      the test fails with a clear message, revert.

---

## 03.2 Wire-protocol roundtrip pin in protocol/tests.rs

**File(s):** `oriterm_mux/src/protocol/tests.rs`,
`oriterm_mux/src/protocol/snapshot.rs`.

**Context:** Two changes:
1. The decode path in `oriterm_mux/src/protocol/snapshot.rs:45,60` must
   switch from `CellFlags::from_bits_truncate(wire.flags)` to
   `CellFlags::from_bits(wire.flags).ok_or(...)?` (or equivalent). This
   is the structural fix — silent flag drop becomes a typed error.
2. The roundtrip test must exercise every SGR-mapped flag.

**Fix approach — 2 options for the decode behavior:**

**(a) `from_bits` returns `Option<CellFlags>` → propagate as decode error**
(recommended — matches WezTerm pattern, makes mismatch loud):

```rust
// snapshot.rs decode
let flags = CellFlags::from_bits(wire.flags)
    .ok_or_else(|| ProtocolError::UnknownCellFlags { bits: wire.flags })?;
```

**Why this is best:** Cross-version mismatch becomes a typed error the
client can log and recover from (e.g. fall back to known bits). Silent
drop is replaced with audited drop.

**Trade-off:** Adds a new `ProtocolError` variant. Worth it.

**(b) Keep `from_bits_truncate` + explicit warn-log on drop**
(alternative — less invasive):

```rust
let raw = wire.flags;
let flags = CellFlags::from_bits_truncate(raw);
if flags.bits() != raw {
    log::warn!(
        "snapshot decode: unknown CellFlags bits dropped: 0x{:x}",
        raw & !flags.bits()
    );
}
```

**Downside:** Drop is still happening; just visible in logs. A test
asserting the warn fires is brittle.

**Recommended path:** Option (a). The whole point of the pin is to
make silent drop impossible.

- [ ] Update `oriterm_mux/src/protocol/snapshot.rs:45` and `:60` (both
      decode call sites) to use `from_bits(...).ok_or(ProtocolError::UnknownCellFlags { bits })`.
- [ ] Add `UnknownCellFlags { bits: u32 }` variant to the protocol error
      enum (wherever `ProtocolError` lives — verify via
      `rg -n "enum ProtocolError" oriterm_mux/src/`).
- [ ] Add the roundtrip pin test in `oriterm_mux/src/protocol/tests.rs`:
  ```rust
  /// Wire-protocol roundtrip pin: every SGR-mapped CellFlags bit
  /// must roundtrip through encode → decode without loss.
  /// Uses `from_bits` (typed) NOT `from_bits_truncate` (silent drop).
  /// Paired with `cell::tests::cell_flags_bit_positions_pin_wire_protocol`
  /// (the bit-position lock).
  #[test]
  fn wire_cell_flags_roundtrip_preserves_every_sgr_bit() {
      for flag in [
          CellFlags::INVERSE, CellFlags::BOLD, CellFlags::ITALIC,
          // ... every SGR-mapped flag
          CellFlags::OVERLINE, CellFlags::SUPERSCRIPT, CellFlags::SUBSCRIPT,
      ] {
          let encoded = WireCellFlags::from(flag);
          let decoded = CellFlags::try_from(encoded).expect("known bits roundtrip");
          assert_eq!(decoded, flag, "{flag:?} did not roundtrip cleanly");
      }
  }

  /// Defensive: a bit pattern with an unknown bit set must produce
  /// a typed error, not a silent drop.
  #[test]
  fn wire_cell_flags_decode_rejects_unknown_bits() {
      let bogus = 1u32 << 30; // not assigned to any CellFlags variant
      let result = decode_cell_flags(bogus);
      assert!(matches!(result, Err(ProtocolError::UnknownCellFlags { .. })));
  }
  ```
- [ ] Confirm the existing snapshot encoder/decoder tests still pass.

---

## 03.3 from_snapshot conversion pin

**File(s):** `oriterm/src/gpu/extract/from_snapshot/tests.rs`,
`oriterm/src/gpu/extract/from_snapshot/mod.rs:74,119`.

**Context:** `from_snapshot::convert_cell` (or equivalent) is the GUI-side
translation from a decoded `WireCell` back into a `Cell` for rendering.
If the wire decode (Section 03.2) is now a typed error, this site must
either propagate the error or use a fallback default. Either way the
new flags must roundtrip end-to-end.

- [ ] Add an end-to-end test:
  ```rust
  #[test]
  fn from_snapshot_preserves_overline_superscript_subscript_through_full_roundtrip() {
      let original = Cell { flags: CellFlags::OVERLINE | CellFlags::SUPERSCRIPT, .. };
      let wire = WireCell::from(original.clone());
      let bytes = wire.encode();
      let decoded_wire = WireCell::decode(&bytes).expect("decode");
      let restored = convert_cell(&decoded_wire).expect("convert");
      assert_eq!(restored.flags, original.flags);
  }
  ```
- [ ] Audit `oriterm/src/gpu/extract/from_snapshot/mod.rs:74,119` for
      any explicit flag handling that filters by name. Confirm OVERLINE,
      SUPERSCRIPT, and SUBSCRIPT are not silently dropped.

---

## 03.R Third Party Review Findings

Track findings from `/tpr-review` runs against Section 03 here. Leave the
block in place even when empty so tooling has a stable anchor.

- None.

Format and rules as documented in `plans/_template/plan.md`.

---

## 03.N Build & Verify

### TDD Matrix

| Test | Pin type | Lock-in target |
|---|---|---|
| `cell_flags_bit_positions_pin_wire_protocol` (`cell/tests.rs`) | semantic | every flag's bit position locked |
| `wire_cell_flags_roundtrip_preserves_every_sgr_bit` (`protocol/tests.rs`) | semantic | encode→decode preserves every flag |
| `wire_cell_flags_decode_rejects_unknown_bits` (`protocol/tests.rs`) | **negative** | unknown bits produce typed error, not silent drop |
| `from_snapshot_preserves_overline_superscript_subscript_through_full_roundtrip` (`from_snapshot/tests.rs`) | semantic | end-to-end pin |

### Completion Checklist

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] `cell/tests.rs` has `cell_flags_bit_positions_pin_wire_protocol`
      with literal bit-position asserts for every CellFlags variant
- [ ] `protocol/snapshot.rs` decode uses `CellFlags::from_bits(...)` (typed)
      NOT `from_bits_truncate(...)` (silent drop)
- [ ] `ProtocolError::UnknownCellFlags { bits }` variant exists and is
      returned on unknown bits
- [ ] `protocol/tests.rs` has the two new roundtrip pins (positive +
      negative)
- [ ] `from_snapshot/tests.rs` has end-to-end roundtrip pin for OVERLINE,
      SUPERSCRIPT, SUBSCRIPT
- [ ] `/tpr-review` against this section returns clean (or all findings
      `[x]` resolved in 03.R)
- [ ] Repo grep `rg -n "from_bits_truncate" oriterm_mux/src/protocol/`
      returns zero hits

**Exit Criteria:** Reordering OVERLINE in `cell/mod.rs` (temporarily) makes
both `cell_flags_bit_positions_pin_wire_protocol` and the roundtrip test
fail with clear messages. Adding a new flag without updating the pin
test forces a test failure. Section 03 is complete.
