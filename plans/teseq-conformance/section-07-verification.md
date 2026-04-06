---
section: "07"
title: "Verification & CI Integration"
status: not-started
reviewed: true
goal: "Verify the complete teseq test framework, document coverage gaps, integrate with CI, and ensure graceful degradation on all platforms"
success_criteria:
  - "Test matrix documents all scenarios by protocol family with pass/fail status"
  - "Coverage gap analysis identifies sequences not yet covered and prioritizes future additions"
  - "CI integration: Linux CI installs teseq (ci.yml updated) and runs tests; macOS/Windows gracefully skip"
  - "CLAUDE.md updated with teseq test commands and scenario authoring instructions"
  - "All 176 test functions pass at their designated sizes (168 .teseq scenarios + 7 pure-Rust tests + 1 scenario-variant)"
  - "Skip path correctness audited: every family module's reseq guard verified, no unguarded reseq-dependent code paths"
  - "Every mission success criterion from 00-overview.md explicitly verified and checked off — plan fully closed out"
inspired_by:
  - "ori_term vttest-conformance section-07-verification — conformance metric tracking"
  - "ori_term vttest session.rs:232-239 — vttest_available() graceful skip pattern"
depends_on: ["01", "02", "03", "04", "05", "06"]
third_party_review:
  status: resolved
  updated: 2026-04-06
sections:
  - id: "07.1"
    title: "Test Matrix & Coverage Analysis"
    status: not-started
  - id: "07.2"
    title: "Platform & CI Integration"
    status: not-started
  - id: "07.3"
    title: "Documentation Updates"
    status: not-started
  - id: "07.4"
    title: "Build & Verify"
    status: not-started
  - id: "07.5"
    title: "Mission Success Criteria Verification"
    status: not-started
  - id: "07.R"
    title: "Third Party Review Findings"
    status: complete
  - id: "07.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 07: Verification & CI Integration

**Status:** Not Started
**Goal:** Prove the framework works as a cohesive whole. Document what's covered, what's not, and ensure it runs correctly across all platforms and CI environments.

**Success Criteria:**

- [ ] Complete test matrix documented with scenario counts per family
- [ ] Coverage gap analysis identifies priority areas for future scenarios
- [ ] CI handles teseq availability gracefully on all platforms
- [ ] Documentation updated in CLAUDE.md with test commands
- [ ] All scenarios pass; ./test-all.sh green
- [ ] Satisfies mission criteria for CI and verification
- [ ] Skip path correctness verified (not just documented) — every family module's skip boundary tested
- [ ] Every mission success criterion from 00-overview.md explicitly verified and checked off

**Context:** This section is the final verification gate. It doesn't add new scenarios — it verifies the complete framework, documents its coverage, and ensures it integrates cleanly with the project's CI and development workflow.

**Depends on:** All sections (01-06).

**Subsection execution order:** 07.1 (matrix/gaps) -> 07.2 (CI/skip audit) -> 07.4 (build/verify) -> 07.3 (documentation, uses facts from 07.1/07.2/07.4) -> 07.5 (mission criteria collation, uses evidence from all prior). The numbering is kept for stable references; execution order differs.

---

## 07.1 Test Matrix & Coverage Analysis

Build a comprehensive test matrix documenting every scenario.

