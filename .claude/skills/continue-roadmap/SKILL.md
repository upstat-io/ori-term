---
name: continue-roadmap
description: Resume work on the ori_term roadmap. Runs the scanner, evaluates gates inline, asks pacing, then loops over subsections with opportunistic sub-agent dispatch for bounded work.
argument-hint: "[section]"
---

# /continue-roadmap

`/continue-roadmap [section]` — resume roadmap work.

- No args → auto-detect first incomplete section.
- Arg (`section-4`, `4`, keyword) → focus that section.

## Rules

- Main context (Opus) owns orchestration + plan state. Bounded work is dispatched via `Agent()` where sub-agent isolation saves context; inline where dispatch overhead exceeds the savings.
- Plan-doc cleanup is NOT this skill's job — `/commit-push` Step 4 (`scripts/plan-cleanup.py`) owns it.
- Escalations use `AskUserQuestion` with the scanner's `payload.question` + `payload.options` verbatim.
- Commits via `/commit-push` only.
- Pacing is asked ONCE per invocation at Step 4 (section boundary) and is binding for the entire loop. See §Step 6 Pacing enforcement.

## Sub-agent dispatch guidance

Dispatch via `Agent()` for bounded work that would otherwise drag rule files + source reads into the orchestrator's context:

- Intelligence recon sweeps (`symbols` / `callers` / `callees` / `similar` queries that emit long output).
- Heavy source-reading sweeps (multi-file traversal to produce a synthesized summary).
- TDD cell authoring (bounded scope, one or two test files).
- `/tpr-review`, `/impl-hygiene-review`, `/improve-tooling` — already isolated skills; invoke via `Skill()` which handles its own dispatch.
- `/commit-push` — invoke via `Skill()`; it spawns its own sub-agent.

Inline (no sub-agent) for:

- Plan-file reads and edits (small, load-bearing for orchestration state).
- Checkbox flips, frontmatter status updates, subsection status transitions.
- Quick `Grep` / `Glob` against the plan corpus.
- `diagnostics/state.sh show --json` and similar one-shot status probes.

The choice is per-task: if a single file-read + inline reasoning is under a few hundred lines, keep it inline; if it would require loading 2+ rule files or a large source tree, dispatch.

## Step 1 — Run scanner

```
python3 .claude/skills/continue-roadmap/roadmap_scan.py --json [<user-args>]
```

User args (section number, plan dir, keyword) flow through as positional args.

Parse JSON. Three top-level fields:
- `focus_context` — `plan_full_name`, `plan_description`, `section_goal`, `subsections`, `plan_progress_pct`, `plan_progress_text`, `section_progress_text`, `section_file`, `section_number`, `section_title`, `plan_dir`, `plan_name`.
- `next_unblocked` — `{subsection_id, item_content, item_lineno, unblocked_count, blocked_count}` or `null`.
- `gates` — 10 entries, each `{fires, severity, payload}`. Unfired entries have empty `payload: {}`.

Scanner exit ≠ 0 or JSON parse fails → STOP, report stderr.

## Step 2 — Read cached repo state

```
diagnostics/state.sh show --json
```

Extract:
- `.test_suite.status` / `.test_suite.totals.failed` / `.test_suite.known_failing_count` / `.test_suite.remediation[0].plan` + `.subsection`
- `.clippy.status`
- `.hygiene.status`

Missing/non-zero exit → set values to `"unknown"`. Never run `refresh` from here.

## Step 2.5 — Mid-pipeline resume check (MANDATORY — runs BEFORE Step 3 gate eval)

MUST execute on every invocation. Skipping this step is the regression that produced 2026-04-19 session waste (see `continue-roadmap-design.md §5`).

Detection:

```
grep -nE '^review_pipeline:|^  stage:|^  next_step:|^  rounds_completed:|^  last_round_commit:|^  last_round_findings:|^  note:' <focus_context.section_file> | head -10
```

