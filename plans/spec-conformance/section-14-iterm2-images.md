---
section: "14"
title: "iTerm2 Inline Images"
status: not-started
reviewed: false
goal: "Drive every iTerm2 inline image catalog row in `catalog/iterm2.md` from `implemented-unverified` to `verified` — full OSC 1337 File= protocol including base64, inline/download, GIF frame extraction, and the iTerm2 OSC suite extensions."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-14-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (iterm2.com/documentation-images.html (primary, iTerm2's own docs are canonical for OSC 1337 File=) + iTerm2 source `escape_codes/it2support.sh` cross-reference) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "Every image-related row in `catalog/iterm2.md` is `verified`"
  - "OSC 1337 File= base64 transmission verified for PNG, JPEG, BMP formats"
  - "Inline vs download mode verified — `inline=1` causes the image to render at the cursor; `inline=0` (download) causes the image to be saved to a host-side download (not rendered inline)"
  - "Image dimension parameters verified: `width=N`, `height=N`, `width=Npx`, `width=Nch`, `preserveAspectRatio=1`"
  - "GIF frame extraction verified — multi-frame GIFs are decoded into per-frame placements (each frame may animate independently per kitty animation pattern from section 13)"
  - "iTerm2 + image lifecycle verified (depends on section 07)"
  - "All existing teseq iTerm2 tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "iTerm2 docs — `https://iterm2.com/documentation-images.html` reference"
  - "ori_term existing `oriterm_core/src/term/handler/image/iterm2.rs` (261 lines per Pass 1) — current implementation surface"
depends_on: ["13"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "14.0"
    title: "Top-down spec audit (BLOCKING)"
    status: not-started
  - id: "14.1"
    title: "Verify OSC 1337 File= base64 + format detection"
    status: not-started
  - id: "14.2"
    title: "Verify inline vs download mode"
    status: not-started
  - id: "14.3"
    title: "Verify image dimension parameters (width/height/preserveAspectRatio)"
    status: not-started
  - id: "14.4"
    title: "Verify GIF frame extraction"
    status: not-started
  - id: "14.5"
    title: "Verify iTerm2 + image lifecycle interactions"
    status: not-started
  - id: "14.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "14.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 14: iTerm2 Inline Images

**Status:** Not Started
**Goal:** Verify every iTerm2 inline image catalog row.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed iTerm2 OSC 1337 is implemented at `oriterm_core/src/term/handler/image/iterm2.rs` (261 lines). This section verifies every catalog row via spec_chain. Section 10's OSC suite covered the non-image OSC 1337 variants; this section covers the image variants.

**Reference implementations:** see frontmatter.

**Depends on:** Section 13 (kitty graphics — shares image cache + lifecycle).

---

## 14.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-14-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA (and the entire DEC private rectangular-ops family) from the catalog. The original Section 01 catalog bootstrap was bottom-up (audit existing dispatch + add tack/teseq-discovered items), which is incomplete by construction — sequences absent from both the catalog AND the test corpus are invisible. The per-section audit file makes top-down coverage mechanically lintable: `spec-coverage-report --check audit-files` fails CI if any audit-file mapping does not resolve to a real catalog row.

**Canonical spec source(s):** iterm2.com/documentation-images.html (primary, iTerm2's own docs are canonical for OSC 1337 File=) + iTerm2 source `escape_codes/it2support.sh` cross-reference

**Files touched:**
- `plans/spec-conformance/audits/section-14-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)
- `plans/spec-conformance/catalog/iterm2.md` (open new rows for any sequences that should be `mapped` but aren't catalogued yet — use the canonical schema per `plans/spec-conformance/00-overview.md §Catalog Row Schema`)

**Completion criteria:**

- [ ] Audit file `plans/spec-conformance/audits/section-14-top-down-inventory.md` is populated with every sequence in the canonical spec source(s).
- [ ] Every row in the audit-file table has a `Decision` of `mapped` (cites a catalog row ID) or `not-targeted` (with one-line rationale).
- [ ] Every `mapped` row resolves to a real catalog row that exists in `plans/spec-conformance/catalog/`.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [ ] Audit file `last_walked` frontmatter is set to today's date and `walked_by` to the implementer's handle.
- [ ] Any new catalog rows opened in this subsection use the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema`.

**No other subsection in this section can begin work until §14.0 is complete.** This is a hard gate.

---

## 14.1 Verify OSC 1337 File= base64 + format detection

- [ ] Spec_chain test: emit OSC 1337 File= with PNG payload, verify the image is decoded and placed
- [ ] Test JPEG payload
- [ ] Test BMP payload
- [ ] Test malformed base64 — assert error is reported (the iTerm2 protocol's response form, if any) and no orphan image is left in the cache
- [ ] Update catalog rows to `verified`

---

## 14.2 Verify inline vs download mode

- [ ] Spec_chain test for inline=1: image renders at cursor
- [ ] Spec_chain test for inline=0: image is NOT rendered inline (download mode); verify the host download mechanism (or the corresponding effect emission) is invoked
- [ ] Update rows to `verified`

---

## 14.3 Verify image dimension parameters

- [ ] Test width=200 (pixels by default), height=100
- [ ] Test width=20px (explicit pixel suffix), width=20ch (cell suffix)
- [ ] Test preserveAspectRatio=1
- [ ] Update rows to `verified`

---

## 14.4 Verify GIF frame extraction

- [ ] Spec_chain test: emit OSC 1337 with a multi-frame GIF base64 payload, verify each frame is decoded and stored (likely as an `AnimRgba8` from the kitty animation infrastructure)
- [ ] Verify frame durations preserved
- [ ] Update rows to `verified`

---

## 14.5 Verify iTerm2 + image lifecycle interactions

- [ ] Test placement survives scrollback eviction, ED/EL erase, alt-screen toggle, resize (per section 07's lifecycle matrix template)
- [ ] Update rows to `verified`

---

## 14.R Third Party Review Findings

- None.

---

## 14.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: format (PNG/JPEG/BMP/GIF) × mode (inline/download) × dimension param × lifecycle event
- [ ] **Semantic pin**: GIF frame extraction round-trip test
- [ ] Every iTerm2 image catalog row is `verified`
- [ ] All existing teseq iTerm2 tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** iTerm2 inline image catalog rows verified.
