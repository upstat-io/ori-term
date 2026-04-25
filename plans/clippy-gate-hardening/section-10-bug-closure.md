---
section: "10"
title: "Cluster + New Bug Closure"
status: not-started
reviewed: false
goal: "Bidirectionally close all 7 bugs (4 cluster: BUG-07-005/006/010/012 + 3 new filed in Section 01: BUG-07-NNN/NNN+1/NNN+2 for oriterm_mux/_ipc/oriterm gaps) as `Superseded by: plans/clippy-gate-hardening/`. Decrement section-07 open count by 7. Archive the plan as resolved."
success_criteria:
  - "All 7 bug entries in `plans/bug-tracker/section-07-ci-build.md` marked `[x]` with the canonical `Resolved: superseded by plans/clippy-gate-hardening/ on YYYY-MM-DD.` resolution line"
  - "`plans/bug-tracker/00-overview.md` Quick Reference open count for section 07 decremented by 7"
  - "Bidirectional supersede invariant satisfied: plan `00-overview.md` frontmatter `supersedes:` lists all 7 bugs; bug-tracker side mirrors via `Superseded by:` markers; `python -m scripts.plan_corpus check` exits 0 with no `bug_marker_drift` finding"
  - "Plan status flipped: `plans/clippy-gate-hardening/00-overview.md` status → `complete`; `plans/clippy-gate-hardening/index.md` status → `resolved`"
  - "Plan moved to `plans/completed/clippy-gate-hardening/` via `git mv` (per plan-schema.md §Status Conventions §Completed Plans)"
inspired_by:
  - "plans/bug-tracker/00-overview.md (canonical supersede resolution format)"
  - "plan-schema.md §Bidirectional supersede invariant"
  - "plans/completed/* (prior plan archival pattern)"
depends_on: ["09"]
third_party_review:
  status: none
  updated: null
sections:
  - id: "10.1"
    title: "Mark all 7 bug entries as superseded"
    status: not-started
  - id: "10.2"
    title: "Decrement bug-tracker overview open count by 7"
    status: not-started
  - id: "10.3"
    title: "Verify bidirectional supersede invariant"
    status: not-started
  - id: "10.4"
    title: "Flip plan status and archive to plans/completed/"
    status: not-started
  - id: "10.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "10.N"
    title: "Completion Checklist"
    status: not-started
---

# Section 10: Cluster + New Bug Closure

**Status:** Not Started
**Goal:** Closure section. The cleanup is done (Sections 02-08); the gate is flipped (Section 09). Section 10 reconciles the bug-tracker with reality and archives the plan. After Section 10, the seven open bugs in `section-07-ci-build.md` are marked resolved, the plan moves to `plans/completed/`, and the workspace's clippy gate is durably hardened.

**Success Criteria:** see frontmatter.

**Context:** The bidirectional supersede invariant from `plan-schema.md §Bidirectional supersede invariant` requires both endpoints (plan-side `supersedes:` frontmatter AND bug-side `Superseded by:` markers) to agree. The plan's `00-overview.md` already lists `supersedes:` (set when the plan was created); Section 10 sets the bug-side markers and decrements the overview count.

**Reference implementations:**
- `plans/bug-tracker/00-overview.md` — canonical supersede resolution format
- `plans/completed/*/index.md` — prior plan archival pattern

**Depends on:** Section 09 (gate flip complete; the bugs cannot be marked resolved until the gate change actually landed).

---

## Intelligence Reconnaissance

Queries run 2026-04-25:

- `scripts/intel-query.sh` — not present in this project; queries below used Grep / Glob / Read / cargo clippy --message-format=json instead.
- `Read plans/bug-tracker/00-overview.md` — supersede resolution format: `Resolved: superseded by plans/{plan-name}/ on {YYYY-MM-DD}.`
- `Read plans/clippy-gate-hardening/00-overview.md` — `supersedes:` frontmatter lists 4 cluster entries (BUG-07-005, -006, -010, -012). Section 10 must extend it with the 3 new entries from Section 01 once their concrete BUG-07-NNN ordinals are known.
- `Glob plans/completed/*/00-overview.md` — confirms the archival pattern: `git mv plans/{plan}/ plans/completed/{plan}/`; `index.md` `status: resolved`; `00-overview.md` `status: complete`.

