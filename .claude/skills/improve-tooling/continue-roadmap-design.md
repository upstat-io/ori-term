# /continue-roadmap — Design Log

## Purpose + Context

`/continue-roadmap [section]` — resumes roadmap work.

- Pipeline: scanner → state.sh → marker check → gate eval → pacing → Focus Context emit → `/roadmap-work` dispatch.
- Inline execution in main context (Opus); no sub-agent; no `workflow.md`.
- Plan-doc cleanup runs in `/commit-push` Step 4, not here.

## §1 Core Design Philosophy

1. **Plan file is SSOT for workflow state.** Status, resume pointers, pipeline stage, subsection status, checkboxes, HISTORY — all in the plan file. Memory is never a workflow-state store.
2. **Scanner JSON + state.sh JSON are the complete world-state.** No compiler runs, no source reads, no git archaeology.
3. **Block-severity gates → `AskUserQuestion` immediately.** Payload options verbatim. No peeking.
4. **Plan-doc cleanup delegated to `/commit-push`.** `scripts/plan-cleanup.py` runs in Step 4 before staging. `/continue-roadmap` reports staleness as info only.
5. **Inline execution in main context (Opus).** No Agent dispatch, no Sonnet cold-start, no handoff-block round-trip, no workflow.md.
6. **`review_pipeline:` marker = resume pointer.** Step 2.5 grep-detects it; paused `/review-plan` session → escalate to resume.

## §2 Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|---|---|
| Never run `cargo` / `ori` / `oric` / `./target/**` / test harnesses / `diagnostics/*.sh` (except `state.sh show`/`check`/`known-failing`). | 2026-04-18: sub-agent ran `cargo check` on test files, 67 tool calls, 10 min wall-clock. |
| Never read `.rs` / `.ori` / `.toml` or anything under `compiler/` / `library/` / `tests/` / `scripts/`. Plan-doc reads + edits in `plans/` are fine. | Investigation masquerading as context-gathering. |
| Never run `git log` / `blame` / `show` / `diff` / `bisect`. | Git archaeology is project-wide banned (CLAUDE.md); scanner already captured `dirty_tree`. |
| Never run `scripts/intel-query.sh`. | Intel-graph consumers are `/review-plan` and `/roadmap-work`. |
| Plan-doc cleanup runs in `/commit-push`, NEVER in `/continue-roadmap`. | Two-commit dirty-tree loop (user work + separate cleanup commit). |
| Step 2.5 MUST grep for `review_pipeline:` in focus section frontmatter before gate eval. | 2026-04-19: missing check dispatched to §08.1.5 pacing while `/review-plan` was paused at `stage: editor-done, next_step: 6`. |
| Never cache scan state across escalation boundaries; re-invocation = fresh scan. | Scanner state changes across escalations (user ran `/commit-push`, etc.). |
| SKILL.md holds the full protocol inline. No separate `workflow.md`. | Re-introducing `workflow.md` brings back the Sonnet sub-agent architecture that was removed 2026-04-19. |

## §3 File Inventory

| File | Lines | Role |
|---|---|---|
| `.claude/skills/continue-roadmap/SKILL.md` | ~85 | Full protocol inline. Steps 1–5 + escalation contract. |
| `.claude/skills/continue-roadmap/roadmap_scan.py` | 2362 | Scanner. See `script-roadmap-scan-design.md`. |
| `.claude/skills/continue-roadmap/roadmap-scan.sh` | tiny | Bash wrapper around the Python scanner. |

**Deleted 2026-04-19:** `workflow.md` (merged into SKILL.md when sub-agent was removed).

## §4 Lessons

### 2026-04-18 — Sub-agent ran `cargo check` for 10 min

Sonnet sub-agent drifted into compiler investigation during a gate-check pass: read test files, ran `cargo check` on them, looped across multiple test files. 67 tool calls. Fix at the time: explicit hard-bans + tool-call budget in `workflow.md`. Historical — sub-agent + `workflow.md` both removed 2026-04-19.

### 2026-04-19 — Memory-as-workflow-state dispatch failure

