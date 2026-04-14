---
section: "08"
title: "ECMA-48 Baseline (absorbs in-flight tack-conformance work)"
status: not-started
reviewed: false
goal: "Drive the row subset of catalog/{ecma-48,xterm-ctlseqs,dec-private-modes,osc}.md that the existing tack-conformance work covers from `implemented-unverified` to `verified`, populate the legacy tack mapping table, and add new baseline rows for gaps tack didn't cover (DECLRMM grid enforcement, REP edge cases, 8-bit C1 controls)."
success_criteria:
  - "Every row in `catalog/ecma-48.md` covered by tack-conformance section 05 (test menu) is `verified` (uses the spec_chain harness from section 04)"
  - "Every row in `catalog/ecma-48.md` covered by tack-conformance section 06 (tools menu) is `verified`"
  - "Every basic ANSI mode (IRM, LNM) and basic DEC private mode (1, 5, 6, 7, 12, 25, 47, 1049, 2004) row in `catalog/dec-private-modes.md` is `verified`"
  - "Every basic OSC row (0, 1, 2, 4, 7, 10, 11, 12, 52) is `verified` in `catalog/osc.md`"
  - "**DECLRMM (left/right margins) grid enforcement implemented**: `oriterm_core/src/grid/mod.rs` has `left_margin: usize` and `right_margin: usize` fields; CUF/CUB/CHA respect them; the corresponding catalog row in `catalog/dec-private-modes.md` is `verified`"
  - "**8-bit C1 controls handled**: VTE parser detects 0x9B (CSI), 0x90 (DCS), 0x9F (APC) as C1 introducers (currently MISSING per Pass 1); the corresponding catalog rows are `verified`"
  - "**REP edge cases handled**: REP (CSI Ps b) with no preceding character is a no-op (per ECMA-48 §8.3.103); REP after a wide character repeats the wide character; the catalog rows are `verified`"
  - "**ISO 8613-6 truecolor SGR forms verified**: the `SGR 38 : 2 : <colorspace-id> : r : g : b : ...` and `SGR 48 : 2 : ...` colon-separated subparameter forms (per ISO 8613-6 §7) are parsed AND handled the same way as the xterm semi-colon variant. Both separators (`;` and `:`) work per the ECMA-48 §5.4.2 subparameter rules. The colorspace-id is ignored (per xterm de-facto) but MUST be tolerated. Catalog rows for both `SGR-38-2-ISO8613-6` and `SGR-38-2-XTERM` are `verified`."
  - "**ISO 8613-6 indexed color forms verified**: `SGR 38 : 5 : <index>` and `SGR 48 : 5 : <index>` forms verified in addition to the `SGR 38 ; 5 ; index` xterm form. Underline color (SGR 58 : 2 : ... and SGR 58 : 5 : ...) is verified at section 10's OSC work or here, whichever is more natural."
  - "`plans/spec-conformance/catalog/_legacy-tack-mapping.md` is populated: every catalog row driven to `verified` in this section has a row in the mapping table linking it to the legacy tack section that originally covered it"
  - "All existing teseq tests pass without modification"
  - "All existing tack tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** (delivers the baseline row subset)"
inspired_by:
  - "ori_term existing tack-conformance work — sections 01-06 already cover the baseline through real PTY scenarios; section 08 converts those into spec verification chains"
  - "wezterm `term/src/terminalstate/mod.rs` — DECLRMM grid enforcement reference (margin-aware cursor movement)"
