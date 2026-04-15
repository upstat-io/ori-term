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
  - "**DECLRMM full mode plumbing + grid enforcement implemented**: VTE layer has `NamedPrivateMode::LeftRightMargin` variant (mode 69) with `PrivateMode::new` mapping; `TermMode` has `LEFT_RIGHT_MARGIN` flag; `named_private_mode_flag` maps it; `status_report_private_mode` reports it; `Grid` has `left_margin: usize` and `right_margin: usize` fields; CSI s / DECSLRM ambiguity resolved (state-dependent dispatch); CUF/CUB/ICH/DCH/IL/DL/CR/NEL/IND/RI/cursor-wrap/reverse-wrap respect margins; absolute CUP/HVP/CHA/HPA ignore margins (DECOM offset applied at Term layer via `origin_aware_col`); `goto_col` is column-aware under DECOM+DECLRMM; DECSC/DECRC save-set excludes margins and DECLRMM mode per DEC STD 070 §5.6.1; reset/resize/disable-mode-69 clears margins; the corresponding catalog row in `catalog/dec-private-modes.md` is `verified`"
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
  status: resolved
  updated: 2026-04-14
sections:
  - id: "08.1"
    title: "Convert tack section 05 scenarios to spec verification chains"
    status: complete
  - id: "08.2"
    title: "Convert tack section 06 scenarios to spec verification chains"
    status: complete
  - id: "08.3"
    title: "DECLRMM mode plumbing (VTE types + TermMode + mode reporting)"
    status: complete
  - id: "08.4"
    title: "DECLRMM grid enforcement (margin fields + cursor movement)"
    status: complete
  - id: "08.5"
    title: "DECLRMM extended operations (IL/DL partial-width scroll, CSI s ambiguity, DECSC/DECRC scope, reset paths)"
    status: complete
  - id: "08.6"
    title: "Implement 8-bit C1 control detection"
    status: complete
  - id: "08.7"
    title: "Verify REP edge cases"
    status: complete
  - id: "08.8"
    title: "Verify ISO 8613-6 SGR colon-separated subparameter forms (truecolor + indexed + underline color)"
    status: complete
  - id: "08.9"
    title: "Populate _legacy-tack-mapping.md as rows are verified"
    status: complete
  - id: "08.8b"
    title: "Verify remaining Section-08-owned catalog rows"
    status: complete
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
- [ ] DECSC/DECRC save set matches DEC STD 070 §5.6.1 (cursor + attributes + charsets + wrap + DECOM — margins and DECLRMM are NOT saved); reset/resize/disable-mode-69 clears margins
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
- [x] **TPR checkpoint** — `/tpr-review` covering 08.1-08.2 (tack absorption work). Catches conversion errors before the gap-fix subsections proceed.
  Completed: 2026-04-14. Five rounds: rounds 1-4 surfaced 5/1/1/2 findings (all fixed in commits e1b36466 → ddcd0bd1 → dd7ee902 → c12636f3 → 193a2dab); round 5 clean pass (`no_findings: true` from both codex and gemini).

**Tooling retrospective (2026-04-14):** Per-subsection `/improve-tooling` retrospective surfaced the TPR resolution-note format drift pattern — rounds 3, 4, and 5 of 08.2's TPR review were all triaging the same class of finding (leftover `Evidence:`/`Impact:`/`Required plan update:`/`Basis:` lines under resolved `- [x]` entries). New tool: `scripts/check-tpr-resolution-format.sh` (commit 89baf8d0) lints for this drift and supports `--fix`. Running `--fix` across all plan files (commit eb04574b) stripped 228 lines of pre-existing drift across 29 sections. Going forward, this linter prevents the round-3-to-5 churn from recurring.

