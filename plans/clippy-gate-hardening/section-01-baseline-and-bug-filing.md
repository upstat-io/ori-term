---
section: "01"
title: "Baseline + Bug Filing"
status: not-started
reviewed: false
goal: "Capture the authoritative pre-cleanup violation baseline for every crate × target × feature combination, and file three new bug-tracker entries for the previously-undocumented gaps (oriterm_mux, oriterm_ipc, oriterm) so the plan progresses against seven concrete tracked artifacts instead of four."
success_criteria:
  - "`plans/clippy-gate-hardening/baseline.md` exists, listing per-crate violation counts AND per-lint-code distribution AND M/S/J classification for every cell of (6 crates × {default, --no-default-features for oriterm_core, --features testing for oriterm_ui, --features gpu-tests + --features profile for oriterm} × {x86_64-unknown-linux-gnu, x86_64-pc-windows-gnu})"
  - "Three new tracker entries filed in `plans/bug-tracker/section-07-ci-build.md`: `BUG-07-NNN` (oriterm_mux — 192 violations), `BUG-07-NNN+1` (oriterm_ipc — 8 violations), `BUG-07-NNN+2` (oriterm — 6 violations). Each entry uses the canonical `/add-bug` shape (severity, repro, subsystem, found date, source). NNN is assigned sequentially after the highest existing BUG-07 ordinal."
  - "`plans/bug-tracker/00-overview.md` Quick Reference open count for section 07 incremented by 3 to reflect the new entries"
  - "`python -m scripts.plan_corpus check plans/bug-tracker/section-07-ci-build.md` exits 0"
  - "Section's mission criterion connection: contributes to '`Cluster bugs closed bidirectionally` + `Three new tracker entries closed bidirectionally`' mission criteria in 00-overview.md by establishing the artifacts those criteria will close in Section 10."
inspired_by:
  - "plans/bug-tracker/section-07-ci-build.md (existing cluster: BUG-07-005, BUG-07-006, BUG-07-010, BUG-07-012) — entry shape, severity grading, repro structure"
  - ".claude/skills/add-bug/SKILL.md — bug-entry canonical format"
depends_on: []
third_party_review:
  status: none
  updated: null
sections:
  - id: "01.1"
    title: "Capture per-crate × per-target × per-feature baseline counts"
    status: not-started
  - id: "01.2"
    title: "File three new tracker entries for unfiled gaps"
    status: not-started
  - id: "01.3"
    title: "Update bug-tracker overview Quick Reference"
    status: not-started
  - id: "01.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "01.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 01: Baseline + Bug Filing

**Status:** Not Started
**Goal:** Establish a frozen empirical baseline of every clippy violation surfaced by `--all-targets` plus the per-crate feature matrix, AND ensure the bug-tracker reflects all seven crates with gate-gap exposure (not just the four already filed). Without this baseline, downstream sections cannot measure progress; without the three new tracker entries, the plan would close violations that have no concrete artifact, violating CLAUDE.md §Bug Discipline.

**Success Criteria:**
- [ ] `plans/clippy-gate-hardening/baseline.md` covers all 6 crates × 2 targets × applicable feature combos with per-lint-code counts and M/S/J classification
- [ ] Three new `BUG-07-NNN` entries filed: oriterm_mux (192), oriterm_ipc (8), oriterm (6), each with severity (medium for all three; same family as cluster), repro command, subsystem, found date, source `clippy-gate-hardening Section 01`
- [ ] Bug-tracker overview Section 07 open count incremented by 3 (4 cluster bugs already counted there + 3 new = 7 total tracked under this plan)
- [ ] `python -m scripts.plan_corpus check plans/bug-tracker/section-07-ci-build.md` exits 0
- [ ] Connects upward to mission criterion: "Three new tracker entries closed bidirectionally"