Results summary (≤500 chars) [ori]: Closure is mechanical — supersede markers + count decrement + archive. The plan's `supersedes:` frontmatter must be extended (in Section 10.3 or earlier) with the 3 BUG-07-NNN ordinals once Section 01 assigned them; this is an internal sync within the plan.

See _(intel graph not available in this project; use Grep/Glob)_ for the full query protocol.

---

## 10.1 Mark all 7 bug entries as superseded

**File(s):** `plans/bug-tracker/section-07-ci-build.md`

For each of the 7 bug entries, edit the entry to mark resolved per the canonical format from `plans/bug-tracker/00-overview.md`:

- [ ] BUG-07-005:
  ```markdown
  - [x] `[BUG-07-005][medium]` **`./clippy-all.sh` does not lint test targets — 11 pre-existing clippy violations in `oriterm_core/tests/vttest/`** — found by continue-roadmap.
    Resolved: superseded by plans/clippy-gate-hardening/ on YYYY-MM-DD. Cleanup landed in section-03-oriterm-core-cleanup.md; gate flip landed in section-09-gate-flip-and-meta-test.md.
    {... rest of entry preserved as historical record ...}
  ```
- [ ] BUG-07-006: same pattern; cleanup in section-05-oriterm-ui-cleanup.md.
- [ ] BUG-07-010: same pattern; cleanup in section-03-oriterm-core-cleanup.md (subset of BUG-07-005 scope).
- [ ] BUG-07-012: same pattern; cleanup in section-04-oriterm-test-support-cleanup.md.
- [ ] BUG-07-NNN (oriterm_mux, filed Section 01.2): same pattern; cleanup in section-06-oriterm-mux-cleanup.md.
- [ ] BUG-07-NNN+1 (oriterm_ipc, filed Section 01.2): same pattern; cleanup in section-02-oriterm-ipc-cleanup.md.
- [ ] BUG-07-NNN+2 (oriterm, filed Section 01.2): same pattern; cleanup in section-07-oriterm-cleanup.md.
- [ ] Update plan's `00-overview.md` `supersedes:` frontmatter to include the 3 new BUG-07-NNN ordinals (the placeholders in the existing list become concrete IDs).
- [ ] Verify `python -m scripts.plan_corpus check plans/bug-tracker/section-07-ci-build.md` exits 0.
- [ ] Commit: `chore(bug-tracker): close BUG-07-005/006/010/012 + 3 new entries (superseded by clippy-gate-hardening)`.

- [ ] **Subsection close-out (10.1)**: standard template.

---

## 10.2 Decrement bug-tracker overview open count by 7

**File(s):** `plans/bug-tracker/00-overview.md`

- [ ] Read the Quick Reference table; locate Section 07 row.
- [ ] Decrement the open count by 7 (the count was incremented by 3 in Section 01.3 to account for the 3 new entries; the cluster bugs were already counted; the net change from pre-plan to post-plan is the original 4 cluster bugs cleared).
- [ ] Verify total open count is consistent.
- [ ] Commit: `chore(bug-tracker): decrement section-07 open count by 7 (clippy-gate-hardening cluster closed)`.

- [ ] **Subsection close-out (10.2)**: standard template.

---

## 10.3 Verify bidirectional supersede invariant

**File(s):** verification only

