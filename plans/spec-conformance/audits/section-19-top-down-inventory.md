---
section: "19"
title: "Historical Legacy Control Stacks (VT52, DEC LK201, Wyse 50/60, ADM-3A, IBM PC ANSI.SYS, Microsoft Console VT)"
canonical_spec_sources:
  - "DEC EK-VT52-RM — VT52 Video Terminal User Guide"
  - "DEC EK-VT100-TM — VT100 User Guide (Technical Manual)"
  - "DEC EK-VT220-RM — VT220 Programmer Reference Manual"
  - "DEC EK-VT320-RM — VT320 Programmer Reference Manual"
  - "DEC EK-VT420-RM — VT420 Programmer Reference Manual"
  - "DEC EK-VT520-RM — VT520/VT525 Video Terminal Programmer Information"
  - "DEC LK201 Keyboard Technical Reference Manual"
  - "Wyse Technology WY-50 Reference Manual"
  - "Wyse Technology WY-60 Reference Manual"
  - "Lear-Siegler ADM-3A Operator's Manual"
  - "IBM PC MS-DOS 6.22 Reference — ANSI.SYS chapter"
  - "Microsoft Documentation — console-virtual-terminal-sequences.md (MS Console VT)"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 19: Historical Legacy Control Stacks

## Canonical spec source(s)

This section covers six distinct legacy-control stacks, each with its own authoritative manual. The VT52 manual defines the minimal DEC pre-ANSI escape set; the VT100–VT520 series progressively extended it (but the bulk of VT100+ is subsumed by Sections 08/09; this audit focuses on the VT52-specific sequences). The LK201 keyboard manual defines the physical key-report byte sequences that VT220/320/420/520 terminals expected. Wyse 50/60 manuals define the proprietary attribute-byte, protected-mode, status-line, and function-key-programming extensions. The ADM-3A manual defines the ESC = cursor-addressing idiom. The DOS 6.22 ANSI.SYS chapter defines the IBM PC CSI extensions (keyboard reassignment, SCO save/restore). The Microsoft Console VT doc defines the exact subset of xterm-compatible sequences Microsoft guarantees on Windows.

Each sub-stack has its own mapping table below because their sequence spaces are disjoint.

## Sequence-to-catalog mapping — VT52

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 19. Walk EK-VT52-RM row-by-row. Every VT52 escape sequence gets a row here.**_ | | | |

## Sequence-to-catalog mapping — VT100/VT220/VT320/VT420/VT520 (legacy-specific subset)

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table for VT100–VT520 sequences that are NOT already catalogued by Sections 08/09. Walk EK-VT100-TM, EK-VT220-RM, EK-VT320-RM, EK-VT420-RM, EK-VT520-RM for VT-series-specific sequences beyond ECMA-48 baseline.**_ | | | |

## Sequence-to-catalog mapping — DEC LK201 keyboard

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 19. Walk DEC LK201 Keyboard Technical Reference Manual. Every key-report byte sequence gets a row here.**_ | | | |

## Sequence-to-catalog mapping — Wyse 50/60

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 19. Walk WY-50/WY-60 Reference Manuals row-by-row. Every Wyse-specific sequence gets a row here.**_ | | | |

## Sequence-to-catalog mapping — ADM-3A

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 19. Walk ADM-3A Operator's Manual. Every ADM-3A-specific sequence (focus on ESC = cursor addressing; C0 overlap rows cite existing handlers) gets a row here.**_ | | | |

## Sequence-to-catalog mapping — IBM PC ANSI.SYS

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 19. Walk DOS 6.22 ANSI.SYS chapter row-by-row. Every PC ANSI.SYS extension (keyboard reassignment CSI p, SCO save/restore cursor, etc.) gets a row here.**_ | | | |

## Sequence-to-catalog mapping — Microsoft Console VT

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 19. Walk console-virtual-terminal-sequences.md row-by-row. Every Microsoft-documented sequence gets a row here; most will resolve to existing catalog rows (xterm-compatible overlap), a few will be MS-specific rows.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
