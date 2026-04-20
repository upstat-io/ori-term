---
section: "26"
title: "Historical Vector Stacks (ReGIS + Tek 4010/4014 + shared vector_raster helper)"
canonical_spec_sources:
  - "DEC EK-VT250-RM — VT200 Series Technical Manual, Appendix A (ReGIS Reference)"
  - "Tektronix 4014 Computer Display Terminal Operator's Manual — Appendix B (byte-pair coordinate format, graphics/alpha mode switching)"
  - "xterm source — graphics/regis.c (implementation cross-reference for xterm-observable ReGIS subset)"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 26: Historical Vector Stacks

## Canonical spec source(s)

This section covers two sub-stacks with separate canonical manuals. The DEC EK-VT250-RM Appendix A is the row-by-row enumerator for ReGIS commands (screen commands S, position commands P, vector commands V, arc/circle commands C, write commands W, text commands T, macro definitions @, rubber banding R). The Tektronix 4014 Operator's Manual Appendix B defines the byte-pair coordinate format, draw/move modes, and alpha/graphics mode switching. xterm `graphics/regis.c` is a cross-reference for the xterm-observable subset of ReGIS but is NOT the primary enumerator — the DEC manual takes precedence.

Note: Section 26 owns the vector-graphics rows of `catalog/historical.md`. Section 19 owns the legacy-control rows of the same file. The audit tables below are scoped to vector-graphics sequences only.

## Sequence-to-catalog mapping — ReGIS

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 26. Walk EK-VT250-RM Appendix A row-by-row. Every ReGIS command (S, P, V, C, W, T, @, R, L, F, and sub-parameters) gets a row here.**_ | | | |

## Sequence-to-catalog mapping — Tektronix 4010/4014

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 26. Walk Tektronix 4014 Operator's Manual Appendix B row-by-row. Every Tek sequence (byte-pair coordinate encoding, GS graphics mode, US alpha mode, ESC FF clear screen, ESC ETB end-of-text, pen state encoding) gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
