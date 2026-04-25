---
plan: "clippy-gate-hardening"
title: "Clippy Gate Hardening + Workspace Test-Target Violation Cleanup: Exhaustive Implementation Plan"
status: in-progress
reviewed: false
supersedes:
  - "plans/bug-tracker/section-07-ci-build.md (BUG-07-005)"
  - "plans/bug-tracker/section-07-ci-build.md (BUG-07-006)"
  - "plans/bug-tracker/section-07-ci-build.md (BUG-07-010)"
  - "plans/bug-tracker/section-07-ci-build.md (BUG-07-012)"
  - "plans/bug-tracker/section-07-ci-build.md (BUG-07-NNN: oriterm_mux — concrete ID assigned in Section 01.2)"
  - "plans/bug-tracker/section-07-ci-build.md (BUG-07-NNN+1: oriterm_ipc — concrete ID assigned in Section 01.2)"
  - "plans/bug-tracker/section-07-ci-build.md (BUG-07-NNN+2: oriterm — concrete ID assigned in Section 01.2)"
references:
  - "plans/bug-tracker/section-07-ci-build.md"
  - "plans/tack-conformance/section-04-scenario-framework.md"
  - "oriterm/tests/architecture.rs"
  - "Cargo.toml (workspace.lints)"
  - "clippy.toml (workspace root)"
---

<!-- Plan type (informational, not in PlanOverviewSchema): application -->
<!-- All sections use the full application template (TPR, impl-hygiene, improve-tooling, sync-claude). -->
<!-- Three new BUG-07-NNN ordinals will be assigned in Section 01.2 and Section 01.N updates the supersedes: list with concrete IDs. -->


# Clippy Gate Hardening + Workspace Test-Target Violation Cleanup: Exhaustive Implementation Plan

## Mission

Flip the workspace clippy gate (`./clippy-all.sh`, `.github/workflows/ci.yml` clippy jobs, and `lefthook.yml` pre-commit clippy hook) from `--workspace` (lib + bin only) to `--workspace --all-targets` plus a per-crate feature matrix; clean up the ~1480 violations that surface across all 6 workspace crates; and pin the gate via a source-text regression test (modeled on `oriterm/tests/architecture.rs:238-294`) so the gate scope cannot silently regress in any of the three locations. Cluster bugs `BUG-07-005`, `BUG-07-006`, `BUG-07-010`, `BUG-07-012` plus three new bugs filed during this plan for `oriterm_mux` / `oriterm_ipc` / `oriterm` gaps close as `Superseded by: plans/clippy-gate-hardening/`.

## Mission Success Criteria

The mission is complete when ALL of these are true:

- [ ] `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` exits 0 (default features)
- [ ] `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` exits 0
- [ ] Per-crate feature combos exit 0 — verified in Section 09:
  - `cargo clippy -p oriterm_core --no-default-features --all-targets -- -D warnings`
  - `cargo clippy -p oriterm_ui --features testing --all-targets -- -D warnings`
  - `cargo clippy -p oriterm --features gpu-tests --all-targets -- -D warnings`
  - `cargo clippy -p oriterm --features profile --all-targets -- -D warnings`
- [ ] `./clippy-all.sh` exits 0 with the new gate flags (Section 09 cleanup landed; Section 10 flips the script)
- [ ] `./build-all.sh` and `./test-all.sh` green throughout — no test regression introduced by lint fixes
- [ ] `.github/workflows/ci.yml` `clippy` and `clippy-windows-cross` jobs invoke `--all-targets` with the matching feature matrix; CI green
- [ ] `lefthook.yml` `clippy:` pre-commit hook invokes `--workspace --all-targets --target x86_64-pc-windows-gnu` (Windows GNU only by Section 09.3 design decision; CI host clippy job covers host); pre-commit green
- [ ] Source-text meta-test in `oriterm/tests/architecture.rs` (or sibling) asserts `--all-targets` appears in `clippy-all.sh` AND in both `ci.yml` clippy invocations AND in `lefthook.yml` clippy hook — fails immediately if any of the three loses the flag
- [ ] Cluster bugs closed bidirectionally: `BUG-07-005` / `BUG-07-006` / `BUG-07-010` / `BUG-07-012` marked `[x] Superseded by: plans/clippy-gate-hardening/`; `plans/bug-tracker/section-07-ci-build.md` open count decremented by 4
- [ ] Three new tracker entries (filed in Section 01) closed bidirectionally: `BUG-07-NNN` (oriterm_mux), `BUG-07-NNN` (oriterm_ipc), `BUG-07-NNN` (oriterm) marked `[x] Superseded by: plans/clippy-gate-hardening/`; section-07 open count decremented by 3 more
- [ ] All section success criteria met