- [ ] **Scenario inventory by family:**

  | Family | .teseq | Pure-Rust | Variant | Total | Sizes | Key Coverage |
  |--------|--------|-----------|---------|-------|-------|-------------|
  | C0 (`c0/`) | 8 | 0 | 0 | 8 | 80x24 | CR, LF, BS, TAB, BEL, FF, VT, SO/SI |
  | CSI Cursor (`csi/cursor/`) | 10 | 0 | 0 | 10 | 80x24, 97x33, 120x40 | CUP, CUU, CUD, CUF, CUB, VPA, HPA, CHA |
  | CSI Erase (`csi/erase/`) | 7 | 0 | 0 | 7 | 80x24 | ED 0-3, EL 0-2 |
  | CSI Insert/Delete (`csi/insert_delete/`) | 4 | 0 | 0 | 4 | 80x24 | ICH, DCH, IL, DL |
  | CSI Reports (`csi/reports/`) | 15 | 3 | 0 | 18 | 80x24 | DA1/2/3, DSR, DECRQM, analyze_response, DA2 drift, pipe error |
  | CSI Modes (`csi/modes/`) | 34 | 0 | 0 | 34 | 80x24, 97x33, 120x40 | DECOM, DECCOLM, alt screen, IRM, wrap, cross-cutting |
  | CSI SGR (`csi/sgr/`) | 54 | 0 | 1 | 55 | 80x24 | Attributes, 16/256/TrueColor, combos, resets, underlines |
  | ESC (`esc/`) | 5 | 0 | 0 | 5 | 80x24 | DECSC/RC, RIS, SCS G0, IND, RI |
  | OSC (`osc/`) | 4 | 0 | 0 | 4 | 80x24 | Title (0/2), icon name (1), clipboard (52), color query (4/10/11) |
  | Workflows (`workflows/`) | 27 | 4 | 0 | 31 | 80x24, 97x33, 120x40 | Mode combos, DECSC attrs, handshakes, real-world, charset, edge cases |
  | **Total** | **168** | **7** | **1** | **176** | | |

  **Test categories:**
  - **.teseq** — scenario-driven tests that compile a `.teseq` file via `reseq` and assert terminal state
  - **Pure-Rust** — tests with no `.teseq` file dependency; exercise code paths directly (chunked feed, DA2 drift, pipe error, DECCOLM lifecycle, DECSC sidecar isolation)
  - **Variant** — tests that reuse an existing `.teseq` file with different configuration (`color_bold_bright_disabled` reuses `color_bold_bright.teseq` with `set_bold_is_bright(false)`)

- [ ] **Coverage gap analysis** — identify sequences NOT yet covered:
  - OSC 7 (CWD) — handled by `RawInterceptor` in `oriterm_mux`, not `Term<T>`; tested at mux layer (`oriterm_mux/src/shell_integration/tests.rs::interceptor_osc7_sets_cwd`), not teseq harness
  - DCS sequences (DECRQSS, Sixel, Kitty image protocol) — complex, may warrant future plan. Note: DECRQSS IS implemented in `status.rs` and could be added as a teseq scenario (it uses PtyWrite events)
  - OSC color set (OSC 4/10/11 with color values, not just queries) — Section 06.4 covers queries only
  - Tab stops (HTS, TBC) — not tested in teseq yet
  - Soft terminal reset (DECSTR via CSI ! p) — not tested
  - Conformance level (DECSCL) — not tested
  - Wide characters (CJK, emoji) — partially covered: `irm_wide_char` and `wrap_disabled_wide_char` in mode_interactions test wide-char behavior with IRM and DECAWM. Standalone CJK cell placement and emoji ZWJ sequences are handled by GPU golden tests. Additional teseq scenarios could cover wide-char wrap, overwrite, and erase interactions

- [ ] **Priority ranking** for future scenario additions — produce a ranked list (table or bullet list) with columns: Sequence/Family, Bug Risk (high/medium/low), Interop Importance (high/medium/low), Effort (small/medium/large), Notes. Rank by (Bug Risk, Interop Importance) descending. Place this list immediately after the coverage gap analysis in a `### Future Scenario Priorities` subsection of `main.rs` module docs (alongside the scenario authoring guide). At minimum, rank these gaps: DCS (DECRQSS, Sixel, Kitty), OSC color set, tab stops (HTS/TBC), DECSTR, DECSCL, standalone CJK cell placement.

---

## 07.2 Platform & CI Integration

