---
section: "08"
title: "ECMA-48 Baseline (absorbs in-flight tack-conformance work)"
status: in-progress
reviewed: true
goal: "Drive the row subset of catalog/{ecma-48,xterm-ctlseqs,dec-private-modes,osc}.md that the existing tack-conformance work covers from `implemented-unverified` to `verified`, populate the legacy tack mapping table, and add new baseline rows for gaps tack didn't cover (DECLRMM full mode plumbing + grid enforcement, REP edge cases, 8-bit C1 controls, ISO 8613-6 SGR colon forms)."
success_criteria:
  - "Every row in `catalog/ecma-48.md` covered by tack-conformance section 05 (test menu) is `verified` (uses the spec_chain harness from section 04)"
  - "Every row in `catalog/ecma-48.md` covered by tack-conformance section 06 (tools menu) is `verified`"
  - "Every basic ANSI mode (IRM, LNM) and basic DEC private mode (1, 5, 6, 7, 12, 25, 47, 1049, 2004) row in `catalog/dec-private-modes.md` is `verified`"
  - "Every basic OSC row (0, 1, 2, 4, 7, 10, 11, 12, 52) is `verified` in `catalog/osc.md` — these rows are verified by converting tack section 06's direct-VTE cap cross-checks into spec_chain tests (subsections 08.1-08.2). If any basic OSC rows are NOT covered by tack section 05/06 scenarios, they are deferred to Section 10 (OSC Suite) which owns the full OSC stack."
  - "**DECLRMM full mode plumbing + grid enforcement implemented**: VTE layer has `NamedPrivateMode::LeftRightMargin` variant (mode 69) with `PrivateMode::new` mapping; `TermMode` has `LEFT_RIGHT_MARGIN` flag; `named_private_mode_flag` maps it; `status_report_private_mode` reports it; `Grid` has `left_margin: usize` and `right_margin: usize` fields; CSI s / DECSLRM ambiguity resolved (state-dependent dispatch); CUF/CUB/CHA/ICH/DCH/IL/DL/CR/NEL/IND/RI/cursor-wrap/reverse-wrap respect margins; `goto_origin_aware` is column-aware under DECOM+DECLRMM; save/restore includes margin state; reset/resize/disable-mode-69 clears margins; the corresponding catalog row in `catalog/dec-private-modes.md` is `verified`"
  - "**8-bit C1 controls handled**: VTE parser in `crates/vte/src/lib.rs:advance_ground()` detects 0x9B (CSI), 0x90 (DCS), 0x9D (OSC), 0x9F (APC), 0x98 (SOS), 0x9E (PM), 0x9C (ST) as C1 introducers — entering the same parser states as their 7-bit ESC-prefixed equivalents; the corresponding catalog rows are `verified`"
  - "**REP edge cases handled**: REP (CSI Ps b) with no preceding character is a no-op (per ECMA-48 sect.8.3.103); REP after a wide character repeats the wide character; the catalog rows are `verified`"
  - "**ISO 8613-6 truecolor SGR forms verified**: the `SGR 38 : 2 : <colorspace-id> : r : g : b : ...` and `SGR 48 : 2 : ...` colon-separated subparameter forms (per ISO 8613-6 sect.7) are parsed AND handled the same way as the xterm semi-colon variant. Both separators (`;` and `:`) work per the ECMA-48 sect.5.4.2 subparameter rules. The colorspace-id is ignored (per xterm de-facto) but MUST be tolerated. Catalog rows `ECMA48-SGR-38`, `ECMA48-SGR-48`, and `ECMA48-SGR-58` are `verified` (both semicolon and colon separator variants confirmed within each row)."
  - "**ISO 8613-6 indexed color forms verified**: `SGR 38 : 5 : <index>` and `SGR 48 : 5 : <index>` forms verified in addition to the `SGR 38 ; 5 ; index` xterm form. Underline color colon forms (SGR 58 : 2 : ... and SGR 58 : 5 : ...) verified."
  - "**Mixed separator negative pin documented**: `38:2::255;128;64` (mixed colon+semicolon) behavior is documented as unsupported with a negative-pin test asserting the failure mode"
  - "**Empty subparameter negative pin documented**: `::` vs `:0:` indistinguishability at dispatch time is documented with a negative-pin test"
  - "**BSU/ESU 7-bit only acknowledged**: sync-update path (`BSU_CSI`/`ESU_CSI` in `crates/vte/src/ansi/mod.rs:47-50`) uses 7-bit CSI only; if 8-bit C1 routing is added globally this path stays 7-bit-only; documented with a NOTE pin"
  - "**All 14 remaining Section-08-owned catalog rows verified**: SGR 53/55/73/74/75, DECSTR, DECSED, DECSEL, SL, SR, DECRQSS-DECSLRM, XT-DECSLRM, XT-PUSHSGR, XT-POPSGR — each has a spec_chain test and its catalog row is `verified`"
  - "`plans/spec-conformance/catalog/_legacy-tack-mapping.md` is populated: every catalog row driven to `verified` in this section has a row in the mapping table linking it to the legacy tack section that originally covered it"
  - "All existing teseq tests pass without modification"
  - "All existing tack tests pass without modification"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Verification chain complete per row** (delivers the baseline row subset)"
inspired_by:
  - "ori_term existing tack-conformance work — sections 01-06 already cover the baseline through real PTY scenarios; section 08 converts those into spec verification chains"
  - "wezterm `term/src/terminalstate/mod.rs` — DECLRMM grid enforcement reference (margin-aware cursor movement, scroll_up_within_margins, scroll_down_within_margins)"
  - "ghostty `src/lib_vt.zig` — CSI s / DECSLRM ambiguity resolution (explicit ambiguous action)"
depends_on: ["02", "04", "06"]
third_party_review:
  status: findings
  updated: 2026-04-14
sections:
  - id: "08.1"
    title: "Convert tack section 05 scenarios to spec verification chains"
    status: complete
  - id: "08.2"
    title: "Convert tack section 06 scenarios to spec verification chains"
    status: in-progress
  - id: "08.3"
    title: "DECLRMM mode plumbing (VTE types + TermMode + mode reporting)"
    status: not-started
  - id: "08.4"
    title: "DECLRMM grid enforcement (margin fields + cursor movement)"
    status: not-started
  - id: "08.5"
    title: "DECLRMM extended operations (IL/DL partial-width scroll, CSI s ambiguity, save/restore, reset paths)"
    status: not-started
  - id: "08.6"
    title: "Implement 8-bit C1 control detection"
    status: not-started
  - id: "08.7"
    title: "Verify REP edge cases"
    status: not-started
  - id: "08.8"
    title: "Verify ISO 8613-6 SGR colon-separated subparameter forms (truecolor + indexed + underline color)"
    status: not-started
  - id: "08.9"
    title: "Populate _legacy-tack-mapping.md as rows are verified"
    status: not-started
  - id: "08.8b"
    title: "Verify remaining Section-08-owned catalog rows"
    status: not-started
  - id: "08.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "08.N"
    title: "Completion Checklist"
    status: not-started
# TPR Checkpoint Placement: 08.2 (after tack absorption work — covers .1-.2),
# 08.5 (after DECLRMM — covers .3-.5), 08.8 (after gap fixes — covers .6-.8b),
# final in 08.N
---

# Section 08: ECMA-48 Baseline

**Status:** In Progress
**Goal:** Establish ECMA-48 + xterm extensions baseline conformance for the row subset that tack-conformance already covers (sections 01-06 of that plan), then close gaps tack didn't cover: DECLRMM full mode plumbing + grid enforcement, 8-bit C1 control detection, REP edge cases, and ISO 8613-6 SGR colon forms. This section is the entry point for Phase 3 — every Phase 3 stack section depends on baseline correctness.

**Success Criteria:**
- [ ] Every tack-covered row in `catalog/ecma-48.md` is `verified`
- [ ] Basic ANSI/DEC modes verified
- [ ] Basic OSC rows verified
- [ ] DECLRMM full mode plumbing implemented (VTE types, TermMode flag, mode reporting)
- [ ] DECLRMM grid enforcement implemented (margin fields, cursor movement, extended operations)
- [ ] CSI s / DECSLRM ambiguity resolved
- [ ] Save/restore includes margin state; reset/resize/disable-mode-69 clears margins
- [ ] 8-bit C1 controls handled; rows verified
- [ ] REP edge cases verified
- [ ] ISO 8613-6 colon-separated SGR subparameter forms verified (38/48/58, truecolor + indexed)
- [ ] Mixed separator and empty subparameter negative pins documented
- [ ] BSU/ESU 7-bit-only scope documented
- [ ] `_legacy-tack-mapping.md` populated
- [ ] Existing tack and teseq tests pass
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] All 14 remaining Section-08-owned catalog rows verified (SGR 53/55/73/74/75, DECSTR, DECSED, DECSEL, SL, SR, DECRQSS-DECSLRM, XT-DECSLRM, XT-PUSHSGR, XT-POPSGR)
- [ ] Connects to mission criterion: **Verification chain complete per row** (baseline subset)