## Architecture

```
Current state (THE BUG):
  ./clippy-all.sh ──┐
  ci.yml clippy job ──┤── all run: cargo clippy --workspace -- -D warnings
  ci.yml win-cross   ──┤   (lib + bin only — TEST TARGETS, INTEGRATION TESTS,
  lefthook clippy   ──┘    AND FEATURE-GATED CODE INVISIBLE)

  ~1480 violations accumulated across 6 crates because they were never seen.

Target state (THE FIX):
  ./clippy-all.sh ──┐
  ci.yml clippy job ──┤── all run: cargo clippy --workspace --all-targets [features] -- -D warnings
  ci.yml win-cross   ──┤   (test targets + integration tests + feature-gated code COVERED)
  lefthook clippy   ──┘
                       └── Source-text meta-test pins the flag set in all THREE files.
                           A flag-removal in any single file fails CI immediately.

Cleanup pipeline (Phase 2 of the plan, dependency direction):
  oriterm_ipc → oriterm_core → oriterm_test_support → oriterm_ui → oriterm_mux → oriterm
       8        485+0nd*           27                  761         192            6
                                                       (616 = 50 test files
                                                        with intentional float_cmp;
                                                        all use module-level
                                                        #![expect(...reason)])
  *0nd = additional surface from --no-default-features pass

  Per-crate workflow:
    1. cargo clippy --fix --all-targets -p {crate}    # auto-fixable lints
    2. Manual diff review (Gemini's manual_let_else / drop-timing concern)
    3. Commit auto-fix
    4. Manual cleanup of remaining structural + judgment lints
    5. Cross-target verification (host + Windows GNU)
    6. cargo clippy --all-targets -p {crate} -- -D warnings exits 0
```

## Design Principles

**1. Per-crate scoping; depth over breadth.**
Each crate gets ONE complete cleanup section. No interleaving across crates. Once a crate is clean, it stays clean and gets the per-crate `cargo clippy -p {crate} --all-targets -- -D warnings` checkpoint as the section's exit gate.

*Why:* Lint fixes can compound — a `manual_let_else` rewrite in `oriterm_core` may force callers in `oriterm_mux` to re-clippy. Linear dependency-order processing means each downstream crate sees an already-clean upstream. Smallest-first ordering (which `/tp-help` round-1 considered) loses this benefit because the small crates are mostly leaves.

**2. Mechanical-first auto-fix with mandatory diff review; judgment-last.**
Every per-crate section has a `cargo clippy --fix --all-targets -p {crate}` subsection that captures the diff, REQUIRES manual review (not blind acceptance), and commits as a single atomic per-crate auto-fix commit. Manual cleanup of structural and judgment lints follows.

