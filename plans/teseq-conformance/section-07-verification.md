---
section: "07"
title: "Verification & CI Integration"
status: not-started
reviewed: false
goal: "Verify the complete teseq test framework, document coverage gaps, integrate with CI, and ensure graceful degradation on all platforms"
success_criteria:
  - "Test matrix documents all scenarios by protocol family with pass/fail status"
  - "Coverage gap analysis identifies sequences not yet covered and prioritizes future additions"
  - "CI integration: Linux CI installs teseq and runs tests; macOS/Windows gracefully skip"
  - "CLAUDE.md updated with teseq test commands and scenario authoring instructions"
  - "All 106+ scenarios pass at their designated sizes"
  - "Satisfies mission criteria: CI green, test-all.sh passes, platform graceful skip"
inspired_by:
  - "ori_term vttest-conformance section-07-verification — conformance metric tracking"
  - "ori_term vttest session.rs:232-239 — vttest_available() graceful skip pattern"
depends_on: ["01", "02", "03", "04", "05", "06"]
third_party_review:
  status: none
  updated: null
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
  - id: "07.R"
    title: "Third Party Review Findings"
    status: not-started
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

**Context:** This section is the final verification gate. It doesn't add new scenarios — it verifies the complete framework, documents its coverage, and ensures it integrates cleanly with the project's CI and development workflow.

**Depends on:** All sections (01-06).

---

## 07.1 Test Matrix & Coverage Analysis

Build a comprehensive test matrix documenting every scenario.

- [ ] **Scenario inventory by family:**

  | Family | Scenarios | Sizes | Key Coverage |
  |--------|-----------|-------|-------------|
  | C0 (`c0/`) | 8+ | 80x24 | CR, LF, BS, TAB, BEL, FF, VT, SO/SI |
  | CSI Cursor (`csi/cursor/`) | 8+ | 80x24, 97x33, 120x40 | CUP, CUU, CUD, CUF, CUB, VPA, HPA, CHA |
  | CSI Erase (`csi/erase/`) | 7 | 80x24 | ED 0-3, EL 0-2 |
  | CSI Insert/Delete (`csi/insert_delete/`) | 4 | 80x24 | ICH, DCH, IL, DL |
  | CSI Reports (`csi/reports/`) | 10+ | 80x24 | DA1/2/3, DSR, DECRQM |
  | CSI Modes (`csi/modes/`) | 34+ | 80x24, 97x33, 120x40 | DECOM, DECCOLM, alt screen, IRM, wrap, cross-cutting |
  | CSI SGR (`csi/sgr/`) | 15+ | 80x24 | Attributes, 16/256/TrueColor, combos |
  | ESC (`esc/`) | 4+ | 80x24 | DECSC/RC, RIS, SCS, IND/RI |
  | OSC (`osc/`) | 4+ | 80x24 | Title (0/2), icon name (1), clipboard (52), color query (4/10/11) |
  | Workflows (`workflows/`) | 12+ | 80x24, 97x33, 120x40 | Mode combos, handshakes, real-world |
  | **Total** | **106+** | | |

- [ ] **Coverage gap analysis** — identify sequences NOT yet covered:
  - OSC 7 (CWD) — handled by `RawInterceptor` in `oriterm_mux`, not `Term<T>`; tested at mux layer (`oriterm_mux/src/shell_integration/tests.rs::interceptor_osc7_sets_cwd`), not teseq harness
  - DCS sequences (DECRQSS, Sixel, Kitty image protocol) — complex, may warrant future plan. Note: DECRQSS IS implemented in `status.rs` and could be added as a teseq scenario (it uses PtyWrite events)
  - OSC color set (OSC 4/10/11 with color values, not just queries) — Section 06.4 covers queries only
  - Tab stops (HTS, TBC) — not tested in teseq yet
  - Soft terminal reset (DECSTR via CSI ! p) — not tested
  - Conformance level (DECSCL) — not tested
  - Wide characters (CJK, emoji) — teseq doesn't represent width, handled by GPU golden tests

- [ ] **Priority ranking** for future scenario additions based on bug risk and interoperability importance.

---

## 07.2 Platform & CI Integration

- [ ] **Graceful skip verification:**
  - Verify `reseq_available()` returns false when `reseq` is not installed
  - Verify all teseq tests print "reseq not installed, skipping" and return Ok (not panic/fail)
  - Test by temporarily renaming `/usr/bin/reseq` and running `timeout 150 cargo test -p oriterm_core --test teseq`

- [ ] **CI considerations:**
  - Linux CI: `sudo apt install teseq` in CI setup (Debian/Ubuntu package available)
  - macOS CI: `brew install teseq` if available, or skip gracefully
  - Windows CI: Always skip (teseq is Unix-only, matching vttest pattern)
  - Document in CI config or CLAUDE.md

- [ ] **`test-all.sh` integration:**
  - Verify `./test-all.sh` runs teseq tests as part of `cargo test --workspace`
  - No modifications needed to `test-all.sh` — Cargo auto-discovers integration tests
  - Verify: `timeout 150 ./test-all.sh` passes with teseq tests included (mandatory timeout per CLAUDE.md)

---

## 07.3 Documentation Updates

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

---

## 07.4 Build & Verify

- [ ] `./build-all.sh` green (all platforms)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `timeout 150 ./test-all.sh` green (all tests pass, including teseq)
- [ ] `timeout 150 cargo test -p oriterm_core --test teseq` — all scenarios pass
- [ ] `timeout 150 cargo test -p oriterm_core --test vttest` — no regressions in vttest
- [ ] `timeout 150 cargo test -p oriterm_core` — no regressions in handler unit tests

---

## 07.R Third Party Review Findings

- None.

---

## 07.N Completion Checklist

- [ ] Test matrix documented with scenario counts per family (106+ total)
- [ ] Coverage gap analysis completed with priority ranking
- [ ] Graceful skip verified on platform without reseq
- [ ] CI integration documented
- [ ] CLAUDE.md updated with teseq commands and paths
- [ ] Scenario authoring guide in main.rs module docs
- [ ] All builds green: `./build-all.sh`, `./clippy-all.sh`, `timeout 150 ./test-all.sh`
- [ ] No regressions in vttest or handler tests
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] **Plan sync** — update plan metadata:
  - [ ] This section's frontmatter `status` → `complete`
  - [ ] `00-overview.md` status → `complete`, all Quick Reference statuses updated
  - [ ] `00-overview.md` mission success criteria checkboxes updated
  - [ ] `index.md` all section statuses updated
- [ ] `/tpr-review` passed (final, full-section)
- [ ] `/impl-hygiene-review last commit` passed

**Exit Criteria:** Complete test matrix showing 106+ scenarios across 10 protocol families, all passing. Coverage gaps documented and prioritized. CI graceful degradation verified. CLAUDE.md updated. `timeout 150 ./test-all.sh` green with zero regressions across all existing test suites. The teseq framework is production-ready for ongoing scenario additions.
