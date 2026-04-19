---
section: "20"
title: "Audio + Print"
canonical_spec_sources:
  - "DEC technical manual — DECPS §ESC [ Vol Note Tones p (DEC play sound)"
  - "IBM PC MS-DOS 6.22 Reference — ANSI.SYS §CSI M (ANSI music MML-like notation)"
  - "ECMA-48 (5th ed.) §8.3.7 BEL — audible signal"
  - "ECMA-48 (5th ed.) §8.3.91 MC (CSI i) — media copy / printer functions"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 20: Audio + Print

## Canonical spec source(s)

This section covers three sub-stacks with separate spec sources. The DEC technical manual §DECPS defines the DEC play-sound sequence (volume/note/duration). The IBM PC ANSI.SYS chapter §CSI M defines the ANSI music notation (MML-like note encoding). ECMA-48 §8.3.7 defines BEL semantics; ECMA-48 §8.3.91 defines MC (media copy / CSI i) printer functions. The DEC visual bell (DECVB, mode 12) appears in the DEC private modes table in xterm ctlseqs.txt — its sequence is `CSI ? 12 h/l`, which section 09 owns for mode-setting, but the audio emission from that mode is in scope here.

Each sub-stack has its own mapping table below.

## Sequence-to-catalog mapping — Audio (DECPS + BEL + DECVB)

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 20. Walk DEC technical manual §DECPS and ECMA-48 §8.3.7 BEL. Every audio-related sequence (DECPS ESC [ Vol Note Tones p, BEL \x07, DECVB mode trigger) gets a row here.**_ | | | |

## Sequence-to-catalog mapping — ANSI music (CSI M)

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 20. Walk IBM PC ANSI.SYS §CSI M. The CSI M sequence with MML-like music notation gets a row here.**_ | | | |

## Sequence-to-catalog mapping — Print / media copy (CSI i)

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 20. Walk ECMA-48 §8.3.91 MC and xterm ctlseqs.txt CSI i variants. Every print/media-copy sequence (CSI i print screen, auto print mode, print form, print extent, Zmodem/Kermit passthrough) gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
