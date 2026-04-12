---
section: "12"
title: "Sixel"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/sixel.md` from `implemented-unverified` to `verified` via the spec_chain harness — first full visual stack section, exercising the entire pipeline (parser → DCS state → image cache → GPU image render → golden image)."
success_criteria:
  - "Every row in `catalog/sixel.md` is `verified`"
  - "Sixel parser tests verified for: DCS q introducer, P1 pan / P2 pad / P3 horizontal grid / P5 width / P6 height raster attributes, color define (#) / color select / repeat (!) / CR / NL operators, sixel data byte (vertical 6-pixel column)"
  - "Sixel decoder tests verified: HLS-to-RGB conversion (already correct per Pass 1 — `hue - 120.0` at color.rs:41), color map state, repeat optimization, background-transparent vs background-filled modes"
  - "Sixel grid integration verified: SIXEL_SCROLLING mode 80 cursor positioning, SIXEL_CURSOR_RIGHT mode 8452 cursor positioning, image placement creation, orphan cleanup"
  - "Sixel GPU rendering verified via golden image apex: a sixel raster fills the expected pixels in the rendered output (tested in section 04 pilot already; this section adds more golden scenarios)"
  - "Sixel + image lifecycle interactions verified: sixel image survives scrollback eviction, ED/EL erase, alt-screen toggle, resize (depends on section 07's image lifecycle fix)"
  - "All existing teseq sixel tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "DEC STD 070 — primary spec; defines DCS q semantics, P1-P6 raster attrs, color operators"
  - "libsixel `src/decoder.c` — reference implementation for parsing edge cases"
  - "wezterm `term/src/terminalstate/sixel.rs` — production reference for HLS rotation, raster attrs, transparency"
depends_on: ["05", "07", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "12.1"
    title: "Verify sixel parser rows (DCS q + raster attrs + color ops + data)"
    status: not-started
  - id: "12.2"
    title: "Verify sixel decoder rows (HLS, color map, repeat, transparency)"
    status: not-started
  - id: "12.3"
    title: "Verify sixel grid integration (SIXEL_SCROLLING, SIXEL_CURSOR_RIGHT)"
    status: not-started
  - id: "12.4"
    title: "Verify sixel GPU rendering via golden image apex"
    status: not-started
  - id: "12.5"
    title: "Verify sixel + image lifecycle interactions"
    status: not-started
  - id: "12.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "12.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 12.3 (after parser+decoder+grid — covers .1-.3),
# 12.5 (after GPU + lifecycle — covers .4-.5), final in 12.N
---

# Section 12: Sixel

**Status:** Not Started
**Goal:** Sixel is the first full visual stack — its verification chain exercises the entire pipeline from byte parsing to golden image. This section drives every sixel catalog row to `verified`.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed sixel parser, decoder, grid integration, and GPU rendering all exist. Section 04's pilot already verified one minimal scenario. This section drives every other catalog row. The HLS rotation bug suspected by the audit memory turned out to be CORRECT (`hue - 120.0` at color.rs:41 — verified by Pass 1), so no fix needed. Section 07's image lifecycle fix is a hard prerequisite for the lifecycle interaction tests.

**Blocker note:** Additionally blocked by `BUG-08-8` (kitty.rs BLOAT split — 476 lines, ≤24 lines from the hard 500-line limit) — see `plans/bug-tracker/section-08-core-terminal.md` for the bug entry. Although BUG-08-8's fix target is `oriterm_core/src/term/handler/image/kitty.rs`, Sections 12 (Sixel) and 13 (Kitty Graphics) are the implementation consumers that MUST NOT begin implementation until the kitty.rs split lands — any new per-action code added on top of the current 476-line baseline would push the file through the 500-line hard limit defined in `.claude/rules/code-hygiene.md` §File Size. This blocker is intentionally NOT recorded in frontmatter `depends_on:` because that field takes section-number tokens, not bug-tracker IDs; `/continue-roadmap` Step 1.92 surfaces BUG-08-8 to implementers when Section 12 becomes focus. Section 12's own completion checklist (see `## 12.N` below) contains a scanner-parsed gate on BUG-08-8 closure.

**Reference implementations:** see frontmatter.

**Depends on:** Section 05 (deterministic golden lane), Section 07 (image lifecycle fix), Section 08 (baseline correct).

---

## 12.1 Verify sixel parser rows

**File(s):** `oriterm_core/tests/spec_chain/sixel/parser.rs` (new)

For every parser-level catalog row (DCS q introducer, P1-P6 raster attrs, color define/select, repeat, CR, NL, sixel data byte), write a spec_chain test that asserts the parser tokenizes correctly.

- [ ] Walk `catalog/sixel.md` and identify rows with apex layer = parser
- [ ] For each row, write a spec_chain test feeding the byte sequence and asserting `observe_parser_rung(...)` matches the expected tokenization
- [ ] Update each row's `Verification` to `verified` (after the entire chain for that row passes — multi-rung rows verify only when every applicable rung is green)
- [ ] **Validation**: parser-level rows verified.

