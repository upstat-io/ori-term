---
section: "23"
title: "Cross-Stack Regression Sweep + Coverage Report CI"
status: not-started
reviewed: false
goal: "Wire the cross-stack regression sweep into CI: every PR runs every stack's verification chain, every per-stack test binary stays under the 150s test cap, the coverage report (delivered by section 04) runs in `--check` mode and fails the build on any regression."
success_criteria:
  - "`.github/workflows/spec-conformance.yml` exists with: per-stack test job (one job per Phase 3 stack), the coverage-report `--check` job, and a per-platform apex matrix for OS-dependent layers (clipboard, audio, focus, kitty file/shm transports, title, shell integration)"
  - "Every per-stack test binary runs in under 150 seconds (the CLAUDE.md test timeout cap). If any binary approaches the cap, it MUST be split into per-stack-subset binaries."
  - "Coverage report `--check` mode reads the previous baseline from `plans/spec-conformance/coverage-baseline.toml` and fails CI on any drop in the ABSOLUTE `verified` row count per stack (NOT percentage — see section 04.8 monotonicity rationale)."
  - "Coverage report `--check` mode ALSO fails CI when: (a) the cataloging safety net (04.9) finds uncataloged sequences in committed captures, (b) a row is marked `verified` in the catalog but no test cites the row ID, or (c) a test cites a row ID that doesn't exist in any catalog file."
  - "Per-platform apex matrix runs the OS-dependent layers (clipboard, audio, etc.) on each platform — Linux x86_64 is the canonical lane (gating); macOS and Windows are smoke (non-gating, but still run)"
  - "Lands once Phase 3 has produced ~3 verified stacks (so the report has something meaningful to display)"
  - "Legacy external-tool-dependent test suites (teseq, tack, vttest) are deleted once the spec verification chain fully covers their scenarios. Zero external binary dependencies remain for test execution."
  - "All existing CI workflows continue to pass"
  - "`./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green debug + release"
  - "Section's mission criterion connection: contributes to **Cross-stack regression sweep green** AND **Coverage report green**"
inspired_by:
  - "ori_term existing `.github/workflows/` (the repo's current CI setup) — pattern for adding the new workflow"
  - "tack-conformance section 09 (verification) — original plan for cross-platform verification, now subsumed by this section"
