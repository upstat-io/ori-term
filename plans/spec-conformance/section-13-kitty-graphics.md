---
section: "13"
title: "Kitty Graphics Protocol"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/kitty-graphics.md` from `implemented-unverified` to `verified` — full APC `_G` protocol including chunked transmission, animation, virtual placements, and unicode placeholders."
success_criteria:
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

**Reference implementations:** see frontmatter.

**Depends on:** Section 12 (sixel landed; image cache + GPU pipeline shared with kitty; section 12's lifecycle tests cover the shared infrastructure).

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