| Result | Action |
|---|---|
| No `review_pipeline:` line | Proceed to Step 3. |
| `review_pipeline:` + valid `stage` + numeric `next_step` | AUTO-DISPATCH — no `AskUserQuestion`. Emit one-line banner `Marker found: stage=<stage>, next_step=<next_step>, rounds_completed=<N\|0>. Auto-resuming /review-plan.`, then `Skill: review-plan <focus_context.section_file>`. After return → re-run `/continue-roadmap` fresh. |
| `review_pipeline:` present but `stage`/`next_step` missing or non-numeric | STOP, escalate via `AskUserQuestion`: (1) `Fix marker manually (Recommended)` — user repairs, re-run; (2) `Clear marker` — Edit to remove, proceed to Step 3; (3) `Abort`. |

Rules:
- `/continue-roadmap` NEVER asks about resume — `/review-plan` Step 1b owns the sole resume/start-over prompt. Double-prompt is a contract violation.
- Auto-dispatch reason: user invoked `/continue-roadmap` (intent = continue existing work); a marker is explicit continuation state.
- See `/review-plan SKILL.md §Step 1d` for marker schema.

## Step 3 — Evaluate gates

Any `severity: block` gate fires → STOP, escalate via `AskUserQuestion` before entering the Step 6 subsection loop.

| Gate | Severity | Action |
|---|---|---|
| `parse_error_sections` | block | Escalate. Fix YAML before proceeding. |
| `stale_frontmatter` | info | Include focus-plan mismatch count in handoff. `/commit-push` cleans on next commit. |
| `stale_plan_annotations` | info | Include count. `/commit-push` cleans on next commit. |
| `bug_marker_drift` | info | Include `missing_marker_count` + `orphan_marker_count`. `/commit-push` auto-fixes missing; orphans stay info. |
| `unmet_dependencies` | block or info | `block` on dep cycle or unresolvable+unmet; `info` on stale refs only. |
| `unreviewed_plan` | block | Escalate. Options: `/review-plan`, proceed, pick-different. |
| `tpr_findings` | block | Escalate. `/verify-tpr` with `payload.next_skill_arg`. |
| `critical_bugs` | block | Escalate. `/fix-bug` with bug IDs from `payload.bugs`. Includes high→critical elevations. |
| `high_bugs` | info | Include IDs in handoff. |
| `dirty_tree` | block | Escalate. Options: `/commit-push` (runs `fmt-all.sh` + `plan-cleanup.py` before staging), proceed-dirty. Never destructive git. |

Multiple block-gates fire together → sequential `AskUserQuestion` calls, one per gate.

## Step 4 — Pacing question (ONCE per invocation, section boundary only)

Fires only when no block-gates fire. This is the SOLE pacing prompt; Step 6 NEVER re-asks.

```
AskUserQuestion({
  question: "How should I pace Section <focus_context.section_number>?",
  options: [
    { label: "subsection-by-subsection (Recommended)", ... },
    { label: "full-section", ... },
  ],
})
```

Record the user's choice. `/improve-tooling` retrospective fires regardless of pacing.

## Step 5 — Emit Focus Context

Emit the Focus Context block as plain text to the user BEFORE entering the Step 6 loop:

```
## Focus: <focus_context.plan_full_name> — Section <focus_context.section_number>: <focus_context.section_title>

**Plan**: <focus_context.plan_description>
**Section goal**: <focus_context.section_goal>
**Plan progress**: <plan_progress_pct>% (<plan_progress_text>)
**Section progress**: <section_progress_text>

Subsections:
  <id>  <title>  [<status>]
  ...

### Gate results
- Stale frontmatter: <count or "none">
- Stale plan annotations: <count or "none">
- Unreviewed plan: <pass | block>
- TPR findings: <none | N open>
- Critical bugs: <none | N>
- High bugs: <none | N (IDs)>
- Dirty tree: <clean | N files>

### Cached repo state (from state.sh)
- Test suite: <status> (<passed>/<failed>/<skipped> @ <head_sha>)
- Known-failing files: <known_failing_count>
- Remediation: <remediation[0].plan> §<subsection> — or "none"
- Clippy: <clippy.status>
- Hygiene: <hygiene.status>
- Cache freshness: <fresh | stale (dirty tree) | obsolete | missing>

### Next unblocked item
Subsection <next_unblocked.subsection_id>: <item_content>
(<unblocked_count> unblocked, <blocked_count> blocked)
```

One-line gate summary (e.g. "0 block-gates fired"), then enter Step 6.

## Step 6 — Subsection execution loop

