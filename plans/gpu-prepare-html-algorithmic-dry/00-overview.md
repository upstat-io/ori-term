---
plan: "gpu-prepare-html-algorithmic-dry"
title: "GPU Prepare + HTML Algorithmic DRY — BUG-06-014 Hygiene Fallout"
status: not-started
disposable: true
references:
  - "plans/bug-tracker/fix-BUG-06-014.md"
  - ".claude/rules/impl-hygiene.md"
---

# GPU Prepare + HTML Algorithmic DRY — Overview

## Mission

Eliminate the algorithmic-duplication and wire-protocol drift surfaced by the
implementation-hygiene review of BUG-06-014 (SGR 53/73/74 — overline,
superscript, subscript). The end state is a cohesive cell-emission pipeline
where the per-cell GPU emit step is implemented in **one** canonical helper
(consumed by the shaped, incremental, and unshaped paths alike), where the
spacer-skip predicate has a single canonical home on `CellFlags`, where the
HTML serializer's cell-iteration skeleton lives in **one** shared run-builder
(consumed by both the html-only and html+text dual paths), and where wire
protocol bit-position invariants are pinned by tests so cross-version mux
snapshots cannot silently drop newly-added flags.

This sweep addresses **`oriterm/src/gpu/prepare/`** — where the per-cell emit
body of `fill_frame_shaped`, `fill_frame_incremental`, and the test-only
unshaped `fill_frame` is duplicated three ways, forcing every per-cell
behavioral change (such as BUG-06-014's `glyph_y` shift) to land at three
sync points simultaneously; **`oriterm_core/src/cell/`** + **`oriterm_mux/src/protocol/`**
— where the new OVERLINE / SUPERSCRIPT / SUBSCRIPT bits have no bit-position
pin and the wire codec uses `from_bits_truncate` (silently dropping unknown
bits during cross-version replay); **`oriterm_core/src/selection/html/`** — where
`append_html_cells` and `append_cells_dual` reproduce the same span-coalescing
+ spacer-skip + zerowidth iteration skeleton with only the text-callback
differing; and **`oriterm_core/src/term/handler/status.rs`** — where the
DECRQSS SGR responder emits the new BUG-06-014 codes (SGR 53/73/74) but
omits the older underline-variant codes (SGR 21, 4:3, 4:4, 4:5, colored
underline), an asymmetry exposed by the BUG-06-014 fix.

The standard is `.claude/rules/impl-hygiene.md`.

This is a **disposable cleanup plan**. It exists only to track the substantive
architectural findings from BUG-06-014's hygiene review until they are fixed.
The trivial findings (F-01/F-02 stale TPR annotations, F-13 Vec allocation,
F-14 line-cite, F-17/F-20 missing matrix tests, F-21 `Copy` derive) are being
fixed inline in BUG-06-014's close-out commit and do **not** appear in this
plan. Once every checkbox in every section is complete, the entire
`plans/gpu-prepare-html-algorithmic-dry/` directory must be deleted via the
final cleanup step in `section-05-decrqss-underline-asymmetry.md`.

## Source Review

- **Reviewer**: `/impl-hygiene-review` over BUG-06-014 (SGR 53/73/74)
- **Scope**: `oriterm/src/gpu/prepare/{mod.rs,dirty_skip/mod.rs,unshaped.rs,decorations.rs,tests.rs}`,
  `oriterm_core/src/selection/html/{mod.rs,style.rs,tests.rs}`. Cross-crate
  exhaustiveness against `oriterm_core/src/cell/`, `oriterm_core/src/term/handler/`,
  `oriterm_mux/src/protocol/`, `oriterm/src/gpu/extract/`.
- **Phase 5 report**: 4 LEAK, 2 DRIFT, 0 GAP, 4 WASTE, 0 EXPOSURE, 4 BLOAT,
  5 NOTE (1 withdrawn). 2 Critical + 2 Major findings exceed inline-fix capacity
  and motivate this plan.
- **Anchor**: BUG-06-014's `## Hygiene Findings` block cites this plan path as
  the implementation owner for the substantive findings, satisfying CLAUDE.md
  §"NEVER reason out of TPR findings" / §"ALL Deferrals" requirement.

## Architecture

```
GPU per-cell emit (Section 01 target)
+-- fill_frame_shaped       (oriterm/src/gpu/prepare/mod.rs)         ----+
+-- fill_frame_incremental  (oriterm/src/gpu/prepare/dirty_skip/...) ----+--> emit_cell.rs
+-- fill_frame  (test-only) (oriterm/src/gpu/prepare/unshaped.rs)    ----+    (one canonical helper)

CellFlags wire-protocol pin (Section 03)
+-- CellFlags definition    (oriterm_core/src/cell/mod.rs)
+-- WireCellFlags codec     (oriterm_mux/src/protocol/snapshot.rs)
+-- snapshot conversion     (oriterm/src/gpu/extract/from_snapshot/mod.rs)
+-- new pin tests           (cell/tests.rs + protocol/tests.rs)

Spacer predicate (Section 02 target)
+-- 5 call sites across 2 crates  -------> CellFlags::is_spacer()
                                           (one canonical predicate)

HTML cell-run extraction (Section 04 target)
+-- append_html_cells       (selection/html/mod.rs)  --+
+-- append_cells_dual       (selection/html/mod.rs)  --+--> append_html_run
                                                          (one shared iterator)

DECRQSS SGR query coverage (Section 05)
+-- build_sgr_string        (term/handler/status.rs)
    +-- emits SGR 53/73/74 (BUG-06-014, correct)
    +-- MISSING: SGR 21 (DOUBLE_UNDERLINE), 4:3 / 4:4 / 4:5 (CURLY/DOTTED/DASHED),
        colored underline
```

## Design Principles

**Algorithmic SSOT.** When the same control-flow skeleton appears in 3+ places,
the duplication IS the bug regardless of how short any individual copy is.
BUG-06-014's `glyph_y` shift had to be applied at three sync points
simultaneously — that is the textbook 3-strike Critical-LEAK fingerprint
from `.claude/rules/impl-hygiene.md` §Algorithmic DRY. The fix is a single
canonical helper that all three callers consume.

**Wire-protocol invariants are compile-time + test-time locked.** Bit positions
on `CellFlags` are part of the cross-version mux protocol contract. The
existing exhaustiveness test (`INTERNAL_CELL_STATE`) pins the *set* of flags
but does not pin *bit positions*. Adding a new flag without pinning its bit
position is the silent-drop fingerprint — paired with `from_bits_truncate`
(which discards unknown bits) it produces a cross-version data loss with
no failing test.

**One predicate, one home.** When `flags.intersects(WIDE_CHAR_SPACER |
LEADING_WIDE_CHAR_SPACER)` appears in 5 places across 2 crates, that's not
a coincidence — it's a missing method on `CellFlags`. Adding a new spacer
variant (e.g. emoji ZWJ continuation) requires updating five sites in lock-step;
the canonical home reduces this to one.

## Section Dependency Graph

```
01 emit_cell extraction (Critical LEAK F-03)
      |
      +-- dissolves F-04, F-05, F-06, F-08 as side-effects
      |
      v
02 CellFlags::is_spacer (Critical LEAK F-15)         [independent of 01]
      |
      v
03 CellFlags bit-position + wire-protocol pin (DRIFT F-07 + F-19)  [independent]
      |
      v
04 append_html_run extraction (DRIFT F-16)           [independent]
      |
      v
05 DECRQSS underline-variant coverage (Minor GAP F-10)  [independent]
      |
      v
[Final cleanup: delete plan directory]
```

**All five sections are independent.** They touch separate code paths and have
no cross-section dependencies. Implementation order is by severity:
Section 01 (Critical) and Section 02 (Critical) first, then Section 03 (Major
DRIFT pair), then Section 04 (Major DRIFT), then Section 05 (Minor GAP), then
delete the plan.

**Cross-section interactions:** none. Each section's tests lock its own scope.

## Implementation Sequence

```
Phase 0 - Section 01 (Critical LEAK F-03)
  +-- 01.1 Identify the per-cell emit skeleton across the three call sites
  +-- 01.2 Extract emit_cell.rs (or inline emit_cell helper) consumed by all three
  +-- 01.3 Migrate fill_frame_shaped to call the helper
  +-- 01.4 Migrate fill_frame_incremental to call the helper (dirty-row path only)
  +-- 01.5 Migrate test-only unshaped fill_frame to call the helper
  +-- 01.R Third-party review
  +-- 01.N Build & verify (test count unchanged, BUG-06-014 regression tests green)

Phase 1 - Section 02 (Critical LEAK F-15)
  +-- 02.1 Add CellFlags::is_spacer predicate + sibling test
  +-- 02.2 Migrate 5 call sites across oriterm/src/gpu/prepare and oriterm_core/src/selection
  +-- 02.R Third-party review
  +-- 02.N Build & verify

Phase 2 - Section 03 (DRIFT F-07 + F-19, paired)
  +-- 03.1 Add cell_flags_bit_positions_pin_wire_protocol test in cell/tests.rs
  +-- 03.2 Add wire-protocol roundtrip pin test in protocol/tests.rs (use from_bits, not from_bits_truncate)
  +-- 03.3 Audit from_snapshot conversion path; add pin test
  +-- 03.R Third-party review
  +-- 03.N Build & verify

Phase 3 - Section 04 (DRIFT F-16)
  +-- 04.1 Extract append_html_run shared iterator
  +-- 04.2 Migrate append_html_cells to call append_html_run with no-op text callback
  +-- 04.3 Migrate append_cells_dual to call append_html_run with text-buffer callback
  +-- 04.R Third-party review
  +-- 04.N Build & verify

Phase 4 - Section 05 (Minor GAP F-10)
  +-- 05.1 Extend build_sgr_string to emit SGR 21 (DOUBLE_UNDERLINE)
  +-- 05.2 Extend build_sgr_string to emit SGR 4:3 / 4:4 / 4:5 (CURLY / DOTTED / DASHED)
  +-- 05.3 Extend build_sgr_string to emit colored-underline SGR
  +-- 05.4 Add DECRQSS roundtrip tests for each variant
  +-- 05.R Third-party review
  +-- 05.N Build & verify
  +-- 05.Z Final cleanup: delete plan directory
```

**Why this order:**
- Critical findings (Sections 01 + 02) come first — LEAKs cascade per
  `.claude/rules/impl-hygiene.md`.
- Section 03 (paired DRIFT) is independent but lower severity — runs after
  the structural cleanup so test coverage is added against the already-clean
  structure, not against a moving target.
- Section 04 (DRIFT) is local to `selection/html/` and orthogonal.
- Section 05 (GAP) is the lowest severity and most localized — perfect
  closing item that doubles as the cleanup gate.

**Known failing tests (expected until plan completion):** None. Each section's
verification gate keeps the suite green; this plan does not introduce any
expected-to-fail tests.

## Metrics (Current State)

| Module | LOC of duplicate body | Sync points | Severity |
|--------|----------------------|-------------|----------|
| `gpu/prepare/{mod,dirty_skip/mod,unshaped}.rs` per-cell emit | ~120 | 3 | Critical |
| `is_spacer` predicate (5 sites) | 2 each (10 total) | 5 | Critical |
| `selection/html/mod.rs` cell-run iteration | ~60 | 2 | Major |
| `cell/mod.rs` + `protocol/snapshot.rs` bit-position drift | n/a | 0 pin tests | Major |
| `term/handler/status.rs` DECRQSS underline coverage | n/a | 0 emitters | Minor |

**Target end state:**

| Module | Canonical home | Sync points after |
|--------|---------------|-------------------|
| Per-cell emit | `oriterm/src/gpu/prepare/emit_cell.rs` (or `emit.rs`) | 1 |
| Spacer predicate | `CellFlags::is_spacer()` | 1 |
| HTML cell-run | `oriterm_core/src/selection/html/mod.rs::append_html_run` | 1 |
| Wire-protocol bit positions | `cell/tests.rs` + `protocol/tests.rs` pin | 2 (intentional, paired) |
| DECRQSS SGR coverage | `term/handler/status.rs::build_sgr_string` | 1 |

## Mission Success Criteria

- [ ] Section 01 — `emit_cell` helper extracted; all 3 callers (`fill_frame_shaped`,
      `fill_frame_incremental`, test-only `fill_frame`) consume it; BUG-06-014
      regression tests still green.
- [ ] Section 02 — `CellFlags::is_spacer()` exists; all 5 call sites migrated;
      no remaining `intersects(WIDE_CHAR_SPACER | LEADING_WIDE_CHAR_SPACER)`
      pattern in repo grep.
- [ ] Section 03 — `cell_flags_bit_positions_pin_wire_protocol` test in
      `cell/tests.rs` asserts each SGR-mapped bit's exact position;
      `protocol/tests.rs` roundtrip uses `CellFlags::from_bits` (not
      `from_bits_truncate`) and exercises OVERLINE / SUPERSCRIPT / SUBSCRIPT.
- [ ] Section 04 — `append_html_run` extracted; both `append_html_cells` and
      `append_cells_dual` delegate to it; HTML test count unchanged and green.
- [ ] Section 05 — `build_sgr_string` emits SGR 21 (DOUBLE_UNDERLINE), SGR
      4:3/4:4/4:5 (CURLY/DOTTED/DASHED), and colored-underline; DECRQSS
      roundtrip tests cover each variant.
- [ ] `./build-all.sh` green (debug + release cross-compile).
- [ ] `./clippy-all.sh` green.
- [ ] `./test-all.sh` green.
- [ ] Each section ran `/tpr-review` and converged clean.
- [ ] `plans/gpu-prepare-html-algorithmic-dry/` directory has been deleted via
      the section-05 cleanup step and the deletion is committed + pushed.

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Per-cell emit helper extraction (F-03 → F-04/05/06/08) | `section-01-emit-cell-extraction.md` | Not Started |
| 02 | `CellFlags::is_spacer` predicate (F-15) | `section-02-cellflags-is-spacer.md` | Not Started |
| 03 | Wire-protocol bit-position + roundtrip pins (F-07 + F-19) | `section-03-wire-protocol-pins.md` | Not Started |
| 04 | `append_html_run` shared iterator (F-16) | `section-04-html-run-extraction.md` | Not Started |
| 05 | DECRQSS SGR underline-variant coverage (F-10) | `section-05-decrqss-underline-asymmetry.md` | Not Started |