Scanner dispatched §08.3 sibling-pass implementation based on stale plan content; the corrected diagnosis sat in memory (`project_bug_04_042_pool_merge_diagnosis.md`) instead of plan §08.1.R. First fix: two-layer guard (producer CLAUDE.md rule + consumer Step A.5 parent-side cross-check). User flagged the guard as a Band-Aid normalizing the rule violation. Final fix: CLAUDE.md rule rewritten to ban ALL workflow state from memory; Step A.5 removed; memory entry deleted.

### 2026-04-19 — Rule-based simplification + inline execution

Converted SKILL.md from prose to bullets/tables. Deleted `workflow.md`. Dropped the Sonnet sub-agent. Inline execution removes Agent-dispatch tax (~2k) + Sonnet re-read of `workflow.md` (~5k). Net per invocation: 3 Bash calls.

### 2026-04-19 — Plan-cleanup moved to /commit-push

Created `scripts/plan-cleanup.py`. `/commit-push` Step 4 now runs `fmt-all.sh` + `plan-cleanup.py` before staging. `/continue-roadmap` Step 2 (auto-fix) deleted. No more two-commit dirty-tree loop — user's commit + cleanup land together.

### 2026-04-19 — `review_pipeline:` marker enforcement

Fresh `/continue-roadmap` on §08 dispatched to §08.1.5 pacing while `/review-plan` was paused mid-pipeline. Added Step 2.5: grep focus section frontmatter for `review_pipeline:`. If present → `AskUserQuestion` with Resume `/review-plan` (Recommended) / Clear marker / Proceed anyway. The marker auto-clears on `/review-plan` clean exit per its Step 1d contract.

## §5 Regressions To Watch For