**Context:** The cluster bugs `BUG-07-005`, `BUG-07-006`, `BUG-07-010`, `BUG-07-012` filed in `plans/bug-tracker/section-07-ci-build.md` document ~185 violations across 4 crates. Phase 1 research surfaced ~1480 violations across 6 crates — the cluster underestimates by 8x and misses 3 entire crates (oriterm_mux 192, oriterm_ipc 8, oriterm 6). CLAUDE.md §Bug Discipline mandates concrete tracked artifacts for every discovered bug; the three undocumented gaps must be filed before any cleanup work begins, so the plan progresses against the same artifact discipline as the existing cluster.

**Reference implementations:**
- `plans/bug-tracker/section-07-ci-build.md` BUG-07-005, BUG-07-006, BUG-07-010, BUG-07-012 entries — exact format to mirror for the three new entries
- `.claude/skills/add-bug/SKILL.md` — canonical bug-entry shape, severity grading rubric

**Depends on:** None — this section has no upstream sections. It is the plan's entry point.

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / cargo clippy --message-format=json instead. `(intel-query not available) — used Grep + cargo clippy --message-format=json` — `cargo clippy --workspace --all-targets --target x86_64-unknown-linux-gnu --message-format=json -- -D warnings` produces ~1480 distinct diagnostic events; per-crate JSON parse via `python3` with `code.startswith('clippy::')` filter yields the per-crate counts in `00-overview.md` Metrics table.
- `Glob plans/bug-tracker/section-07-ci-build.md` — 4 existing BUG-07 cluster entries: 005 (vttest), 006 (oriterm_ui::testing), 010 (151 across oriterm_core test targets), 012 (oriterm_test_support).
- `Glob plans/completed/*/section-*.md` — no completed plan covers workspace-wide clippy gate hardening; the closest precedent is `plans/tack-conformance/section-04-scenario-framework.md` which ran `cargo clippy -p oriterm_test_support --all-targets` per-checkpoint inline (`section-04-scenario-framework.md:269,545,558`) but explicitly deferred the global gate flip to the bug-tracker (`section-04-scenario-framework.md:1817`).
- `Read clippy.toml` — workspace root sets `too-many-arguments-threshold = 5` and `avoid-breaking-exported-api = false`; affects baseline counts but not the gate flip.

Results summary (≤500 chars) [ori]: Per-crate baseline must capture (485 oriterm_core, 761 oriterm_ui, 192 oriterm_mux, 27 oriterm_test_support, 8 oriterm_ipc, 6-9 oriterm) × {host, x86_64-pc-windows-gnu} × applicable feature combos. Cluster bugs 07-005/006/010/012 already filed; 3 new entries needed for oriterm_mux/_ipc/_oriterm gaps. Workspace `clippy.toml` thresholds influence count but not gate scope.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 01.1 Capture per-crate × per-target × per-feature baseline counts

**File(s):** `plans/clippy-gate-hardening/baseline.md` (new)

The baseline file is the frozen empirical record against which Sections 02-07 measure progress. Every cell of (6 crates × 2 targets × applicable feature combos) must be filled with: total violation count, per-lint-code distribution (top 15), and M/S/J classification.

