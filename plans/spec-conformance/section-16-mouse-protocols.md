---
section: "16"
title: "Mouse Protocols"
status: not-started
reviewed: false
goal: "Drive every catalog row in `catalog/mouse.md` from `implemented-unverified` to `verified` — every numbered mouse protocol (X10/9, normal/1000, locator/1001, button-event/1002, any-event/1003, focus/1004, UTF-8/1005, SGR/1006, URXVT/1015, SGR pixels/1016) including the encoding side."
success_criteria:
  - "Top-down spec audit committed at `plans/spec-conformance/audits/section-16-top-down-inventory.md`. Every sequence in the canonical spec source(s) for this stack (xterm `ctlseqs.txt` §Mouse Tracking + URXVT mouse docs + DEC locator extensions DECEFR/DECELR/DECSLE/DECRQLP) maps to a catalog row ID OR carries an explicit `not-targeted` decision with rationale. `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file. This is enforced PER `plans/spec-conformance/audits/README.md` lint contract — added by Section 09A as the SSOT for top-down catalog coverage to prevent the bottom-up gap that hid DECRQCRA from the catalog."
  - "DEC locator extensions catalogued: `catalog/mouse.md` includes new rows for DECEFR (`CSI Pt;Pl;Pb;Pr ' w`), DECELR (`CSI Ps;Pu ' z`), DECSLE (`CSI Pm ' {`), and DECRQLP (`CSI Ps ' \\|`). These 4 sequences were discovered during Section 09A's top-down audit but routed to Section 16's ownership because they are mouse/locator protocol extensions, NOT rectangle or presentation ops. Each reaches `verified` status via the standard mouse-protocol verification chain."
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
  - id: "16.0"
    title: "Top-down spec audit (BLOCKING)"
    status: not-started
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

**OSC 22 push-vs-poll handoff (from Section 10.5 / scope clarification E):** Section 10 exposed `Term::mouse_cursor_icon` as a polling getter consumed via `RenderableContent::mouse_cursor_icon` (embedded path) and `PaneSnapshot::mouse_cursor_icon` (daemon path). UI consumers read this on every frame or render signal. A push-style alternative (`Effect::Ui(UiEffect::MouseCursorChanged(icon))`) would let UI consumers update lazily — but the architecturally correct home for that decision is Section 16, because this section owns the broader "what mouse-facing state does the UI consume, and via what interface" question (OSC 22 cursor-icon, mouse-mode toggles 1000–1016, locator mode 1001, mouse encoders). If Section 16 decides to switch to push semantics, it owns the migration: the polling surface MUST stay live until every UI consumer is converted so mid-migration consumers are not stranded. No action required at Section 16 kickoff — OSC 22 remains `verified` via the polling path regardless of Section 16's eventual decision.

**Reference implementations:** see frontmatter.

**Depends on:** Section 08 (baseline correct, basic CSI parsing solid).

---

## 16.0 Top-down spec audit (BLOCKING — precedes all other subsections)

**Goal:** Walk the canonical spec source(s) for this stack TOP-DOWN. Every sequence the spec defines gets a row in this section's audit file at `plans/spec-conformance/audits/section-16-top-down-inventory.md`, mapped to either an existing catalog row ID or an explicit `not-targeted` decision with rationale.

**Why this exists:** Section 09A introduced the `audits/` SSOT to close the bottom-up catalog construction gap that hid DECRQCRA (and the entire DEC private rectangular-ops family) from the catalog. The original Section 01 catalog bootstrap was bottom-up — sequences absent from both the catalog AND the test corpus are invisible. The per-section audit file makes top-down coverage mechanically lintable: `spec-coverage-report --check audit-files` fails CI if any audit-file mapping does not resolve to a real catalog row.

**Canonical spec source(s):** xterm `ctlseqs.txt` §Mouse Tracking (numbered protocols X10/9, Normal/1000, Locator/1001, Button-event/1002, Any-event/1003, Focus/1004, UTF-8/1005, SGR/1006, URXVT/1015, SGR pixels/1016) + URXVT mouse docs (1015 extension) + xterm `ctlseqs.txt` §DEC Locator (DECEFR `CSI Pt;Pl;Pb;Pr ' w`, DECELR `CSI Ps;Pu ' z`, DECSLE `CSI Pm ' {`, DECRQLP `CSI Ps ' \|`).

**Files touched:**
- `plans/spec-conformance/audits/section-16-top-down-inventory.md` (NEW — stub created by Section 09A's §09A.10; populated by this subsection)
- `plans/spec-conformance/catalog/mouse.md` (open new rows for DECEFR, DECELR, DECSLE, DECRQLP — 4 sequences discovered during Section 09A's top-down audit and assigned to Section 16's ownership per §09A.12)

**Completion criteria:**

- [ ] Audit file `plans/spec-conformance/audits/section-16-top-down-inventory.md` is populated with every sequence in the canonical spec source(s).
- [ ] Every row has a `Decision` of `mapped` (cites catalog row ID) or `not-targeted` (with rationale).
- [ ] Every `mapped` row resolves to a real catalog row.
- [ ] `cargo run -p oriterm_test_support --bin spec-coverage-report -- --check audit-files` passes for this audit file.
- [ ] Audit file `last_walked` and `walked_by` set.
- [ ] Any new catalog rows use the canonical 10-column schema.
- [ ] New `catalog/mouse.md` rows created for DECEFR (`MOUSE-DECEFR`), DECELR (`MOUSE-DECELR`), DECSLE (`MOUSE-DECSLE`), and DECRQLP (`MOUSE-DECRQLP`).

**No other subsection in this section can begin work until §16.0 is complete.**

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