*Why:* `cargo clippy --fix` is **not always semantics-preserving** — Gemini round-1 specifically flagged `manual_let_else` rewrites that can change drop-timing behavior. Auto-fix is a tool, not a verdict; the diff is reviewed by a human (or by the section's TPR gate) before the commit lands. Mechanical lints (`doc_markdown`, `redundant_closure_for_method_calls`, `needless_raw_strings`, `field_reassign_with_default`, `format_push_string`) ARE safe to auto-fix; structural lints (`manual_let_else`, `redundant_clone`, `manual_assert`) are not.

**3. Gate flip LAST — only after every crate is clean.**
The single biggest risk is flipping the gate prematurely and breaking CI for everyone (including bystander branches that haven't yet rebased). Section 09 (gate flip) runs ONLY after every per-crate cleanup section reports `cargo clippy -p {crate} --all-targets -- -D warnings` exits 0.

*Why:* The repo's other developers (and any in-flight PRs) will hit the new gate on their next CI run. Premature flip = mass regression. Per-crate completion is the safe ratchet.

**4. Three places, one truth — pinned by meta-test.**
The current gate gap exists in THREE files (`./clippy-all.sh`, `.github/workflows/ci.yml` × 2 jobs, `lefthook.yml` clippy hook). A future "DRY" refactor that consolidates them is out of scope (CI jobs have platform setup, cache keys, and OS-package installs that don't fit a shared script — verified at `ci.yml:50-63`); instead, a source-text regression test reads all four files and asserts the canonical flag set appears in each. Removal of `--all-targets` from any single location fails the test.

*Why:* DRY-ing a shared `scripts/clippy-gate.sh` invoked from CI would force the CI jobs to bend around the script's structure, losing the explicit per-job flag visibility that helps someone reading `ci.yml` understand what the job actually checks. The meta-test enforces the SSOT invariant ("all gates check `--all-targets`") without forcing a single execution surface. Prior art for source-text pin: `oriterm/tests/architecture.rs:238-294` (call-sequence pin) and `:330-342` (removal pin).

**5. Tracker artifacts up front.**
Section 01 files three new bug-tracker entries for the previously-undocumented `oriterm_mux` (192), `oriterm_ipc` (8), and `oriterm` (6 — the "9" included 3 surfaced from `oriterm_ui/testing` via dev-dep) gaps BEFORE any cleanup work begins. The plan then progresses against the (4 cluster + 3 new = 7) tracked artifacts.

*Why:* CLAUDE.md §Bug Discipline mandates concrete tracked artifacts for every discovered bug. Filing in Section 01 makes the cleanup observable against the bug-tracker; closing them (bidirectional supersede) in Section 10 produces the audit trail.

## Section Dependency Graph

```
                  Section 01 (Baseline + 3 bug filings)
                                │
                                ▼
                  Section 02 (oriterm_ipc — 8)
                                │
                                ▼
                  Section 03 (oriterm_core — 485)
                                │
                                ▼
                  Section 04 (oriterm_test_support — 27)
                                │
                                ▼
                  Section 05 (oriterm_ui — 761; 145 mech/structural + 616 float_cmp)
                                │
                                ▼
                  Section 06 (oriterm_mux — 192)
                                │
                                ▼
                  Section 07 (oriterm — 6)
                                │
                                ▼
                  Section 08 (Feature matrix verification)
                                │
                                ▼
                  Section 09 (Gate flip + meta-test)
                                │
                                ▼
                  Section 10 (Cluster + new bug closure)
```

Sections are STRICTLY linear in dependency order. No section is independent — each downstream section's "cargo clippy -p {crate} --all-targets" checkpoint depends on every upstream crate already being clean (transitive dependency closure).

**Cross-section interactions (must be co-implemented):**
- **Section 09 + Section 10**: gate flip and bug closure must land together. Flipping the gate without closing the bugs leaves the bug-tracker out-of-sync with reality (bugs marked open while their root cause is fixed). Closing the bugs without the gate flip leaves the cluster claiming `Superseded by:` to a plan that hasn't actually delivered the gate change.

## Implementation Sequence

```
Phase 0 — Inventory & Tracking
  └─ Section 01: baseline lint counts captured per crate per target;
                  three new BUG-07-NNN entries filed for the unfiled gaps;
                  bug-tracker section-07 reflects the seven-bug cluster.
  Gate: `python -m scripts.plan_corpus check plans/bug-tracker/section-07-ci-build.md` exits 0;
        section 01 frontmatter status: complete.

Phase 1 — Per-crate cleanup (dependency order, single-file commits per phase)
  └─ Section 02: oriterm_ipc clean
  └─ Section 03: oriterm_core clean
  └─ Section 04: oriterm_test_support clean
  └─ Section 05: oriterm_ui clean (split: structural lints first, then 50-file float_cmp expect review)
  └─ Section 06: oriterm_mux clean
  └─ Section 07: oriterm clean
  Gate per section: `cargo clippy -p {crate} --all-targets --target x86_64-unknown-linux-gnu -- -D warnings`
                    AND `cargo clippy -p {crate} --all-targets --target x86_64-pc-windows-gnu -- -D warnings`
                    both exit 0; `cargo test -p {crate}` green; `./test-all.sh` green.
  Gate cross-section: cumulative — Section N's gate also includes Sections 02..N-1's gates.

Phase 2 — Cross-cutting verification
  └─ Section 08: per-crate feature matrix combos (default, --no-default-features for oriterm_core,
                  --features testing for oriterm_ui, --features gpu-tests + --features profile for oriterm)
                  × host + Windows GNU; document any new violations surfaced and fix them inline.
  Gate: every feature combo exits 0; full workspace `cargo clippy --workspace --all-targets -- -D warnings`
        on host AND Windows GNU exits 0.

Phase 3 — Gate flip [CRITICAL PATH]
  └─ Section 09: update `./clippy-all.sh`, both `ci.yml` clippy jobs, and `lefthook.yml` clippy hook;
                  add source-text meta-test to `oriterm/tests/architecture.rs` (or new sibling);
                  if CI 15-min timeouts at ci.yml:44 / :69 are exceeded, raise to 25 min in same commit.
  Gate: `cargo test -p oriterm --test clippy_gate` green (meta-test passes — see Section 09.4 which creates the new sibling test file `oriterm/tests/clippy_gate.rs`);
        `./clippy-all.sh` exits 0; CI green on the PR introducing the change.

Phase 4 — Bidirectional bug closure
  └─ Section 10: mark all 7 bugs `[x] Superseded by: plans/clippy-gate-hardening/`;
                  decrement section-07 open count by 7; archive plan as `resolved`.
  Gate: `python -m scripts.plan_corpus check` exits 0;
        bidirectional supersede invariant satisfied per `plans/bug-tracker/00-overview.md`.
```

**Why this order:**
- Phase 0 is pure tracking — no code changes — and produces the seven concrete artifacts the plan progresses against.
- Phase 1 sections are STRICTLY linear because the per-crate clippy gate checkpoint at each section's close depends on every upstream crate being clean. Skipping ahead leaves a dirty upstream that re-fails when its lints surface in downstream targets.
- Phase 2 catches anything the default-feature passes missed (oriterm_core's `image-protocol` flag, oriterm_ui's `testing` flag, oriterm's `gpu-tests` and `profile` flags).
- Phase 3 is the critical path — flipping any of the three gates without all upstream clean breaks CI for everyone.
- Phase 4 closes the loop bidirectionally per the bug-tracker schema's supersede invariant.

**Known failing tests (expected until plan completion):** None. The plan never introduces a failing test; existing tests remain green throughout (lint fixes are semantics-preserving by construction; the auto-fix diff review subsection in each per-crate section explicitly catches `manual_let_else` / drop-timing changes that could break tests).

## Metrics (Current State, captured 2026-04-25)

| Crate | Default `--all-targets` violations | Mechanical (M) | Structural (S) | Judgment (J) | Notes |
|---|---:|---:|---:|---:|---|
| `oriterm_ipc` | 8 | 3 | 5 | 0 | redundant_clone × 5 dominates |
| `oriterm_core` | 485 | ~430 | ~30 | ~35 | doc_markdown 301, field_reassign 42, needless_raw 29, float_cmp 21, string_slice 14 |
| `oriterm_test_support` | 27 | 19 | 5 | 0 (nominally) | doc_markdown 7, format_push_string 4 |
| `oriterm_ui` | 761 | ~120 | ~25 | ~616 | float_cmp 616 = 50 test files (per-file `#![expect]`); structural ~25 |
| `oriterm_mux` | 192 | 121 | 18 | 14 | doc_markdown 85, used_underscore_binding 37 |
| `oriterm` | 6-9 | 3 | 0 | 3-6 | float_cmp surfaced via `oriterm_ui/testing` dev-dep |
| **Total (default features)** | **~1480** | **~700** | **~80** | **~660** | |

Cross-target deltas (default features, `--target x86_64-pc-windows-gnu`): expected to be small (most lint patterns are platform-independent), but Section 08 verifies.

Per-crate `clippy.toml` overrides: workspace root `clippy.toml` sets `too-many-arguments-threshold = 5` (lower than default 7) and `avoid-breaking-exported-api = false`.

Per-crate `[lints]` overrides: `oriterm_ipc/Cargo.toml` declares its own `[lints]` block (NOT `workspace = true`) to set `unsafe_code = "allow"` while copying every other workspace lint. Section 02 verifies this divergence is preserved (or migrated to `workspace = true` with a per-file `#[allow(unsafe_code)]` if cleaner).

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| 01 Baseline + Bug Filing | ~200 | Low | — |
| 02 oriterm_ipc cleanup | ~150 | Low | 01 |
| 03 oriterm_core cleanup | ~350 | Medium | 02 |
| 04 oriterm_test_support cleanup | ~150 | Low | 03 |
| 05 oriterm_ui cleanup | ~400 | Medium-High (50 test files × per-file judgment) | 04 |
| 06 oriterm_mux cleanup | ~250 | Medium | 05 |
| 07 oriterm cleanup | ~150 | Low | 06 |
| 08 Feature matrix verification | ~200 | Medium | 07 |
| 09 Gate flip + meta-test | ~250 | Medium-High (cross-cutting; meta-test design) | 08 |
| 10 Cluster + new bug closure | ~150 | Low | 09 |
| **Total plan content** | **~2250** | | |

Estimated execution wall-clock: 1-2 days for the mechanical sections (02, 04, 07), 1-2 days for the larger structural cleanup (03, 06), 2-3 days for oriterm_ui (05) given the 50-file float_cmp review, 0.5-1 day for verification + gate flip + closure (08-10). Total ~5-9 working days.

## Known Bugs (Pre-existing, captured during research)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| `lefthook.yml` clippy hook does NOT run `--workspace` AND does NOT run `--all-targets` (`lefthook.yml` `clippy:` block) | Pre-commit hook only checks the default member crate (`oriterm`) on Windows GNU target | Section 09 (gate flip) updates `lefthook.yml` to `cargo clippy --workspace --all-targets --target x86_64-pc-windows-gnu -- -D warnings` | Will be Fixed |
| `oriterm_ipc/Cargo.toml` declares `[lints]` independently of workspace (NOT `workspace = true`) — duplicates workspace lints to override `unsafe_code` | SSOT divergence: any workspace lint change has to be manually mirrored in oriterm_ipc | Section 02 evaluates either (a) keep divergence + add comment + Section 09 meta-test pin, OR (b) revert to `workspace = true` with per-file `#[allow(unsafe_code)]` annotations | Will be Fixed (option chosen during Section 02 execution) |
| 10 builtin color scheme files (`oriterm/src/scheme/builtin/*.rs`) use bare `#![allow(unreadable_literal)]` with NO `reason=` | Pre-dates the `#[expect(reason=...)]` style; tech debt | Section 07 (oriterm cleanup) upgrades to `#![expect(unreadable_literal, reason="generated color hex literals")]` matching the precedent at `oriterm_ui/src/icons/footer.rs:7` | Will be Fixed |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | Baseline + Bug Filing | `section-01-baseline-and-bug-filing.md` | Not Started |
| 02 | oriterm_ipc Cleanup | `section-02-oriterm-ipc-cleanup.md` | Not Started |
| 03 | oriterm_core Cleanup | `section-03-oriterm-core-cleanup.md` | Not Started |
| 04 | oriterm_test_support Cleanup | `section-04-oriterm-test-support-cleanup.md` | Not Started |
| 05 | oriterm_ui Cleanup | `section-05-oriterm-ui-cleanup.md` | Not Started |
| 06 | oriterm_mux Cleanup | `section-06-oriterm-mux-cleanup.md` | Not Started |
| 07 | oriterm Cleanup | `section-07-oriterm-cleanup.md` | Not Started |
| 08 | Feature Matrix Verification | `section-08-feature-matrix-verification.md` | Not Started |
| 09 | Gate Flip + Meta-Test | `section-09-gate-flip-and-meta-test.md` | Not Started |
| 10 | Cluster + New Bug Closure | `section-10-bug-closure.md` | Not Started |