**Context:** The ECMA-48 baseline is the gate for Phase 3 stacks. Sixel needs SGR + cursor + scrollback + DECSDM. Kitty needs OSC parsing + grid integration. Mouse needs CSI encoding. Without baseline correctness, every subsequent section would fight through baseline bugs. Per Codex Q5, this section also populates `_legacy-tack-mapping.md` (created empty by section 02) as it converts tack scenarios into spec verification chains. The mapping table preserves traceability without renaming files.

**Reference implementations:**
- **plans/tack-conformance/section-05-test-menu-scenarios.md** — covers modes/glitches/ACS/cursor/SGR/color (~18 scenarios) — this section converts those to spec_chain tests
- **plans/tack-conformance/section-06-tools-menu-scenarios.md** — covers ANSI status reports, SGR modes, character sets, ENQ/ACK — this section absorbs the in-flight work
- **wezterm** `term/src/terminalstate/mod.rs` — DECLRMM cursor movement reference, `scroll_up_within_margins` / `scroll_down_within_margins` for partial-width scroll
- **ghostty** `src/lib_vt.zig` — CSI s / DECSLRM ambiguity resolution reference

**Depends on:** Section 02 (the empty mapping file exists), Section 04 (SpecHarness + frozen catalog schema), Section 06 (Mode 2026 timeout wired + mode sync-point reduction complete for any mode-related catalog rows).

---

## Tack-to-spec_chain conversion rule

The phrase "convert tack scenario X to a spec_chain test" in 08.1/08.2 has a precise meaning. Follow these rules exactly:

1. **Source bytes**: The bytes fed to the `SpecHarness` come from the tack scenario's **FIXTURE/TRANSCRIPT** (the raw PTY input captured for that scenario), NOT from the scenario's rendered snapshot or grid-text expectation. Look in `crates/oriterm_test_support/src/tack_framework/scenarios/<family>/` — each scenario family has a `mod.rs` with constants, a `tests.rs` with test functions, and potentially fixture/transcript data embedded as byte literals or loaded from files. Read the scenario's test function to identify the exact byte sequence it feeds to the PTY. If the raw bytes are not directly available as a constant, derive them from the test's `PtySession::write()` / `session.send()` calls. If the scenario is parameterized (e.g. multiple cases in one file), emit one spec_chain test per parameterized case.
2. **One tack scenario maps to one or more catalog rows**: A single tack scenario may exercise multiple catalog rows (e.g. the `status_reports_inventory` scenario covers DA1 + DA2 + DA3 + DSR). Each exercised row gets its own spec_chain test — do NOT bundle multiple rows into one test. The per-row granularity is required so the citation scanner can cross-check each row independently.
3. **Mapping record**: Every converted row gets a row in `plans/spec-conformance/catalog/_legacy-tack-mapping.md` with the catalog row ID, the originating tack scenario path, and a `converted` status marker. The mapping file is the permanent audit trail.
4. **Assertion port**: If the tack scenario asserted against grid text (e.g. `assert_grid_line(...)`), the spec_chain test ports that assertion into the `StateExpectation` apex. If the tack scenario asserted against emitted events (e.g. `assert_event("ClipboardLoad")`), the spec_chain test ports that into the `EffectExpectation` apex via `PtyEffect::Write`, `HostEffect::*`, or `HostRequest::*` as appropriate (see Section 03 for the Effect type family).
5. **Behavioral equivalence gate**: The original tack scenario MUST continue to pass after conversion — the tack scenario is the legacy regression guard. The spec_chain test is the new forward-looking guard. Both tests co-exist during the migration; `plans/tack-conformance/` is superseded but not deleted (see Section 02).
6. **No rewriting of inputs**: The byte sequence from the tack fixture goes into `SCENARIO.bytes` verbatim. Do NOT rephrase, simplify, or "clean up" the input — any divergence risks changing the behavior under test.

Example conversion trace for the DA1 row (already implemented in Section 04.6's DA1 pilot, documented here for reference):
- Tack source: (nothing — DA1 isn't covered by tack yet, which is why it's a pilot)
- Catalog row: `ECMA48-CSI-DA1`
- spec_chain test file: `oriterm_core/tests/spec_chain/pilots/da1_query.rs`
- `SCENARIO.bytes = b"\x1b[c"`
- Apex: `EffectPtyWrite`
- Expectation: `EffectExpectation::pty("DeviceAttribute")`

Example conversion trace for a tack-covered row (hypothetical CUP row from tack section 05 cursor scenarios):
- Tack source: `crates/oriterm_test_support/src/tack_framework/scenarios/cursor_movement/mod.rs` — fixture bytes `b"\x1b[5;10H"`
- Catalog row: `ECMA48-CUP`
- spec_chain test file: `oriterm_core/tests/spec_chain/baseline/tack_section_05/ecma48_cup.rs`
- `SCENARIO.bytes = b"\x1b[5;10H"`
- Apex: `State` (cursor at (4, 9) zero-based)
- `_legacy-tack-mapping.md`: `| ECMA48-CUP | tack-conformance/section-05 (cursor_movement/mod.rs) | converted |`

---

## 08.1 Convert tack section 05 scenarios to spec verification chains

**File(s):** `oriterm_core/tests/spec_chain/baseline/tack_section_05/*.rs` (new), populated catalog rows

For each scenario family in `plans/tack-conformance/section-05-test-menu-scenarios.md`, write corresponding `spec_chain` tests that drive the same sequence through every applicable rung and verify the catalog row. The tack scenario families in section 05 live under `crates/oriterm_test_support/src/tack_framework/scenarios/` in these directories: `modes/`, `acs/`, `cursor_movement/`, `graphic_rendition/`, `color/`, `padding/`.

- [x] Read `plans/tack-conformance/section-05-test-menu-scenarios.md` to enumerate every scenario family tack section 05 covers.
- [x] Read each scenario family directory under `crates/oriterm_test_support/src/tack_framework/scenarios/` to extract the fixture byte sequences (in `mod.rs`) and test assertions (in `tests.rs`).
- [x] For each scenario family, for each exercised catalog row:
  1. Identify the catalog row(s) in `catalog/ecma-48.md` (or `catalog/dec-private-modes.md`/`catalog/osc.md`) that the scenario verifies
  2. Write a `spec_chain` test in `oriterm_core/tests/spec_chain/baseline/tack_section_05/<scenario_name>.rs` following the pattern in `oriterm_core/tests/spec_chain/pilots/da1_query.rs`
  3. Update the catalog row's `Verification` to `verified` and `Test chain` to list the new test
  4. Add a row to `catalog/_legacy-tack-mapping.md`: `| <catalog row> | tack-conformance/section-05 (<scenario>) | converted |`
- [x] Register all new test modules in `oriterm_core/tests/spec_chain/main.rs` (add `mod baseline;` path).
- [x] **Validation**: every tack section 05 scenario has a corresponding spec_chain test; coverage report shows the converted rows as `verified`; original tack tests still pass.

**Implementation notes (2026-04-14):** Six scenario families (`acs`, `graphic_rendition`, `cursor_movement`, `modes`, `color`, `padding`) → six spec_chain modules under `oriterm_core/tests/spec_chain/baseline/tack_section_05/`. Five new catalog rows driven to `verified`: `ECMA48-C0-BEL` (acs/graphic_rendition), `ECMA48-CSI-CUP`, `ECMA48-CSI-ED` (cursor_movement via `clear` cap), `DEC-DECAWM`, `DEC-DECREVWRAP` (modes via `am`/`bw`). Three families contribute zero new ECMA-48 rows against tack v1.08 + `extra/ori_term.info`: `graphic_rendition` (combined screen with `acs`, no SGR sample text emitted), `color` (only numeric terminfo caps `colors`/`pairs` exercised — no protocol-row mapping), `padding` (only DA1 probe — already covered by the `da1_query` pilot; `rs1` reported as absent in our terminfo). Each "no new rows" finding is documented as a negative-pin test + module rustdoc so the absence is visible to future readers and the legacy mapping table is honest.

---

## 08.2 Convert tack section 06 scenarios to spec verification chains

**File(s):** `oriterm_core/tests/spec_chain/baseline/tack_section_06/*.rs` (new), populated catalog rows

Tack section 06 (TOOLS_MENU_INVENTORY) covers ANSI status reports (DA1/DA2/DA3, DSR, DECRQM), SGR modes, character set banks, ENQ/ACK. The scenario families live under `crates/oriterm_test_support/src/tack_framework/scenarios/` in: `tools_menu_inventory/`, `status_reports_inventory/`, `status_reports/`, `sgr_modes/`, `character_sets/`, `enq_ack/`.

- [x] For each tack section 06 scenario family:
  1. Identify the catalog row(s)
  2. Write a `spec_chain` test
  3. Update the catalog row's verification status
  4. Add a row to `_legacy-tack-mapping.md`
- [x] **Validation**: every tack section 06 scenario has a corresponding spec_chain test (tack section 06 is fully complete — all subsections 06.0-06.N are landed).
- [x] **OSC row ownership audit**: After converting all tack section 05/06 scenarios, audit which basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) now have spec_chain coverage from the conversion. Any rows NOT covered by tack scenarios remain owned by Section 10 (OSC Suite) — update catalog notes if needed to clarify ownership.
- [ ] **TPR checkpoint** — `/tpr-review` covering 08.1-08.2 (tack absorption work). Catches conversion errors before the gap-fix subsections proceed.

