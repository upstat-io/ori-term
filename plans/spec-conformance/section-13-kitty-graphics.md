---
section: "13"
title: "Kitty Graphics Protocol"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/kitty-graphics.md` from `implemented-unverified` to `verified` — full APC `_G` protocol including chunked transmission, animation, virtual placements, and unicode placeholders."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-13-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (sw.kovidgoyal.net/kitty/graphics-protocol/ docs (primary, kitty source is the de facto SPEC for this protocol) + kitty source `kittens/icat/icat.py` cross-reference) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "Every row in `catalog/kitty-graphics.md` is `verified`"
  - "Every kitty action (a=t transmit, a=p place, a=d delete, a=f frame, a=c compose, a=q query — verified by Pass 1 to be implemented at parse.rs:197) verified via spec_chain"
  - "Every transmission format (f=24 RGB, f=32 RGBA, f=100 PNG) verified"
  - "Chunked transmission (m=1 more chunks, m=0 final) verified — feed split chunks, assert coalesced + decoded correctly"
  - "Animation (a=f TransmitFrame, a=c ComposeFrame) with both Overwrite and AlphaBlend modes verified — Pass 1 confirmed both modes are implemented at kitty_animation.rs:58-62"
  - "Virtual placements (U=1 unicode placeholders) verified"
  - "Image protocol replies (kitty ACK/error) verified via Effect transcript apex (`PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply }`) — emission point at handler/image/kitty.rs:465 confirmed by Pass 2"
  - "Kitty graphics + image lifecycle interactions verified (depends on section 07 fix)"
  - "Kitty + sixel cross-stack regression: placing kitty image then sixel image into the same grid does NOT corrupt either; verified in spec_chain"
  - "All existing teseq kitty tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "kitty source itself — `~/projects/reference_repos/console_repos/kitty/kitty/graphics.py` — kitty IS the spec"
  - "sw.kovidgoyal.net/kitty/graphics-protocol/ — public protocol documentation"
  - "wezterm `term/src/terminalstate/kitty.rs` — production reference for chunked transmission, animation, frame composition"
depends_on: ["12"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "13.0"
    title: "Top-down spec audit (BLOCKING)"
    status: not-started
  - id: "13.1"
    title: "Verify kitty action + format combinations (transmit/place/delete/query)"
    status: not-started
  - id: "13.2"
    title: "Verify chunked transmission (m=1 / m=0 coalesce + decode)"
    status: not-started
  - id: "13.3"
    title: "Verify animation (a=f, a=c with Overwrite + AlphaBlend modes)"
    status: not-started
  - id: "13.4"
    title: "Verify virtual placements (U=1 unicode placeholders)"
    status: not-started
  - id: "13.5"
    title: "Verify image protocol replies via Effect transcript apex"
    status: not-started
  - id: "13.6"
    title: "Verify kitty + sixel cross-stack regression"
    status: not-started
  - id: "13.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "13.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 13.3 (after action+chunked+animation — covers .1-.3),
# 13.6 (after virtual+reply+cross-stack — covers .4-.6), final in 13.N
---

# Section 13: Kitty Graphics Protocol

**Status:** Not Started
**Goal:** Verify every kitty graphics catalog row. Kitty is the second full visual stack and shares the image cache + GPU image pipeline with sixel; cross-stack regression sweeps catch interactions.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed kitty graphics is implemented end-to-end at `oriterm_core/src/image/kitty/parse.rs:141-291` + `oriterm_core/src/term/handler/image/kitty.rs` + `oriterm_core/src/term/handler/image/kitty_animation.rs`. The audit memory's "kitty q=1 query NOT IMPLEMENTED" claim is stale — Pass 1 confirmed the query IS handled (parse.rs:197 + kitty.rs:320). Animation supports both Overwrite and AlphaBlend modes (kitty_animation.rs:58-62). The image protocol replies (ACK/error) emit via `Event::PtyWrite` at kitty.rs:465 — after section 03's migration, these are `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply })`.

**Blocker note:** Additionally blocked by `BUG-08-8` (kitty.rs BLOAT split — 476 lines, ≤24 lines from the hard 500-line limit) — see `plans/bug-tracker/section-08-core-terminal.md` for the bug entry. Section 13's implementation work targets `oriterm_core/src/term/handler/image/kitty.rs` directly (the per-action handlers for transmit / place / delete / query / animate / frame compose); any new code added on top of the current 476-line baseline would push the file through the 500-line hard limit defined in `.claude/rules/code-hygiene.md` §File Size. This blocker is intentionally NOT recorded in frontmatter `depends_on:` because that field takes section-number tokens, not bug-tracker IDs; `/continue-roadmap` Step 1.92 surfaces BUG-08-8 to implementers when Section 13 becomes focus. Section 13's own completion checklist (see `## 13.N` below) contains a scanner-parsed gate on BUG-08-8 closure. BUG-08-7 (a separate semantic-correctness bug on the delete specifiers) ALSO targets the same file; the two bugs should ideally be fixed in the same sitting — the split from BUG-08-8 creates the natural file structure that makes BUG-08-7's delete-arm fix straightforward.