depends_on: ["04", "08"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "23.1"
    title: "Build per-stack test binary structure"
    status: not-started
  - id: "23.2"
    title: "Wire coverage-report --check mode into CI"
    status: not-started
  - id: "23.3"
    title: "Build per-platform apex matrix"
    status: not-started
  - id: "23.4"
    title: "Verify per-stack binary runs under 150s"
    status: not-started
  - id: "23.5"
    title: "Remove external-tool-dependent legacy test suites"
    status: not-started
  - id: "23.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "23.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 23: Cross-Stack Regression Sweep + Coverage Report CI

**Status:** Not Started
**Goal:** Wire the cross-stack regression sweep into CI. Every PR runs every stack's verification chain; the coverage report fails CI on any regression. Per-platform apex matrix runs OS-dependent layers on each platform.

**Success Criteria:** see frontmatter.

**Context:** The cross-stack regression sweep catches the class of bugs where fixing kitty silently breaks sixel (they share the image cache + GPU pipeline). Per-stack tests structurally cannot find these — only running every stack against every PR can. The coverage report (delivered by section 04) is the gating metric: a row dropping from `verified` to any lower status is a build failure.

**Reference implementations:** see frontmatter.

**Absorbed from tack-conformance section 09:** This section explicitly inherits
the cross-platform skip matrix, the flake-proofing gate (5 runs × 2 thread-modes
× debug/release), the DA/DSR cross-validation contract against vttest menu6, the
alloc/RSS regression checks, and the archival-in-place covenant (no `git mv`).
Tack-conformance sections 01–08 are already `complete` on `main` — the shared
`PtySession`, pinned `ori_term.info`, scenario framework, test/tools menu scenario
families, GPU goldens for the visual subset, and the kf1–kf63 terminfo cross-check
all exist. Section 23 **consumes** those artifacts as existing regression fixtures
and wires them into the new coverage-report `--check` CI gate. The two currently
unchecked mission criteria in `plans/tack-conformance/00-overview.md` (cross-platform
skip cleanliness and the `./test-all.sh`/`./build-all.sh`/`./clippy-all.sh` green
gate) are this section's responsibility. Canonical absorption policy:
see [plans/spec-conformance/00-overview.md §Tack Absorption Strategy](./00-overview.md#tack-absorption-strategy-delivered-by-section-02).

**Depends on:** Section 04 (coverage report binary exists), Section 08 (baseline + ~3 stacks verified so the report has meaningful content).

---

## 23.1 Build per-stack test binary structure

**File(s):** `oriterm_core/tests/spec_chain/main.rs` (existing — extended), `crates/oriterm_test_support/Cargo.toml` (extended)

- [ ] Verify the per-stack tests are organized into separate test binaries (or test modules within a single binary). The 150-second cap applies per binary, so per-stack binaries scale better than one giant binary.
- [ ] Naming convention: `cargo test -p oriterm_core --test spec_chain_<stack>` (e.g., `spec_chain_sixel`, `spec_chain_kitty`, `spec_chain_osc`)
- [ ] Each binary exposes only its stack's tests; cross-stack regression tests live in a dedicated `spec_chain_cross_stack` binary
- [ ] **Validation**: each per-stack binary compiles and runs its own tests in isolation.

---

## 23.2 Wire coverage-report --check mode + uncataloged backlog gate into CI

**File(s):** `.github/workflows/spec-conformance.yml` (new), `plans/spec-conformance/coverage-baseline.toml` (committed initial baseline), `plans/spec-conformance/uncataloged-backlog.md` (committed — starts empty)

CI runs `spec-coverage-report --check` on every PR. The `--check` mode enforces all four gates:
1. **Absolute verified count monotonic** — per-stack `verified` row count only ever increases (stale `coverage-baseline.toml` compared against current report).
2. **No false-verified rows** — every `verified` row in the catalog must have at least one test citing its row ID.
3. **No uncataloged citations** — every test citation to a row ID must resolve to an existing catalog row.
4. **Empty uncataloged backlog** — `plans/spec-conformance/uncataloged-backlog.md` must be empty (beyond headers), OR the current PR must include an accompanying catalog-update that adds all backlog rows AND clears the backlog.
A failure in ANY of the four gates fails CI.

- [ ] Create the GitHub Actions workflow:
  ```yaml
  name: Spec Conformance
  on:
    pull_request:
      branches: [main, dev]
    push:
      branches: [main, dev]
  jobs:
    coverage-check:
      runs-on: ubuntu-latest
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - name: Run coverage report --check
          run: cargo run -p oriterm_test_support --bin spec-coverage-report -- --check
    per-stack-tests:
      runs-on: ubuntu-latest
      strategy:
        matrix:
          stack: [ecma_48, dec_private_modes, osc, sixel, kitty_graphics, iterm2, unicode_subcell, mouse, kitty_keyboard, charsets, historical, audio_print]
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - name: Test stack ${{ matrix.stack }}
          run: timeout 150 cargo test -p oriterm_core --test spec_chain_${{ matrix.stack }}
  ```
- [ ] Commit the initial coverage baseline at `plans/spec-conformance/coverage-baseline.toml` — the output of `cargo run -p oriterm_test_support --bin spec-coverage-report` at the start of CI integration
- [ ] As stacks reach `verified`, the baseline updates automatically (or manually via a CI bot) — the rule is "verified count only goes up"
- [ ] **Validation**: CI workflow passes on a clean main; CI fails on a manually-injected regression

---

## 23.3 Build per-platform apex matrix

**File(s):** `.github/workflows/spec-conformance.yml` (extended)

OS-dependent apex layers (clipboard, audio, focus, kitty file/shm transports, title, shell integration) need to be tested on each platform.

- [ ] Add a per-platform job matrix to the workflow:
  ```yaml
    os-dependent-tests:
      strategy:
        matrix:
          os: [ubuntu-latest, macos-latest, windows-latest]
      runs-on: ${{ matrix.os }}
      steps:
        - uses: actions/checkout@v4
        - uses: dtolnay/rust-toolchain@stable
        - name: Test OS-dependent apices
          run: cargo test -p oriterm_core --test spec_chain_os_dependent
  ```
- [ ] Linux is the canonical gating lane; macOS and Windows are non-gating (`continue-on-error: true`) per Codex's recommendation — but they still RUN so regressions surface
- [ ] **Validation**: per-platform job runs and reports per-platform pass/fail

---

## 23.4 Verify per-stack binary runs under 150s

- [ ] Run each per-stack binary locally and time it: `time cargo test -p oriterm_core --test spec_chain_<stack>`
- [ ] Any binary > 100s should be split (leave 50s buffer for slower CI runners)
- [ ] Document the per-stack runtime in `plans/spec-conformance/coverage-baseline.toml` so future additions can verify they don't exceed the cap
- [ ] **Validation**: every per-stack binary runs in under 150s on the dev machine.

---

## 23.5 Remove external-tool-dependent legacy test suites

**Goal:** Once the spec verification chain fully covers every scenario that tack, vttest, and teseq currently test, delete the legacy test files and their external-tool dependencies. The spec-conformance verification chain is self-contained Rust — no platform-specific binaries, no graceful-skip logic, runs identically on Linux/macOS/Windows.

**Legacy test suites to remove:**

| Suite | Test file | External binary | Platform |
|-------|-----------|-----------------|----------|
| teseq | `oriterm_core/tests/teseq/` | `reseq` (from `teseq` package) | Linux only |
| tack | `oriterm_core/tests/tack/` | `tack` | Linux only |
| vttest | `oriterm_core/tests/vttest/` | `vttest` | Linux only |

**Pre-removal gate (all must be true):**
- [ ] Every catalog row that cites a teseq scenario has `verified` status in the coverage report (the spec chain test covers it end-to-end)
- [ ] Every catalog row that cites a tack scenario has `verified` status
- [ ] Every catalog row that cites a vttest scenario has `verified` status
- [ ] The `_legacy-tack-mapping.md` table shows `covered` for every row that was originally tested by tack
- [ ] `spec-coverage-report --check` passes with the legacy suites removed (no coverage regression)

**Cleanup steps:**
- [ ] Remove `oriterm_core/tests/teseq/` directory and all snapshot fixtures
- [ ] Remove `oriterm_core/tests/tack/` directory
- [ ] Remove `oriterm_core/tests/vttest/` directory
- [ ] Remove `PtySession` helper code that is exclusively used by legacy suites (keep shared helpers still used by spec chain)
- [ ] Remove `ScenarioRunner::available()`, `tack_version_supported()`, and other external-tool gate functions that become dead code
- [ ] Remove teseq/tack/vttest references from `CLAUDE.md` Commands section and `.claude/rules/tests.md`
- [ ] Remove `sudo apt install teseq` instructions from docs
- [ ] Update `test-all.sh` if it has explicit teseq/tack/vttest invocations
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green — no dead code warnings from removed imports
- [ ] **Validation**: `cargo test --workspace` passes on all three platforms without any external tool installed. Zero `SKIP:` messages in test output.

---

## 23.R Third Party Review Findings

- None.

---

## 23.N Completion Checklist

- [ ] Failing test matrix written FIRST: CI workflow tested on a regression PR
- [ ] **Matrix dimensions**: stack × OS × CI lane (gating/non-gating)
- [ ] **Semantic pin**: coverage report `--check` mode is the regression guard for the entire conformance percentage
- [ ] CI workflow exists and runs on every PR
- [ ] Per-stack test binaries all under 150s
- [ ] Coverage report `--check` fails CI on regression
- [ ] Per-platform apex matrix runs on macOS / Linux / Windows
- [ ] Legacy test suites (teseq, tack, vttest) removed — zero external tool dependencies
- [ ] All existing CI workflows still pass
- [ ] Alloc regression unchanged
- [ ] `./build-all.sh`, `./test-all.sh`, `./clippy-all.sh` green
- [ ] Plan annotation cleanup
- [ ] Section frontmatter `status` → `complete`
- [ ] `00-overview.md` Quick Reference + mission criteria updated
- [ ] `index.md` section 23 status updated
- [ ] `/tpr-review` passed
- [ ] `/impl-hygiene-review last commit` passed (after `/tpr-review` is clean)

**Exit Criteria:** CI runs the cross-stack regression sweep on every PR; coverage report fails CI on regression; per-platform apex matrix tests OS-dependent layers.