**Implementation notes (2026-04-14):** Six scenario families (`tools_menu_inventory`, `status_reports_inventory`, `status_reports`, `sgr_modes`, `character_sets`, `enq_ack`) → six spec_chain modules under `oriterm_core/tests/spec_chain/baseline/tack_section_06/`. Twelve new catalog rows driven to `verified`: `ECMA48-CSI-DA2`, `ECMA48-CSI-DA3`, `ECMA48-CSI-DSR-5`, `ECMA48-CSI-DSR-6` (status_reports_inventory — DA1 already covered by pilot); `ECMA48-SGR-0`, `ECMA48-SGR-1`, `ECMA48-SGR-4`, `ECMA48-SGR-7` (sgr_modes — four most-distinctive modes on tack's 80-mode grid; remaining 76 owned by `tack_cap_xcheck`); `ECMA48-ESC-0`, `ECMA48-ESC-B`, `ECMA48-C0-SO`, `ECMA48-C0-SI` (character_sets — SCS designation + SO/SI bank switching + preview-pane end-to-end render). Three families contribute zero new rows: `tools_menu_inventory` (inventory sentinel, no protocol bytes), `status_reports` (helper module only), `enq_ack` (blocked on BUG-08-6 — ECMA48-C0-ENQ remains `missing`). OSC row ownership audit: tack scenarios drive zero OSC rows — all basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) remain owned by Section 10. 26 new spec_chain tests, all green on host + Windows cross-compile.

---

## 08.3 DECLRMM mode plumbing (VTE types + TermMode + mode reporting)

**File(s):** `crates/vte/src/ansi/types.rs`, `oriterm_core/src/term/mode/mod.rs`, `oriterm_core/src/term/handler/helpers.rs`, `oriterm_core/src/term/handler/status.rs`, `oriterm_core/src/term/handler/modes.rs`, sibling tests

Before the grid can enforce left/right margins, the mode plumbing must exist end-to-end. Currently mode 69 is completely absent from the VTE type layer, TermMode flags, and mode reporting. This subsection adds the full vertical slice.

**Why a separate subsection from grid enforcement:** Mode plumbing is a prerequisite for grid enforcement. Attempting both in one subsection creates an untestable intermediate state — the mode flag would exist but nothing would observe it, or the grid fields would exist but the mode couldn't be toggled. Splitting lets each subsection be independently testable.

- [ ] **VTE type layer** — Add `LeftRightMargin = 69` variant to `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs:226`. Add `69 => Self::Named(NamedPrivateMode::LeftRightMargin)` mapping in `PrivateMode::new()` at `types.rs:177-204`.
- [ ] **TermMode flag** — Add `const LEFT_RIGHT_MARGIN = 1 << 32` to `TermMode` bitflags in `oriterm_core/src/term/mode/mod.rs:14`. All 32 bits of the current `u32` representation are fully occupied by real mode flags (bits 0-31). Computed unions like `ANY_MOUSE` are ORs of existing bits — they do NOT occupy reclaimable bit positions. The ONLY viable approach is to widen `TermMode` from `u32` to `u64` (change `bitflags! { pub struct TermMode: u32` to `u64`). This is a mechanical change — all downstream code uses the bitflags API, not raw bit manipulation.
- [ ] **named_private_mode_flag** — Add `NamedPrivateMode::LeftRightMargin => Some(TermMode::LEFT_RIGHT_MARGIN)` mapping in `oriterm_core/src/term/handler/helpers.rs:22-51`. This is the exhaustive match — adding the variant without updating it is a compile error.
- [ ] **DECSET/DECRST handler** — Add `NamedPrivateMode::LeftRightMargin` arms to the `set_private_mode` and `unset_private_mode` match blocks in `oriterm_core/src/term/handler/modes.rs`. Setting mode 69 inserts the flag; unsetting removes it AND resets left/right margins to full width (see 08.5 for the reset path details).
- [ ] **Mode reporting** — Verify `status_report_private_mode` in `oriterm_core/src/term/handler/status.rs:108` correctly reports mode 69 state via the existing `named_private_mode_flag` lookup (should work automatically once the flag mapping exists).
- [ ] **Tests — TDD, failing first:**
  - `mode_69_set_inserts_left_right_margin_flag()` — feed `\x1b[?69h`, assert `TermMode::LEFT_RIGHT_MARGIN` is set
  - `mode_69_reset_removes_left_right_margin_flag()` — feed `\x1b[?69l`, assert flag removed
  - `mode_69_decrqm_reports_correctly()` — feed `\x1b[?69$p`, assert reply contains mode-set/reset value
  - `mode_69_survives_named_private_mode_flag_exhaustive_test()` — the existing exhaustive test in `oriterm_core/src/term/handler/tests.rs:5125` must pass with the new variant
- [ ] **Validation**: `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green. The exhaustive mode-flag sync test passes.

---

## 08.4 DECLRMM grid enforcement (margin fields + cursor movement)

**File(s):** `oriterm_core/src/grid/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`, `oriterm_core/src/term/handler/mod.rs`, `oriterm_core/src/term/handler/helpers.rs`, sibling tests

With mode 69 plumbed, implement the actual left/right margin enforcement in the grid. This subsection covers the margin fields and cursor movement operations (CUF, CUB, CHA, CR, goto_origin_aware).

- [ ] **Grid margin fields** — Add `left_margin: usize` and `right_margin: usize` fields to `Grid` in `oriterm_core/src/grid/mod.rs:35`. Default: `left_margin = 0`, `right_margin = cols - 1`. Add public accessor `left_right_margins(&self) -> (usize, usize)` and `set_left_right_margins(&mut self, left: usize, right: usize)`.
- [ ] **DECSLRM handler** — Add a handler for `CSI Pl ; Pr s` (when mode 69 is active) that calls `grid.set_left_right_margins(left, right)`. This is the DECSLRM sequence. **CSI s ambiguity is handled in 08.5** — this item only adds the margin-setting method on Grid.
- [ ] **CUF (move_forward)** — In `oriterm_core/src/grid/navigation/mod.rs`, when DECLRMM is active and cursor is within the margin band, clamp rightward movement to `right_margin`. Current implementation in `move_forward()` clamps to `cols - 1`. Add a `right_bound` parameter or query the margin fields.
- [ ] **CUB (move_backward)** — Clamp leftward movement to `left_margin` when DECLRMM is active and cursor is within the margin band.
- [ ] **CHA (move_to_column / goto_col)** — When DECLRMM + DECOM are active, column addressing is relative to `left_margin`. The handler at `oriterm_core/src/term/handler/mod.rs:167` calls `grid.move_to_column()` — this must offset by `left_margin` when both modes are set.
- [ ] **CR (carriage_return)** — When DECLRMM is active and cursor is within the margin band, CR moves to `left_margin` (not column 0). The current `grid.carriage_return()` unconditionally moves to column 0.
- [ ] **goto_origin_aware column awareness** — `oriterm_core/src/term/handler/helpers.rs:102` currently handles DECOM for vertical (scroll region) only. When both DECOM and DECLRMM are active, column `col` should be relative to `left_margin` and clamped to `[left_margin, right_margin]`. This affects CUP (goto), HVP, and any origin-aware positioning.
- [ ] **NEL (next_line) / IND (linefeed)** — When DECLRMM active and cursor within margin band, NEL moves to `left_margin` (not column 0) at the next line. IND scrolls only within the margin band if cursor is at the bottom margin row.
- [ ] **RI (reverse_index)** — When DECLRMM active and cursor is at the top of the scroll region, reverse scroll should respect left/right margins (content outside the margin band survives).
- [ ] **Cursor wrap** — When DECLRMM active and cursor reaches `right_margin`, auto-wrap wraps to `left_margin` of the next line (not column 0).
- [ ] **Reverse wrap** — When DECLRMM active and mode 45 set, BS at `left_margin` wraps to `right_margin` of the previous line.
- [ ] **HT (horizontal tab)** — When DECLRMM active, tab stops beyond `right_margin` are not reachable; HT stops at `right_margin`.
- [ ] **CBT (cursor backward tab)** — When DECLRMM active, backward tab stops before `left_margin` are not reachable; CBT stops at `left_margin`.
- [ ] **Tests — TDD, failing first — write ALL tests before implementing any movement logic:**
  - `cuf_respects_right_margin_under_declrmm()` — cursor stops at right_margin
  - `cub_respects_left_margin_under_declrmm()` — cursor stops at left_margin
  - `cha_relative_to_left_margin_under_decom_declrmm()` — CHA col=1 goes to left_margin
  - `cr_goes_to_left_margin_under_declrmm()` — not column 0
  - `cup_relative_to_margins_under_decom_declrmm()` — CUP (1,1) = (scroll_top, left_margin)
  - `auto_wrap_at_right_margin_wraps_to_left_margin()` — character at right_margin+1 wraps to left_margin
  - `reverse_wrap_at_left_margin_wraps_to_right_margin()` — BS wraps backward correctly
  - `nel_goes_to_left_margin_not_col_0()` — NEL with margins
  - `declrmm_disabled_restores_full_width_movement()` — disabling mode 69 removes margin constraints
  - `cursor_outside_margin_band_not_constrained()` — cursor positioned outside [left, right] is NOT constrained by margins (WezTerm behavior)
- [ ] **Negative pins:**
  - `cuf_without_declrmm_ignores_margins()` — margins set but mode 69 off = no effect
  - `ht_stops_at_right_margin_under_declrmm()` — tab doesn't cross right margin
  - `cbt_stops_at_left_margin_under_declrmm()` — backward tab doesn't cross left margin
  - `declrmm_does_not_affect_vertical_scroll_region()` — DECSTBM still works independently
- [ ] **File size gate**: `handler/mod.rs` is at 490 lines. Any DECLRMM logic added to handler/mod.rs must NOT push it over 500 lines. If it would, extract margin-related handler methods into `oriterm_core/src/term/handler/margins.rs` (new submodule) FIRST.
- [ ] **Validation**: tests pass; existing teseq cursor tests still pass; no alloc regression.

---

## 08.5 DECLRMM extended operations (IL/DL partial-width scroll, CSI s ambiguity, save/restore, reset paths)

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`, `crates/vte/src/ansi/dispatch/csi.rs`, `oriterm_core/src/term/handler/modes.rs`, `oriterm_core/src/term/handler/mod.rs`, sibling tests

This subsection handles the operations that are architecturally more complex than simple cursor movement: partial-width scrolling for IL/DL under margins, the CSI s / DECSLRM sequence ambiguity, save/restore of margin state, and all reset paths that must clear margins.

### 08.5a: IL/DL/ICH/DCH with horizontal margins (partial-width scroll)

- [ ] **Partial-width scroll primitives** — Current `scroll_range_up` and `scroll_range_down` in `oriterm_core/src/grid/scroll/mod.rs:128-168` rotate full rows. When DECLRMM is active, IL/DL must scroll only the columns within `[left_margin, right_margin]` — content outside the margin band survives unchanged. This requires new primitives: `scroll_region_partial_up(row_range, col_range, count)` and `scroll_region_partial_down(row_range, col_range, count)` that operate on sub-row cell ranges. Reference: WezTerm's `scroll_up_within_margins` / `scroll_down_within_margins`.
- [ ] **ICH/DCH within margins** — `insert_blank` and `delete_chars` in `oriterm_core/src/grid/editing/mod.rs` must shift cells only within `[left_margin, right_margin]` when DECLRMM is active. Cells outside the margin band are not affected.
- [ ] **SL/SR within margins** — Scroll Left (`CSI Ps SP @`) and Scroll Right (`CSI Ps SP A`) shift content horizontally. When DECLRMM is active, SL/SR must operate within `[left_margin, right_margin]` using the same margin-constrained shift primitives as ICH/DCH. Content outside the margin band is not affected. These are implemented in 08.8b; this item ensures they respect margin constraints.
- [ ] **Tests:**
  - `il_with_margins_scrolls_only_margin_band()` — content outside margins survives
  - `dl_with_margins_scrolls_only_margin_band()` — content outside margins survives
  - `ich_within_margins_shifts_only_margin_band()` — insertion respects right boundary
  - `dch_within_margins_shifts_only_margin_band()` — deletion fills from right boundary
  - `sl_within_margins_shifts_only_margin_band()` — scroll left respects margin band
  - `sr_within_margins_shifts_only_margin_band()` — scroll right respects margin band

### 08.5b: CSI s / DECSLRM ambiguity

- [ ] **Problem**: Plain `CSI s` (no `?` intermediate) is hard-coded as `save_cursor_position()` at `crates/vte/src/ansi/dispatch/csi.rs:240`. But DECSLRM (Set Left and Right Margins) uses the same sequence when mode 69 is active. The ambiguity exists ONLY for the zero-parameter form `CSI s`. With one or two parameters (`CSI Pl ; Pr s`), the sequence is always DECSLRM — no ambiguity. WezTerm resolves the zero-param case in terminal state (`mod.rs:2567-2579`): if DECLRMM is set, call `set_left_and_right_margins()` with defaults; else call `dec_save_cursor()`. With-params calls go directly to `set_left_and_right_margins(left, right)` with no mode check (`mod.rs:2318-2345`). Ghostty similarly dispatches with-params (1 or 2) directly to `left_and_right_margin` handler; zero-params defers to `left_and_right_margin_ambiguous` (`stream.zig:1696-1708`).
- [ ] **Solution**: VTE dispatches ALL `CSI ... s` forms (zero-param and with-params) to a single `Handler::decslrm_or_save_cursor(params: &[u16])` method. The Term handler checks:
  - **With params (1 or 2)**: always DECSLRM — call `grid.set_left_right_margins(left, right)`. If mode 69 is inactive, DECSLRM is a no-op (per WezTerm/Ghostty behavior).
  - **Zero params**: if mode 69 active, call DECSLRM with defaults (1, cols); if mode 69 inactive, call `save_cursor_position()`.
  **Do NOT hard-code mode state into the VTE crate** — VTE is a vendored parser that must not contain oriterm-specific terminal state (per crate-boundaries.md).
- [ ] **Tests:**
  - `csi_s_zero_params_mode_69_off_saves_cursor()` — the backward-compat case
  - `csi_s_zero_params_mode_69_on_sets_default_margins()` — DECSLRM with defaults (1, cols)
  - `csi_s_with_params_always_decslrm()` — `CSI 5 ; 20 s` sets margins regardless of mode 69
  - `csi_s_with_params_mode_69_off_is_noop()` — DECSLRM with params but mode 69 inactive = no-op (NOT save cursor)

### 08.5c: Save/restore margin state

- [ ] **Problem**: `Grid::save_cursor` / `restore_cursor` at `oriterm_core/src/grid/navigation/mod.rs:188-203` stores only `Cursor`. The handler-level save at `oriterm_core/src/term/handler/mod.rs:301` saves cursor + charset + origin mode. **Neither saves left/right margin state.** Per DEC VT420 spec, DECSC/DECRC should save/restore the margin state (specifically whether DECLRMM was active and the margin values).
- [ ] **Solution**: Extend the saved state (either in `Grid::saved_cursor` or in the handler-level save) to include `left_margin`, `right_margin`, and whether `LEFT_RIGHT_MARGIN` mode was set. Match WezTerm's behavior.
- [ ] **Tests:**
  - `save_restore_preserves_margin_state()` — set margins, save, change margins, restore, verify original margins
  - `save_restore_preserves_declrmm_mode_flag()` — mode 69 state round-trips

### 08.5d: Reset paths that must clear margins

- [ ] **Problem**: Multiple reset operations should reset left/right margins to full width. Current code only resets vertical margins (scroll region). The following paths must also reset horizontal margins:
  - **Disabling mode 69** (`DECRST ?69`) — already in 08.3, but verify margins are actually cleared (not just the flag)
  - **DECCOLM** (mode 3 toggle) — `oriterm_core/src/term/handler/modes.rs` already resets scroll region; add horizontal margin reset
  - **RIS (full reset)** — `oriterm_core/src/term/handler/mod.rs` or wherever hard reset lives
  - **DECSTR (soft terminal reset)** — `CSI ! p` resets terminal state including margins (subsection 08.8b implements DECSTR; its reset path must clear horizontal margins)
  - **DECALN** — alignment test resets margins
  - **Resize** — `Grid::resize()` in `oriterm_core/src/grid/resize/mod.rs` should reset margins (margin column values may be invalid after a width change)
- [ ] **Tests:**
  - `decrst_69_resets_margins_to_full_width()` — disabling DECLRMM clears margins
  - `deccolm_resets_horizontal_margins()` — mode 3 toggle clears margins
  - `ris_resets_horizontal_margins()` — hard reset clears margins
  - `resize_resets_horizontal_margins()` — width change clears margins
  - `decstr_resets_horizontal_margins()` — soft reset clears margins
  - `decaln_resets_horizontal_margins()` — alignment test clears margins

- [ ] **spec_chain test** — `oriterm_core/tests/spec_chain/baseline/declrmm.rs` (new): at least one test driving DECLRMM through parser-dispatch-state apex. **Do NOT use the Renderable apex** — `observe_renderable` in `crates/oriterm_test_support/src/spec_chain/observers/renderable.rs:21-29` is currently a stub that unconditionally returns pass. Using it would give a false green rung 4. Use `ApexLayer::State` until the renderable observer has concrete assertions.
- [ ] Update `catalog/dec-private-modes.md` row for DECLRMM (mode 69) to `verified`.
- [ ] **TPR checkpoint** — `/tpr-review` covering 08.3-08.5 (DECLRMM work). This is the largest implementation block and has the highest interaction surface.
- [ ] **Validation**: tests pass; existing teseq cursor tests still pass; no alloc regression.

---

## 08.6 Implement 8-bit C1 control detection

**File(s):** `crates/vte/src/lib.rs` (the byte-level state machine), sibling tests

The VTE parser currently only handles 7-bit ESC-prefixed C1 forms (ESC [ for CSI, ESC P for DCS, ESC _ for APC). 8-bit C1 bytes (0x80-0x9F) are partially handled: `advance_ground()` at `crates/vte/src/lib.rs:649` detects bytes in the 0x80-0x9F range during UTF-8 error recovery (line 685: `if len == 1 && bytes[valid_bytes] <= 0x9F`) and routes them to `performer.execute(byte)`. Additionally, 0x9C (C1 ST) is explicitly handled as a state terminator at `lib.rs:341` and `lib.rs:475`.

**The real parser state machine is in `crates/vte/src/lib.rs` (`advance_ground`, `advance`), NOT in `crates/vte/src/ansi/processor.rs`.** The processor is a higher-level wrapper; the byte-level dispatch happens in `lib.rs`. Any C1 detection work must target `lib.rs`.

**Key constraint**: The `advance_ground()` function uses `memchr(0x1B, bytes)` for fast scanning (line 652). Naive byte-by-byte C1 scanning in the 0x80-0x9F range would regress hot-path performance. Implementation must either:
1. Extend the memchr scan to also stop on C1 bytes (e.g., use `memchr2` or `memchr3` for the most common C1 introducers), OR
2. Handle C1 bytes only during the UTF-8 error recovery path (which already detects them), ensuring they transition to the correct parser state

- [ ] **Audit current C1 handling** — Read `crates/vte/src/lib.rs:649-710` carefully. The UTF-8 error path already calls `performer.execute(byte)` for bytes <= 0x9F. Verify what `execute()` does with these bytes in the `Processor` (at `crates/vte/src/ansi/processor.rs`). If `execute()` already routes 0x9B to CSI state, 0x90 to DCS state, etc., then the gap is narrower than assumed.
- [ ] **If C1 handling is incomplete**: Add proper state transitions for each C1 introducer byte:
  - 0x90 (DCS) — enter DCS state (same as ESC P)
  - 0x9B (CSI) — enter CSI state (same as ESC [)
  - 0x9C (ST) — already handled as terminator
  - 0x9D (OSC) — enter OSC state (same as ESC ])
  - 0x9E (PM) — enter PM discard state (same as ESC ^)
  - 0x9F (APC) — enter APC state (same as ESC _)
  - 0x98 (SOS) — enter SOS discard state (same as ESC X)
- [ ] **BSU/ESU scope note** — The sync-update path (`BSU_CSI`/`ESU_CSI` constants at `crates/vte/src/ansi/mod.rs:47-50`) uses hardcoded 7-bit CSI sequences (`\x1b[?2026h` / `\x1b[?2026l`). These match byte-for-byte in `advance_sync_csi`. Adding global 8-bit C1 support must NOT break the BSU/ESU matcher. If an application sends `0x9b ?2026h` as an 8-bit BSU, the current sync path will NOT recognize it. This is acceptable for now (no real-world app does this) but must be documented with a NOTE pin test.
- [ ] **Performance guard** — Run the alloc regression tests AND time the existing teseq suite before and after the change. Any measurable regression in the parse hot path must be investigated.
- [ ] **Tests — TDD, failing first:**
  - `c1_0x9b_enters_csi_state()` — input `\x9b0m` (8-bit CSI + SGR reset), assert SGR is dispatched
  - `c1_0x90_enters_dcs_state()` — input `\x90q...ST`, assert DCS hook is called
  - `c1_0x9d_enters_osc_state()` — input `\x9d0;title\x9c`, assert title is set
  - `c1_0x9f_enters_apc_state()` — input `\x9f...\x9c`, assert APC content captured
  - `c1_0x98_enters_sos_discard_state()` — input `\x98...\x9c`, assert SOS discarded
  - `c1_0x9e_enters_pm_discard_state()` — input `\x9e...\x9c`, assert PM discarded
  - `c1_0x9c_terminates_sequence()` — 0x9C as ST within DCS/APC/OSC
  - **Negative pin**: `bsu_esu_7bit_not_matched_by_8bit_csi()` — `0x9b?2026h` does NOT trigger sync update (the BSU matcher expects the 7-bit form)
  - **Semantic pin**: `c1_csi_sgr_reset_only_passes_with_8bit_support()` — a test that feeds `\x9b0m` and asserts the cell template's SGR is reset; this ONLY passes when 8-bit C1 routing is correct
- [ ] **Matrix dimensions**: 7 C1 bytes (0x90, 0x98, 0x9B, 0x9C, 0x9D, 0x9E, 0x9F) x 2 context positions (ground state, mid-sequence as terminator) = 14 test cells.
- [ ] Update catalog rows in `catalog/ecma-48.md` for 8-bit C1 controls to `verified`.
- [ ] **Validation**: tests pass; existing C0 + 7-bit ESC tests still pass; alloc regression unchanged; teseq suite timing stable.

---

## 08.7 Verify REP edge cases

**File(s):** `oriterm_core/tests/spec_chain/baseline/rep_edge_cases.rs` (new), possibly `oriterm_core/src/term/handler/mod.rs` (REP handler refinement if needed)

REP (CSI Ps b) repeats the preceding graphic character N times. Edge cases per ECMA-48 sect.8.3.103:
- REP with no preceding graphic character (e.g., immediately after CR or after a control sequence) is a no-op
- REP after a wide character repeats the wide character (occupies 2 columns per repeat)
- REP after a SGR change uses the current SGR state (not the SGR at the time of the original character)

- [ ] Read the existing REP handler in ori_term (search for `repeat_preceding` or `repeat` in `oriterm_core/src/term/handler/`). Verify it implements the edge cases correctly. If not, fix.
- [ ] **Tests — TDD, spec_chain format:**
  - `rep_no_preceding_char_is_noop()` — feed `\x1b[3b` with no prior graphic char, assert grid unchanged
  - `rep_after_cr_is_noop()` — feed `A\r\x1b[3b`, assert no repeated chars
  - `rep_after_wide_char_repeats_wide()` — feed a CJK character + `\x1b[3b`, assert 3 wide chars (6 columns occupied)
  - `rep_uses_current_sgr_not_original()` — feed `A\x1b[31m\x1b[3b`, assert repeated chars have red foreground (SGR 31)
  - `rep_at_right_margin_wraps()` — repeating at the edge triggers auto-wrap
  - **Negative pin**: `rep_count_zero_repeats_once()` — `CSI 0 b` should repeat 1 time (per spec, Ps defaults to 1)
- [ ] Update catalog row for REP to `verified`.
- [ ] **Validation**: tests pass.

---

## 08.8 Verify ISO 8613-6 SGR colon-separated subparameter forms

**File(s):** `oriterm_core/tests/spec_chain/baseline/iso_8613_6_sgr.rs` (new), verify existing code in `crates/vte/src/ansi/dispatch/csi.rs`

ECMA-48 sect.5.4.2 allows subparameters separated by `:` (colon) in addition to `;` (semicolon). ISO 8613-6 sect.7 defines the `SGR 38 : 2 : <colorspace-id> : r : g : b` truecolor form. The existing parser at `crates/vte/src/ansi/dispatch/csi.rs` ALREADY handles colon forms via `handle_colon_rgb()` (line 371) and `parse_sgr_color()` (line 380). The SGR dispatch match at lines 315-340 routes:
- `[38]` (semicolon-separated, single subparam) to `parse_sgr_color` via an iterator over remaining params
- `[38, params @ ..]` (colon-separated, multiple subparams in one param group) to `handle_colon_rgb`
- Same pattern for `[48]` (background) and `[58]` (underline color)

**This subsection is primarily verification + gap-filling, not reimplementation.** The heavy lifting is already done. Focus on:
1. Testing that the existing code works correctly for all forms
2. Testing underline color colon forms (58:2:: and 58:5:) which may lack test coverage
3. Documenting and pinning the mixed-separator failure mode
4. Documenting the empty-subparam indistinguishability

- [ ] **Verify existing code** — Read `handle_colon_rgb` at `csi.rs:371-377` carefully. It skips the colorspace-id when `params.len() > 4` (by setting `rgb_start = 2`). Verify this handles:
  - `38:2::255:128:64` (empty colorspace-id) — the VTE parser represents the leading `:2:` as subparams `[38, 2, 0, 255, 128, 64]` or `[38, 2, ?, 255, 128, 64]`; verify the empty value becomes 0 and the skip logic handles it
  - `38:2:0:255:128:64` (explicit colorspace-id=0) — same path, colorspace-id ignored
  - `38:5:123` (indexed color colon form) — verify `parse_sgr_color` handles `5` branch
- [ ] **Test cases — TDD, failing first:**
  - `sgr_38_semicolon_truecolor()` — `CSI 38;2;255;128;64 m` — already working, regression pin
  - `sgr_38_colon_truecolor_no_colorspace()` — `CSI 38:2::255:128:64 m` — verify accepted
  - `sgr_38_colon_truecolor_with_colorspace()` — `CSI 38:2:0:255:128:64 m` — verify accepted, colorspace-id ignored
  - `sgr_48_colon_truecolor()` — `CSI 48:2::255:128:64 m` — background variant
  - `sgr_38_semicolon_indexed()` — `CSI 38;5;123 m` — already working, regression pin
  - `sgr_38_colon_indexed()` — `CSI 38:5:123 m` — verify accepted
  - `sgr_48_colon_indexed()` — `CSI 48:5:123 m` — verify accepted
  - `sgr_58_colon_truecolor()` — `CSI 58:2::255:128:64 m` — underline color truecolor colon form
  - `sgr_58_colon_indexed()` — `CSI 58:5:123 m` — underline color indexed colon form
  - `sgr_58_semicolon_truecolor()` — `CSI 58;2;255;128;64 m` — underline color semicolon form (regression pin)
  - **Negative pin — mixed separators**: `sgr_38_mixed_separators_does_not_parse()` — `CSI 38:2::255;128;64 m` (mixed colon+semicolon). The VTE parser splits on `;` at the top level, so `38:2::255` becomes one param group and `128` and `64` become separate params. `handle_colon_rgb` receives only `[2, 0, 255]` — not enough for RGB. Assert this either produces no color change or produces a wrong color (document whichever behavior occurs). This is a known limitation, NOT a bug to fix.
  - **Negative pin — empty subparam indistinguishability**: `sgr_38_double_colon_vs_zero_indistinguishable()` — `38:2::255:128:64` vs `38:2:0:255:128:64` should produce the same color (both arrive as `[38, 2, 0, 255, 128, 64]` at dispatch time because `::` and `:0:` are indistinguishable). Assert both produce `Rgb(255, 128, 64)`.
- [ ] **Matrix dimensions**: 3 color targets (fg=38, bg=48, underline=58) x 2 color modes (truecolor=2, indexed=5) x 2 separator forms (semicolon, colon) = 12 positive cells, plus 2 negative pins. The 12 positive tests are:
  1. `sgr_38_semicolon_truecolor` (fg, 2, semicolon)
  2. `sgr_38_colon_truecolor_no_colorspace` (fg, 2, colon)
  3. `sgr_38_semicolon_indexed` (fg, 5, semicolon)
  4. `sgr_38_colon_indexed` (fg, 5, colon)
  5. `sgr_48_semicolon_truecolor` (bg, 2, semicolon)
  6. `sgr_48_colon_truecolor` (bg, 2, colon)
  7. `sgr_48_semicolon_indexed` (bg, 5, semicolon)
  8. `sgr_48_colon_indexed` (bg, 5, colon)
  9. `sgr_58_semicolon_truecolor` (underline, 2, semicolon)
  10. `sgr_58_colon_truecolor` (underline, 2, colon)
  11. `sgr_58_semicolon_indexed` (underline, 5, semicolon)
  12. `sgr_58_colon_indexed` (underline, 5, colon)
- [ ] Update catalog rows in `catalog/ecma-48.md`: `ECMA48-SGR-38`, `ECMA48-SGR-48`, `ECMA48-SGR-58` all to `verified` (these are the actual catalog row IDs; the ISO 8613-6 colon forms are variant behaviors within the same rows, not separate catalog entries).
- [ ] **TPR checkpoint** — `/tpr-review` covering 08.6-08.8b (gap fixes). Catches parser/handler interaction issues.
- [ ] **Validation**: both separator forms work; negative pins document known limitations; existing SGR tests unchanged.

---

## 08.8b Verify remaining Section-08-owned catalog rows

**File(s):** Various handler files in `oriterm_core/src/term/handler/`, `crates/vte/src/ansi/dispatch/csi.rs`, sibling tests, catalog files

The catalog assigns 14 additional rows to Section 08 (11 in `catalog/ecma-48.md` + 3 in `catalog/xterm-ctlseqs.md`) that have `status: missing` or `stub` and are not covered by other subsections. These must be implemented/verified before the section can be marked complete.

**SGR rows (5):**
- [ ] `ECMA48-SGR-53` — Overlined. Verify `oriterm_core` handles SGR 53 (set overline) and SGR 55 (reset overline). Add spec_chain test. Update catalog row to `verified`.
- [ ] `ECMA48-SGR-55` — Not overlined (reset for SGR 53). Covered by the same implementation as SGR 53.
- [ ] `ECMA48-SGR-73` — Superscript. Verify handler exists or implement. Add spec_chain test. Update catalog.
- [ ] `ECMA48-SGR-74` — Subscript. Verify handler exists or implement. Add spec_chain test. Update catalog.
- [ ] `ECMA48-SGR-75` — Neither superscript nor subscript (reset). Covered by same implementation as 73/74.

**CSI rows (4):**
- [ ] `ECMA48-CSI-DECSTR` — Soft Terminal Reset. Verify the DECSTR handler exists and resets the correct terminal state. Add spec_chain test. Update catalog.
- [ ] `ECMA48-CSI-DECSED` — Selective Erase in Display. Verify handler. Add spec_chain test. Update catalog.
- [ ] `ECMA48-CSI-DECSEL` — Selective Erase in Line. Verify handler. Add spec_chain test. Update catalog.
- [ ] `ECMA48-CSI-SL` — Scroll Left (CSI Ps SP @). Verify handler or implement. Add spec_chain test. Update catalog.
- [ ] `ECMA48-CSI-SR` — Scroll Right (CSI Ps SP A). Verify handler or implement. Add spec_chain test. Update catalog.

**DCS rows (1):**
- [ ] `ECMA48-DCS-DECRQSS-DECSLRM` — DECRQSS for DECSLRM (query left/right margin values). Depends on 08.3-08.5 (DECLRMM plumbing). Verify the DECRQSS handler can report DECSLRM state. Add spec_chain test. Update catalog.

**xterm-ctlseqs rows (3):**
- [ ] `XT-DECSLRM` — Set Left and Right Margins (`CSI Ps ; Ps s`). Already implemented by 08.5b (CSI s ambiguity resolution). Update catalog row from `stub` to `verified` once 08.5b work lands.
- [ ] `XT-PUSHSGR` — Push current SGR attributes onto stack (`CSI # {`). Not dispatched in `csi::dispatch`. Implement handler (add `('{', [b'#'])` arm), add SGR attribute stack to `Term` or `Grid`, add spec_chain test. Update catalog.
- [ ] `XT-POPSGR` — Pop SGR attributes from stack (`CSI # }`). Pair with XTPUSHSGR. Add `('}', [b'#'])` arm, pop from stack, apply attrs. Add spec_chain test. Update catalog.

- [ ] **Validation**: all 14 catalog rows (11 ECMA-48 + 3 xterm) updated from `missing`/`stub` to `verified`. No Section-08-owned rows remain at `missing` or `stub`.

---

## 08.9 Populate _legacy-tack-mapping.md as rows are verified

**File(s):** `plans/spec-conformance/catalog/_legacy-tack-mapping.md`

This file was created empty in section 02. As 08.1-08.8 verify catalog rows that originated from tack scenarios, populate the mapping.

- [ ] After every catalog row drives from `implemented-unverified` to `verified` in this section, add a row to `_legacy-tack-mapping.md` linking the row ID to the original tack section.
- [ ] **Validation**: `_legacy-tack-mapping.md` has one entry per tack-originated catalog row that this section verified.

---

## 08.R Third Party Review Findings

- [x] `[TPR-08-001-codex][high]` `plans/spec-conformance/section-08-ecma-48-baseline.md:6` — Cover the catalog rows Section 08 already owns (ECMA48-SGR-53/55/73/74/75, ECMA48-CSI-DECSTR/DECSED/DECSEL/SL/SR, ECMA48-DCS-DECRQSS-DECSLRM).
  Resolved: Fixed on 2026-04-14. Added subsection 08.8b covering all 11 missing catalog rows assigned to Section 08.
- [x] `[TPR-08-002-codex][high]` `plans/spec-conformance/section-08-ecma-48-baseline.md:250` — Restrict CSI s ambiguity to zero-parameter form only; with-params dispatches directly to DECSLRM.
  Resolved: Fixed on 2026-04-14. Rewrote 08.5b to match WezTerm/Ghostty behavior: zero-param CSI s is the only ambiguous case; with-params goes directly to DECSLRM. Removed Option 2 (parameterized dispatch).
- [x] `[TPR-08-003-codex][medium]` `plans/spec-conformance/section-08-ecma-48-baseline.md:280` — Renderable rung (observe_renderable) is a stub that unconditionally returns pass; don't claim verification through it.
  Resolved: Fixed on 2026-04-14. Changed spec_chain test to use State apex (not Renderable) until rung 4 is implemented.
- [x] `[TPR-08-004-codex][medium]` `plans/spec-conformance/section-08-ecma-48-baseline.md:381` — SGR catalog row IDs don't match actual catalog; matrix incomplete.
  Resolved: Fixed on 2026-04-14. Retargeted to actual catalog rows (ECMA48-SGR-38/48/58). Completed the 12-cell matrix with all missing positive test entries.
- [x] `[TPR-08-005-codex][low]` `plans/spec-conformance/section-08-ecma-48-baseline.md:180` — ANY_MOUSE is a computed union, not a reclaimable bit. Widening to u64 is the only option.
  Resolved: Fixed on 2026-04-14. Removed the ANY_MOUSE escape hatch suggestion. Specified widening to u64 as the required approach.
- [x] `[TPR-08-001-gemini][high]` `plans/spec-conformance/section-08-ecma-48-baseline.md:180` — Same as [TPR-08-005-codex]: TermMode must be widened to u64.
  Resolved: Fixed on 2026-04-14. Same fix as [TPR-08-005-codex].
- [x] `[TPR-08-002-gemini][high]` `plans/spec-conformance/section-08-ecma-48-baseline.md:250` — Same as [TPR-08-002-codex]: CSI s ambiguity must use single handler, not parameterized dispatch.
  Resolved: Fixed on 2026-04-14. Same fix as [TPR-08-002-codex].
- [x] `[TPR-08-001-codex-r2][medium]` `section-08:15` — Stale ISO 8613-6 row IDs in frontmatter success criteria.
  Resolved: Fixed on 2026-04-14. Updated to reference actual catalog rows ECMA48-SGR-38/48/58.
- [x] `[TPR-08-002-codex-r2][medium]` `catalog/dec-private-modes.md:49` — DECLRMM ownership conflict (catalog said Section 09, plan says Section 08).
  Resolved: Fixed on 2026-04-14. Updated catalog row to Section 08 ownership; updated Section 09 context to note DECLRMM moved to Section 08.
- [x] `[TPR-08-003-codex-r2][low]` `section-08:444` — TPR-08-001 resolved note referenced nonexistent "08.5e" instead of "08.8b".
  Resolved: Fixed on 2026-04-14. Corrected to 08.8b.
- [x] `[TPR-08-001-codex-r3][high]` `catalog/xterm-ctlseqs.md:23,31,32` — Three xterm catalog rows (XT-DECSLRM, XT-PUSHSGR, XT-POPSGR) owned by Section 08 but not in 08.8b.
  Resolved: Fixed on 2026-04-14. Added all 3 xterm rows to 08.8b. Updated row count from 11 to 14 throughout.
- [x] `[TPR-08-002-codex-r3][medium]` `section-08:168` — Stale wait-for-tack-06 guidance (tack section 06 is complete).
  Resolved: Fixed on 2026-04-14. Removed wait-for-landing instructions; tack section 06 is fully complete.
- [x] `[TPR-08-001-gemini-r3][low]` `section-08:168` — Same as TPR-08-002-codex-r3 (stale tack section 06 wait).
  Resolved: Fixed on 2026-04-14. Same fix as TPR-08-002-codex-r3.
- [x] `[TPR-08-002-gemini-r3][medium]` `section-08 success criteria` — 08.8b catalog rows missing from success criteria.
  Resolved: Fixed on 2026-04-14. Added to both frontmatter and markdown success criteria blocks.
- [x] `[TPR-08-001-codex-r4][high]` `section-08 success criteria` — Basic OSC ownership ambiguity with Section 10.
  Resolved: Fixed on 2026-04-14. Clarified in success criteria that basic OSC rows are verified via tack conversion (08.1-08.2); uncovered rows defer to Section 10.
- [x] `[TPR-08-002-codex-r4][low]` `catalog/xterm-ctlseqs.md:23` — XT-DECSLRM ownership note still references Section 09/06.
  Resolved: Fixed on 2026-04-14. Updated to reference Section 08 subsection 08.5b.
- [x] `[TPR-08-001-gemini-r4][high]` `section-08 08.5d` — DECSTR missing from margin-clearing reset paths.
  Resolved: Fixed on 2026-04-14. Added DECSTR to 08.5d reset list + test.
- [x] `[TPR-08-002-gemini-r4][high]` `section-08 08.5a` — SL/SR not integrated with margin-constrained shift operations.
  Resolved: Fixed on 2026-04-14. Added SL/SR to 08.5a with tests.
- [x] `[TPR-08-003-gemini-r4][medium]` `section-08 08.4` — HT/CBT not in margin-constrained movement list.
  Resolved: Fixed on 2026-04-14. Added HT/CBT to 08.4 with tests.
- [x] `[TPR-08-004-gemini-r4][low]` `section-08 08.N` — Missing matrix dimensions for 08.8b.
  Resolved: Fixed on 2026-04-14. Added 08.8b matrix to 08.N.
- [x] `[TPR-08-001-codex-r5][medium]` `oriterm_core/tests/spec_chain/baseline/tack_section_06/status_reports_inventory.rs:50,126` — LEAK: Drive the actual DA2 and DA3 fixture bytes through spec_chain.
  Resolved: Fixed on 2026-04-14. Added `da2_query_explicit_zero_param_drives_to_effect_apex`, `da3_query_explicit_zero_param_drives_to_effect_apex`, and `da2_explicit_and_implicit_zero_replies_match` to pin the verbatim `CSI > 0 c` / `CSI = 0 c` fixture forms and prove explicit/implicit-zero equivalence.
  Evidence: Tack fixtures model DA2/DA3 as `CSI > 0 c` / `CSI = 0 c` (explicit-0 param) per `crates/oriterm_test_support/src/tack_framework/scenarios/status_reports/tests.rs:25-27`, but the conversion hardcodes normalized `\x1b[>c` / `\x1b[=c` (implicit-0).
  Impact: A regression in explicit-0 DA2/DA3 handling could hide behind a green 08.2 conversion; the test stops being a faithful replay of the tack scenario.
  Required plan update: Add matrix cells for `CSI > 0 c` and `CSI = 0 c` (explicit-0 form) alongside the existing implicit-0 tests.
  Basis: direct_file_inspection. Confidence: high.
- [x] `[TPR-08-002-codex-r5][medium]` `plans/spec-conformance/catalog/ecma-48.md:49-50` — DRIFT: Split the charset designation rows or cover the missing charset cells.
  Resolved: Fixed on 2026-04-14. Added G2/G3 coverage (`esc_g2_dec_special_graphics_designates_without_panic`, `esc_g3_dec_special_graphics_designates_without_panic`, `esc_g2_ascii_designates_without_panic`, `esc_g3_ascii_designates_without_panic`) and a negative pin (`esc_g1_dec_graphics_is_inert_before_so`) proving `ESC ) 0` is inert on G0 rendering until SO fires. Updated catalog cells for `ECMA48-ESC-B` and `ECMA48-ESC-0` to cite the G2/G3 tests.
  Evidence: `ECMA48-ESC-B` and `ECMA48-ESC-0` are now marked `verified` for `ESC ( / ) / * / +` (all four banks G0–G3), but `character_sets.rs` only exercises `ESC ( 0`, `ESC ) 0`, and `ESC ( B`. The module lacks a negative pin proving `ESC ) 0` remains inert until SO activates G1.
  Impact: Verification metadata overstates what 08.2 pinned — bugs in G2/G3 designation or the "designated-but-not-active" path can ship while the row reads as fully verified.
  Required plan update: Add spec_chain tests for `ESC * 0` (G2), `ESC + 0` (G3), `ESC * B` (G2 ASCII), `ESC + B` (G3 ASCII); add negative pin proving `ESC ) 0` without SO keeps ASCII glyph rendering.
  Basis: direct_file_inspection. Confidence: high.
- [x] `[TPR-08-003-codex-r5][low]` `oriterm_core/tests/spec_chain/baseline/tack_section_06/{tools_menu_inventory,status_reports,enq_ack}.rs` — GAP: Replace zero-row stub no-op tests with meaningful assertions.
  Resolved: Fixed on 2026-04-14. Deleted empty `#[test]` functions from `tools_menu_inventory.rs` and `status_reports.rs` (pure documentation modules now — rustdoc-only). Replaced `enq_ack.rs` empty test with `ecma48_c0_enq_catalog_row_still_missing_pending_bug_08_6` — a load-bearing regression guard that reads the catalog and fails when ENQ's status flips away from `missing`, forcing the BUG-08-6 implementer to land the real spec_chain test here.
  Evidence: All three stub files contain bare passing `#[test]` shells with no assertions — violates `.claude/rules/tests.md` §Test Hygiene Rule 1 ("every test file must contain at least one assertion"). The BUG-08-6 citation in `enq_ack.rs` is documentation only, not a load-bearing guard.
  Impact: Files can never fail if their documented rationale drifts; they don't actually protect the zero-row decisions.
  Required plan update: Delete empty `#[test]` functions from `tools_menu_inventory.rs` and `status_reports.rs` (pure documentation modules); replace `enq_ack.rs` empty test with a regression guard asserting `ECMA48-C0-ENQ` catalog row still has `status: missing` (forces reopening this file when BUG-08-6 is fixed).
  Basis: direct_file_inspection. Confidence: high.
- [x] `[TPR-08-001-gemini-r5][medium]` `oriterm_core/tests/spec_chain/baseline/tack_section_06/tools_menu_inventory.rs:22` — GAP: Empty tests violate No Orphan Tests hygiene rule.
  Resolved: Fixed on 2026-04-14. Same fix as [TPR-08-003-codex-r5]. Both reviewers flagged the same hygiene violation; the single fix addresses both.
  Evidence: Dummy `#[test]` functions with empty bodies in `tools_menu_inventory.rs`, `status_reports.rs`, `enq_ack.rs` violate `.claude/rules/tests.md` §Test Hygiene Rule 1.
  Impact: Fails test hygiene standard; empty tests provide false confidence.
  Required plan update: Same as [TPR-08-003-codex-r5] (overlaps on diagnosis; single fix covers both).
  Basis: direct_file_inspection. Confidence: high.
- [x] `[TPR-08-002-gemini-r5][low]` `oriterm_core/tests/spec_chain/baseline/tack_section_06/status_reports_inventory.rs:221` — GAP: Missing negative clamp for DSR-6.
  Resolved: Fixed on 2026-04-14. Added `dsr_6_does_not_emit_device_status` negative pin that asserts `CSI 6 n` must not emit a `DeviceStatus` effect, symmetric to the pre-existing `dsr_5_does_not_emit_cursor_report`.
- [x] `[TPR-08-001-codex-r6][medium]` `plans/spec-conformance/catalog/ecma-48.md:49-50` — Align the ESC-B and ESC-0 catalog citations with the actual G-bank tests (round 2 follow-up).
  Evidence: ECMA48-ESC-B's `state:` column cited `esc_g3_ascii_designates_without_panic` which uses `ApexLayer::Dispatch` (only runs parser+dispatch rungs — not a state pin). ESC ) B (G1 ASCII) had no spec_chain test. ECMA48-ESC-0's new G3 test was not cited in its row.
  Resolved: Fixed on 2026-04-14. (1) Added `esc_g1_ascii_designates_without_panic` spec_chain test to cover G1 ASCII designation. (2) Corrected ECMA48-ESC-B citations so the `state:` column cites `esc_g0_ascii_round_trip` (the only ASCII test with `ApexLayer::State`) and added a note row cataloging all four per-G-bank pins. (3) Corrected ECMA48-ESC-0 citations so the `state:` column cites state-apex tests and the prose cites the G3 test alongside G2.
  Evidence: The matrix has `dsr_5_does_not_emit_cursor_report` negative pin, but DSR-6 lacks a symmetrical negative clamp proving it doesn't mistakenly emit a `DeviceStatus` effect.
  Impact: Incomplete matrix squeeze — a regression routing `CSI 6 n` to `DeviceStatus` could pass the positive pins.
  Required plan update: Add `dsr_6_does_not_emit_device_status` negative pin to `status_reports_inventory.rs`.
  Basis: direct_file_inspection. Confidence: high.

---

## 08.N Completion Checklist

- [ ] Failing test matrix written FIRST (TDD) for each subsection
- [ ] **Matrix dimensions documented**:
  - Tack conversion: tack scenario x catalog row x verification rung
  - DECLRMM mode plumbing: mode operation (set/reset/query) x sync-point (flag/handler/reporting)
  - DECLRMM grid enforcement: cursor operation (CUF/CUB/CHA/CR/CUP/NEL/IND/RI/wrap/reverse-wrap/HT/CBT) x margin state (active/inactive) x cursor position (inside/outside margin band)
  - DECLRMM extended: edit operation (IL/DL/ICH/DCH/SL/SR) x margin state x content-outside-margin-survives
  - DECLRMM reset paths: reset trigger (DECRST-69/DECCOLM/RIS/DECSTR/DECALN/resize) x margin-cleared
  - CSI s ambiguity: form (zero-param/with-params) x mode 69 state (on/off)
  - C1 controls: C1 byte (0x90/0x98/0x9B/0x9C/0x9D/0x9E/0x9F) x context (ground/mid-sequence)
  - REP edge cases: preceding state (none/CR/wide/SGR-change/at-margin) x count (0/1/N)
  - SGR colon forms: color target (38/48/58) x color mode (2/5) x separator (semicolon/colon)
  - 08.8b remaining rows: operation (SGR 53/55/73/74/75, DECSTR, DECSED, DECSEL, SL, SR, DECRQSS-DECSLRM, PUSHSGR, POPSGR) x margin-state (active/inactive where applicable)
- [ ] **Semantic pins**: DECLRMM cursor-constrained tests, 8-bit C1 state-transition tests, and colon-separator tests are the regression guards for new behavior
- [ ] **Negative pins**: CSI s with params when mode 69 inactive, BSU/ESU 7-bit-only scope, mixed separator failure mode, empty subparam indistinguishability, cursor outside margin band not constrained
- [ ] Tack section 05 scenarios converted to spec_chain tests
- [ ] Tack section 06 scenarios converted (all subsections complete and landed)
- [ ] DECLRMM mode plumbing complete (VTE types, TermMode flag, mode reporting)
- [ ] DECLRMM grid enforcement complete (margin fields, cursor movement, wrap behavior)
- [ ] DECLRMM extended operations complete (partial-width scroll, CSI s ambiguity, save/restore, reset paths)
- [ ] 8-bit C1 controls detected and verified
- [ ] REP edge cases verified
- [ ] ISO 8613-6 colon-separated SGR subparameter forms verified (38/48/58, both `2` truecolor and `5` indexed variants)
- [ ] Mixed separator and empty subparam negative pins documented
- [ ] BSU/ESU 7-bit-only NOTE pin documented
- [ ] Remaining Section-08-owned catalog rows verified (SGR 53/55/73/74/75, DECSTR, DECSED, DECSEL, SL, SR, DECRQSS-DECSLRM, XT-DECSLRM, XT-PUSHSGR, XT-POPSGR)
- [ ] `_legacy-tack-mapping.md` populated
- [ ] All existing tack tests pass without modification
- [ ] All existing teseq tests pass without modification
- [ ] Alloc regression unchanged
- [ ] `handler/mod.rs` still under 500 lines (or extracted to submodule)
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` -> `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 08 status updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** ECMA-48 baseline catalog row subset is `verified`; DECLRMM full mode plumbing + grid enforcement + extended operations verified; CSI s / DECSLRM ambiguity resolved (zero-param form only per WezTerm/Ghostty); save/restore includes margin state; reset paths clear margins; 8-bit C1 controls verified; REP edge cases verified; ISO 8613-6 SGR colon forms verified with negative pins for known limitations; all Section-08-owned catalog rows (SGR 53/55/73/74/75, DECSTR, DECSED, DECSEL, SL, SR, DECRQSS-DECSLRM, XT-DECSLRM, XT-PUSHSGR, XT-POPSGR) verified; legacy tack mapping populated; existing tack + teseq tests still pass; ready for Phase 3 stacks to depend on baseline correctness.
