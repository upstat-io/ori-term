---
section: "14"
title: "iTerm2 Inline Images"
canonical_spec_sources:
  - "iterm2.com/documentation-images.html — primary protocol documentation (iTerm2's own docs are canonical for OSC 1337 File=)"
  - "iTerm2 source escape_codes/it2support.sh — cross-reference for parameter enumeration and edge-case behavior"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 14: iTerm2 Inline Images

## Canonical spec source(s)

The iTerm2 image protocol documentation at iterm2.com/documentation-images.html is the authoritative top-down enumerator for iTerm2 inline image coverage. This is an OSC 1337-based protocol (`OSC 1337 ; File= ... ST`) where the iTerm2 terminal IS the spec. Every parameter key (`name=`, `size=`, `width=`, `height=`, `preserveAspectRatio=`, `inline=`), format variant (PNG, JPEG, BMP, GIF), and behavioral mode (inline/download, dimension units px/ch/%) defined in the protocol must map to a catalog row or carry an explicit `not-targeted` decision. `escape_codes/it2support.sh` is used as a secondary cross-reference for parameter enumeration and edge-case behavior not covered by the documentation.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 14. Walk the canonical spec source(s) row-by-row. Every sequence the spec defines gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
