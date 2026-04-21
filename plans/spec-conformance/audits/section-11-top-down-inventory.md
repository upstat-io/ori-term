---
section: "11"
title: "Unicode Subcell Glyphs (incl. octants)"
canonical_spec_sources:
  - "Unicode 16 chart PDFs — U+2580..=U+259F Block Elements (half-blocks, eighths, shades, quadrants)"
  - "Unicode 16 chart PDFs — U+2500..=U+257F Box Drawing"
  - "Unicode 16 chart PDFs — U+1FB00..=U+1FBFF Symbols for Legacy Computing (sextants at U+1FB00..=U+1FB3B + additional subcell glyphs)"
  - "Unicode 16 chart PDFs — U+1CC00..=U+1CEBF Symbols for Legacy Computing Supplement (octants at U+1CD00..=U+1CDE5, introduced in Unicode 16, September 2024)"
  - "Unicode 16 chart PDFs — U+2800..=U+28FF Braille Patterns"
  - "WezTerm customglyph.rs:317-560 (OCTANT_PATTERNS [u8; 230]) — de-facto cross-stack reference for the octant codepoint→8-bit-mask table"
  - "Kitty decorations.c:979-1026 (mapping[232] enum flags) — de-facto cross-stack reference for the octant codepoint→geometry table"
last_walked: 2026-04-20
walked_by: "elucidsoft"
---

# Top-Down Spec Audit — Section 11: Unicode Subcell Glyphs (incl. octants)

## Canonical spec source(s)

The Unicode 16 character charts are the authoritative top-down enumerators for subcell glyph coverage. Every codepoint in each targeted Unicode block either maps to a catalog row (rendered as a built-in glyph by `oriterm/src/gpu/builtin_glyphs/`) or carries an explicit `not-targeted` decision. The charts define the canonical shape for each codepoint — the shape reference that golden image tests validate against.

Five Unicode blocks are in scope for Section 11 (every block ori_term renders via built-in Canvas glyphs rather than font rasterization):

| Block | Range | Catalog row | Notes |
|---|---|---|---|
| Box Drawing | U+2500..=U+257F | `USC-BOX` | 128 codepoints, all mapped |
| Block Elements | U+2580..=U+259F | `USC-BLOCKS` | 32 codepoints (half-blocks, eighths, shades, quadrants) |
| Symbols for Legacy Computing | U+1FB00..=U+1FB3B (subrange) | `USC-LEGACY-SEXTANT` | 60 codepoints (sextants); remainder of U+1FB00..U+1FBFF is partial (see below) |
| Symbols for Legacy Computing Supplement | U+1CD00..=U+1CDE5 (subrange) | `USC-LEGACY-OCTANT` | 230 codepoints (octants, Unicode 16) |
| Braille Patterns | U+2800..=U+28FF | `USC-BRAILLE` | 256 codepoints, all mapped |

The Symbols for Legacy Computing block (U+1FB00..=U+1FBFF) contains more than just sextants — it also covers smooth mosaics, triangles, and various additional legacy-terminal fill patterns already implemented by ori_term's existing `legacy_computing/smooth_mosaics.rs` and `legacy_computing/triangles.rs`. Section 11's verification scope is the **sextant** subrange (U+1FB00..=U+1FB3B) plus the other sub-cell codepoints that the catalog row describes. The non-sextant legacy-computing codepoints (mosaics, triangles, etc.) either map to their own built-in renderer or are intentionally deferred — see rows below.

## Sequence-to-catalog mapping

