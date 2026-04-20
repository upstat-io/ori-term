---
section: "12"
title: "Sixel"
canonical_spec_sources:
  - "DEC STD 070 §5 — Sixel Color Extension (primary; defines DCS q introducer, P1-P6 raster attributes, color operators)"
  - "DEC STD 070 §6 — Sixel Graphics Extension (primary; defines sixel data byte encoding, CR/NL, repeat operator)"
  - "libsixel src/decoder.c — reference implementation cross-reference for parsing edge cases"
  - "wezterm term/src/terminalstate/sixel.rs — production cross-reference for HLS rotation, raster attrs, transparency"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 12: Sixel

## Canonical spec source(s)

DEC STD 070 §5 and §6 are the authoritative top-down enumerators for sixel coverage. §5 defines the color extension (DCS q introducer, P1-P6 raster attributes, `#` color define/select operator) and §6 defines the sixel graphics extension (data byte encoding, `!` repeat, CR/NL line control). Every operator, parameter, and encoding variant defined in DEC STD 070 must map to a catalog row or carry an explicit `not-targeted` decision. libsixel and wezterm are secondary cross-references used to resolve ambiguities in the primary spec.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 12. Walk the canonical spec source(s) row-by-row. Every sequence the spec defines gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