- [ ] Sub-agent re-introduced (any Agent({}) dispatch in /continue-roadmap's flow).
- [ ] Step 2 auto-fix logic re-appears in SKILL.md. Cleanup belongs in /commit-push Step 4.
- [ ] Hard bans softened ("prefer not to run cargo" → "NEVER"). §2 rows are load-bearing.
- [ ] Step 2.5 removed or softened to "best-effort". `review_pipeline:` is the SSOT resume pointer.
- [ ] `workflow.md` re-created. Inline execution means no separate protocol file.
- [ ] `/continue-roadmap` starts running intelligence-graph queries or "reconnaissance" steps.
- [ ] Parent caches scanner output across escalation boundaries. Scan state changes across escalations.
- [ ] Plan-doc cleanup duplicated in /continue-roadmap even as "defense in depth". Single source: plan-cleanup.py in /commit-push.
- [ ] Memory entries re-introduced as workflow-state pointers. CLAUDE.md §"Plan state lives in the plan file" forbids this.
- [ ] Step 2.5 re-introduces AskUserQuestion for marker-present path. Auto-dispatch is the rule; /review-plan Step 1b owns the sole resume prompt.

## §6 Improvement Log

### Open items

_None._

### Recently closed

- [x] 2026-04-20 — **Skill merged: /roadmap-work absorbed into /continue-roadmap.** The two-skill split (/continue-roadmap orchestrator → Skill() dispatch to /roadmap-work) was nominal: same main context, same Opus model, same rule injections, no actual isolation. The handoff was the exploit surface — it let the agent re-open the pacing decision and invoke AskUserQuestion by rationalizing it as "architectural." New structure: one skill, Step 6 subsection execution loop runs inline, with Agent() sub-agent dispatch for bounded work (intel sweeps, multi-file source reads, TDD authoring) where real context isolation matters. Also closed the "architectural decision" loophole with a tight definition (code-shape only) and an explicit banned list of execution-dimension framings (pacing, scope, workload, runway, context). Motivating session: 2026-04-20 /continue-roadmap on §08 full-section — agent loaded ~200K tokens of rule content via system reminders across the handoff, then issued AskUserQuestion about pacing despite user having answered "full-section" at Step 4. Files touched: `.claude/skills/continue-roadmap/SKILL.md` (rewritten with merged content), `.claude/skills/roadmap-work/` (deleted), `.claude/rules/intelligence.md`, `.claude/rules/ask-user-question.md`, `.claude/skills/query-intel/compose-intel-summary.md`, `.claude/skills/commit-push/workflow.md`, `.claude/skills/continue-roadmap/roadmap_scan.py` (comment). Commit: pending.
- [x] 2026-04-20 — **Pacing now binding across subsection boundaries.** `/continue-roadmap` Step 5 dispatch passes `--pacing=<full-section|subsection-by-subsection>` to `/roadmap-work`. `/roadmap-work` Step 8 branches on pacing: `full-section` loops internally to the next unblocked subsection without returning to parent; `subsection-by-subsection` returns to parent. Banned self-halting enumerated ("honest status" pauses, "clean handoff" bailouts, context-anxiety bailouts, re-confirming pacing). Motivating session: user chose full-section, agent still stopped per-subsection to "be honest about context" — pacing choice died at the first subsection boundary because parent re-ran from Step 1, hitting Step 4 pacing prompt again. Superseded by the 2026-04-20 skill merge entry above (merger replaces Step 8's pacing branch with an inline Step 6.8). Commit: `9d567cc3`.
- [x] 2026-04-19 — **Step 2.5 auto-dispatch on marker present (no AskUserQuestion).** Marker-present path now directly invokes `Skill: review-plan <section>`; /review-plan Step 1b owns the sole resume/start-over prompt. Removes the double-prompt where /continue-roadmap asked "Resume?" and /review-plan asked again. Malformed-marker path still escalates (fix/clear/abort). Motivating session: 2026-04-19 three-/clear cycle burned ~200k tokens before reviewer CLI launched; double-prompt was one of four waste points. Commit: pending.
- [x] 2026-04-19 — **Step 2.5 resume prompt extended to surface TPR round progress.** Grep widened to include `rounds_completed`, `last_round_commit`, `last_round_findings`, `note` alongside `stage` / `next_step`. Paired with `/review-plan` Step 6 now persisting these per-round fields on every exit. Observed-bug source: 2026-04-19 pause-without-round-count incident where a Round-0-done state displayed identically to a never-started state. See `review-plan-design.md §6`.
- [x] 2026-04-19 — **Step 2.5 `review_pipeline:` marker check.** Greps focus section frontmatter; if present → `AskUserQuestion` with Resume/Clear/Proceed options. Closes the dispatch-to-wrong-step regression observed on §08 (paused at `editor-done, next_step: 6`). Commit: `f0bc2140`.
- [x] 2026-04-19 — **Sonnet sub-agent removed; inline execution; `workflow.md` deleted.** SKILL.md holds the full protocol. Per-invocation: 3 Bash calls. Removes ~2k Agent-dispatch tax + Sonnet re-read. Commit: `551d03df`.
- [x] 2026-04-19 — **Plan-doc cleanup moved to /commit-push Step 4.** `scripts/plan-cleanup.py` applies stale-frontmatter / annotations / bug-marker fixes alongside `fmt-all.sh`. /continue-roadmap Step 2 deleted; gate severities dropped from auto-fix to info. Closes the two-commit dirty-tree loop. Commit: `db747ab7`. See `script-plan-cleanup-design.md`.
- [x] 2026-04-19 — **SKILL.md + workflow.md + roadmap-work/SKILL.md converted to rule-based form.** Prose stripped, bullets + tables dominate. No behavior change. Commit: `0372e1df`.
- [x] 2026-04-19 — **Step A.5 memory cross-check removed; CLAUDE.md rule tightened.** Prior Band-Aid obsolete under stricter "no workflow state in memory" rule. Memory entry for BUG-04-042 deleted. Commit: `073e283d`.
- [x] 2026-04-18 — _(superseded)_ `unreviewed_plan` gate surfaces resume option when `review_pipeline` marker is present. Replaced 2026-04-19 by consumer-side Step 2.5 which owns the check directly.
- [x] 2026-04-18 — _(historical)_ Added §"Tool-call budget" + §"Hard bans" to `workflow.md`; mirrored in SKILL.md dispatcher. `workflow.md` deleted 2026-04-19 with sub-agent removal; hard bans preserved in SKILL.md §2.

## §7 How To Use This File In Future Sessions

Open before editing `.claude/skills/continue-roadmap/SKILL.md` or `roadmap_scan.py`. §2 invariants are load-bearing — check before relaxing any row. §5 is the pre-edit regression checklist. After every /improve-tooling invocation, add a §6 Recently closed entry with date + one-line description + commit sha.
