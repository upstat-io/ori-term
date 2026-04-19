---
section: "11"
title: "Unicode Subcell Glyphs (incl. octants)"
canonical_spec_sources:
  - "Unicode 16 chart PDFs — U+2580–U+259F Block Elements (half-blocks, quadrants)"
  - "Unicode 16 chart PDFs — U+1FB00–U+1FBFF Symbols for Legacy Computing (sextants, additional subcell glyphs)"
  - "Unicode 16 chart PDFs — U+1CC00–U+1CEBF Symbols for Legacy Computing Supplement (octants at U+1CD00–U+1CDE5)"
  - "Unicode 16 chart PDFs — U+2800–U+28FF Braille Patterns"
  - "Wezterm customglyph.rs octant table (`~/projects/reference_repos/console_repos/wezterm/wezterm-gui/src/customglyph.rs:317-559`) and Kitty decorations.c octant remap table (`~/projects/reference_repos/console_repos/kitty/kitty/decorations.c:979-1024`) — de-facto references for the 230-entry octant codepoint→8-bit-mask mapping used cross-stack"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 11: Unicode Subcell Glyphs (incl. octants)

## Canonical spec source(s)

The Unicode 16 character charts are the authoritative top-down enumerators for subcell glyph coverage. Each Unicode block that ori_term renders as a builtin glyph (rather than via font rasterization) maps to a distinct codepoint range. Every codepoint in these ranges must either be catalogued as a targeted subcell glyph or carry an explicit `not-targeted` decision. The charts define the canonical shape for each codepoint — the shape reference that golden image tests validate against.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 11. Walk the canonical spec source(s) row-by-row. Every sequence the spec defines gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Sequences intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