Enter the loop at `next_unblocked.subsection_id`. Each iteration runs Steps 6.0 through 6.7 inline (with sub-agent dispatch where the §Sub-agent dispatch guidance table indicates).

### 6.0 — Re-read CLAUDE.md

Mandatory on every subsection iteration. Context compression may drop rules between iterations.

### 6.1 — Load subsection detail

Read `<plan-path>/<section-file>` for the current `<subsection-id>`. Identify the specific `- [ ]` items. Inline (plan-doc read).

### 6.2 — Blast-radius check (optional)

Before touching code: Grep/Glob the section's cited symbols/paths to inventory callers. Skip for tightly-scoped subsections (test helpers, single-file widgets). Dispatch via `Agent()` when the subsection touches a broad symbol surface; inline for 1-2 call-site checks. No intelligence graph in this project — Grep/Glob is the SSOT.

### 6.3 — Read affected source

Read code paths the subsection will touch before modifying. Dispatch via `Agent()` for multi-file sweeps (>3 files or >1000 lines total); inline for targeted reads (1-2 files, <500 lines).

### 6.4 — Invariant anchor (mandatory before coding)

Answer in scratchpad before touching code:

1. What invariant does this subsection enforce? (cite `typeck.md §PC-*`, `aims-rules.md` dimension, ARC RC balance, phase-purity contract, spec clause).
2. Which downstream system consumes it?
3. If tests fail, response is ALWAYS (a) fix code-under-test, NEVER (b) weaken invariant.

### 6.5 — Execute subsection checkboxes

Implementation Guidelines:

- ZERO DEFERRAL — implement, don't document for later. Discovery IS the assignment.
- ALL Deferrals Must Have Implementation Anchors per CLAUDE.md.
- Plan Boundary Integrity — cross-section fix → update affected section's plan.
- Scope Rule — ALL Checkboxes In Section Are In Scope.
- Verification Rule — Empty Checkboxes Must Be Verified.
- Matrix Testing Rule → `.claude/rules/tests.md`.
- TDD for Bugs → CLAUDE.md §TDD for Bugs.

Dispatch TDD cell authoring + bulk implementation via `Agent()` when the scope exceeds ~200 LOC or touches ≥2 crates. Inline for single-file edits.

### 6.6 — Run tests + handle failures

```
timeout 150 cargo test --all
```

Classify each failure:

| Classification | Definition | Action |
|---|---|---|
| **A — Known Failing** | Matches pattern in section's Known Failing Tests block AND plan points to concrete follow-up anchor. | Record in Known Failing Tests table. Continue. |
| **B — Blocker surfaced by THIS deliverable** | Subsection's validator/check correctly surfaced a real PC-2 / lattice / contract / spec violation. Code IS doing its job. | Blocking → `/fix-bug` NOW (full rigor: root cause, TDD matrix, TPR, hygiene). Non-blocking to this subsection → `/add-bug`. Do NOT mark subsection complete until fix lands AND deliverable is active on the previously-failing path. |
| **C — Regression caused by THIS subsection's code** | Something that should NOT have broken is now broken. | Shelve subsection, fix regression, re-apply. (CLAUDE.md §Stabilization Discipline.) |

Banned responses to test failures:

- Feature-flag / gate / early-return that skips the subsection's validator on failing path.
- Widening a validator's exemption set to silence failing tests (unless spec-correct, verified by fresh spec reading).
- Moving failing case to "Known Failing Tests" without concrete anchor.
- Adding a test for the neutered behavior.
- Marking subsection complete when the deliverable is gated off on any code path.

Required response: architecturally-correct fix per CLAUDE.md §The One Rule. Cross-crate scope IS the work.

### 6.7 — Subsection close-out

1. Verify all subsection tasks `[x]` and behavior verified.
2. `Skill: tpr-review` — dispatched sub-agent, isolated context.
3. Resolve every TPR finding inline or with concrete `- [ ]` anchor per `.claude/rules/impl-hygiene.md` §Findings Disposition. Filing via `/add-bug` is banned.
4. `Skill: impl-hygiene-review` AFTER TPR clean — dispatched sub-agent.
5. Resolve every hygiene finding inline or with concrete anchor. Any open unanchored finding BLOCKS close-out.
6. Update subsection `status: complete` in section frontmatter. Inline edit.
7. `Skill: improve-tooling` retrospectively on THIS subsection — dispatched sub-agent.
8. `diagnostics/repo-hygiene.sh --check` (and `--clean` if needed).
9. `Skill: commit-push` — dispatched sub-agent.