**Reference implementations:** see frontmatter.

**Depends on:** Section 12 (sixel landed; image cache + GPU pipeline shared with kitty; section 12's lifecycle tests cover the shared infrastructure).

---

## 13.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-13-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA (and the entire DEC private rectangular-ops family) from the catalog. The original Section 01 catalog bootstrap was bottom-up (audit existing dispatch + add tack/teseq-discovered items), which is incomplete by construction — sequences absent from both the catalog AND the test corpus are invisible. The per-section audit file makes top-down coverage mechanically lintable: `spec-coverage-report --check audit-files` fails CI if any audit-file mapping does not resolve to a real catalog row.

**Canonical spec source(s):** sw.kovidgoyal.net/kitty/graphics-protocol/ docs (primary, kitty source is the de facto SPEC for this protocol) + kitty source `kittens/icat/icat.py` cross-reference

**Files touched:**
- `plans/spec-conformance/audits/section-13-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)
- `plans/spec-conformance/catalog/kitty-graphics.md` (open new rows for any sequences that should be `mapped` but aren't catalogued yet — use the canonical schema per `plans/spec-conformance/00-overview.md §Catalog Row Schema`)

**Completion criteria:**

- [ ] Audit file `plans/spec-conformance/audits/section-13-top-down-inventory.md` is populated with every sequence in the canonical spec source(s).
- [ ] Every row in the audit-file table has a `Decision` of `mapped` (cites a catalog row ID) or `not-targeted` (with one-line rationale).
- [ ] Every `mapped` row resolves to a real catalog row that exists in `plans/spec-conformance/catalog/`.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [ ] Audit file `last_walked` frontmatter is set to today's date and `walked_by` to the implementer's handle.
- [ ] Any new catalog rows opened in this subsection use the canonical 10-column schema from `plans/spec-conformance/00-overview.md §Catalog Row Schema`.

**No other subsection in this section can begin work until §13.0 is complete.** This is a hard gate.

---

## 13.1 Verify kitty action + format combinations

**File(s):** `oriterm_core/tests/spec_chain/kitty/actions.rs` (new)

- [ ] For each kitty action (a=t, a=p, a=d, a=f, a=c, a=q), spec_chain test that drives the action through every applicable rung (parser, dispatch, state, effect for replies, golden image for visual placements).
- [ ] For each format (f=24, f=32, f=100), test transmit + display.
- [ ] Update catalog rows to `verified`.

---

## 13.2 Verify chunked transmission

**File(s):** `oriterm_core/tests/spec_chain/kitty/chunked.rs` (new)

- [ ] Test: feed an image transmission split across N chunks (m=1, m=1, ..., m=0). Assert the coalesced + decoded image matches the expected pixel data.
- [ ] Test: feed chunks out of order (verify the kitty parser's accumulator handles or rejects this per spec).
- [ ] Test: feed a chunk with malformed base64; assert the parser emits an error reply via `HostRequest`/effect and discards the partial image.
- [ ] Update rows to `verified`.

---

## 13.3 Verify animation (a=f, a=c with Overwrite + AlphaBlend)

**File(s):** `oriterm_core/tests/spec_chain/kitty/animation.rs` (new)

- [ ] Transmit a base frame (a=t)
- [ ] Transmit additional frames (a=f) with frame numbers
- [ ] Compose frames (a=c) with both Overwrite mode (cell_x_offset == 1) and AlphaBlend mode (default) — verify the composition matches kitty's reference behavior
- [ ] Test frame durations (the animation timing field)
- [ ] Update rows to `verified`.
- [ ] **TPR checkpoint** — `/tpr-review` covering 13.1–13.3.

---

## 13.4 Verify virtual placements (U=1 unicode placeholders)

**File(s):** `oriterm_core/tests/spec_chain/kitty/virtual_placements.rs` (new)

Virtual placements use the Unicode placeholder protocol: the terminal rasterizes the image but the cells use the unicode placeholder codepoints to position the image. This is how kitty supports image placement without scrolling.

- [ ] Test: transmit image with `U=1`, then write the unicode placeholder codepoints at the cells where the image should appear. Verify the image is rendered at those cells.
- [ ] Test: scrolling moves the placeholder cells but the image stays attached
- [ ] Update rows to `verified`.

---

## 13.5 Verify image protocol replies via Effect transcript apex

**File(s):** `oriterm_core/tests/spec_chain/kitty/replies.rs` (new)

- [ ] For each kitty action that produces a reply (q=query, error replies on bad input, OK replies on success), spec_chain test that observes the `PtyEffect::Write { kind: PtyWriteKind::ImageProtocolReply, bytes: ... }` in the effect transcript.
- [ ] Verify the bytes match the kitty protocol response format (`OK`, `EBADF`, `EBIG`, `EINVAL`, `ENOENT`).
- [ ] Update rows to `verified`.

---

## 13.6 Verify kitty + sixel cross-stack regression

**File(s):** `oriterm_core/tests/spec_chain/kitty/cross_stack_regression.rs` (new)

Sixel and kitty share the image cache + GPU pipeline. A bug in either can corrupt the other. Cross-stack tests catch this.

- [ ] Test: place a kitty image at row 5, then place a sixel image at row 10, verify both are rendered correctly via golden image apex
- [ ] Test: place a sixel image at row 5, then place a kitty image at row 5 (same cell), verify the kitty image overwrites or stacks (per protocol semantics)
- [ ] Test: rapid alternation of sixel + kitty image transmits stress-tests the image cache eviction policy
- [ ] Update rows to `verified`.
- [ ] **TPR checkpoint** — `/tpr-review` covering 13.4–13.6.

---

## 13.R Third Party Review Findings

- None.

---

## 13.N Completion Checklist

- [ ] `BUG-08-8` (kitty.rs BLOAT split) is CLOSED in `plans/bug-tracker/section-08-core-terminal.md` — verified by grepping the bug entry for `[x]`. This gate is MANDATORY: Section 13 cannot close while `oriterm_core/src/term/handler/image/kitty.rs` remains above the 500-line hard limit in `.claude/rules/code-hygiene.md` §File Size. See the `**Blocker note:**` in the Context paragraph above for the full rationale. Section 13's per-action implementation targets the split files, not the monolithic `kitty.rs` — closing the split is a prerequisite for any per-action code lands here.
- [ ] `BUG-08-7` (kitty delete dispatch — 4 wrong specifier mappings + missing d=q/Q/f/F) is CLOSED in `plans/bug-tracker/section-08-core-terminal.md`. This is a semantic-correctness bug on the delete arm of kitty graphics dispatch — Section 13's delete-action verification would be meaningless against the broken specifier mappings. Ideally fixed in the same sitting as BUG-08-8 (the file split creates the natural `delete.rs` file where the corrected specifier logic lives).
- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: action × format × chunked-state × animation-mode × placement-type × reply-status
- [ ] **Semantic pin**: cross-stack regression test (sixel + kitty mixed) is the SSOT regression guard for the shared image infrastructure
- [ ] Every row in `catalog/kitty-graphics.md` is `verified`
- [ ] All actions, formats, chunked transmission, animation modes verified
- [ ] Virtual placements verified
- [ ] Image protocol replies verified via Effect apex
- [ ] Cross-stack regression verified (sixel + kitty don't corrupt each other)
- [ ] All existing teseq kitty tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 13 status updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every kitty graphics catalog row is `verified`; cross-stack regression with sixel green.