- [ ] Run `python -m scripts.plan_corpus check plans/clippy-gate-hardening/`. Expect: exit 0 with no `bug_marker_drift` findings.
- [ ] Run `python -m scripts.plan_corpus check plans/bug-tracker/section-07-ci-build.md`. Expect: exit 0.
- [ ] Spot-verify: each of the 7 bug entries has a `Superseded by:` line; plan's `00-overview.md` `supersedes:` frontmatter lists all 7 bug-tracker references.
- [ ] If any drift surfaces, fix the missing endpoint (typically the bug-side marker if the plan-side was the SSOT; or vice versa).

- [ ] **Subsection close-out (10.3)**: standard template.

---

## 10.4 Flip plan status and archive to plans/completed/

**File(s):** `plans/clippy-gate-hardening/00-overview.md`, `plans/clippy-gate-hardening/index.md`, plan directory move

- [ ] Edit `plans/clippy-gate-hardening/00-overview.md` frontmatter: `status: in-progress` → `status: complete`. Check off all mission success criteria checkboxes.
- [ ] Edit `plans/clippy-gate-hardening/index.md` frontmatter: `status: active` → `status: resolved`. Update `Quick Reference` table to show all sections `Complete`.
- [ ] Move plan to `plans/completed/`: `git mv plans/clippy-gate-hardening/ plans/completed/clippy-gate-hardening/`.
- [ ] Verify the move did not break any cross-references (`grep -rn 'plans/clippy-gate-hardening' .` should now show `plans/completed/clippy-gate-hardening/...` for any references). Update references if needed.
- [ ] Verify `/continue-roadmap` no longer picks up the plan as an active reroute (the `index.md` status: resolved removes it from the active queue).
- [ ] Commit: `chore(plans): archive clippy-gate-hardening to plans/completed/ (status: resolved)`.

- [ ] **Subsection close-out (10.4)**: standard template; final-section retrospective: any cross-cutting tooling needs that emerged across all 10 sections that the per-subsection retrospectives didn't catch?

---

## 10.R Third Party Review Findings

- None.

---

## 10.N Completion Checklist

- [ ] All 7 bug entries marked `[x] Resolved: superseded by plans/clippy-gate-hardening/...`
- [ ] `plans/bug-tracker/00-overview.md` Section 07 open count decremented by 7
- [ ] `python -m scripts.plan_corpus check` exits 0 across plan-tracker and bug-tracker; no `bug_marker_drift` findings
- [ ] Plan `00-overview.md` status → `complete`; mission success criteria all checked
- [ ] Plan `index.md` status → `resolved`
- [ ] Plan moved via `git mv` to `plans/completed/clippy-gate-hardening/`
- [ ] Cross-references updated for the archival path move
- [ ] `cargo test --all` green; `./test-all.sh` green; `./build-all.sh` green; `./clippy-all.sh` green (with the new gate)
- [ ] `cargo test -p oriterm --test clippy_gate` green (the meta-test from Section 09 still pins the gate)
- [ ] **Plan sync** (last invocation): all section frontmatter `complete`; overview Quick Reference all `Complete`
- [ ] `/tpr-review` passed (final integration check across the entire plan)
- [ ] `/impl-hygiene-review` passed (after TPR clean)
- [ ] `/improve-tooling` section-close sweep — also serves as the plan-close sweep; capture any cross-section tooling pattern that emerged from Sections 01-10 collectively
- [ ] `/sync-claude` section-close doc sync — verify CLAUDE.md, code-hygiene.md, tests.md, impl-hygiene.md still accurate after gate flip + meta-test introduction
- [ ] **Repo hygiene check** — final pass before plan archival

**Exit Criteria:** All 7 bugs `[x] Superseded by: plans/clippy-gate-hardening/`; bug-tracker overview reflects 7-bug decrement; plan archived to `plans/completed/clippy-gate-hardening/` with `status: complete` and `index.md status: resolved`; bidirectional supersede invariant clean (plan_corpus check exits 0 with no bug_marker_drift); workspace gate is durably hardened with the meta-test pin enforcing the SSOT invariant across all three gate-checking files.