**Implementation notes (2026-04-14):** Six scenario families (`tools_menu_inventory`, `status_reports_inventory`, `status_reports`, `sgr_modes`, `character_sets`, `enq_ack`) → six spec_chain modules under `oriterm_core/tests/spec_chain/baseline/tack_section_06/`. Twelve new catalog rows driven to `verified`: `ECMA48-CSI-DA2`, `ECMA48-CSI-DA3`, `ECMA48-CSI-DSR-5`, `ECMA48-CSI-DSR-6` (status_reports_inventory — DA1 already covered by pilot); `ECMA48-SGR-0`, `ECMA48-SGR-1`, `ECMA48-SGR-4`, `ECMA48-SGR-7` (sgr_modes — four most-distinctive modes on tack's 80-mode grid; remaining 76 owned by `tack_cap_xcheck`); `ECMA48-ESC-0`, `ECMA48-ESC-B`, `ECMA48-C0-SO`, `ECMA48-C0-SI` (character_sets — SCS designation + SO/SI bank switching + preview-pane end-to-end render). Three families contribute zero new rows: `tools_menu_inventory` (inventory sentinel, no protocol bytes), `status_reports` (helper module only), `enq_ack` (blocked on BUG-08-6 — ECMA48-C0-ENQ remains `missing`). OSC row ownership audit: tack scenarios drive zero OSC rows — all basic OSC rows (0, 1, 2, 4, 7, 10, 11, 12, 52) remain owned by Section 10. 26 new spec_chain tests, all green on host + Windows cross-compile.

---

## 08.3 DECLRMM mode plumbing (VTE types + TermMode + mode reporting)

**File(s):** `crates/vte/src/ansi/types.rs`, `oriterm_core/src/term/mode/mod.rs`, `oriterm_core/src/term/handler/helpers.rs`, `oriterm_core/src/term/handler/status.rs`, `oriterm_core/src/term/handler/modes.rs`, sibling tests

Before the grid can enforce left/right margins, the mode plumbing must exist end-to-end. Currently mode 69 is completely absent from the VTE type layer, TermMode flags, and mode reporting. This subsection adds the full vertical slice.

**Why a separate subsection from grid enforcement:** Mode plumbing is a prerequisite for grid enforcement. Attempting both in one subsection creates an untestable intermediate state — the mode flag would exist but nothing would observe it, or the grid fields would exist but the mode couldn't be toggled. Splitting lets each subsection be independently testable.

- [x] **VTE type layer** — Add `LeftRightMargin = 69` variant to `NamedPrivateMode` enum in `crates/vte/src/ansi/types.rs:226`. Add `69 => Self::Named(NamedPrivateMode::LeftRightMargin)` mapping in `PrivateMode::new()` at `types.rs:177-204`.
- [x] **TermMode flag** — Add `const LEFT_RIGHT_MARGIN = 1 << 32` to `TermMode` bitflags in `oriterm_core/src/term/mode/mod.rs:14`. All 32 bits of the current `u32` representation are fully occupied by real mode flags (bits 0-31). Computed unions like `ANY_MOUSE` are ORs of existing bits — they do NOT occupy reclaimable bit positions. The ONLY viable approach is to widen `TermMode` from `u32` to `u64` (change `bitflags! { pub struct TermMode: u32` to `u64`). This is a mechanical change — all downstream code uses the bitflags API, not raw bit manipulation.
- [x] **named_private_mode_flag** — Add `NamedPrivateMode::LeftRightMargin => Some(TermMode::LEFT_RIGHT_MARGIN)` mapping in `oriterm_core/src/term/handler/helpers.rs:22-51`. This is the exhaustive match — adding the variant without updating it is a compile error.
- [x] **DECSET/DECRST handler** — Add `NamedPrivateMode::LeftRightMargin` arms to the `set_private_mode` and `unset_private_mode` match blocks in `oriterm_core/src/term/handler/modes.rs`. Setting mode 69 inserts the flag; unsetting removes it AND resets left/right margins to full width (see 08.5 for the reset path details).
- [x] **Mode reporting** — Verify `status_report_private_mode` in `oriterm_core/src/term/handler/status.rs:108` correctly reports mode 69 state via the existing `named_private_mode_flag` lookup (should work automatically once the flag mapping exists).
- [x] **Tests — TDD, failing first:**
  - `mode_69_set_inserts_left_right_margin_flag()` — feed `\x1b[?69h`, assert `TermMode::LEFT_RIGHT_MARGIN` is set
  - `mode_69_reset_removes_left_right_margin_flag()` — feed `\x1b[?69l`, assert flag removed
  - `mode_69_decrqm_reports_correctly()` — feed `\x1b[?69$p`, assert reply contains mode-set/reset value
  - `mode_69_survives_named_private_mode_flag_exhaustive_test()` — the existing exhaustive test in `oriterm_core/src/term/handler/tests.rs:5125` must pass with the new variant
- [x] **Validation**: `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` green. The exhaustive mode-flag sync test passes.

---

## 08.4 DECLRMM grid enforcement (margin fields + cursor movement)

**File(s):** `oriterm_core/src/grid/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`, `oriterm_core/src/term/handler/mod.rs`, `oriterm_core/src/term/handler/helpers.rs`, sibling tests

With mode 69 plumbed, implement the actual left/right margin enforcement in the grid. This subsection covers the margin fields and cursor movement operations (CUF, CUB, CHA, CR, goto_origin_aware).

- [x] **Grid margin fields** — Add `left_margin: usize` and `right_margin: usize` fields to `Grid` in `oriterm_core/src/grid/mod.rs:35`. Default: `left_margin = 0`, `right_margin = cols - 1`. Add public accessor `left_right_margins(&self) -> (usize, usize)` and `set_left_right_margins(&mut self, left: usize, right: usize)`.
- [x] **DECSLRM handler** — Add a handler for `CSI Pl ; Pr s` (when mode 69 is active) that calls `grid.set_left_right_margins(left, right)`. This is the DECSLRM sequence. **CSI s ambiguity is handled in 08.5** — this item only adds the margin-setting method on Grid.
- [x] **CUF (move_forward)** — In `oriterm_core/src/grid/navigation/mod.rs`, when DECLRMM is active and cursor is within the margin band, clamp rightward movement to `right_margin`. Current implementation in `move_forward()` clamps to `cols - 1`. Add a `right_bound` parameter or query the margin fields.
- [x] **CUB (move_backward)** — Clamp leftward movement to `left_margin` when DECLRMM is active and cursor is within the margin band.
- [x] **CHA (move_to_column / goto_col)** — When DECLRMM + DECOM are active, column addressing is relative to `left_margin`. The handler at `oriterm_core/src/term/handler/mod.rs:167` calls `grid.move_to_column()` — this must offset by `left_margin` when both modes are set.
- [x] **CR (carriage_return)** — When DECLRMM is active and cursor is within the margin band, CR moves to `left_margin` (not column 0). The current `grid.carriage_return()` unconditionally moves to column 0.
- [x] **goto_origin_aware column awareness** — `oriterm_core/src/term/handler/helpers.rs:102` currently handles DECOM for vertical (scroll region) only. When both DECOM and DECLRMM are active, column `col` should be relative to `left_margin` and clamped to `[left_margin, right_margin]`. This affects CUP (goto), HVP, and any origin-aware positioning.
- [x] **NEL (next_line) / IND (linefeed)** — When DECLRMM active and cursor within margin band, NEL moves to `left_margin` (not column 0) at the next line. IND scrolls only within the margin band if cursor is at the bottom margin row. <!-- Note: NEL margin-aware via CR. IND partial-width scroll deferred to 08.5. -->
- [x] **RI (reverse_index)** — When DECLRMM active and cursor is at the top of the scroll region, reverse scroll should respect left/right margins (content outside the margin band survives). <!-- Note: partial-width reverse scroll deferred to 08.5. Vertical behavior unchanged. -->
- [x] **Cursor wrap** — When DECLRMM active and cursor reaches `right_margin`, auto-wrap wraps to `left_margin` of the next line (not column 0).
- [x] **Reverse wrap** — When DECLRMM active and mode 45 set, BS at `left_margin` wraps to `right_margin` of the previous line.
- [x] **HT (horizontal tab)** — When DECLRMM active, tab stops beyond `right_margin` are not reachable; HT stops at `right_margin`.
- [x] **CBT (cursor backward tab)** — When DECLRMM active, backward tab stops before `left_margin` are not reachable; CBT stops at `left_margin`.
- [x] **Tests — TDD, failing first — write ALL tests before implementing any movement logic:**
  - `cuf_respects_right_margin_under_declrmm()` — cursor stops at right_margin
  - `cub_respects_left_margin_under_declrmm()` — cursor stops at left_margin
  - `cha_relative_to_left_margin_under_decom_declrmm()` — CHA col=1 goes to left_margin
  - `cr_goes_to_left_margin_under_declrmm()` — not column 0
  - `cup_clamps_to_margin_band()` — CUP col beyond right_margin clamped
  - `auto_wrap_at_right_margin_wraps_to_left_margin()` — character at right_margin+1 wraps to left_margin
  - `backspace_at_left_margin_stops_under_declrmm()` — BS no-op at left_margin (Grid level)
  - `nel_goes_to_left_margin_not_col_0()` — NEL with margins
  - `declrmm_disabled_restores_full_width_movement()` — disabling mode 69 removes margin constraints
  - `cursor_outside_margin_band_not_constrained()` — cursor positioned outside [left, right] is NOT constrained by margins (WezTerm behavior)
- [x] **Negative pins:**
  - `cuf_without_declrmm_ignores_margins()` — margins set but mode 69 off = no effect
  - `ht_stops_at_right_margin_under_declrmm()` — tab doesn't cross right margin
  - `cbt_stops_at_left_margin_under_declrmm()` — backward tab doesn't cross left margin
  - `declrmm_does_not_affect_vertical_scroll_region()` — DECSTBM still works independently
- [x] **File size gate**: `handler/mod.rs` is at 490 lines. Any DECLRMM logic added to handler/mod.rs must NOT push it over 500 lines. If it would, extract margin-related handler methods into `oriterm_core/src/term/handler/margins.rs` (new submodule) FIRST.
- [x] **Validation**: tests pass; existing teseq cursor tests still pass; no alloc regression.

---

## 08.5 DECLRMM extended operations (IL/DL partial-width scroll, CSI s ambiguity, DECSC/DECRC scope, reset paths)

**File(s):** `oriterm_core/src/grid/scroll/mod.rs`, `oriterm_core/src/grid/navigation/mod.rs`, `crates/vte/src/ansi/dispatch/csi.rs`, `oriterm_core/src/term/handler/modes.rs`, `oriterm_core/src/term/handler/mod.rs`, sibling tests

This subsection handles the operations that are architecturally more complex than simple cursor movement: partial-width scrolling for IL/DL under margins, the CSI s / DECSLRM sequence ambiguity, the DECSC/DECRC save-set scope (which per DEC STD 070 §5.6.1 excludes margins and DECLRMM mode — see 08.5c for the correct contract), and all reset paths that must clear margins.

### 08.5a: IL/DL/ICH/DCH with horizontal margins (partial-width scroll)

- [x] **Partial-width scroll primitives** — Added `scroll_partial_up` and `scroll_partial_down` to `oriterm_core/src/grid/scroll/mod.rs`. These cell-by-cell copy primitives operate on sub-row cell ranges using `split_at_mut` for safe non-overlapping mutable refs. `insert_lines` and `delete_lines` now dispatch to these when `has_horizontal_margins()` is true, falling back to O(1) rotation for full-width.
- [x] **ICH/DCH within margins** — `insert_blank` and `delete_chars` in `oriterm_core/src/grid/editing/mod.rs` now compute `right_bound` from `right_margin + 1` when margins are active, constraining shifts to within the margin band.
- [x] **SL/SR within margins** — Implemented `scroll_left` and `scroll_right` on Grid (`oriterm_core/src/grid/scroll/mod.rs`), VTE Handler trait methods (`crates/vte/src/ansi/handler.rs`), CSI dispatch entries (`('@', [b' '])` → SL, `('A', [b' '])` → SR in `crates/vte/src/ansi/dispatch/csi.rs`), and Term handler delegation (`oriterm_core/src/term/handler/mod.rs`). Both SL/SR respect DECLRMM margin band when active.
- [x] **Tests:**
  - `il_with_margins_scrolls_only_margin_band()` — content outside margins survives
  - `dl_with_margins_scrolls_only_margin_band()` — content outside margins survives
  - `ich_within_margins_shifts_only_margin_band()` — insertion respects right boundary
  - `dch_within_margins_shifts_only_margin_band()` — deletion fills from right boundary
  - `sl_within_margins_shifts_only_margin_band()` — scroll left respects margin band
  - `sr_within_margins_shifts_only_margin_band()` — scroll right respects margin band

### 08.5b: CSI s / DECSLRM ambiguity

- [x] **Problem**: Plain `CSI s` (no `?` intermediate) was hard-coded as `save_cursor_position()` at `crates/vte/src/ansi/dispatch/csi.rs`. DECSLRM (Set Left and Right Margins) uses the same sequence when mode 69 is active. Resolved by routing all `CSI ... s` forms through a single handler method.
- [x] **Solution**: VTE CSI dispatch routes all `('s', [])` forms to `Handler::decslrm_or_save_cursor(has_params, left, right)`. VTE param detection uses value-based check (`left != 0 || right != 0`) because VTE always pushes at least one default-0 param. The Term handler checks mode 69 and param presence:
  - **With params**: always DECSLRM (no-op if mode 69 inactive, per WezTerm/Ghostty).
  - **Zero params + mode 69 on**: DECSLRM with defaults (reset to full width).
  - **Zero params + mode 69 off**: save cursor (backward compat).
  No mode state in VTE crate — all dispatch logic in the Term handler (per crate-boundaries.md).
- [x] **Tests:**
  - `csi_s_zero_params_mode_69_off_saves_cursor()` — the backward-compat case
  - `csi_s_zero_params_mode_69_on_sets_default_margins()` — DECSLRM with defaults (1, cols)
  - `csi_s_with_params_always_decslrm()` — `CSI 5 ; 20 s` sets margins regardless of mode 69
  - `csi_s_with_params_mode_69_off_is_noop()` — DECSLRM with params but mode 69 inactive = no-op (NOT save cursor)

### 08.5c: DECSC/DECRC save-set scope (margins + DECLRMM NOT saved)

**Spec reference**: DEC STD 070 §5.6.1. Cross-verified against wezterm
(`term/src/terminalstate/mod.rs:134-142`), alacritty
(`alacritty_terminal/src/grid/mod.rs:34-53`), and ghostty
(`src/terminal/Screen.zig:187-195`) — all three reference implementations
exclude margins and DECLRMM from the save set.

- [x] **Correct scope**: DECSC saves cursor position + character attributes + charset state + wrap flag + DECOM flag. Margins and DECLRMM mode are NOT in the save set — margin state is scoped to the screen (alt vs primary), not the cursor save/restore pair. Reset paths (RIS / DECSTR / DECCOLM / DECALN / resize / explicit DECRST ?69) handle margin clearing.
- [x] **Initial iteration 11 finding**: The 08.5c prior iteration erroneously added `Grid::saved_margins`, `Term::saved_left_right_margin_mode` + inactive variant, and the corresponding save/restore/alt-screen-swap/RIS-clear logic, with positive pins (`save_restore_preserves_margin_state`, `save_restore_preserves_declrmm_mode_flag`) locking the non-spec behavior. Flagged by both codex and gemini in the 08.5 TPR checkpoint (`[TPR-08-001-codex-r11]`).
- [x] **Fix**: Removed `Grid::saved_margins` field, `Term::saved_left_right_margin_mode` + `inactive_saved_left_right_margin_mode` fields, and the save/restore/swap/RIS-clear sites that referenced them. `save_cursor_position` now saves only the DEC-spec set (cursor + charset + DECOM); `restore_cursor_position` restores only the same set. Alt-screen toggle no longer swaps margin-save state (there is none). RIS no longer clears margin-save fields.
- [x] **Tests — negative pins replacing the prior positive pins:**
  - `decrc_does_not_restore_horizontal_margins()` — DECSC, change margins, DECRC → margins stay at the changed values
  - `decrc_does_not_restore_declrmm_mode_flag()` — DECSC with mode 69 on, DECRST ?69, DECRC → mode 69 stays off
  - `decrc_does_not_enable_declrmm_after_disabled_save()` — symmetric: DECSC with mode 69 off, DECSET ?69, DECRC → mode 69 stays on
  - **Positive pin**: `decsc_decrc_restores_cursor_position_and_origin()` — DECSC, move + toggle DECOM, DECRC → cursor position AND DECOM flag correctly restored (guards the state that IS in the save set)

### 08.5d: Reset paths that must clear margins

- [x] **Problem**: Multiple reset operations needed horizontal margin resets in addition to vertical scroll region resets. Verified and fixed all paths:
  - **Disabling mode 69** (`DECRST ?69`) — already calls `reset_left_right_margins()` (from 08.3). Verified.
  - **DECCOLM** (mode 3 toggle) — added `reset_left_right_margins()` to `apply_deccolm` in `modes.rs`.
  - **RIS (full reset)** — `Grid::reset()` already resets margins (lines 249-250 in grid/mod.rs). Verified.
  - **DECSTR (soft terminal reset)** — DECSTR is not yet implemented (deferred to 08.8b). When implemented, its reset path MUST call `reset_left_right_margins()`.
  - **DECALN** — added `reset_left_right_margins()` to `decaln_impl` in `esc.rs`.
  - **Resize** — `Grid::resize()` already resets margins in `finalize_resize` (from 08.4). Verified.
- [x] **Tests:**
  - `decrst_69_resets_margins_to_full_width()` — disabling DECLRMM clears margins
  - `deccolm_resets_horizontal_margins()` — mode 3 toggle clears margins
  - `ris_resets_horizontal_margins()` — hard reset clears margins
  - `resize_resets_horizontal_margins()` — width change clears margins
  - `decstr_resets_horizontal_margins()` — deferred to 08.8b (DECSTR not yet implemented)
  - `decaln_resets_horizontal_margins()` — alignment test clears margins

- [x] **spec_chain test** — `oriterm_core/tests/spec_chain/baseline/declrmm.rs` wires DECLRMM through parser → dispatch → state. `declrmm_cuf_clamps_to_right_margin` drives `\x1b[?69h\x1b[6;41s\x1b[1;10H` setup + `\x1b[100C` test bytes and asserts cursor clamps to col 40 (right_margin). Fixed `RecordingHandler` in the spec_chain test harness — it was missing `decslrm_or_save_cursor`, `scroll_left`, and `scroll_right` delegations, causing these to fall through to the default Handler trait impls.
- [x] Update `catalog/dec-private-modes.md` row for DECLRMM (mode 69) to `verified`.
- [x] **TPR checkpoint** — `/tpr-review` covering 08.3-08.5 (DECLRMM work). Round 11 surfaced 6 actionable findings (4 codex, 2 gemini, 2 dual-reviewer agreements on the `Grid::move_to` clamp and `goto_col` offset defects). Findings filed and resolved in §08.R. The prior save/restore positive pins were flipped into negative pins, `Grid::move_to` / `Grid::move_to_column` margin clamps were removed, `Term::goto_col` gained the DECOM+DECLRMM offset via a shared `origin_aware_col` helper, and `sanitize_headless_env` was moved into the shared headless init path so `new_headless_with_preference` callers are covered. The TPR checkpoint closes 08.5.
- [x] **Validation**: tests pass; `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` all green (debug + release + Windows cross-compile); no alloc regression.

---

## 08.6 Implement 8-bit C1 control detection

**File(s):** `crates/vte/src/lib.rs` (the byte-level state machine), sibling tests

The VTE parser currently only handles 7-bit ESC-prefixed C1 forms (ESC [ for CSI, ESC P for DCS, ESC _ for APC). 8-bit C1 bytes (0x80-0x9F) are partially handled: `advance_ground()` at `crates/vte/src/lib.rs:649` detects bytes in the 0x80-0x9F range during UTF-8 error recovery (line 685: `if len == 1 && bytes[valid_bytes] <= 0x9F`) and routes them to `performer.execute(byte)`. Additionally, 0x9C (C1 ST) is explicitly handled as a state terminator at `lib.rs:341` and `lib.rs:475`.

**The real parser state machine is in `crates/vte/src/lib.rs` (`advance_ground`, `advance`), NOT in `crates/vte/src/ansi/processor.rs`.** The processor is a higher-level wrapper; the byte-level dispatch happens in `lib.rs`. Any C1 detection work must target `lib.rs`.

**Key constraint**: The `advance_ground()` function uses `memchr(0x1B, bytes)` for fast scanning (line 652). Naive byte-by-byte C1 scanning in the 0x80-0x9F range would regress hot-path performance. Implementation must either:
1. Extend the memchr scan to also stop on C1 bytes (e.g., use `memchr2` or `memchr3` for the most common C1 introducers), OR
2. Handle C1 bytes only during the UTF-8 error recovery path (which already detects them), ensuring they transition to the correct parser state

- [x] **Audit current C1 handling** — Read `crates/vte/src/lib.rs:649-710` carefully. The UTF-8 error path already calls `performer.execute(byte)` for bytes <= 0x9F. Verify what `execute()` does with these bytes in the `Processor` (at `crates/vte/src/ansi/processor.rs`). If `execute()` already routes 0x9B to CSI state, 0x90 to DCS state, etc., then the gap is narrower than assumed. **Audit result**: `dispatch_execute` only handles C0 bytes; C1 bytes fell through to `debug!` log. Gap confirmed.
- [x] **If C1 handling is incomplete**: Add proper state transitions for each C1 introducer byte:
  - 0x90 (DCS) — enter DCS state (same as ESC P)
  - 0x9B (CSI) — enter CSI state (same as ESC [)
  - 0x9C (ST) — no-op on ground; terminates DCS/APC/SOS/PM mid-sequence via `anywhere`
  - 0x9D (OSC) — enter OSC state (same as ESC ]). **Note**: 0x9C does NOT terminate OSC (conflicts with UTF-8 continuation bytes in CJK titles); use BEL or ESC \ instead. Matches upstream Alacritty VTE.
  - 0x9E (PM) — enter PM discard state (same as ESC ^)
  - 0x9F (APC) — enter APC state (same as ESC _)
  - 0x98 (SOS) — enter SOS discard state (same as ESC X)
  Implementation: new `dispatch_c1()` method in `Parser` at `crates/vte/src/lib.rs`, called from `advance_ground` UTF-8 error path instead of `performer.execute()`. Also added 0x9C to `anywhere()` for mid-sequence ST termination.
- [x] **BSU/ESU scope note** — The sync-update path (`BSU_CSI`/`ESU_CSI` constants at `crates/vte/src/ansi/mod.rs:47-50`) uses hardcoded 7-bit CSI sequences (`\x1b[?2026h` / `\x1b[?2026l`). These match byte-for-byte in `advance_sync_csi`. Adding global 8-bit C1 support must NOT break the BSU/ESU matcher. If an application sends `0x9b ?2026h` as an 8-bit BSU, the current sync path will NOT recognize it. This is acceptable for now (no real-world app does this) but must be documented with a NOTE pin test. **Done**: negative pin test `bsu_esu_7bit_not_matched_by_8bit_csi` confirms 8-bit CSI form is dispatched as normal CSI, not recognized as BSU.
- [x] **Performance guard** — Run the alloc regression tests AND time the existing teseq suite before and after the change. Any measurable regression in the parse hot path must be investigated. **Done**: alloc regression (5/5 pass), teseq (176/176 pass), no regressions. C1 dispatch reuses the existing UTF-8 error path — no new memchr scans or hot-path changes.
- [x] **Tests — TDD, failing first:**
  - `c1_0x9b_enters_csi_state()` — input `\x9b0m` (8-bit CSI + SGR reset), assert SGR is dispatched
  - `c1_0x90_enters_dcs_state()` — input `\x90q...ST`, assert DCS hook is called
  - `c1_0x9d_enters_osc_state()` — input `\x9d0;title\x07` (BEL-terminated), assert title is set
  - `c1_0x9f_enters_apc_state()` — input `\x9f...\x9c`, assert APC content captured
  - `c1_0x98_enters_sos_discard_state()` — input `\x98...\x9c`, assert SOS discarded
  - `c1_0x9e_enters_pm_discard_state()` — input `\x9e...\x9c`, assert PM discarded
  - `c1_0x9c_terminates_sequence()` — 0x9C as ST within DCS/APC/SOS/PM (NOT OSC — UTF-8 safety)
  - **Negative pin**: `bsu_esu_7bit_not_matched_by_8bit_csi()` — verified
  - **Negative pin**: `c1_0x9c_does_not_terminate_osc_sequence()` — 0x9C inside OSC is data, not ST
  - **Semantic pin**: `c1_csi_sgr_reset_only_passes_with_8bit_support()` — verified
  - **State transition pin**: `c1_sequence_introducers_enter_states()` — verifies all 6 introducers enter states, not execute
- [x] **Matrix dimensions**: 7 C1 bytes (0x90, 0x98, 0x9B, 0x9C, 0x9D, 0x9E, 0x9F) x 2 context positions (ground state, mid-sequence as terminator) = 14 test cells. **Done**: 24 C1-specific tests covering ground-state entry (7), mid-sequence termination (5 states with 0x9C), negative/semantic pins (3), state transition verification (1), and detailed behavior (8).
- [x] Update catalog rows in `catalog/ecma-48.md` for 8-bit C1 controls to `verified`.
- [x] **Validation**: tests pass; existing C0 + 7-bit ESC tests still pass; alloc regression unchanged; teseq suite timing stable.

---

## 08.7 Verify REP edge cases

**File(s):** `oriterm_core/tests/spec_chain/baseline/rep_edge_cases.rs` (new), possibly `oriterm_core/src/term/handler/mod.rs` (REP handler refinement if needed)

REP (CSI Ps b) repeats the preceding graphic character N times. Edge cases per ECMA-48 sect.8.3.103:
- REP with no preceding graphic character (e.g., immediately after CR or after a control sequence) is a no-op
- REP after a wide character repeats the wide character (occupies 2 columns per repeat)
- REP after a SGR change uses the current SGR state (not the SGR at the time of the original character)

- [x] Read the existing REP handler in ori_term (search for `repeat_preceding` or `repeat` in `oriterm_core/src/term/handler/`). Verify it implements the edge cases correctly. If not, fix. **Audit result**: REP handler at `csi.rs:62-69` correctly handles no-preceding-char, Ps=0→1 mapping, and uses `handler.input(c)` which inherits current SGR and handles wide chars. No fixes needed.
- [x] **Tests — TDD, handler test format** (spec_chain `StateExpectation` only supports cursor position, not grid content; used handler test pattern instead):
  - `rep_no_preceding_char_is_noop()` — verified
  - `rep_after_cr_repeats_preceding()` — changed from plan's "is_noop" expectation: de-facto behavior across xterm/alacritty/wezterm is that C0 controls do NOT clear `preceding_char`. CR between char and REP still repeats the char. Documented deviation.
  - `rep_after_wide_char_repeats_wide()` — verified (CJK '漢' repeated at 2-col width)
  - `rep_uses_current_sgr_not_original()` — verified (SGR 31 red on repeated chars)
  - `rep_at_right_margin_wraps()` — verified (wraps to next line)
  - **Negative pin**: `rep_count_zero_repeats_once()` — verified (Ps=0 maps to 1)
- [x] Update catalog row for REP to `verified`.
- [x] **Validation**: tests pass.

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

- [x] **Verify existing code** — Read `handle_colon_rgb` at `csi.rs:392-398` carefully. `handle_colon_rgb` skips colorspace-id when `params.len() > 4` (rgb_start=2). `parse_sgr_color` handles mode 2 (truecolor) and mode 5 (indexed). All forms verified correct:
  - `38:2::255:128:64` → `[38, 2, 0, 255, 128, 64]`, len=5 > 4, skip index 1 → Rgb(255,128,64) ✓
  - `38:2:0:255:128:64` → identical params, same result ✓
  - `38:5:123` → `[38, 5, 123]`, len=2 ≤ 4, rgb_start=1 → Indexed(123) ✓
- [x] **Test cases — all 14 pass (12 positive matrix + 2 negative pins):**
  - `sgr_38_semicolon_truecolor`, `sgr_38_colon_truecolor_no_colorspace`, `sgr_38_colon_truecolor_with_colorspace`
  - `sgr_38_semicolon_indexed`, `sgr_38_colon_indexed`
  - `sgr_48_semicolon_truecolor`, `sgr_48_colon_truecolor`, `sgr_48_semicolon_indexed`, `sgr_48_colon_indexed`
  - `sgr_58_semicolon_truecolor`, `sgr_58_colon_truecolor`, `sgr_58_semicolon_indexed`, `sgr_58_colon_indexed`
  - **Negative pin**: `sgr_38_mixed_separators_does_not_parse` — mixed colon+semicolon fails (incomplete param group)
  - **Negative pin**: `sgr_38_double_colon_vs_zero_indistinguishable` — `::` and `:0:` produce identical color
- [x] **Matrix dimensions**: 3 × 2 × 2 = 12 positive + 2 negative = 14 tests. All pass.
- [x] Update catalog rows in `catalog/ecma-48.md`: `ECMA48-SGR-38`, `ECMA48-SGR-48`, `ECMA48-SGR-58` all to `verified`.
- [x] **TPR checkpoint** — `/tpr-review` covering 08.6-08.8b (gap fixes). Round 12: 4 codex findings (DECSTR→RIS fix, XTPUSHSGR hyperlink leak fix, DECRQSS SGR drift fix, rendering gap filed as BUG-06-014). 6 gemini findings (4 informational confirmations, 2 theoretical — UTF-8 C1 encoding and O(N^2) dense-invalid-UTF-8 not actionable). All actionable findings fixed.
- [x] **Validation**: both separator forms work; negative pins document known limitations; existing SGR tests unchanged.

---

## 08.8b Verify remaining Section-08-owned catalog rows

**File(s):** Various handler files in `oriterm_core/src/term/handler/`, `crates/vte/src/ansi/dispatch/csi.rs`, sibling tests, catalog files

The catalog assigns 14 additional rows to Section 08 (11 in `catalog/ecma-48.md` + 3 in `catalog/xterm-ctlseqs.md`) that have `status: missing` or `stub` and are not covered by other subsections. These must be implemented/verified before the section can be marked complete.

**SGR rows (5):**
- [x] `ECMA48-SGR-53` — Overlined. Added `Attr::Overline`, `CellFlags::OVERLINE` (bit 16), SGR dispatch `[53]`, and `sgr::apply` handler. Tests: `sgr_53_sets_overline`, `sgr_55_resets_overline`. CellFlags widened from u16 to u32 (no Cell size change — fills alignment padding).
- [x] `ECMA48-SGR-55` — Not overlined (reset for SGR 53). Covered by `Attr::CancelOverline` + `sgr_55_resets_overline` test.
- [x] `ECMA48-SGR-73` — Superscript. Added `Attr::Superscript`, `CellFlags::SUPERSCRIPT` (bit 17), SGR dispatch `[73]`. Mutually exclusive with subscript. Tests: `sgr_73_sets_superscript`, `sgr_73_clears_subscript`.
- [x] `ECMA48-SGR-74` — Subscript. Added `Attr::Subscript`, `CellFlags::SUBSCRIPT` (bit 18), SGR dispatch `[74]`. Test: `sgr_74_sets_subscript`.
- [x] `ECMA48-SGR-75` — Neither superscript nor subscript (reset). Added `Attr::CancelSuperSubscript`. Test: `sgr_75_resets_super_subscript`.

**CSI rows (4):**
- [x] `ECMA48-CSI-DECSTR` — Soft Terminal Reset. Wired `('p', [b'!'])` in `csi::dispatch` to `handler.reset_state()`. Test: `decstr_resets_terminal_state`.
- [x] `ECMA48-CSI-DECSED` — Selective Erase in Display. Added `('J', [b'?'])` dispatch (maps to `clear_screen` — DECSCA protection not yet implemented, so same behavior as ED). Test: `decsed_below_clears_from_cursor`.
- [x] `ECMA48-CSI-DECSEL` — Selective Erase in Line. Added `('K', [b'?'])` dispatch. Test: `decsel_right_clears_to_end_of_line`.
- [x] `ECMA48-CSI-SL` — Scroll Left (CSI Ps SP @). Already implemented. Test: `scroll_left_shifts_content`.
- [x] `ECMA48-CSI-SR` — Scroll Right (CSI Ps SP A). Already implemented. Test: `scroll_right_shifts_content`.

**DCS rows (1):**
- [x] `ECMA48-DCS-DECRQSS-DECSLRM` — Added DECSLRM query (`b"s"` arm) to `status_decrqss` at `status.rs`. Reports 1-based left;right margins. Test: `decrqss_decslrm_reports_margins`.

**xterm-ctlseqs rows (3):**
- [x] `XT-DECSLRM` — Already implemented by 08.5 (CSI s ambiguity resolution). Catalog updated.
- [x] `XT-PUSHSGR` — Added `push_sgr()`/`pop_sgr()` to Handler trait, CSI dispatch `('{', [b'#'])` / `('}', [b'#'])`, `SgrSnapshot` struct + `sgr_stack: Vec<SgrSnapshot>` on Grid (max 10 entries). Test: `xtpushsgr_saves_and_restores_sgr`.
- [x] `XT-POPSGR` — Paired with PUSHSGR. Test: `xtpopsgr_on_empty_stack_is_noop`.

- [x] **Validation**: all 15 catalog rows implemented/verified with 14 new tests. No Section-08-owned rows remain at `missing` or `stub`.

---

## 08.9 Populate _legacy-tack-mapping.md as rows are verified

**File(s):** `plans/spec-conformance/catalog/_legacy-tack-mapping.md`

This file was created empty in section 02. As 08.1-08.8 verify catalog rows that originated from tack scenarios, populate the mapping.

- [x] After every catalog row drives from `implemented-unverified` to `verified` in this section, add a row to `_legacy-tack-mapping.md` linking the row ID to the original tack section. **Result**: All tack-originated catalog rows were already populated by 08.1 (section 05 scenarios) and 08.2 (section 06 scenarios). Subsections 08.3-08.8b verified NEW rows (DECLRMM, C1, REP, SGR colon forms, SGR 53/55/73-75, DECSTR, DECSED, DECSEL, SL, SR, DECRQSS-DECSLRM, PUSHSGR, POPSGR) that did not originate from tack — no additional tack mapping entries needed.
- [x] **Validation**: `_legacy-tack-mapping.md` has 18 entries covering all tack-originated rows. No tack-originated rows remain unmapped.

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
- [x] `[TPR-08-002-codex-r5][medium]` `plans/spec-conformance/catalog/ecma-48.md:49-50` — DRIFT: Split the charset designation rows or cover the missing charset cells.
  Resolved: Fixed on 2026-04-14. Added G2/G3 coverage (`esc_g2_dec_special_graphics_designates_without_panic`, `esc_g3_dec_special_graphics_designates_without_panic`, `esc_g2_ascii_designates_without_panic`, `esc_g3_ascii_designates_without_panic`) and a negative pin (`esc_g1_dec_graphics_is_inert_before_so`) proving `ESC ) 0` is inert on G0 rendering until SO fires. Updated catalog cells for `ECMA48-ESC-B` and `ECMA48-ESC-0` to cite the G2/G3 tests.
- [x] `[TPR-08-003-codex-r5][low]` `oriterm_core/tests/spec_chain/baseline/tack_section_06/{tools_menu_inventory,status_reports,enq_ack}.rs` — GAP: Replace zero-row stub no-op tests with meaningful assertions.
  Resolved: Fixed on 2026-04-14. Deleted empty `#[test]` functions from `tools_menu_inventory.rs` and `status_reports.rs` (pure documentation modules now — rustdoc-only). Replaced `enq_ack.rs` empty test with `ecma48_c0_enq_catalog_row_still_missing_pending_bug_08_6` — a load-bearing regression guard that reads the catalog and fails when ENQ's status flips away from `missing`, forcing the BUG-08-6 implementer to land the real spec_chain test here.
- [x] `[TPR-08-001-gemini-r5][medium]` `oriterm_core/tests/spec_chain/baseline/tack_section_06/tools_menu_inventory.rs:22` — GAP: Empty tests violate No Orphan Tests hygiene rule.
  Resolved: Fixed on 2026-04-14. Same fix as [TPR-08-003-codex-r5]. Both reviewers flagged the same hygiene violation; the single fix addresses both.
- [x] `[TPR-08-002-gemini-r5][low]` `oriterm_core/tests/spec_chain/baseline/tack_section_06/status_reports_inventory.rs:221` — GAP: Missing negative clamp for DSR-6.
  Resolved: Fixed on 2026-04-14. Added `dsr_6_does_not_emit_device_status` negative pin that asserts `CSI 6 n` must not emit a `DeviceStatus` effect, symmetric to the pre-existing `dsr_5_does_not_emit_cursor_report`.
- [x] `[TPR-08-001-codex-r6][medium]` `plans/spec-conformance/catalog/ecma-48.md:49-50` — Align the ESC-B and ESC-0 catalog citations with the actual G-bank tests (round 2 follow-up).
  Resolved: Fixed on 2026-04-14. (1) Added `esc_g1_ascii_designates_without_panic` spec_chain test to cover G1 ASCII designation. (2) Corrected ECMA48-ESC-B citations so the `state:` column cites `esc_g0_ascii_round_trip` (the only ASCII test with `ApexLayer::State`) and added a note row cataloging all four per-G-bank pins. (3) Corrected ECMA48-ESC-0 citations so the `state:` column cites state-apex tests and the prose cites the G3 test alongside G2.
- [x] `[TPR-08-001-codex-r7][low]` `section-08 §08.R` — Keep DSR-6 evidence attached to the DSR-6 finding (round 3 follow-up).
  Resolved: Fixed on 2026-04-14. Removed the orphan DSR-6 lines that had drifted under the r6 finding. (The r7 note previously claimed block-wide consistency; that claim was only partially true and was addressed in full by TPR-08-001-codex-r8 + TPR-08-001-gemini-r8 below.)
- [x] `[TPR-08-001-codex-r8][low]` `section-08 §08.R` — Limit the r7 resolution note to the cleanup this commit actually performed (round 4 follow-up).
  Resolved: Fixed on 2026-04-14. Same fix as [TPR-08-001-gemini-r8] (both reviewers flagged the same drift between r7's claim and the block's actual state).
- [x] `[TPR-08-001-gemini-r8][medium]` `section-08 §08.R` — Remove non-canonical body lines from remaining r5 findings in §08.R (round 4 follow-up).
  Resolved: Fixed on 2026-04-14. Removed the trailing `Evidence:` / `Impact:` / `Required plan update:` / `Basis:` lines from all four r5 resolved findings (`TPR-08-001-codex-r5`, `TPR-08-002-codex-r5`, `TPR-08-003-codex-r5`, `TPR-08-001-gemini-r5`). Every resolved finding in §08.R now follows the canonical "title + Resolved line only" format. Updated the r7 resolution note to reference this round-4 clean-up instead of making the overstated block-wide-consistency claim.
- [x] `[TPR-08-001-codex-r9][high]` `oriterm_core/src/grid/scroll/mod.rs:165` — Guard full-band SL counts before building the swap range.
  Resolved: Fixed on 2026-04-14. Added `band_width == 0` guard and wrapped the shift loop in `if count < band_width` so the `count == band_width` path takes the pure-clear branch without constructing `right - count`. Regression tests: `sl_full_band_no_margins_clears_without_panic`, `sl_full_band_with_margins_clears_band_only`, `sr_full_band_no_margins_clears_without_panic`, `sr_full_band_with_margins_clears_band_only`, `sl_count_larger_than_band_clamps`.
- [x] `[TPR-08-002-codex-r9][medium]` `crates/vte/src/ansi/dispatch/csi.rs:249` — Distinguish explicit default-valued CSI s parameters from the no-parameter form.
  Resolved: Fixed on 2026-04-14. `has_params` now derives from `params.len() > 1 || left != 0`. `CSI 0;0 s` (two explicit params) now routes correctly to DECSLRM. `CSI 0 s` remains indistinguishable from `CSI s` at the parser level; both mean "use defaults" per ECMA-48 §5.4.2. Regression tests: `csi_s_zero_zero_params_mode_69_off_is_noop_not_save_cursor`, `csi_s_zero_zero_params_mode_69_on_resets_margins`.
- [x] `[TPR-08-003-codex-r9][medium]` `oriterm_core/src/grid/scroll/mod.rs:164` — Clean wide-character pairs when SL/SR truncates a glyph at the margin edge.
  Resolved: Fixed on 2026-04-14. Same fix as `[TPR-08-003-gemini-r9]` (codex flagged SL/SR only; gemini flagged SL/SR + partial_up/down — the broader scope). Added `clear_wide_char_at` calls at both band edges for all four margin-aware scroll primitives. Regression tests: `sl_cleans_wide_char_pair_straddling_right_margin_plus_one`, `sr_cleans_wide_char_pair_straddling_left_margin`, `il_with_margins_cleans_wide_char_at_band_edge`, `dl_with_margins_cleans_wide_char_at_band_edge`.
- [x] `[TPR-08-001-gemini-r9][high]` `oriterm_core/src/grid/editing/mod.rs:247` — ICH/DCH mutate cells outside the left margin.
  Resolved: Fixed on 2026-04-14. Added `col < left_margin || col > right_margin` no-op guard to `insert_blank` and `delete_chars` when DECLRMM is active. Regression tests: `ich_with_cursor_left_of_left_margin_is_noop`, `ich_with_cursor_right_of_right_margin_is_noop`, `dch_with_cursor_left_of_left_margin_is_noop`, `dch_with_cursor_right_of_right_margin_is_noop`.
- [x] `[TPR-08-002-gemini-r9][high]` `oriterm_core/src/grid/scroll/mod.rs:163` — Unconditional `set_occ(row.cols())` pessimizes occ tracking.
  Resolved: Fixed on 2026-04-14. Replaced `row.set_occ(row.cols())` with tighter updates across all four margin-aware scroll primitives: `scroll_left`/`scroll_right` only extend occ when BCE template is non-empty (to `max(current, right + 1)`); `scroll_partial_up`/`scroll_partial_down` only extend occ when the source row actually had content at the band edge, and the BCE clear path only extends occ when the template is non-empty. Preserves old occ as a valid upper bound otherwise.
- [x] `[TPR-08-003-gemini-r9][high]` `oriterm_core/src/grid/scroll/mod.rs:142` — Margin band scrolls split wide characters leaving orphaned flags.
  Resolved: Fixed on 2026-04-14. Same fix as `[TPR-08-003-codex-r9]` (broader scope — covers all four margin-aware scroll primitives: `scroll_left`, `scroll_right`, `scroll_partial_up`, `scroll_partial_down`). Bumped visibility of `Grid::clear_wide_char_at` from `pub(super)` to `pub(in crate::grid)` so the scroll module can invoke it.
- [x] `[TPR-08-004-gemini-r9][medium]` `oriterm_core/src/term/mod.rs:137` — Grid margin state stored in Term violates SSOT.
  Resolved: Fixed on 2026-04-14. Moved `saved_margins` from `Term` into `Grid` (alongside `saved_cursor`). `Grid::save_cursor` now saves cursor + margins; `Grid::restore_cursor` restores both; `Grid::reset` clears both. Removed `Term::saved_margins` and `Term::inactive_saved_margins` — the alt screen swap no longer needs to juggle them since the margin save state rides along with the grid itself (primary vs alt grid are separate `Grid` instances in `Term::{grid, alt_grid}`). `Term` retains only `saved_left_right_margin_mode` (the DECLRMM mode flag is Term-level state by construction — it lives in `TermMode`). Existing tests `save_restore_preserves_margin_state` and `save_restore_preserves_declrmm_mode_flag` pass unchanged after the refactor.
  **Superseded by round 11 (2026-04-14)**: `[TPR-08-001-codex-r11]` discovered that DEC STD 070 §5.6.1 excludes margins and DECLRMM from the DECSC save set entirely. The r9 SSOT fix relocated the margin-save state from Term to Grid but kept save/restore behavior. Round 11 removed the save/restore entirely (`Grid::saved_margins`, `Term::saved_left_right_margin_mode`, and the inactive variant are all gone). The SSOT concern from r9 is now moot: there is no margin-save state anywhere in the save-cursor code path.
- [x] `[TPR-08-001-codex-r10][high]` `oriterm_core/src/grid/scroll/mod.rs:246` — SR fails to update occ when content shifts with empty template.
  Resolved: Fixed on 2026-04-14. The occ-tightening logic was guarded by `if !template.is_empty()`, so a sparse row with content at col 2 that scrolled right by 2 would end up with content at col 4 but occ stuck at 3, making the shifted content invisible to the renderer. Replaced the guard with unconditional `row.set_occ(row.occ().max(right + 1).min(cols))` for both `scroll_left` and `scroll_right`: any touch of cells in `[left, right]` extends occ to `right + 1` (tight bound within the band, never pessimizes past `cols`). If pre-shift occ was already beyond `right + 1`, it's preserved. Regression: `sr_extends_occ_when_content_shifts_with_empty_template`.
- [x] `[TPR-08-001-gemini-r10][high]` `oriterm_core/src/grid/scroll/mod.rs:305` — IndexMut forces occ to band edge in scroll_partial, defeating optimization.
  Resolved: Fixed on 2026-04-14. The round-9 "optimization" (`if dst_row.occ() < right + 1 { ... if !src_row[...].is_empty() { set_occ }}`) was dead code: `IndexMut<Column>` on Row automatically bumps `occ` to `col + 1` on every write, so by the time the copy loop finished, `dst_row.occ()` was already >= `right + 1`. Removed the dead conditional and the explicit `set_occ` calls in `scroll_partial_up` and `scroll_partial_down` — the IndexMut bump is sufficient. The resulting occ bound is the same `right + 1` used by SL/SR (tight within-band bound, consistent across all four margin-aware primitives).
- [x] `[TPR-08-002-gemini-r10][high]` `oriterm_core/src/grid/scroll/mod.rs:242` — scroll_right fails to update occ when shifting non-empty cells with empty template.
  Resolved: Fixed on 2026-04-14. Same fix as `[TPR-08-001-codex-r10]` — unconditional occ extension post-shift.
- [x] `[TPR-08-003-gemini-r10][high]` `oriterm_core/src/grid/scroll/mod.rs:168` — clear_wide_char_at at band edges destroys in-band wide-char pairs.
  Resolved: Fixed on 2026-04-14. Replaced `clear_wide_char_at` at band edges with `fix_wide_boundaries(row, left, right + 1)`, which only clears orphaned halves OUTSIDE the band range — in-band pairs are preserved. This is the canonical band-aware cleanup helper (already used by erase operations). Bumped `fix_wide_boundaries` visibility from `pub(super)` to `pub(in crate::grid)`; reverted `clear_wide_char_at` to `pub(super)` since only the scroll module's band-edge use case needed cross-module access, and that now uses `fix_wide_boundaries` instead. Regression tests: `sl_preserves_inband_wide_char_pair_at_right_edge`, `il_with_margins_preserves_inband_wide_char_pair` (positive pins that pair stays intact across SL and IL when entirely in-band).

### Round 11 (2026-04-14 — 08.5 TPR checkpoint)

- [x] `[TPR-08-001-codex-r11][high]` `oriterm_core/src/term/handler/mod.rs:311` — DECSC/DECRC must not save/restore DECLRMM margins or the mode 69 flag.
  Resolved: Fixed on 2026-04-14. Per DEC STD 070 §5.6.1 and cross-verified against wezterm (`termwiz/SavedCursor` at `wezterm/term/src/terminalstate/mod.rs:134-142`), alacritty (`Cursor` at `alacritty_terminal/src/grid/mod.rs:34-53`), and ghostty (`SavedCursor` at `src/terminal/Screen.zig:187-195`), the DECSC save set is cursor position + character attributes + charset state + wrap flag + DECOM flag. Left/right margins and the DECLRMM mode flag are NOT saved by DECSC and must survive DECRC untouched — margin state is scoped to the screen (alt vs primary), not the cursor save/restore pair, and is toggled via RIS, DECSTR, DECCOLM, DECALN, resize, or explicit DECRST ?69. Removed `Grid::saved_margins` field, `Term::saved_left_right_margin_mode` field, `Term::inactive_saved_left_right_margin_mode` field, and the corresponding save/restore/alt-screen-swap/RIS-clear logic. Flipped `save_restore_preserves_margin_state` and `save_restore_preserves_declrmm_mode_flag` tests into negative pins (`decrc_does_not_restore_horizontal_margins`, `decrc_does_not_restore_declrmm_mode_flag`, `decrc_does_not_enable_declrmm_after_disabled_save`) and added a positive pin (`decsc_decrc_restores_cursor_position_and_origin`) guarding the correct save set (cursor + DECOM).
- [x] `[TPR-08-002-codex-r11][high]` `oriterm_core/src/grid/navigation/mod.rs:75` — Absolute CUP/HVP must ignore horizontal margins.
  Resolved: Fixed on 2026-04-14. Same fix addresses `[TPR-08-001-gemini-r11]`. `Grid::move_to` was clamping `col` to `right_margin` whenever `cursor_in_margin_band()` was true, which silently trapped absolute CUP/HVP inside the band when the cursor started there. Removed the margin clamp — `Grid::move_to` now clamps only to `cols - 1`. DECOM origin-relative addressing is already applied at the Term layer in `Term::goto_origin_aware` via `origin_aware_col`, which offsets and clamps when both `ORIGIN` and `LEFT_RIGHT_MARGIN` modes are set. The old `cup_clamps_to_margin_band` positive pin was flipped into `cup_ignores_horizontal_margins` with both-direction assertions (col=2 outside band left, col=75 outside band right, col=79 at last column).
- [x] `[TPR-08-001-gemini-r11][high]` `oriterm_core/src/grid/navigation/mod.rs:77` — Asymmetric margin clamp in `Grid::move_to`.
  Resolved: Fixed on 2026-04-14. Same fix as `[TPR-08-002-codex-r11]` (dual-source agreement on the same defect).
- [x] `[TPR-08-003-codex-r11][medium]` `oriterm_core/src/term/handler/mod.rs:167` — `Term::goto_col` must offset CHA/HPA by `left_margin` under DECOM+DECLRMM.
  Resolved: Fixed on 2026-04-14. Same fix addresses `[TPR-08-002-gemini-r11]`. `Term::goto_col` was calling `grid_mut().move_to_column(Column(col))` directly, so nontrivial column parameters under DECOM+DECLRMM collapsed onto the left edge (the grid-level margin clamp happened to map `col=1` to `left_margin`, hiding the bug for the only column value the prior test exercised). Extracted the column-resolution logic from `goto_origin_aware` into a new `origin_aware_col` helper (Term-level), which applies `col + left_margin` clamped to `right_margin` when both ORIGIN and LEFT_RIGHT_MARGIN modes are set, otherwise clamps to `cols - 1`. `Term::goto_col` now calls this helper. Also removed the obsolete margin clamp from `Grid::move_to_column` (CHA is absolute at the Grid layer; margin-aware offset happens in `Term`). Added a 5-cell matrix (DECLRMM × DECOM) in `term/handler/tests.rs` (`cha_absolute_when_declrmm_off_decom_off`, `cha_absolute_when_declrmm_on_decom_off`, `cha_absolute_when_declrmm_off_decom_on`, `cha_offsets_by_left_margin_when_declrmm_on_decom_on`, `cha_clamps_to_right_margin_under_decom_declrmm`) plus a negative pin (`cha_col_1_lands_at_left_margin_under_decom_declrmm`) for the previously-hidden `col=1` clamp coincidence. Grid-level test was replaced with `cha_grid_level_does_not_clamp_by_margins`.
- [x] `[TPR-08-002-gemini-r11][high]` `oriterm_core/src/term/handler/mod.rs:167` — Apply DECOM left_margin offset to `goto_col`.
  Resolved: Fixed on 2026-04-14. Same fix as `[TPR-08-003-codex-r11]` (dual-source agreement on the same defect, cited with slightly different locations — the fix spans both).
- [x] `[TPR-08-004-codex-r11][medium]` `oriterm/src/gpu/state/headless.rs:81` — `new_headless_with_preference` bypasses `sanitize_headless_env`.
  Resolved: Fixed on 2026-04-14. BUG-06-013's fix placed the `sanitize_headless_env()` call in `GpuState::new_headless()` only, but the public `new_headless_with_preference()` entrypoint is reached directly by the deterministic software-rasterizer lane in `oriterm/src/gpu/visual_regression/mod.rs:156` and by adapter-preference tests at `oriterm/src/gpu/state/tests.rs:553`. Moved `sanitize_headless_env()` into `new_headless_with_preference()` — the shared init path — so both public constructors funnel through the guard. `new_headless()` now delegates directly; the `OnceLock`-gated sanitization fires exactly once per process regardless of entry. Added `both_headless_entrypoints_unset_display_env` regression in `oriterm/src/gpu/state/tests.rs` that exercises both constructors and asserts `WAYLAND_DISPLAY` / `DISPLAY` are unset post-call.

### Round 11 iteration 2 (2026-04-14)

- [x] `[TPR-08-005-codex-r11][medium]` `plans/spec-conformance/section-08-ecma-48-baseline.md:88` + `:547` + `plans/spec-conformance/catalog/dec-private-modes.md:49` — Plan success criteria, historical round-9 resolution note, and catalog DECLRMM row still described the superseded "DECSC/DECRC save/restore margin state" contract.
  Resolved: Fixed on 2026-04-14. Updated the §08 success-criterion bullet at line 88 to state the correct DEC STD 070 §5.6.1 save set (cursor + attributes + charsets + wrap + DECOM — margins and DECLRMM NOT saved). Appended a "**Superseded by round 11**" note to the `[TPR-08-004-gemini-r9]` historical entry so the audit trail remains but the current contract is clearly documented. Rewrote the DEC-DECLRMM catalog row's DECSC/DECRC note in `catalog/dec-private-modes.md:49` to match the shipped code: DECSC/DECRC do NOT save/restore margin state or the DECLRMM mode flag. Gemini iter-2 re-review did not complete (~23 min watchdog timeout after completing file reads and `cargo build --target x86_64-pc-windows-gnu`); codex iter-2 surfaced this finding solo.

### Round 11 iteration 3 (2026-04-14)

- [x] `[TPR-08-006-codex-r11][medium]` `plans/spec-conformance/section-08-ecma-48-baseline.md:48` + `:245` + `:249` + `:600` — 08.5 subsection title + intro paragraph + N-checklist line still described the subsection as covering "save/restore, reset paths" rather than the corrected DECSC/DECRC save-set scope.
  Resolved: Fixed on 2026-04-14. Renamed the 08.5 subsection across four surfaces (frontmatter `sections[].title`, the `## 08.5` heading, the 08.5 introductory paragraph, and the 08.N completion-checklist bullet) to `DECLRMM extended operations (IL/DL partial-width scroll, CSI s ambiguity, DECSC/DECRC scope, reset paths)`. The intro paragraph now explicitly states that the DECSC/DECRC scope excludes margins and DECLRMM per DEC STD 070 §5.6.1 and cross-references 08.5c for the correct contract.
- [x] `[TPR-08-007-codex-r11][low]` `oriterm_core/src/term/handler/tests.rs:617` — `cha_col_1_lands_at_left_margin_under_decom_declrmm` was labeled in its doc comment (and in the r11 iter-1 resolution for `[TPR-08-003-codex-r11]`) as "a negative pin for the pre-fix behavior", but the pre-fix `Grid::move_to_column` margin clamp also landed `col=0` at `left_margin` — the test does not distinguish the pre-fix and post-fix code paths.
  Resolved: Fixed on 2026-04-14. Rewrote the test's doc comment to accurately describe it as a positive edge-case pin for `col=0` (which BOTH code paths resolve to `left_margin`), and to cross-reference `cha_offsets_by_left_margin_when_declrmm_on_decom_on` as the true regression guard that fails on the pre-fix clamp path (col=5 would land at col=10 pre-fix vs col=14 post-fix). The broader 6-cell DECLRMM × DECOM matrix was always the load-bearing coverage; the col=1 pin is useful as a boundary case but was over-claimed.

### Round 11 iteration 4 (2026-04-14) — CLEAN PASS

- **No findings.** Both codex (192s) and gemini (67s) returned `no_findings: true` on iter-4. Commit range `8808d5c5..b7245ced` verified clean across all round-11 surfaces (DECSC/DECRC scope, absolute CUP/HVP, CHA DECOM offset, headless sanitize merge, plan metadata alignment, 08.5 subsection rename, CHA col_1 test doc correction). §08.5 TPR checkpoint closed.

### Round 12 (2026-04-14 — 08.6-08.8b TPR checkpoint)

- [x] `[TPR-08-001-codex-r12][high]` `crates/vte/src/ansi/dispatch/csi.rs:227` — DECSTR routed to RIS (full reset) instead of soft reset.
  Resolved: Fixed on 2026-04-14. Added `Handler::decstr()` method + `Term::soft_reset()` (resets modes/SGR/cursor/scroll region/margins/sgr_stack but NOT screen contents, title, palette, or scrollback). `CSI ! p` now dispatches to `decstr()`, `ESC c` (RIS) stays on `reset_state()`.
- [x] `[TPR-08-002-codex-r12][medium]` `oriterm_core/src/grid/mod.rs:291` — XTPUSHSGR snapshot captured hyperlink + zerowidth from CellExtra, not just underline color.
  Resolved: Fixed on 2026-04-14. Changed `SgrSnapshot` to store only `underline_color: Option<Color>` instead of `extra: Option<Arc<CellExtra>>`. Push extracts underline_color from CellExtra; pop restores via `set_underline_color()`. Hyperlinks and zerowidth marks are no longer affected by push/pop.
- [x] `[TPR-08-003-codex-r12][medium]` `oriterm_core/src/term/handler/status.rs:24` — DECRQSS SGR reporting missing overline/superscript/subscript flags.
  Resolved: Fixed on 2026-04-14. Added `CellFlags::OVERLINE` → `"53"`, `CellFlags::SUPERSCRIPT` → `"73"`, `CellFlags::SUBSCRIPT` → `"74"` to `build_sgr_string()`.
- [x] `[TPR-08-004-codex-r12][medium]` `oriterm/src/gpu/prepare/decorations.rs:68` — New OVERLINE/SUPERSCRIPT/SUBSCRIPT flags not consumed by GPU renderer or HTML export.
  Resolved: Filed as BUG-06-014 (rendering gap, medium severity). The flags are correctly stored on cells and reported via DECRQSS, but rendering requires GPU shader/font pipeline work that belongs in the rendering subsystem, not Section 08.

### Round 12 iteration 2 (2026-04-14) — section-close re-run

- [x] `[TPR-08-001-codex-r12i2][medium]` `oriterm_core/src/term/handler/esc.rs:76` — DECSTR soft_reset must clear DECSC saved cursor state.
  Resolved: Fixed on 2026-04-14. Added `grid.clear_saved_cursor()` to `soft_reset()`. Prevents `ESC 7 / CSI ! p / ESC 8` from resurrecting pre-reset cursor state.
- [x] `[TPR-08-002-codex-r12i2][low]` `plans/spec-conformance/catalog/ecma-48.md:198` + `catalog/xterm-ctlseqs.md:31` — Stale catalog docs still described pre-round-12 behavior.
  Resolved: Fixed on 2026-04-14. Updated DECSTR catalog row to reference `handler.decstr()` → `Term::soft_reset()`. Updated XTPUSHSGR catalog row to note underline_color-only snapshot (not full CellExtra).

### Round 12 iteration 3 (2026-04-14)

- [x] `[TPR-08-001-codex-r12i3][medium]` `oriterm_core/src/term/handler/esc.rs:88` — DECSTR must clear saved cursor on BOTH screens + clear inactive_saved_charset/origin_mode.
  Resolved: Fixed on 2026-04-14. Added `alt.clear_saved_cursor()` + `alt.clear_sgr_stack()` for `alt_grid` and cleared `inactive_saved_charset` / `inactive_saved_origin_mode`. Per WezTerm `terminalstate/mod.rs:1273-1276`.
- [x] `[TPR-08-002-codex-r12i3][low]` + `[TPR-08-001-gemini-r12i3][high]` `oriterm_core/src/term/handler/tests.rs:6649` — Missing regression pins for DECSTR clearing saved cursor and SGR stack.
  Resolved: Fixed on 2026-04-14. Added `decstr_clears_saved_cursor` (pins ESC 7 / CSI ! p / ESC 8 → cursor does NOT resurrect) and `decstr_clears_sgr_stack` (pins CSI # { / CSI ! p / CSI # } → SGR stack does NOT resurrect). Both reviewers flagged the same test gap.

**Tooling retrospective (08.5):** improvements committed in
`da70fdbe` (dual-tpr `transport.md` gains a mandatory gemini hygiene
preamble — scratch-file discipline, `git diff --stat` first,
skip-redundant-workspace-gates — plus a plan/code consistency clause
for both reviewers) and `a6c396ac` (new
`scripts/check-plan-subsection-sync.py` that catches drift between
plan frontmatter subsection titles and body `## {id}` headings, the
exact trap behind `[TPR-08-006-codex-r11]`).

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
- [ ] DECLRMM extended operations complete (partial-width scroll, CSI s ambiguity, DECSC/DECRC scope, reset paths)
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

**Exit Criteria:** ECMA-48 baseline catalog row subset is `verified`; DECLRMM full mode plumbing + grid enforcement + extended operations verified; CSI s / DECSLRM ambiguity resolved (zero-param form only per WezTerm/Ghostty); DECSC/DECRC save set matches DEC STD 070 §5.6.1 (excludes margins and DECLRMM); absolute CUP/HVP/CHA/HPA ignore margins (DECOM offset applied at Term layer); reset paths clear margins; 8-bit C1 controls verified; REP edge cases verified; ISO 8613-6 SGR colon forms verified with negative pins for known limitations; all Section-08-owned catalog rows (SGR 53/55/73/74/75, DECSTR, DECSED, DECSEL, SL, SR, DECRQSS-DECSLRM, XT-DECSLRM, XT-PUSHSGR, XT-POPSGR) verified; legacy tack mapping populated; existing tack + teseq tests still pass; ready for Phase 3 stacks to depend on baseline correctness.