- [ ] **Graceful skip verification:**
  - Verify `reseq_available()` returns false when `reseq` is not in PATH
  - Verify all .teseq-dependent tests print "reseq not installed, skipping" and return Ok (not panic/fail)
  - Verify by code inspection that `reseq_available()` in `harness/reseq.rs` uses `Command::new("reseq").arg("--version")` PATH lookup (same proven pattern as `vttest_available()`). Optionally test with PATH manipulation: `timeout 150 env PATH=$(echo $PATH | tr ':' '\n' | grep -v $(dirname $(which reseq)) | tr '\n' ':') cargo test -p oriterm_core --test teseq` (removes reseq's directory from PATH; 169 reseq-dependent tests should skip, 7 pure-Rust tests should pass)

- [ ] **Skip path correctness audit** — verify every family module enforces the skip boundary:
  - **Pattern 1 (parent `run_scenario` with guard):** Grep `fn run_scenario` in all `.rs` files under `oriterm_core/tests/teseq/`. Expected: 10 definitions, each containing `if !reseq_available()` as the first statement:
    - `c0.rs` — guard present, 8 tests call it
    - `csi_cursor.rs` — guard present, 10 tests call it
    - `csi_erase.rs` — guard present, 6 of 7 tests call it (`ed_scrollback` has inline guard)
    - `csi_insert_delete.rs` — guard present, 4 tests call it
    - `csi_reports.rs` — guard present (returns `Option<ScenarioOutcome>`), 15 .teseq tests call it
    - `esc.rs` — guard present, 5 tests call it
    - `mode_interactions.rs` — guard present (returns `Option<ScenarioOutcome>`), 34 tests call it
    - `osc.rs` — guard present (returns `Option<ScenarioOutcome>`), 4 tests call it
    - `sgr/mod.rs` — guard present (returns `Option<ScenarioOutcome>`), sub-modules (`attributes.rs`, `colors.rs`, `combinations.rs`, `edge_cases.rs`, `resets.rs`, `underlines.rs`) call `super::run_scenario`
    - `workflows/mod.rs` — guard present (returns `Option<ScenarioOutcome>`), sub-modules (`edge.rs`, `mode.rs`, `query.rs`, `real_world.rs`) call `super::run_scenario`
  - **Pattern 2 (inline guard):** `csi_erase.rs::ed_scrollback` has its own `if !reseq_available()` guard because it calls `TeseqHarness::from_scenario()` directly (not via `run_scenario()`). Verify this guard is present.
  - **Pure-Rust tests (must NOT guard):** Verify these 7 tests do not call `reseq_available()`:
    - `csi_reports.rs::da2_version_drift_check` — uses `Term::new()` directly
    - `csi_reports.rs::analyze_response_produces_output` — exercises `analyze_response()` helper
    - `csi_reports.rs::pipe_through_command_returns_err_on_nonzero_exit` — exercises `pipe_through_command()` helper
    - `workflows/mode.rs::deccolm_lifecycle_intermediate_assertions` — uses `Term::new()` directly
    - `workflows/mode.rs::decsc_sidecar_isolation_across_alt_screen` — uses `Term::new()` directly
    - `workflows/edge.rs::edge_chunked_osc` — uses `Term::new()` directly
    - `workflows/edge.rs::edge_chunked_csi` — uses `Term::new()` directly
  - **Fix on discovery:** If any module is missing the guard or any pure-Rust test incorrectly guards, fix it immediately (not deferred)

- [ ] **CI configuration changes:**
  - Linux CI (`test-linux` job): add `teseq` to the `sudo apt-get install` list in `.github/workflows/ci.yml` line ~105, right after the existing `vttest` entry (same continuation line). This enables teseq tests to actually RUN in CI, not just skip. The `teseq` Debian package provides both `teseq` and `reseq` binaries
  - macOS/Windows CI (`cross-platform` job): no changes needed — `reseq_available()` returns false, tests skip gracefully (same as vttest pattern)
  - Verify after ci.yml change: `teseq --version` and `reseq --version` both succeed in the Linux CI environment
  - Document the CI teseq install in the CLAUDE.md Commands section

- [ ] **`test-all.sh` integration:**
  - Verify `./test-all.sh` runs teseq tests as part of `cargo test --workspace --features oriterm/gpu-tests`
  - Note: `test-all.sh` includes `--features oriterm/gpu-tests` which CI does not — both paths must pass
  - No modifications needed to `test-all.sh` — Cargo auto-discovers integration tests
  - Verify: `timeout 150 ./test-all.sh` passes with teseq tests included (mandatory timeout per CLAUDE.md)

---

## 07.3 Documentation Updates

**Execution order:** Finalize 07.3 AFTER 07.1, 07.2, and 07.4 are complete. The scenario counts, coverage gaps, and CI commands referenced here depend on verified facts from those subsections.

- [ ] **Update CLAUDE.md** — add teseq test commands to the Commands section:
  ```
  **Teseq scenarios**: `cargo test -p oriterm_core --test teseq`
  **Update teseq snapshots**: `INSTA_UPDATE=1 cargo test -p oriterm_core --test teseq`
  ```

- [ ] **Update CLAUDE.md Key Paths** — add teseq test location:
  ```
  **oriterm_core/tests/teseq/** — Teseq scenario-based escape sequence tests
  ```

- [ ] **Scenario authoring guide** — add brief instructions to `oriterm_core/tests/teseq/main.rs` module docs:
  - How to create a new scenario (`.teseq` file + optional `.toml` sidecar)
  - How to register a new family module
  - How to update golden snapshots
  - Link to teseq/reseq documentation

- [ ] **Expected test count in `main.rs` module docs** — add a `# Test Count` section to the `//!` doc comment in `oriterm_core/tests/teseq/main.rs` noting the expected total (176: 168 .teseq + 7 pure-Rust + 1 variant). Include a validation command: `cargo test -p oriterm_core --test teseq -- --list 2>/dev/null | grep -c '::.*test$'`. This serves as a lightweight drift-detection checkpoint (not a compile-time test, which would couple test organization to correctness).

- [ ] **Future scenario priorities** — add a `# Future Scenario Priorities` section to the `//!` doc comment in `oriterm_core/tests/teseq/main.rs` with the ranked gap table produced in 07.1.

---

## 07.4 Build & Verify

- [ ] `./build-all.sh` green (all platforms)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `timeout 150 ./test-all.sh` green (all tests pass, including teseq)
- [ ] `timeout 150 cargo test -p oriterm_core --test teseq` — all 176 tests pass (debug profile)
- [ ] `timeout 150 cargo test -p oriterm_core --test teseq --release` — all 176 tests pass (release profile; catches optimizer-sensitive bugs in chunked-feed and workflow tests)
- [ ] `timeout 150 cargo test -p oriterm_core --test vttest` — no regressions in vttest
- [ ] `timeout 150 cargo test -p oriterm_core` — no regressions in oriterm_core unit tests (handler, grid, cell, selection, search, palette)

---

## 07.5 Mission Success Criteria Verification

Collate evidence from 07.1-07.4 against every mission success criterion in `00-overview.md`. This is the final gate before plan completion. No new testing here — only verification that prior subsections produced the required evidence.

- [ ] **Criterion-by-criterion verification** — for each checkbox in 00-overview.md's "Mission Success Criteria", record the evidence source (subsection + artifact):
  - `TeseqHarness` exists → evidence: files in `oriterm_core/tests/teseq/harness/` (`mod.rs`, `loader.rs`, `runner.rs`, `assertions.rs`, `reseq.rs`, `events.rs`)
  - `RecordedEvent` enum → evidence: enum definition in `harness/events.rs` (15 variants, exhaustive `From<&Event>` match)
  - Scenario sidecar TOML → evidence: `ScenarioSpec` struct in `harness/loader.rs`
  - `reseq` graceful skip → evidence: 07.2 skip audit results
  - C0 (CR, LF, BS, TAB, BEL, FF, VT, SO, SI) → evidence: 07.1 matrix row (8 tests in `c0.rs`)
  - CSI cursor (CUP, CUU, CUD, CUF, CUB, VPA, HPA, CHA) → evidence: 07.1 matrix row (10 tests in `csi_cursor.rs`)
  - CSI erase (ED 0-3, EL 0-2) → evidence: 07.1 matrix row (7 tests in `csi_erase.rs`)
  - Erase-with-attributes → evidence: `edge_erase_with_attrs` in `workflows/edge.rs`
  - CSI insert/delete (ICH, DCH, IL, DL) → evidence: 07.1 matrix row (4 tests in `csi_insert_delete.rs`)
  - Mode interactions (already marked [x] in 00-overview.md) → evidence: 34 tests in `mode_interactions.rs`
  - SGR (all specified attributes) → evidence: 07.1 matrix row (54 .teseq + 1 variant = 55 tests across `sgr/*.rs`)
  - Reports (already marked [x] in 00-overview.md) → evidence: 18 tests in `csi_reports.rs`
  - ESC (DECSC/RC, RIS, SCS, IND, RI) → evidence: 07.1 matrix row (5 tests in `esc.rs`)
  - OSC (title, icon, clipboard, color query) → evidence: 07.1 matrix row (4 tests in `osc.rs`)
  - OSC 7 documented limitation → evidence: comment in 07.1 gap analysis
  - Workflow scenarios → evidence: 07.1 matrix row (31 tests across `workflows/*.rs`)
  - Multi-size testing → evidence: `.toml` sidecars with 97x33 and 120x40 sizes
  - `cargo test --test teseq` passes → evidence: 07.4 debug run output
  - `./test-all.sh` green → evidence: 07.4 full suite run
  - CI graceful skip → evidence: 07.2 CI config + skip audit

- [ ] **Check off mission criteria** — update `00-overview.md` to check every satisfied criterion checkbox. If any criterion is NOT satisfied, fix it in this section (not deferred).

---

## 07.R Third Party Review Findings

- [x] `[TPR-07-001][low]` [plans/teseq-conformance/section-07-verification.md](/home/eric/projects/ori_term/plans/teseq-conformance/section-07-verification.md#L12) — Section 07's headline success criteria and test matrix carried incorrect counts.
  Evidence: original plan had `171 .teseq + 5 pure-Rust` but actual counts are `168 .teseq + 7 pure-Rust + 1 scenario-variant = 176`.
  Resolution: Fixed during /review-plan Mode A review. Success criteria, test matrix, completion checklist, and exit criteria all updated to `168 .teseq + 7 pure-Rust + 1 variant = 176`. Test matrix restructured with Variant column and per-family counts verified against `find` and `cargo test --list`.

---

## 07.N Completion Checklist

- [ ] Test matrix documented with scenario counts per family (176 total test functions)
- [ ] Coverage gap analysis completed with priority ranking
- [ ] Skip path correctness audit passed — every family module's reseq guard verified
- [ ] Graceful skip verified on platform without reseq
- [ ] CI configuration updated: `teseq` added to Linux CI apt install in `.github/workflows/ci.yml`
- [ ] CI integration documented in CLAUDE.md
- [ ] CLAUDE.md updated with teseq commands and paths
- [ ] Scenario authoring guide in main.rs module docs
- [ ] Expected test count documented in main.rs module docs (drift checkpoint)
- [ ] Future scenario priorities documented in main.rs module docs
- [ ] All builds green: `./build-all.sh`, `./clippy-all.sh`, `timeout 150 ./test-all.sh`
- [ ] Release profile green: `timeout 150 cargo test -p oriterm_core --test teseq --release`
- [ ] No regressions in vttest or oriterm_core unit tests
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] **Mission success criteria** — every checkbox in `00-overview.md` verified and checked off
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` status → `complete`, all Quick Reference statuses updated
  - [ ] `00-overview.md` mission success criteria checkboxes — all checked
  - [ ] `index.md` all section statuses updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** Complete test matrix showing 176 test functions (168 .teseq + 7 pure-Rust + 1 scenario-variant) across 10 protocol families, all passing in both debug and release profiles. Skip path correctness audited across all family modules. Coverage gaps documented and prioritized. CI updated to install teseq on Linux (macOS/Windows skip gracefully). Every mission success criterion from 00-overview.md explicitly verified and checked off. CLAUDE.md updated. `timeout 150 ./test-all.sh` green with zero regressions across all existing test suites. The teseq conformance plan is fully closed out — no open criteria, no deferred items.