- [ ] Run baseline capture script and write `plans/clippy-gate-hardening/baseline.md` with this structure:

  ```markdown
  # Clippy Gate Hardening — Baseline (frozen 2026-MM-DD)

  Captured against commit `<git rev-parse HEAD>` at <ISO-8601 timestamp>.

  ## Per-crate × per-target × per-feature counts

  ### oriterm_ipc

  | Target | Features | Total | Top lints (count, M/S/J) |
  |--------|----------|------:|--------------------------|
  | x86_64-unknown-linux-gnu | default | 8 | redundant_clone (5, S), manual_assert (2, M), doc_markdown (1, M) |
  | x86_64-pc-windows-gnu | default | <N> | <list> |

  ### oriterm_core

  | Target | Features | Total | Top lints (count, M/S/J) |
  |--------|----------|------:|--------------------------|
  | x86_64-unknown-linux-gnu | default (image-protocol) | 485 | doc_markdown (301, M), field_reassign_with_default (42, M), needless_raw_strings (29, M), float_cmp (21, J), redundant_closure_for_method_calls (14, M), string_slice (14, J), match_same_arms (6, S), iter_cloned_collect (6, M), unnested_or_patterns (5, S), items_after_statements (5, M) |
  | x86_64-unknown-linux-gnu | --no-default-features | <N> | <list> |
  | x86_64-pc-windows-gnu | default | <N> | <list> |
  | x86_64-pc-windows-gnu | --no-default-features | <N> | <list> |

  ### oriterm_test_support
  ... (same shape)

  ### oriterm_ui
  ... (same shape, including --features testing combo)

  ### oriterm_mux
  ... (same shape)

  ### oriterm
  ... (same shape, including --features gpu-tests, --features profile combos)

  ## Workspace totals

  | Target | Features | Total |
  |--------|----------|------:|
  | host | default | <N> |
  | host | --features matrix | <N> |
  | x86_64-pc-windows-gnu | default | <N> |
  | x86_64-pc-windows-gnu | --features matrix | <N> |

  ## M/S/J overall classification

  | Class | Count | Pct | Treatment |
  |-------|------:|----:|-----------|
  | M (mechanical, auto-fixable, semantics-preserving) | <N> | ~<%> | Section 02-07 each runs `cargo clippy --fix --all-targets -p {crate}` with manual diff review |
  | S (structural, manual review required) | <N> | ~<%> | Per-site fix in each per-crate cleanup section |
  | J (judgment, per-instance verdict required) | <N> | ~<%> | Section 05 (oriterm_ui float_cmp 50-file expect review); per-site `#[expect(reason=...)]` elsewhere |
  ```

- [ ] Capture script (run inline, not a committed script — this is a one-shot baseline):
  ```bash
  for crate in oriterm_ipc oriterm_core oriterm_test_support oriterm_ui oriterm_mux oriterm; do
    for target in x86_64-unknown-linux-gnu x86_64-pc-windows-gnu; do
      for features in "" "--no-default-features" "--features testing" "--features gpu-tests" "--features profile"; do
        # Skip combos that don't apply (e.g., --features testing only valid for oriterm_ui)
        # Run: timeout 150 cargo clippy -p $crate --all-targets --target $target $features --message-format=json 2>/dev/null
        # Pipe to python3 lint-counter; capture into baseline.md cell
        true
      done
    done
  done
  ```

- [ ] Verify M/S/J classification rubric is consistent with `00-overview.md` Metrics table classifications. Reference rubric:
  - **M (mechanical)**: `cargo clippy --fix` produces a semantics-preserving rewrite (no behavioral change, no drop-timing change, no API contract change). Examples: `doc_markdown`, `redundant_closure_for_method_calls`, `needless_raw_strings`, `field_reassign_with_default`, `format_push_string`, `items_after_statements`, `redundant_clone` (when value is provably unused after), `manual_assert`, `case_sensitive_file_extension_comparisons`.
  - **S (structural)**: Manual review required; `cargo clippy --fix` may apply but the rewrite changes structure (control flow, ownership, drop-timing). Examples: `manual_let_else`, `redundant_clone` (when value is reused after — still safe but worth verifying), `into_iter_on_single_item`, `needless_pass_by_value`, `decimal_bitwise_operands`, `unnested_or_patterns`, `match_same_arms`.
  - **J (judgment)**: Per-instance verdict required — the lint flags a real concern that may or may not be valid given the surrounding context. Examples: `float_cmp` (epsilon vs exact-representable?), `string_slice` (UTF-8 char-boundary safety?), `too_many_lines`, `too_many_arguments`, `unchecked_time_subtraction`, `map_err_ignore`.

- [ ] Commit the baseline as `chore(clippy-gate-hardening): freeze pre-cleanup violation baseline` so the file is permanently archived against the plan.

- [ ] **Subsection close-out (01.1)** — MANDATORY before starting 01.2:
  - [ ] All tasks above are `[x]` and the baseline file is committed
  - [ ] Update this subsection's `status` in section frontmatter to `complete`
  - [ ] **Run `/improve-tooling` retrospectively on THIS subsection** — reflect on the baseline-capture journey: was the python3 lint-counter snippet reusable enough that it should live in `diagnostics/`? Did the cross-target × cross-feature matrix benefit from a wrapper? Did `--message-format=json` discoveries (e.g., `code.startswith('clippy::')` filter) merit documentation in `diagnostics/README.md`? Implement every accepted improvement NOW (zero deferral) and commit each via SEPARATE `/commit-push` (`build(diagnostics): add clippy-bucket.py — surfaced by clippy-gate-hardening/section-01.1 retrospective`). Mandatory even when nothing felt painful — document briefly: "Retrospective 01.1: no tooling gaps — baseline captured via inline python3, no friction."
  - [ ] **Run `/sync-claude` on THIS subsection** — three quick questions: (1) Did I add/rename/remove any public API? (2) Did I add/change any command, env var, or script? Possibly — if a `diagnostics/clippy-bucket.py` was added in the retrospective, CLAUDE.md §Commands may need a one-line addition. (3) Did I change any pipeline phase behavior? No. Document: "Claude artifact sync 01.1: no API/phase changes; commands updated if diagnostics tool added."
  - [ ] **Repo hygiene check** — run `diagnostics/repo-hygiene.sh --check` and clean any temp/scratch files (per-crate clippy stdout dumps, /tmp/clippy-output.txt detritus from baseline capture).

---

## 01.2 File three new tracker entries for unfiled gaps

**File(s):** `plans/bug-tracker/section-07-ci-build.md`

Three new entries are filed BEFORE any cleanup work begins. Each follows the canonical bug-entry format from `.claude/skills/add-bug/SKILL.md` and mirrors the cluster's shape (`BUG-07-005` is the canonical exemplar at `plans/bug-tracker/section-07-ci-build.md:203-216`).

- [ ] Determine the next available `BUG-07-NNN` ordinal by reading the highest existing BUG-07 entry. Reserve three sequential ordinals.
- [ ] File entry for **oriterm_mux** (192 violations) at the bottom of the open-bugs list:
  ```markdown
  - [ ] `[BUG-07-NNN][medium]` **`./clippy-all.sh` does not lint test targets — 192 pre-existing clippy violations in `oriterm_mux/`** — found by clippy-gate-hardening Section 01.
    Repro: `cargo clippy -p oriterm_mux --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` produces 192 errors. `./clippy-all.sh` runs `cargo clippy --workspace -- -D warnings` which only checks lib + bin targets, so test-target violations have been silently passing CI.
    Subsystem: `clippy-all.sh` + `oriterm_mux/src/`
    Found: 2026-MM-DD | Source: clippy-gate-hardening Section 01
    Top violations: doc_markdown 85, used_underscore_binding 37, decimal_bitwise_operands 12, default_trait_access 11, items_after_statements 8, unchecked_time_subtraction 8, manual_assert 5, redundant_clone 4, needless_continue 4, no_effect_underscore_binding 3, string_slice 3.
    Same root cause family as BUG-07-005, BUG-07-006, BUG-07-010, BUG-07-012: clippy-all.sh's scope is too narrow. Filed by clippy-gate-hardening so the cleanup is tracked as a concrete artifact per CLAUDE.md §Bug Discipline.
    Fix: cleanup lives in `plans/clippy-gate-hardening/section-06-oriterm-mux-cleanup.md`; gate flip lives in `plans/clippy-gate-hardening/section-09-gate-flip-and-meta-test.md`. Closes as `Superseded by: plans/clippy-gate-hardening/`.
  ```

- [ ] File entry for **oriterm_ipc** (8 violations):
  ```markdown
  - [ ] `[BUG-07-NNN+1][medium]` **`./clippy-all.sh` does not lint test targets — 8 pre-existing clippy violations in `oriterm_ipc/`** — found by clippy-gate-hardening Section 01.
    Repro: `cargo clippy -p oriterm_ipc --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` produces 8 errors. Same root cause as BUG-07-005/006/010/012: gate scope too narrow.
    Subsystem: `clippy-all.sh` + `oriterm_ipc/`
    Found: 2026-MM-DD | Source: clippy-gate-hardening Section 01
    Top violations: redundant_clone 5, manual_assert 2, doc_markdown 1.
    Note: `oriterm_ipc/Cargo.toml` declares its own `[lints]` block (NOT `workspace = true`) to set `unsafe_code = "allow"`. Section 02 evaluates whether to keep the divergence or migrate to `workspace = true` with per-file `#[allow(unsafe_code)]`.
    Fix: cleanup in `plans/clippy-gate-hardening/section-02-oriterm-ipc-cleanup.md`; gate flip in section 09. Closes as `Superseded by: plans/clippy-gate-hardening/`.
  ```

- [ ] File entry for **oriterm** (6 violations — note: the original count of 9 included 3 surfaced via `oriterm_ui/testing` dev-dep, which are oriterm_ui's responsibility, not oriterm's):
  ```markdown
  - [ ] `[BUG-07-NNN+2][medium]` **`./clippy-all.sh` does not lint test targets — 6 pre-existing clippy violations in `oriterm/`** — found by clippy-gate-hardening Section 01.
    Repro: `cargo clippy -p oriterm --all-targets --target x86_64-unknown-linux-gnu -- -D warnings` produces 6 errors (note: 3 additional `float_cmp` are surfaced in oriterm's `--features gpu-tests` build via the `oriterm_ui/testing` dev-dep at `oriterm/Cargo.toml:73-79`; those are owned by oriterm_ui Section 05, not by this oriterm-owned bug).
    Subsystem: `clippy-all.sh` + `oriterm/src/`
    Found: 2026-MM-DD | Source: clippy-gate-hardening Section 01
    Top violations: float_cmp 3 (PRODUCTION code, judgment required), doc_markdown 1, manual_assert 1, many_single_char_names 1.
    Also includes 10 `#![allow(clippy::unreadable_literal)]` declarations in `oriterm/src/scheme/builtin/*.rs` without `reason=` — to be upgraded to `#![expect(clippy::unreadable_literal, reason="generated color hex literals")]` in Section 07.
    Fix: cleanup in `plans/clippy-gate-hardening/section-07-oriterm-cleanup.md`; gate flip in section 09. Closes as `Superseded by: plans/clippy-gate-hardening/`.
  ```

- [ ] Verify the three entries against `python -m scripts.plan_corpus check plans/bug-tracker/section-07-ci-build.md` — exit 0.

- [ ] Commit: `chore(bug-tracker): file 3 new BUG-07 entries for clippy-gate-hardening cluster (oriterm_mux/_ipc/oriterm)`.

- [ ] **Subsection close-out (01.2)** — MANDATORY before starting 01.3:
  - [ ] All tasks above are `[x]` and the three entries appear in section-07
  - [ ] Update this subsection's `status` to `complete`
  - [ ] **Run `/improve-tooling` retrospectively** — reflect on the bug-filing journey: did `/add-bug` accept the entries cleanly? Did the canonical format leave any ambiguity? If yes, fix the skill template (the SSOT). Document if no gaps.
  - [ ] **Run `/sync-claude`** — entries are bug-tracker artifacts, no Claude rule changes expected. Document: "Claude artifact sync 01.2: bug-tracker only; no rules drift."
  - [ ] **Repo hygiene check** — run `diagnostics/repo-hygiene.sh --check`.

---

## 01.3 Update bug-tracker overview Quick Reference

**File(s):** `plans/bug-tracker/00-overview.md`

The bug-tracker overview tracks per-section open counts. Filing 3 new entries in section 07 means the open count for that section must increase by 3.

- [ ] Read `plans/bug-tracker/00-overview.md` Quick Reference table; locate the row for section 07 (CI & Build).
- [ ] Increment the open count by 3 (e.g., if section 07 currently shows N open, update to N+3).
- [ ] Verify total open count across all sections is consistent (sum of per-section counts equals the overview's total).
- [ ] Commit: `chore(bug-tracker): update section-07 open count for clippy-gate-hardening cluster`.

- [ ] **Subsection close-out (01.3)** — MANDATORY before 01.R / 01.N:
  - [ ] Quick Reference reflects N+3 for section 07; total consistent
  - [ ] Update this subsection's `status` to `complete`
  - [ ] `/improve-tooling` retrospective: was the manual count update tedious? If a `diagnostics/` script could autocount sections from filenames + parse `[ ]` checkboxes, file as a tooling improvement.
  - [ ] `/sync-claude`: bug-tracker overview is internal; no rules drift.
  - [ ] Repo hygiene check.

---

## 01.R Third Party Review Findings

<!-- Reserved for the dual-source `/tpr-review` (Codex + Gemini) and other external reviewers. Findings may be tagged `-codex`, `-gemini`, or carry `agreement: true` when both reviewers flagged the same location/title.
If unresolved findings exist here:
- section frontmatter `status` must be `in-progress`
- `third_party_review.status` must be `findings`

When all findings are triaged:
- accepted findings are integrated into the relevant implementation subsection(s)
- rejected findings are closed with rationale
- all items in this block are marked resolved
- `third_party_review.status` becomes `resolved` or `none`
-->

- None.

---

## 01.N Completion Checklist

- [ ] `plans/clippy-gate-hardening/baseline.md` exists and is committed
- [ ] Three new `BUG-07-NNN` entries appear in `plans/bug-tracker/section-07-ci-build.md` and pass `python -m scripts.plan_corpus check` (exit 0)
- [ ] `plans/bug-tracker/00-overview.md` Quick Reference open count for section 07 incremented by 3
- [ ] `cargo test --all` green — regression canary (Section 01 introduces no production code; any test failure is unrelated)
- [ ] All intermediate findings from 01.1-01.3 close-outs are committed
- [ ] **Plan sync** — update plan metadata to reflect this section's completion:
  - [ ] This section's frontmatter `status` → `complete`, all subsection statuses → `complete`
  - [ ] `00-overview.md` Quick Reference table: Section 01 status → `Complete`
  - [ ] `00-overview.md` mission success criteria: check off any criteria satisfied (none — Section 01 establishes baseline only; mission criteria flip in Sections 09 and 10)
  - [ ] `index.md` Section 01 status → `Complete`
- [ ] `/tpr-review` passed (final, full-section) — no critical or major findings (or all findings triaged). Note: Section 01 is pure tracking; TPR is fast.
- [ ] `/impl-hygiene-review` passed — MUST run AFTER `/tpr-review` is clean.
- [ ] `/improve-tooling` **section-close sweep** — verify per-subsection retrospectives ran; add only NEW cross-subsection patterns (the python3 lint-counter could become `diagnostics/clippy-bucket.py` if reused across sections 02-08; if so, file as cross-subsection finding). Implement immediately, commit separately. Most likely outcome: per-subsection captures covered everything.
- [ ] `/sync-claude` **section-close doc sync** — verify CLAUDE.md §Commands lists any new diagnostic script added.
- [ ] **Repo hygiene check** — `diagnostics/repo-hygiene.sh --check`.

**Exit Criteria:** `plans/clippy-gate-hardening/baseline.md` exists with all crate × target × feature cells populated; `plans/bug-tracker/section-07-ci-build.md` carries 7 total open BUG-07 entries from this plan's cluster (4 pre-existing + 3 new); `python -m scripts.plan_corpus check plans/bug-tracker/section-07-ci-build.md` exits 0; `cargo test --all` green; all section frontmatter and overview/index reflect Section 01 complete.