### 6.8 — Pacing-aware loop continuation

Branch on the pacing choice recorded at Step 4:

| Pacing | Next subsection available in this section? | Action |
|---|---|---|
| `full-section` | yes, unblocked | Loop back to Step 6.0 with the next unblocked subsection. Do NOT ask the user. |
| `full-section` | no (all complete or only blocked remain) | Exit loop. Report final status (`section complete` OR `blocked: <reason>`). |
| `subsection-by-subsection` | any | Exit loop. Re-run `/continue-roadmap` fresh to prompt for next subsection. |

Banned self-halting (applies to ALL pacings):

- "Let me stop and be honest about context" — context management is a runtime concern, not a decision point. Keep coding.
- "Clean handoff point" unsolicited pauses — every subsection boundary is a clean handoff; that is the structure, not a reason to stop.
- Re-confirming pacing — Step 4's choice is binding. Do NOT ask "should I continue?" or "do you want the next one?".
- Pre-emptive "I'll probably run out of context mid-TPR" bailouts — the 1M-token window and auto-compression exist for this. Use them. Dispatch the heavy work to sub-agents if orchestrator context is crowded.
- Reframing pacing/scope/workload/size/runway/context as "architectural" to bypass the above bans. These are EXECUTION dimensions, not architecture (see next block).

"Architectural decision" — TIGHT definition:

An architectural decision is a choice about CODE SHAPE with multiple demonstrably-correct implementations. Concretely:
- Which data structure owns this state (e.g., `Vec<u32>` vs `FxHashSet<u32>`).
- Which module/crate/phase owns this logic (e.g., `unify/` vs `check/bodies/`).
- Which algorithm implements this step (e.g., substitute-map walk vs folder trait).
- Which API shape this function exposes (e.g., `&mut FunctionSig` vs loose `&mut [Idx]`).
- Which spec clause / invariant governs this behavior when two apparently apply.

NOT architectural — banned as `AskUserQuestion` subjects regardless of framing:
- Pacing, work size, workload, scope, "how much to do this session".
- Session runway, context budget, token usage, compression likelihood.
- Whether to continue, pause, checkpoint, or commit now vs later.
- Whether TPR/hygiene/commit-push should run (they are mandatory per Step 6.7).
- Retry-vs-skip on reviewer timeouts (the reviewer-liveness probe decides).

Test before any `AskUserQuestion` inside Step 6:
1. Would the user's answer change a `.rs` / `.md` / `.ori` file's content, shape, or placement? → architectural, allowed.
2. Would the user's answer change how many items you execute, when you stop, or whether mandatory gates run? → execution dimension, banned.

Valid stop conditions inside Step 6 (only these):

1. `AskUserQuestion` for a genuine architectural decision per the tight definition above (multiple correct code shapes).
2. `/fix-bug` nested invocation (Classification B blocker).
3. Tool failure preventing progress (build broken, harness crashes — not test failures).
4. Loop exit per the pacing table above.

## Escalation contract (Step 3 block-gates only)

For each block-gate fired:
1. `AskUserQuestion` with `payload.question` + `payload.options` verbatim.
2. User picks option with `next_skill` → `Skill: <next_skill> <arg>`.
3. User picks `proceed` / `pick-different` → no skill, next question.
4. After all questions + skills resolve → re-run `/continue-roadmap` fresh (new scan).

Never cache scan state across invocations. Never re-execute steps inline on re-entry — always start from Step 1.

## Files

- `SKILL.md` — this file.
- `roadmap-scan.sh` / `roadmap_scan.py` — scanner.

## Related

- `.claude/skills/fix-bug/SKILL.md`, `.claude/skills/tpr-review/SKILL.md`, `.claude/skills/impl-hygiene-review/SKILL.md`, `.claude/skills/commit-push/SKILL.md`, `.claude/skills/improve-tooling/SKILL.md` — nested skills dispatched from Step 6.
