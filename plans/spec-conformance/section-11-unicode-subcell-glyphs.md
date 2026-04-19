---
section: "11"
title: "Unicode Subcell Glyphs (incl. octants)"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/unicode-subcell.md` from `implemented-unverified` to `verified`, and ADD the missing octant implementation (U+1CD00–U+1CDE5, Unicode 16) which is currently NOT implemented per Pass 1."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-11-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (Unicode 16 chart PDFs — U+2580-U+259F half-blocks/quadrants, U+1FB00-U+1FBFF Symbols for Legacy Computing, U+1CD00-U+1CDE5 octants, U+2800-U+28FF braille) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "Every row in `catalog/unicode-subcell.md` is `verified`: half blocks (U+2580/U+2584), quadrants (U+2596–U+259F), sextants (U+1FB00–U+1FB3B), **octants (U+1CD00–U+1CDE5 — NEW)**, braille (U+2800–U+28FF)"
  - "Octants implemented: `oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs` exists with the 8-bit bitmask Canvas implementation; every U+1CD00–U+1CDE5 codepoint renders the correct shape per Unicode 16 chart PDF"
  - "Spec_chain golden tests for every subcell glyph family — render a representative codepoint from each family at the canonical 12pt cell, compare against committed PNG with exact-or-tiny tolerance"
  - "All existing visual_regression glyph tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "ori_term existing `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` — sextant pattern (2x3 grid, 6-bit bitmask) is the template for octants (2x4 grid, 8-bit bitmask)"
  - "Unicode 16 chart PDFs — definitive shape reference for every glyph"
depends_on: ["05", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "11.0"
    title: "Top-down spec audit (BLOCKING)"
    status: not-started
  - id: "11.1"
    title: "Implement octants U+1CD00–U+1CDE5"
    status: not-started
  - id: "11.2"
    title: "Spec_chain golden tests for every subcell glyph family"
    status: not-started
  - id: "11.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "11.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 11: Unicode Subcell Glyphs

**Status:** Not Started
**Goal:** Verify every subcell glyph catalog row + add the missing octant implementation. Octants are required by notcurses `keller`/`uniblock` blitter exhaustive tests (section 24 / 21 milestones).

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed half blocks, quadrants, sextants, braille all exist; octants NOT implemented. The existing `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` implements sextants via a 2x3 grid + 6-bit bitmask Canvas pattern. Octants follow the same shape but with a 2x4 grid + 8-bit bitmask for U+1CD00–U+1CDE5 (the Unicode 16 Symbols for Legacy Computing extension).

**Reference implementations:**
- ori_term existing `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` (sextants — template)
- ori_term existing `oriterm/src/gpu/builtin_glyphs/blocks.rs` (half blocks + quadrants)
- ori_term existing `oriterm/src/gpu/builtin_glyphs/braille.rs` (braille — 2x4 dot grid, similar shape to octants)
- Unicode 16 chart PDFs at `plans/spec-conformance/specs/unicode-symbols-legacy.pdf`

**Depends on:** Section 05 (deterministic GPU env for goldens), Section 08 (baseline correct so glyph rendering tests aren't fighting through baseline bugs).

---

## 11.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-11-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA (and the entire DEC private rectangular-ops family) from the catalog. The original Section 01 catalog bootstrap was bottom-up (audit existing dispatch + add tack/teseq-discovered items), which is incomplete by construction — sequences absent from both the catalog AND the test corpus are invisible. The per-section audit file makes top-down coverage mechanically lintable: `spec-coverage-report --check audit-files` fails CI if any audit-file mapping does not resolve to a real catalog row.

**Canonical spec source(s):** Unicode 16 chart PDFs (U+2580-U+259F half-blocks/quadrants, U+1FB00-U+1FBFF Symbols for Legacy Computing, U+1CD00-U+1CDE5 octants, U+2800-U+28FF braille)

**Files touched:**
- `plans/spec-conformance/audits/section-11-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)
- `plans/spec-conformance/catalog/unicode-subcell.md` (open new rows for any sequences that should be `mapped` but aren't catalogued yet — use the canonical schema per `plans/spec-conformance/00-overview.md §Catalog Row Schema`)

**Completion criteria:**

- [ ] Audit file `plans/spec-conformance/audits/section-11-top-down-inventory.md` is populated with every sequence in the canonical spec source(s).
- [ ] Every row in the audit-file table has a `Decision` of `mapped` (cites a catalog row ID) or `not-targeted` (with one-line rationale).
- [ ] Every `mapped` row resolves to a real catalog row that exists in `plans/spec-conformance/catalog/`.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [ ] Audit file `last_walked` frontmatter is set to today's date and `walked_by` to the implementer's handle.
- [ ] Any new catalog rows opened in this subsection use the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema`.

**No other subsection in this section can begin work until §11.0 is complete.** This is a hard gate.

---

## 11.1 Implement octants U+1CD00–U+1CDE5

**File(s):** `oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs` (new), `oriterm/src/gpu/builtin_glyphs/legacy_computing/mod.rs` (extended dispatch), sibling tests

- [ ] Create `oriterm/src/gpu/builtin_glyphs/legacy_computing/octants.rs` following the sextant pattern. The Canvas is a 2x4 grid (2 columns × 4 rows) per cell. Each U+1CD00–U+1CDE5 codepoint maps to an 8-bit bitmask describing which of the 8 sub-cells are filled.
- [ ] Add the dispatch to `legacy_computing/mod.rs` so `is_legacy_computing_glyph(ch)` returns true for U+1CD00..=U+1CDE5 and `render_legacy_computing_glyph(ch, canvas)` routes to the octant renderer.
- [ ] Sibling tests: render a few representative octant codepoints, verify the bitmask matches the Unicode 16 chart.
- [ ] **Validation**: octant codepoints render correct shapes per chart PDF.

---

## 11.2 Spec_chain golden tests for every subcell glyph family

**File(s):** `oriterm_core/tests/spec_chain/glyphs/subcell.rs` (new), goldens in `crates/oriterm_test_support/tests/references/spec_chain/glyphs/`

- [ ] For each glyph family (half blocks, quadrants, sextants, octants, braille), pick a representative codepoint that exercises the family's bitmask (e.g., quadrant `▟` U+259F = 0x0F mask)
- [ ] Spec_chain test that renders the codepoint at canonical 12pt @ 96 DPI cell, captures the golden via `ORITERM_UPDATE_GOLDEN=1`, then asserts reproducibility on subsequent runs
- [ ] Use the deterministic golden lane from section 05
- [ ] Update catalog rows in `catalog/unicode-subcell.md` to `verified`
- [ ] **Validation**: every glyph family golden test passes; back-to-back runs produce 0-pixel diff.

---

## 11.R Third Party Review Findings

- None.

---

## 11.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: glyph family (half/quad/sex/oct/braille) × representative codepoint × golden image apex
- [ ] **Semantic pin**: octant golden tests are the regression guard for the new implementation
- [ ] Octants U+1CD00–U+1CDE5 implemented
- [ ] Every subcell glyph family has a golden test
- [ ] Catalog rows `verified`
- [ ] All existing visual_regression glyph tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 11 status updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every Unicode subcell glyph catalog row is `verified`; octants implemented and golden-tested.
