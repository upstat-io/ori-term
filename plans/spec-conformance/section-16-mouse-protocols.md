---
section: "16"
title: "Mouse Protocols"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/mouse.md` from `implemented-unverified` to `verified` — every numbered mouse protocol (X10/9, normal/1000, locator/1001, button-event/1002, any-event/1003, focus/1004, UTF-8/1005, SGR/1006, URXVT/1015, SGR pixels/1016) including the encoding side."
success_criteria:
  - "Every row in `catalog/mouse.md` is `verified`"
  - "Every mouse encoding format (X10/Normal, UTF-8, SGR, URXVT, SGR pixels) verified via Effect transcript apex (`PtyEffect::Write { kind: PtyWriteKind::MouseEvent, bytes }`)"
  - "Modifier encoding verified: shift +4, alt +8, ctrl +16; verified for every protocol that supports modifiers"
  - "Locator mode (1001) IMPLEMENTED — Pass 1 found no locator handler; this section implements it per the xterm `ctlseqs.html` locator report spec (DECELR/DECLRP). NO deferral: the maximalist mission says every numbered mouse protocol in scope. Catalog row is `verified`, not `verified-with-deviation`."
  - "Mouse + focus event interaction verified: enabling 1004 alongside 1006 produces both focus events AND mouse events through the same SGR encoder pipeline"
  - "All existing mouse encoder tests in `oriterm/src/app/mouse_report/encode.rs` (or wherever) pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row**"
inspired_by:
  - "xterm `ctlseqs.html` mouse section — definitive numbered protocol reference"
  - "URXVT docs — URXVT protocol (1015) extensions"
  - "ori_term existing `oriterm/src/app/mouse_report/encode.rs` (per Pass 1) — current encoder surface"
depends_on: ["03", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "16.1"
    title: "Verify X10 / Normal / Button-event / Any-event encoding"
    status: not-started
  - id: "16.2"
    title: "Verify UTF-8 / SGR / URXVT / SGR pixels encoding"
    status: not-started
  - id: "16.3"
    title: "Verify modifier encoding for every protocol"
    status: not-started
  - id: "16.4"
    title: "Implement locator mode (1001) — DECELR/DECLRP per xterm spec"
    status: not-started
  - id: "16.5"
    title: "Verify mouse + focus event interaction"
    status: not-started
  - id: "16.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "16.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 16: Mouse Protocols

**Status:** Not Started
**Goal:** Verify every mouse protocol catalog row, including the encoding side. The encoders live in `oriterm/src/app/mouse_report/encode.rs` per Pass 1. Each encoded byte stream should produce an `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::MouseEvent })`.

**Success Criteria:** see frontmatter.

**Context:** Pass 1 confirmed mouse encoders exist for X10/UTF-8/SGR/URXVT. Section 09 verified the mode toggles (1000-1016). This section verifies the encoding via spec_chain. Locator mode (1001) was not found in Pass 1; this section IMPLEMENTS it per the xterm spec — no defer-vs-implement fork.

**Reference implementations:** see frontmatter.

**Depends on:** Section 08 (baseline correct, basic CSI parsing solid).

---

## 16.1 Verify X10 / Normal / Button-event / Any-event encoding

- [ ] For each mode (X10/9, 1000, 1002, 1003), spec_chain test that simulates a mouse event and asserts the encoded bytes match the expected wire format
- [ ] Verify clamping: X10 / Normal protocols clamp coordinates at 222 per spec
- [ ] Update catalog rows to `verified`

---

## 16.2 Verify UTF-8 / SGR / URXVT / SGR pixels encoding

- [ ] For each mode (1005, 1006, 1015, 1016), spec_chain test
- [ ] SGR pixels (1016) reports cell positions in pixels rather than cells; verify the pixel computation
- [ ] Update rows to `verified`

---

## 16.3 Verify modifier encoding for every protocol

- [ ] For each protocol that supports modifiers, spec_chain test that simulates mouse + shift / alt / ctrl, asserts the bit flags are encoded correctly
- [ ] Update rows to `verified`

---

## 16.4 Implement locator mode (1001)

**No deferral.** Per the maximalist mission, locator mode 1001 is IN scope. This subsection implements the locator handler. Locator mode is xterm-specific and is used by `DECLOCATE` applications; it reports a single mouse position as a response to a locator-report query rather than continuous event streams. Obscurity is not a reason to skip.

- [ ] Read the xterm locator mode documentation in `ctlseqs.html` (locator report section)
- [ ] Implement the locator handler: DECSET 1001 enables the mode; subsequent `CSI Pe ; Pu ' z` (DECELR — enable locator report) queries are answered with `CSI Pe ; Pm ; Pr ; Pc ; Pp & w` (DECLRP — locator report) where Pe is the event code, Pm is mask, Pr/Pc is row/col, Pp is page
- [ ] Route the locator reply through `Effect::Pty(PtyEffect::Write { kind: PtyWriteKind::MouseEvent, bytes: ... })`
- [ ] Spec_chain test: enable mode 1001, send DECELR, simulate mouse event at known position, assert the DECLRP reply bytes match the expected
- [ ] Update `catalog/mouse.md` row for 1001 to `verified` (NOT `verified-with-deviation`)

---

## 16.5 Verify mouse + focus event interaction

- [ ] Spec_chain test: enable mode 1004 (focus) + mode 1006 (SGR mouse) simultaneously; simulate window focus change + mouse click; verify both Effect emissions occur with the correct byte format
- [ ] Verify focus events use the same encoder pipeline as mouse events (sharing the SGR encoder)
- [ ] Update rows to `verified`

---

## 16.R Third Party Review Findings

- None.

---

## 16.N Completion Checklist

- [ ] Failing test matrix written FIRST
- [ ] **Matrix dimensions**: protocol (X10/1000/1001/1002/1003/1004/1005/1006/1015/1016) × event type (press/release/motion/wheel) × modifier (none/shift/alt/ctrl/combos)
- [ ] **Semantic pin**: SGR encoder format test, locked to xterm spec exactly
- [ ] Every mouse catalog row is `verified`
- [ ] Encoder tests pass
- [ ] Modifier encoding verified
- [ ] Locator mode (1001) implemented and verified (NO deferral fork)
- [ ] Mouse + focus interaction verified
- [ ] All existing mouse tests pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` + `index.md` updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** Every mouse catalog row is `verified`.
