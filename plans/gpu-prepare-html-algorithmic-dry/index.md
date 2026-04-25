---
plan: "gpu-prepare-html-algorithmic-dry"
title: "GPU Prepare + HTML Algorithmic DRY — BUG-06-014 Hygiene Fallout"
status: not-started
disposable: true
---

# GPU Prepare + HTML Algorithmic DRY — Index

> **Disposable cleanup plan.** Tracks the substantive architectural findings
> from BUG-06-014's `/impl-hygiene-review` (Phase 5 report in
> `/tmp/impl-hygiene-ori_term-DlOwW1O8/phase-5.json`): 2 Critical LEAKs, 2 Major
> DRIFT pairs, and 1 Minor GAP. Trivial findings (F-01/F-02, F-13, F-14,
> F-17/F-20, F-21) are fixed inline in BUG-06-014's close-out commit and are
> NOT covered here. Once every section is `[x]` complete, delete this plan
> directory via the cleanup step in `section-05-decrqss-underline-asymmetry.md`.

## How to Use

1. Pick the highest-severity unchecked section (Sections 01 + 02 are Critical).
2. Implement via the standard subsection checklist + TDD matrix.
3. Run `/tpr-review` against the section before marking it complete.
4. Run `./test-all.sh` + `./clippy-all.sh` + `./build-all.sh` after each section.
5. Mark the section `[x]` and commit.
6. Repeat until clean, then delete this plan via Section 05's cleanup step.

## Sections

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Per-cell emit helper extraction (F-03 → F-04/05/06/08) | `section-01-emit-cell-extraction.md` | Not Started |
| 02 | `CellFlags::is_spacer` predicate (F-15) | `section-02-cellflags-is-spacer.md` | Not Started |
| 03 | Wire-protocol bit-position + roundtrip pins (F-07 + F-19) | `section-03-wire-protocol-pins.md` | Not Started |
| 04 | `append_html_run` shared iterator (F-16) | `section-04-html-run-extraction.md` | Not Started |
| 05 | DECRQSS SGR underline-variant coverage (F-10) | `section-05-decrqss-underline-asymmetry.md` | Not Started |

## Keyword Clusters by Section

### Section 01: Per-cell emit helper extraction

**File:** `section-01-emit-cell-extraction.md` | **Status:** Not Started

```
LEAK, algorithmic-duplication, F-03, F-04, F-05, F-06, F-08
fill_frame_shaped, fill_frame_incremental, fill_frame (unshaped, test-only)
emit_cell, per-cell emit body, glyph_y shift, super_sub_glyph_offset
oriterm/src/gpu/prepare/mod.rs, dirty_skip/mod.rs, unshaped.rs, emit.rs
SGR 73 superscript, SGR 74 subscript, decoration draw, BLINK alpha
resolve_cell_colors, DecorationContext, bg_w wide-char branch, shaped emission
3-strike threshold, sync points, BUG-06-014 glyph_y three-site fix
```

### Section 02: CellFlags::is_spacer predicate

**File:** `section-02-cellflags-is-spacer.md` | **Status:** Not Started

```
LEAK, duplicated-spacer-skip-predicate, F-15
WIDE_CHAR_SPACER, LEADING_WIDE_CHAR_SPACER, intersects, continue
CellFlags::is_spacer, canonical predicate, Cell::is_spacer alias
oriterm/src/gpu/prepare/{mod.rs, dirty_skip/mod.rs, unshaped.rs}
oriterm_core/src/selection/html/mod.rs (2 sites)
oriterm_core/src/selection/text.rs
5 call sites across 2 crates, missing abstraction, future-spacer-variant scaling
```

### Section 03: Wire-protocol bit-position + roundtrip pins

**File:** `section-03-wire-protocol-pins.md` | **Status:** Not Started

```
DRIFT, missing-wire-protocol-pin, no-cellflags-exhaustiveness-test
F-07, F-19, paired
WireCellFlags, ProtocolCodec, from_bits vs from_bits_truncate, silent flag drop
OVERLINE 1<<16, SUPERSCRIPT 1<<17, SUBSCRIPT 1<<18, bit-position pin
oriterm_core/src/cell/{mod.rs, tests.rs}
oriterm_mux/src/protocol/{snapshot.rs, tests.rs}
oriterm/src/gpu/extract/from_snapshot/mod.rs
INTERNAL_CELL_STATE exhaustiveness, cross-version mux compatibility
cell_flags_bit_positions_pin_wire_protocol, roundtrip pin
```

### Section 04: append_html_run shared iterator

**File:** `section-04-html-run-extraction.md` | **Status:** Not Started

```
DRIFT, duplicated-cell-iteration-in-html, F-16
append_html_cells, append_cells_dual, append_html_run
oriterm_core/src/selection/html/mod.rs
KITTY_PLACEHOLDER skip, spacer skip, CellStyle::from_cell, span coalescing
push_html_escaped, zerowidth append, FnMut text callback
extract_html, extract_html_with_text, html-only path, html+text dual path
2-instance + shared skeleton >5 lines, Algorithmic DRY extraction trigger
```

### Section 05: DECRQSS SGR underline-variant coverage

**File:** `section-05-decrqss-underline-asymmetry.md` | **Status:** Not Started

```
GAP, decrqss-sgr-asymmetry, F-10, pre-existing
build_sgr_string, oriterm_core/src/term/handler/status.rs
DECRQSS, SGR query, ECMA-48, kitty extension
SGR 21 DOUBLE_UNDERLINE, SGR 4:3 CURLY, SGR 4:4 DOTTED, SGR 4:5 DASHED
colored underline, underline color, asymmetric coverage
BUG-06-014 fix exposed gap, Broken Window Policy
```

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | Per-cell emit helper extraction | `section-01-emit-cell-extraction.md` |
| 02 | `CellFlags::is_spacer` predicate | `section-02-cellflags-is-spacer.md` |
| 03 | Wire-protocol bit-position + roundtrip pins | `section-03-wire-protocol-pins.md` |
| 04 | `append_html_run` shared iterator | `section-04-html-run-extraction.md` |
| 05 | DECRQSS SGR underline-variant coverage | `section-05-decrqss-underline-asymmetry.md` |
