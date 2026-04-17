# `roadmap_scan.py` + `/continue-roadmap` workflow.md — Design Notes and Improvement Log

**Purpose of this file.** Institutional memory for the scanner that powers `/continue-roadmap`. Captures the **design philosophy** of the scanner's auto-fix pipeline (so future edits don't regress it), the **load-bearing invariants** (what must not change without a plan), and a **running log of drift patterns discovered in the wild**.

**Scope.** Covers `compiler/.claude/skills/continue-roadmap/roadmap_scan.py` (the scanner) and `compiler/.claude/skills/continue-roadmap/workflow.md` (the sub-agent protocol that consumes the scanner's JSON). These two files are one logical tool — the scanner produces gate decisions, the workflow applies them.

**When to update this file.** Any time `/continue-roadmap` loops, blocks on a gate that should auto-clear, or an auto-fix is added to handle a new drift class. Add a `- [ ]` item under §Improvement Log for in-the-wild findings; add a `- [x]` when a fix lands.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Scanner is the brain; workflow is the hands.** `roadmap_scan.py` pre-computes every gate decision, focus-context field, and next-unblocked pointer into one JSON envelope. `workflow.md` is a transcription contract — no re-derivation, no re-parsing, no re-running of logic that the scanner already produced. If the sub-agent needs new information, it goes in the scanner output, not in new workflow logic.

2. **Auto-fix cleanup is silent and mandatory.** Any gate with `severity: "auto-fix"` is applied inline without user prompts, without `AskUserQuestion`, without surfacing options. The user has pre-approved mechanical cleanup (see user memory `feedback_auto_fix_cleanup.md`). A gate that "needs human judgment" is NOT an auto-fix — it is a `block` gate.

3. **Every drift pattern must be healable.** If a gate fires (`block`), there must be a skill or auto-fix path that clears it. A gate that fires but has no clearing mechanism is a loop — `/continue-roadmap` blocks on it, the suggested skill does nothing, the scan re-fires the gate, forever. Every new `block` gate MUST have a corresponding skill or auto-fix.

4. **Mismatches flow through one channel.** `detect_all_mismatches()` is the SSOT for "what stale frontmatter exists". Scanner-internal detection lives on `Section.mismatch`, `Section.tpr_mismatch`, `Subsection.mismatch`, and is aggregated by `detect_all_mismatches()`. Gate 1.5 `stale_frontmatter` reads that one aggregate. Adding a new drift pattern means adding a new `*_mismatch` property, NOT a new gate.

5. **Focus plan only for auto-fixes.** The scanner reports mismatches across every plan, but the sub-agent applies fixes ONLY inside the focus plan. Editing sibling plans during a `/continue-roadmap` scan creates cross-plan churn that the user didn't request; let those plans be handled when they are the focus.

6. **Workflow.md Step 2a is the authoritative fix table.** When the scanner emits a new mismatch string, workflow.md Step 2a's table MUST have a row that maps that string to a concrete fix. Adding a detector without adding the fix row is a half-shipped change and the sub-agent will hit the drift and not know what to do.

---

## §2 — Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|-----------|--------------------------|
| Every `block` gate has a clearing path (skill or auto-fix) | `/continue-roadmap` loop — gate fires, suggested skill no-ops, next scan re-fires (the bug this design log was created for, 2026-04-17) |
| Scanner pre-computes every gate in JSON; workflow transcribes | Workflow logic drift — if the sub-agent re-derives gate logic, it drifts from the scanner; the two must always agree |
| `detect_all_mismatches()` is the sole aggregator for stale-frontmatter drift | New drift classes added as parallel gates instead of rows in one table — fractures the auto-fix pipeline |
| TPR status drift is detected per-section via `Section.tpr_mismatch` | Silent `tpr_findings` blocker (the 2026-04-17 incident: `03.R` complete, 0 open findings, frontmatter says `findings`, gate fires forever) |
| Workflow.md Step 2a row strings match scanner's mismatch strings VERBATIM | Sub-agent sees a mismatch but can't find a matching row → skips the fix → the drift persists |
| Auto-fix is scoped to the focus plan only | Cross-plan churn — editing sibling plans during one focus plan's workflow creates noisy commits and risks blind edits to plans the user hasn't reviewed |
| Cleanup is silent (no `AskUserQuestion`) | User fatigue — mechanical cruft does not need approval, approving 5 stale frontmatter fixes per session destroys the flow |

---

## §3 — File Inventory

| Path | Lines | Role |
|------|-------|------|
| `.claude/skills/continue-roadmap/roadmap_scan.py` | ~2000 | Scanner — parses every plan's sections/subsections/frontmatter, detects drift, pre-computes gates |
| `.claude/skills/continue-roadmap/workflow.md` | ~240 | Sub-agent protocol — transcribes scanner JSON into handoff block, applies auto-fixes |
| `.claude/skills/continue-roadmap/SKILL.md` | ~80 | Caller-facing dispatcher (Agent call + handoff parsing) |
| `scripts/plan_corpus/bug_validators.py` | ~370 | `Superseded by:` marker auto-fix pipeline (Gate 1.6) |
| `scripts/plan_corpus/schema.py` | N/A | Frontmatter schema validator (TPR status/updated field shape) — does NOT detect TPR-vs-findings-checkbox drift (pool-level data not in its scope) |

**Recently changed (2026-04-17):** Added `Section.tpr_mismatch` property and wired it into `detect_all_mismatches()`. Added 3 rows to `workflow.md` Step 2a's fix table covering TPR status drift.

---

## §4 — Lessons from Dogfood / Production Runs

### 2026-04-17 — TPR status loop

**Symptom.** User ran `/continue-roadmap`, it focused on `plans/empty-container-typeck-phase-contract/section-03-bodies-pass-integration.md`, and escalated on `tpr_findings` gate. Running the suggested `/verify-tpr` had nothing to verify (0 open finding checkboxes). User re-ran `/continue-roadmap` → same escalation → loop.

**Root cause.** `section-03` has `third_party_review.status: findings` in its frontmatter, `03.R` subsection `complete`, and all 3 TPR finding checkboxes `[x]`. The scanner's `tpr_fires` logic at `roadmap_scan.py:1669`:

```python
tpr_fires = ws.focus_section.tpr_status == "findings" or bool(open_tpr)
```

fires on the frontmatter flag *or* open findings. Commit `a92ae501` flipped `03.R` status to `complete` via the stale-frontmatter auto-fixer, but the auto-fixer had no rule for `third_party_review.status` — it stayed `findings`. `/verify-tpr` triages finding checkboxes (none open → nothing to do → frontmatter stays `findings`). No path cleared the drift.

**Fix.** Added `Section.tpr_mismatch` property detecting three drift cases:
1. `status == "findings"` AND all parsed findings resolved → should be `resolved`
2. `status == "findings"` AND no findings parsed at all → should be `resolved` (or `none` if `updated` is null)
3. `status == "resolved"` AND any finding still open → should be `findings`

Wired into `detect_all_mismatches()` so Gate 1.5 `stale_frontmatter` surfaces it with the same silent-auto-fix treatment as status-vs-checkbox drift. Added three rows to `workflow.md` Step 2a's fix table so the sub-agent knows how to apply each case.

**Verification.** Re-running the scanner on `section-03` after flipping the frontmatter to `resolved` shows `stale_frontmatter.fires == False` and `tpr_findings.fires == False`. The loop is broken.

**Prior art surfaced during the fix.** `scripts/plan_corpus/schema.py::_validate_tpr_info` already validates `tpr_status` vs the `updated` field, but it operates only on frontmatter — it does not have access to the section's `.R` subsection checkbox state. That's why the detection lives in `roadmap_scan.py` (scanner-level, with per-section checkbox data) and not in `schema.py`.

---

## §5 — Regressions To Watch For

- [ ] `tpr_findings` gate firing on a section whose `.R` subsection is `complete` and all finding checkboxes are `[x]`. If seen, the `tpr_mismatch` auto-fix pipeline is broken — either the detector isn't running, or workflow.md Step 2a's row strings have drifted from the scanner's emitted strings.
- [ ] `/continue-roadmap` escalating twice in a row with the same gate and the suggested skill having nothing to do. This is the "loop" pattern — ANY gate that can't be cleared by its suggested skill is a loop waiting to happen.
- [ ] Adding a new gate without adding a clearing path. Every new `block` gate MUST have a corresponding skill invocation, auto-fix, or documented manual step.
- [ ] Adding a scanner detector without adding a matching row in `workflow.md` Step 2a. The sub-agent will see the mismatch string and have no fix to apply.
- [ ] Cross-plan mismatch auto-fix — the sub-agent should NEVER fix mismatches outside the focus plan, even when the scanner reports them. Those reports are informational-only.

---

## §6 — Improvement Log

### Open items

*(none at this time)*

### Recently closed

- [x] **2026-04-17** — TPR status drift auto-fix: added `Section.tpr_mismatch` detector (scanner) + 3 rows in `workflow.md` Step 2a (sub-agent fix table). Closes the `/continue-roadmap` → `/verify-tpr` loop on sections where `third_party_review.status: findings` persists after all finding checkboxes are closed. See §4 lesson entry. Commit: pending.

---

## §7 — How To Use This File In Future Sessions

**When to open.** Any time `/continue-roadmap` loops, escalates with no cleared path, or a new drift class surfaces. Also open when modifying `roadmap_scan.py`'s gate logic or `workflow.md`'s auto-fix table — check §2 Load-Bearing Invariants before the edit.

**When to update.** (1) A new drift pattern is found in the wild → add to §4 Lessons + §6 Improvement Log as `- [ ]`. (2) A fix is landed → flip to `- [x]` with commit sha. (3) An invariant from §2 needs to change → add a dated §4 entry explaining the failure mode the old invariant caused, flip the §2 row, and update `workflow.md` to match.

**Grep cheatsheet.** `grep "tpr_mismatch" roadmap_scan.py workflow.md` — verify the detector + fix-table contract is consistent. `grep "Section.mismatch\|tpr_mismatch\|Subsection.mismatch" roadmap_scan.py` — the full aggregator contract for `detect_all_mismatches()`.
