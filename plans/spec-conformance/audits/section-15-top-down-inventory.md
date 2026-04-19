---
section: "15"
title: "Cell-Level Alpha + Transparency"
canonical_spec_sources:
  - "notcurses source — NCALPHA_OPAQUE, NCALPHA_TRANSPARENT, NCALPHA_BLEND, NCALPHA_HIGHCONTRAST semantics (notcurses defines the per-cell alpha contract; no formal spec exists)"
  - "wezterm cell.rs cell color resolution — cross-reference for per-cell alpha implementation patterns"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 15: Cell-Level Alpha + Transparency

## Canonical spec source(s)

notcurses is the authoritative source for per-cell alpha semantics: it defines four alpha modes (`NCALPHA_OPAQUE`, `NCALPHA_TRANSPARENT`, `NCALPHA_BLEND`, `NCALPHA_HIGHCONTRAST`) that govern how each cell's foreground and background colors composite against the underlying plane. There is no formal spec (RFC, ISO, or DEC standard) — notcurses' source and documentation are the canonical enumerator. wezterm's cell color resolution is a secondary cross-reference for how an existing terminal implements per-cell alpha in a production renderer.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 15. Walk the notcurses NCALPHA_* modes and wezterm cell color resolution path row-by-row. Every alpha mode and compositing variant gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Alpha modes intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