depends_on: ["02", "04", "06"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "08.1"
    title: "Convert tack section 05 scenarios to spec verification chains"
    status: not-started
  - id: "08.2"
    title: "Convert tack section 06 scenarios to spec verification chains"
    status: not-started
  - id: "08.3"
    title: "Implement DECLRMM grid enforcement"
    status: not-started
  - id: "08.4"
    title: "Implement 8-bit C1 control detection"
    status: not-started
  - id: "08.5"
    title: "Verify REP edge cases"
    status: not-started
  - id: "08.5b"
    title: "Verify ISO 8613-6 SGR 38/48/58 colon-separated subparameter forms (truecolor + indexed)"
    status: not-started
  - id: "08.6"
    title: "Populate _legacy-tack-mapping.md as rows are verified"
    status: not-started
  - id: "08.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "08.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 08.2 (after tack absorption work — covers .1-.2),
# 08.5 (after gap fixes — covers .3-.5), final in 08.N
---

# Section 08: ECMA-48 Baseline

**Status:** Not Started
**Goal:** Establish ECMA-48 + xterm extensions baseline conformance for the row subset that tack-conformance already covers (sections 01-06 of that plan), then close gaps tack didn't cover: DECLRMM grid enforcement, 8-bit C1 control detection, and REP edge cases. This section is the entry point for Phase 3 — every Phase 3 stack section depends on baseline correctness.

**Success Criteria:**
- [ ] Every tack-covered row in `catalog/ecma-48.md` is `verified`
- [ ] Basic ANSI/DEC modes verified
- [ ] Basic OSC rows verified
- [ ] DECLRMM grid enforcement implemented; row verified
- [ ] 8-bit C1 controls handled; rows verified
- [ ] REP edge cases verified
- [ ] `_legacy-tack-mapping.md` populated
- [ ] Existing tack and teseq tests pass
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Connects to mission criterion: **Verification chain complete per row** (baseline subset)

**Context:** The ECMA-48 baseline is the gate for Phase 3 stacks. Sixel needs SGR + cursor + scrollback + DECSDM. Kitty needs OSC parsing + grid integration. Mouse needs CSI encoding. Without baseline correctness, every subsequent section would fight through baseline bugs. Per Codex Q5, this section also populates `_legacy-tack-mapping.md` (created empty by section 02) as it converts tack scenarios into spec verification chains. The mapping table preserves traceability without renaming files.

**Reference implementations:**
- **plans/tack-conformance/section-05-test-menu-scenarios.md** — covers modes/glitches/ACS/cursor/SGR/color (~18 scenarios) — this section converts those to spec_chain tests
- **plans/tack-conformance/section-06-tools-menu-scenarios.md** — covers ANSI status reports, SGR modes, character sets, ENQ/ACK — this section absorbs the in-flight work
- **wezterm** `term/src/terminalstate/mod.rs` — DECLRMM cursor movement reference

**Depends on:** Section 02 (the empty mapping file exists), Section 04 (SpecHarness + frozen catalog schema), Section 06 (Mode 2026 timeout wired + mode sync-point reduction complete for any mode-related catalog rows).

---

## Tack → spec_chain conversion rule

The phrase "convert tack scenario X to a spec_chain test" in 08.1/08.2 has a precise meaning. Follow these rules exactly:

1. **Source bytes**: The bytes fed to the `SpecHarness` come from the tack scenario's **FIXTURE/TRANSCRIPT** (the raw PTY input captured for that scenario), NOT from the scenario's rendered snapshot or grid-text expectation. Look in `crates/oriterm_test_support/src/tack_framework/scenarios/<family>/<scenario>/` for a `fixture.rs` / `transcript.*` / `*.teseq` file that contains the input byte sequence the scenario feeds. If the scenario is parameterized (e.g. multiple cases in one file), emit one spec_chain test per parameterized case.
2. **One tack scenario → one or more catalog rows**: A single tack scenario may exercise multiple catalog rows (e.g. the `status_reports_inventory` scenario covers DA1 + DA2 + DA3 + DSR). Each exercised row gets its own spec_chain test — do NOT bundle multiple rows into one test. The per-row granularity is required so the citation scanner can cross-check each row independently.
3. **Mapping record**: Every converted row gets a row in `plans/spec-conformance/catalog/_legacy-tack-mapping.md` with the catalog row ID, the originating tack scenario path, and a `converted` status marker. The mapping file is the permanent audit trail.
4. **Assertion port**: If the tack scenario asserted against grid text (e.g. `assert_grid_line(...)`), the spec_chain test ports that assertion into the `StateExpectation` apex. If the tack scenario asserted against emitted events (e.g. `assert_event("ClipboardLoad")`), the spec_chain test ports that into the `EffectExpectation` apex via `PtyEffect::Write`, `HostEffect::*`, or `HostRequest::*` as appropriate (see Section 03 for the Effect type family).
5. **Behavioral equivalence gate**: The original tack scenario MUST continue to pass after conversion — the tack scenario is the legacy regression guard. The spec_chain test is the new forward-looking guard. Both tests co-exist during the migration; `plans/tack-conformance/` is superseded but not deleted (see Section 02).
6. **No rewriting of inputs**: The byte sequence from the tack fixture goes into `SCENARIO.bytes` verbatim. Do NOT rephrase, simplify, or "clean up" the input — any divergence risks changing the behavior under test.

Example conversion trace for the DA1 row (already implemented in Section 04.6's DA1 pilot, documented here for reference):
- Tack source: (nothing — DA1 isn't covered by tack yet, which is why it's a pilot)
- Catalog row: `ECMA48-DA1`
- spec_chain test file: `oriterm_core/tests/spec_chain/ecma_48/ecma48_da1.rs`
- `SCENARIO.bytes = b"\x1b[c"`
- Apex: `EffectPtyWrite`
- Expectation: `EffectExpectation::pty_write(PtyWriteKind::DeviceAttribute, b"\x1b[?64")`

Example conversion trace for a tack-covered row (hypothetical CUP row from tack section 05 cursor scenarios):
- Tack source: `crates/oriterm_test_support/src/tack_framework/scenarios/modes/cursor_basic.rs` line 42 — fixture bytes `b"\x1b[5;10H"`
- Catalog row: `ECMA48-CUP`
- spec_chain test file: `oriterm_core/tests/spec_chain/baseline/tack_section_05/ecma48_cup.rs`
- `SCENARIO.bytes = b"\x1b[5;10H"`
- Apex: `State` (cursor at (4, 9) zero-based)
- `_legacy-tack-mapping.md`: `| ECMA48-CUP | tack-conformance/section-05 (modes/cursor_basic.rs) | converted |`

---

## 08.1 Convert tack section 05 scenarios to spec verification chains

**File(s):** `oriterm_core/tests/spec_chain/baseline/tack_section_05/*.rs` (new), populated catalog rows

For each scenario in `plans/tack-conformance/section-05-test-menu-scenarios.md` (modes, glitches, ACS, cursor movement, SGR, color), write a corresponding `spec_chain` test that drives the same sequence through every applicable rung and verifies the catalog row.

- [ ] Read `plans/tack-conformance/section-05-test-menu-scenarios.md` and `crates/oriterm_test_support/src/tack_framework/scenarios/*` to enumerate every scenario tack-conformance covers.
- [ ] For each scenario:
  1. Identify the catalog row(s) in `catalog/ecma-48.md` (or `catalog/dec-private-modes.md`/`catalog/osc.md`) that the scenario verifies
  2. Write a `spec_chain` test in `oriterm_core/tests/spec_chain/baseline/tack_section_05/<scenario_name>.rs`
  3. Update the catalog row's `Verification` to `verified` and `Test chain` to list the new test
  4. Add a row to `catalog/_legacy-tack-mapping.md`: `| <catalog row> | tack-conformance/section-05 (<scenario>) | converted |`
- [ ] **Validation**: every tack section 05 scenario has a corresponding spec_chain test; coverage report shows the converted rows as `verified`.

---

## 08.2 Convert tack section 06 scenarios to spec verification chains

**File(s):** `oriterm_core/tests/spec_chain/baseline/tack_section_06/*.rs` (new), populated catalog rows

Tack section 06 (just landed: TOOLS_MENU_INVENTORY) covers ANSI status reports (DA1/DA2/DA3, DSR, DECRQM), SGR modes, character set banks, ENQ/ACK. Section 06 is in flight; this subsection converts the landed parts and absorbs the remaining work as it lands.

- [ ] For each tack section 06 scenario (status reports, SGR modes, character sets, ENQ/ACK):
  1. Identify the catalog row(s)
  2. Write a `spec_chain` test
  3. Update the catalog row's verification status
  4. Add a row to `_legacy-tack-mapping.md`
- [ ] For tack section 06 work that hasn't landed yet (06.0.b, 06.0.c, 06.1-06.7), wait for it to land before converting (or coordinate with the in-flight work to absorb directly).
- [ ] **Validation**: every landed tack section 06 scenario has a corresponding spec_chain test.
- [ ] **TPR checkpoint** — `/tpr-review` covering 08.1–08.2 (tack absorption work). Catches conversion errors before the baseline gap-fix subsections proceed.

---

## 08.3 Implement DECLRMM grid enforcement

**File(s):** `oriterm_core/src/grid/mod.rs`, `oriterm_core/src/term/handler/modes.rs`, sibling tests

Pass 1 confirmed DECLRMM is parsed by the VTE handler but the grid does NOT enforce left/right margins. CUF/CUB/CHA must respect them when DECLRMM is set.

- [ ] Add `left_margin: usize` and `right_margin: usize` fields to `Grid`. Default: `left_margin = 0`, `right_margin = cols - 1`.
- [ ] Implement margin reset on resize.
- [ ] When mode 69 (DECLRMM) is enabled, allow `CSI Ps;Ps s` to set the margins (add a handler in `oriterm_core/src/term/handler/modes.rs` or wherever the mode dispatch lives).
- [ ] Update CUF, CUB, CHA, ICH, DCH, IL, DL handlers to respect the left/right margins (movement constrained within `[left_margin, right_margin]` when DECLRMM is set).
- [ ] Sibling tests in `oriterm_core/src/grid/tests.rs` (existing) AND a spec_chain test in `oriterm_core/tests/spec_chain/baseline/declrmm.rs`:
  - `cuf_respects_right_margin_under_declrmm()`
  - `cub_respects_left_margin_under_declrmm()`
  - `ich_inserts_within_margins_only()`
  - `declrmm_disabled_restores_full_width()`
- [ ] Update `catalog/dec-private-modes.md` row for DECLRMM to `verified`.
- [ ] **Validation**: tests pass; existing teseq cursor tests still pass.

---

## 08.4 Implement 8-bit C1 control detection

**File(s):** `crates/vte/src/ansi/processor.rs` (or wherever the byte-level state machine lives), sibling tests

The VTE parser currently only handles 7-bit ESC-prefixed C1 forms (ESC [ for CSI, ESC P for DCS, ESC _ for APC). 8-bit C1 (0x9B, 0x90, 0x9F) is not detected. Add 8-bit C1 detection.

- [ ] Read `crates/vte/src/ansi/processor.rs` to find the byte-level state machine. Add cases for 0x9B → CSI, 0x90 → DCS, 0x9F → APC (and 0x9D → OSC, 0x98 → SOS, 0x9E → PM, 0x9C → ST).
- [ ] Sibling tests:
  - `c1_csi_8bit_detected_as_csi_introducer()` (input `\x9b[0m`, asserts SGR reset is dispatched)
  - `c1_dcs_8bit_detected()`
  - `c1_apc_8bit_detected()`
- [ ] Update catalog rows in `catalog/ecma-48.md` for 8-bit C1 controls to `verified`.
- [ ] **Validation**: tests pass; existing C0 + 7-bit ESC tests still pass.

---

## 08.5 Verify REP edge cases

**File(s):** `oriterm_core/tests/spec_chain/baseline/rep_edge_cases.rs` (new), possibly `oriterm_core/src/term/handler/mod.rs` (REP handler refinement if needed)

REP (CSI Ps b) repeats the preceding graphic character N times. Edge cases per ECMA-48 §8.3.103:
- REP with no preceding graphic character (e.g., immediately after CR or after a control sequence) is a no-op
- REP after a wide character repeats the wide character
- REP after a SGR change uses the current SGR state (not the SGR at the time of the original character)

- [ ] Read the existing REP handler in ori_term. Verify it implements the edge cases correctly. If not, fix.
- [ ] Add spec_chain tests for each edge case.
- [ ] Update catalog row for REP to `verified`.
- [ ] **Validation**: tests pass.
- [ ] **TPR checkpoint** — `/tpr-review` covering 08.3-08.5 (gap fixes). Catches grid/parser interaction issues.

---

## 08.5b Verify ISO 8613-6 SGR colon-separated subparameter forms

**File(s):** `oriterm_core/tests/spec_chain/baseline/iso_8613_6_sgr.rs` (new), possibly `crates/vte/src/ansi/dispatch/csi.rs` (parser extension if only semicolon form is handled)

ECMA-48 §5.4.2 allows subparameters separated by `:` (colon) in addition to `;` (semicolon). ISO 8613-6 §7 defines the `SGR 38 : 2 : <colorspace-id> : r : g : b` truecolor form. Modern apps emit either form depending on authoring preference. ori_term MUST accept both. Pass 1 did not verify which form ori_term parses; this subsection verifies both.

- [ ] Read `crates/vte/src/ansi/dispatch/csi.rs` SGR parsing. Verify it accepts colon separators per ECMA-48 §5.4.2.
- [ ] If the parser only accepts semicolon: extend to accept colon subparameters, with fallback to semicolon for backwards compatibility
- [ ] Test cases:
  - `CSI 38;2;255;128;64 m` (xterm semicolon form) — already working
  - `CSI 38:2::255:128:64 m` (ISO 8613-6 colon form with empty colorspace-id) — verify accepted
  - `CSI 38:2:0:255:128:64 m` (ISO 8613-6 colon form with colorspace-id=0) — verify accepted, colorspace-id ignored
  - `CSI 48:2::255:128:64 m` (bg truecolor colon form)
  - `CSI 38;5;123 m` (indexed color semicolon form) — already working
  - `CSI 38:5:123 m` (indexed color colon form) — verify accepted
  - `CSI 58:2::255:128:64 m` (underline color colon form, ISO 8613-6 extension)
  - Mixed separators (some tools emit these): `CSI 38:2::255;128;64 m` — document parser behavior
- [ ] Update catalog rows in `catalog/ecma-48.md`: `SGR-38-2-ISO8613-6`, `SGR-48-2-ISO8613-6`, `SGR-58-2-ISO8613-6`, `SGR-38-5-ISO8613-6`, `SGR-48-5-ISO8613-6` all `verified`
- [ ] **Validation**: both separator forms work; mixed-separator edge case has documented behavior (either parsed or explicitly rejected with a consistent rule).

---

## 08.6 Populate _legacy-tack-mapping.md as rows are verified

**File(s):** `plans/spec-conformance/catalog/_legacy-tack-mapping.md`

This file was created empty in section 02. As 08.1-08.5 verify catalog rows that originated from tack scenarios, populate the mapping.

- [ ] After every catalog row drives from `implemented-unverified` to `verified` in this section, add a row to `_legacy-tack-mapping.md` linking the row ID to the original tack section.
- [ ] **Validation**: `_legacy-tack-mapping.md` has one entry per tack-originated catalog row that this section verified.

---

## 08.R Third Party Review Findings

- None.

---

## 08.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD)
- [ ] **Matrix dimensions**: tack scenario × catalog row × verification rung — every tack scenario maps to at least one catalog row; every row reaches every applicable rung
- [ ] **Semantic pin**: 8-bit C1 + DECLRMM tests are the regression guards for the new behavior
- [ ] Tack section 05 scenarios converted to spec_chain tests
- [ ] Tack section 06 scenarios converted (landed parts; remaining absorbed as they land)
- [ ] DECLRMM grid enforcement implemented and verified
- [ ] 8-bit C1 controls detected and verified
- [ ] REP edge cases verified
- [ ] ISO 8613-6 colon-separated SGR subparameter forms verified (38/48/58, both `2` truecolor and `5` indexed variants)
- [ ] `_legacy-tack-mapping.md` populated
- [ ] All existing tack tests pass without modification
- [ ] All existing teseq tests pass without modification
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 08 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** ECMA-48 baseline catalog row subset is `verified`; DECLRMM, 8-bit C1, REP edge cases all verified; legacy tack mapping populated; existing tack + teseq tests still pass; ready for Phase 3 stacks to depend on baseline correctness.