---

## 12.2 Verify sixel decoder rows

**File(s):** `oriterm_core/tests/spec_chain/sixel/decoder.rs` (new)

- [ ] Walk decoder-level rows in `catalog/sixel.md`
- [ ] For each, spec_chain test that feeds the sequence and asserts decoder state (color map updates, transparency mode toggle, etc.)
- [ ] Test HLS rotation explicitly: feed a known HLS triplet, assert the resulting RGB matches the expected (cross-check against libsixel)
- [ ] Update rows to `verified`
- [ ] **Validation**: decoder-level rows verified.

---

## 12.3 Verify sixel grid integration (SIXEL_SCROLLING, SIXEL_CURSOR_RIGHT)

**File(s):** `oriterm_core/tests/spec_chain/sixel/grid_integration.rs` (new)

- [ ] Test cursor positioning under SIXEL_SCROLLING ON (default): after sixel placement, cursor moves to next line below the image
- [ ] Test cursor positioning under SIXEL_SCROLLING OFF (DECRST 80): cursor stays at home
- [ ] Test cursor positioning under SIXEL_CURSOR_RIGHT ON (DECSET 8452): cursor moves to the right of the image rather than below
- [ ] Test image placement creation: after sixel data, the image_cache contains a placement at the expected row/column
- [ ] Test orphan cleanup: place sixel, scroll the placement off-screen, verify the placement is cleaned up by `prune_scrollback`
- [ ] Update rows to `verified`
- [ ] **Validation**: grid integration tests pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 12.1–12.3 (parser + decoder + grid integration). Catches multi-rung integration issues before GPU + lifecycle subsections.

---

## 12.4 Verify sixel GPU rendering via golden image apex

**File(s):** `oriterm_core/tests/spec_chain/sixel/golden_render.rs` (new), goldens in `crates/oriterm_test_support/tests/references/spec_chain/sixel/`

- [ ] Pick a few representative sixel scenarios:
  - Solid rectangle with one color (similar to section 04 pilot but explicit test in this section's directory)
  - Multi-color sixel with palette switching
  - Sixel with repeat optimization (`!` operator)
  - Sixel with CR + NL line wrapping
  - Sixel with transparency
- [ ] For each, capture the golden via `ORITERM_UPDATE_GOLDEN=1` using the deterministic lane from section 05
- [ ] Spec_chain test asserts the golden matches on subsequent runs
- [ ] Update GPU rendering rows to `verified`
- [ ] **Validation**: golden tests pass; back-to-back runs produce 0-pixel diff.

---

## 12.5 Verify sixel + image lifecycle interactions

**File(s):** `oriterm_core/tests/spec_chain/sixel/lifecycle.rs` (new)

- [ ] Test sixel survives scrollback eviction: place sixel near top of scrollback, fill scrollback past the eviction threshold, verify the placement is removed from cache
- [ ] Test sixel survives ED (erase display): place sixel, emit ED, verify the placement is removed from the erased region
- [ ] Test sixel survives EL (erase line): place sixel, emit EL, verify the placement at that row is removed
- [ ] Test sixel survives alt-screen toggle: place sixel, enter alt screen, verify the primary cache placement is preserved (alt cache is separate); exit alt screen, verify the placement is still in the primary cache
- [ ] Test sixel survives resize: place sixel, resize the grid smaller, verify the placement is clamped or removed per the section 07 resize policy
- [ ] Update rows to `verified`
- [ ] **Validation**: all lifecycle tests pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 12.4-12.5.

---

## 12.R Third Party Review Findings

- None.

---

## 12.N Completion Checklist

- [ ] `BUG-08-8` (kitty.rs BLOAT split) is CLOSED in `plans/bug-tracker/section-08-core-terminal.md` — verified by grepping the bug entry for `[x]`. This gate is MANDATORY: Section 12 cannot close while `oriterm_core/src/term/handler/image/kitty.rs` remains above the 500-line hard limit in `.claude/rules/code-hygiene.md` §File Size. See the `**Blocker note:**` in the Context paragraph above for the full rationale. Until BUG-08-8 closes, implementers must not modify `oriterm_core/src/term/handler/image/kitty.rs`.
- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: sixel feature × verification rung × lifecycle event
- [ ] **Semantic pin**: sixel golden tests + lifecycle matrix are the regression guards
- [ ] Every row in `catalog/sixel.md` is `verified`
- [ ] HLS rotation explicitly tested (cross-checked against libsixel)
- [ ] Sixel + image lifecycle survives every grid mutation (depends on section 07 fix)
- [ ] All existing teseq sixel tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 12 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every sixel catalog row is `verified`. Sixel is the first conformance-complete visual stack.
