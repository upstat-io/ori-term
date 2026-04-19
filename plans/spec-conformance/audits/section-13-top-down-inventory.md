---
section: "13"
title: "Kitty Graphics Protocol"
canonical_spec_sources:
  - "sw.kovidgoyal.net/kitty/graphics-protocol/ — primary protocol documentation (kitty source is the de facto spec for this protocol)"
  - "kitty source kittens/icat/icat.py — cross-reference for client-side transmission patterns"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 13: Kitty Graphics Protocol

## Canonical spec source(s)

The kitty graphics protocol documentation at sw.kovidgoyal.net/kitty/graphics-protocol/ is the authoritative top-down enumerator for kitty graphics coverage. This is an APC-based protocol (`ESC _ G ... ESC \`) where the kitty terminal IS the spec — the public documentation and the kitty source are co-authoritative. Every key-value pair, action (`a=`), format (`f=`), transmission mode (`t=`), chunk flag (`m=`), placement flag (`U=`), and response code defined in the protocol must map to a catalog row or carry an explicit `not-targeted` decision. `kittens/icat/icat.py` is used as a cross-reference for client-side transmission patterns that inform expected server-side behavior.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 13. Walk the canonical spec source(s) row-by-row. Every sequence the spec defines gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
