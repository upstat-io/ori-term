---
section: "18"
title: "Charsets + UAX Policy"
canonical_spec_sources:
  - "ISO 2022 — G0/G1/G2/G3 designation sequences (ESC ( / ) / * / + <final> for single-byte sets; ESC $ <intermediate> <final> for multibyte sets; locking shifts LS2/LS3/LS1R/LS2R/LS3R; single shifts SS2/SS3)"
  - "ISO 8859 parts 1-16 — single-byte upper-half charset designation via ESC - / . / / <final>"
  - "DEC technical manuals — NRCS variant tables (British, German, French, FrenchCanadian, Italian, Dutch, NorwegianDanish, Portuguese, Swedish, Spanish, Finnish, Swiss, JIS Roman, JIS Katakana)"
  - "DEC VT320 / VT420 manuals — DEC Special Graphics, Line Drawing, Technical, Supplemental, Dingbats character sets"
  - "Unicode UAX #9 — Bidirectional Algorithm (bidi policy)"
  - "Unicode UAX #11 — East Asian Width (CJK width, halfwidth/fullwidth, ambiguous-width policy)"
  - "Unicode UAX #29 — Unicode Text Segmentation: Grapheme Cluster Break (grapheme clustering, ZWJ sequences, variation selectors)"
last_walked: null
walked_by: null
---

# Top-Down Spec Audit — Section 18: Charsets + UAX Policy

## Canonical spec source(s)

ISO 2022 is the row-by-row enumerator for charset designation sequences: every `ESC ( <final>` (G0 single-byte), `ESC ) <final>` (G1), `ESC * <final>` (G2), `ESC + <final>` (G3), `ESC $ <final>` (G0 multibyte), and the corresponding locking/single shift sequences map to a catalog row. ISO 8859 parts 1-16 define the single-byte upper-half charsets designated via `ESC - / . / / <final>` to G1/G2/G3. DEC technical manuals enumerate the NRCS variant final bytes and the DEC special character sets. The three Unicode UAX annexes enumerate the policies for bidi ordering, East Asian Width classification, and grapheme cluster boundary detection — each policy decision (or explicit non-implementation) maps to a catalog row or `not-targeted` decision.

## Sequence-to-catalog mapping

| Sequence (canonical form) | Spec source citation | Catalog row ID | Decision |
|---|---|---|---|
| _**TODO: implementer populates this table when picking up Section 18. Walk ISO 2022 designation sequences, ISO 8859 parts 1-16, DEC NRCS + special character sets, and Unicode UAX #9/#11/#29 policies top-down. Every charset designation sequence, shift sequence, and Unicode policy variant gets a row here.**_ | | | |

## Decisions

_**TODO: implementer documents every `not-targeted` decision here with rationale. Charset variants and Unicode policy decisions intentionally excluded from ori_term's coverage need a written justification.**_

## Verification

- [ ] Every row with `Decision: mapped` resolves to a real catalog row that exists in `plans/spec-conformance/catalog/` with the cited row ID.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes with this audit file in scope.
- [ ] No row in the canonical spec source is missing from this table (top-down completeness).
- [ ] `last_walked` date is set; `walked_by` is set.