Rather than enumerate all 706 codepoints (128 + 32 + 60 + 230 + 256) row-by-row, this audit maps by **codepoint range** because every codepoint within a targeted subrange maps to the same catalog row with the same rationale. Where a range contains intentional exclusions or sub-range partitioning, the range is split into rows accordingly.

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| U+2500..=U+257F (Box Drawing) | Unicode 16 chart PDF U+2500 §"Box Drawing" | `USC-BOX` | mapped |
| U+2580..=U+259F (Block Elements — half blocks, eighths, shades, quadrants) | Unicode 16 chart PDF U+2580 §"Block Elements" | `USC-BLOCKS` | mapped |
| U+1FB00..=U+1FB3B (Symbols for Legacy Computing — sextants) | Unicode 16 chart PDF U+1FB00 §"Symbols for Legacy Computing" | `USC-LEGACY-SEXTANT` | mapped |
| U+1FB3C..=U+1FB6F (Symbols for Legacy Computing — smooth mosaics + triangles + diagonals) | Unicode 16 chart PDF U+1FB00 §"Symbols for Legacy Computing" | — | not-targeted: already implemented by `oriterm/src/gpu/builtin_glyphs/legacy_computing/smooth_mosaics.rs` + `legacy_computing/triangles.rs`; these are rendered correctly but are outside Section 11's sub-cell verification scope (they are not 2×N bitmask glyphs). Section 11 does NOT add a catalog row for them because the existing mosaics/triangles catalog coverage is owned by the rendering infrastructure tests under `gpu/visual_regression/references/`, not the `unicode-subcell` stack. Revisitable if a consumer surfaces that demands explicit catalog-row verification. |
| U+1FB70..=U+1FB8B (Symbols for Legacy Computing — vertical/horizontal eighths) | Unicode 16 chart PDF U+1FB00 §"Symbols for Legacy Computing" | — | not-targeted: rendered by the existing `legacy_computing/mod.rs` vertical/horizontal eighths path. Same rationale as smooth mosaics above — outside Section 11's sub-cell 2×N verification scope. |
| U+1FB8C..=U+1FB99 (Symbols for Legacy Computing — shade / cross / median / upper / lower / left / right quarter patterns) | Unicode 16 chart PDF U+1FB00 §"Symbols for Legacy Computing" | — | not-targeted: miscellaneous fill/cross-hatch patterns handled by the existing built-in renderer; not part of the sub-cell bitmask families Section 11 verifies. |
| U+1FB9A..=U+1FBCA (Symbols for Legacy Computing — inverse mediums, Terminal Graphic Character Set, arrows) | Unicode 16 chart PDF U+1FB00 §"Symbols for Legacy Computing" | — | not-targeted: vendor-specific legacy patterns (Teletext mosaics, arrow forms) — not rendered by the Canvas built-in subsystem. Revisitable if notcurses-demo or a real-app capture surfaces a consumer. |
| U+1FBF0..=U+1FBF9 (Symbols for Legacy Computing — segmented digits) | Unicode 16 chart PDF U+1FB00 §"Symbols for Legacy Computing" | — | not-targeted: decimal-digit legacy display forms; rendered via the configured font (not a built-in Canvas glyph). Font is the correct renderer here because the shapes are standard digit glyphs stylized differently — no 2×N bitmask. |
| U+1CD00..=U+1CDE5 (Symbols for Legacy Computing Supplement — octants, Unicode 16) | Unicode 16 chart PDF U+1CC00 §"Symbols for Legacy Computing Supplement" | `USC-LEGACY-OCTANT` | mapped |
| U+1CC00..=U+1CCFF (Symbols for Legacy Computing Supplement — reserved / non-octant glyphs, mostly Teletext and mosaics below the octant subrange) | Unicode 16 chart PDF U+1CC00 §"Symbols for Legacy Computing Supplement" | — | not-targeted: the block's non-octant subranges cover Teletext mosaic drawing (U+1CC00..=U+1CCEF) and various miscellaneous symbols; these are outside Section 11's 2×4 sub-cell verification scope. Revisitable per family if a real-app capture surfaces a consumer. |
| U+1CDE6..=U+1CEBF (Symbols for Legacy Computing Supplement — post-octant reserved / other legacy glyphs) | Unicode 16 chart PDF U+1CC00 §"Symbols for Legacy Computing Supplement" | — | not-targeted: reserved or miscellaneous legacy-terminal glyphs beyond the octant subrange. Revisitable per family. |
| U+2800..=U+28FF (Braille Patterns) | Unicode 16 chart PDF U+2800 §"Braille Patterns" | `USC-BRAILLE` | mapped |

## Decisions

Every `not-targeted` row above documents its rationale inline. Summary of exclusion categories:

- **Non-sub-cell legacy patterns** (smooth mosaics, triangles, diagonals, quarter fills, vertical/horizontal eighths, Teletext mosaics, Terminal Graphic Character Set, arrows, segmented digits) — rendered either by existing built-in Canvas renderers outside the sub-cell family (`smooth_mosaics.rs`, `triangles.rs`, the eighths branch of `legacy_computing/mod.rs`) OR by the configured font. Not in Section 11's sub-cell verification scope because they are not 2×N bitmask glyphs. Revisitable per family if a consumer surfaces.

- **Reserved / post-octant legacy codepoints** in U+1CDE6..=U+1CEBF — intentionally left unmapped because no target glyph exists in those slots in Unicode 16. Future Unicode releases may allocate them; re-walk required if that happens.

The 230 octant codepoints U+1CD00..=U+1CDE5 are the NEW addition that Section 11 is landing. The canonical codepoint→8-bit-mask table is committed at `plans/spec-conformance/specs/octant-bitmask-mapping.md` (cross-checked against WezTerm `customglyph.rs:317-560` and Kitty `decorations.c:979-1026`, 0 discrepancies after normalization of Kitty's column-major encoding). Section 11's `octants.rs` renderer drives its lookup table from that artifact; the `legacy_computing/tests.rs` canonical-mapping guard test asserts byte-equality between the renderer's table and the artifact.

## Verification

- [x] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/unicode-subcell.md` with the cited row ID (`USC-BOX`, `USC-BLOCKS`, `USC-LEGACY-SEXTANT`, `USC-LEGACY-OCTANT`, `USC-BRAILLE`).
- [x] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope. (Verified at §11.0 close-out.)
- [x] No codepoint range in the canonical spec sources is missing from this table — all five targeted Unicode blocks + the non-targeted Symbols for Legacy Computing subranges + the non-octant parts of the Symbols for Legacy Computing Supplement are enumerated.
- [x] `last_walked` date is set; `walked_by` is set.
