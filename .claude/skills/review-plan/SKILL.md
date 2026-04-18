---
name: review-plan
description: Review and improve a plan via a 6-phase pipeline. Steps 2–5 and 7+8 run as sub-agents via Agent({}); Step 6 runs inline in main context by invoking /tpr-review via the Skill tool; the parent handles path normalization, cross-plan invalidation, and the final verdict.
argument-hint: "<plan-path>"
---

# Review Plan

`/review-plan <plan-path>` — review and improve a plan. `plan-path` is a plan directory (whole-plan mode) or a single section file (single-section mode).

The parent does Step 1 inline, dispatches Steps 2–5 and Step 7+8 as independent `Agent({})` sub-agents reading step-specific `.md` protocol files, runs Step 6 inline in main context (invoking `/tpr-review` via the Skill tool), then handles Steps 8.5 and 9 inline.

## Reviewed-field semantics

- **Single-section mode** (file path): after the FULL pipeline clean pass, Step 7+8 flips `reviewed: true` on `{target_section}`. If unresolved issues remain, leaves `reviewed: false`.
- **Whole-plan mode** (directory path): NEVER touch `reviewed` fields. Fix content issues only.

## Step 1 — Path Normalization + Scratch Dir (parent, inline)

Inspect `$ARGUMENTS`:

- **Single file** (e.g. `plans/foo/section-03.md`): `mode = "single-section"`, `plan_dir = "plans/foo"`, `target_section = "plans/foo/section-03.md"`.
- **Directory** (e.g. `plans/foo/`): `mode = "whole-plan"`, `plan_dir = "plans/foo"`, `target_section = null`.

If the path does not exist, stop and report.

**Create an orchestrator-owned scratch dir BEFORE any sub-agent dispatch** (matches `/tpr-review` §8 invariant I1 — per-invocation `mktemp -d` prevents cross-session collision; the `${repo}` prefix makes parallel sessions in different repos visually distinguishable in `/tmp/` listings):

```bash
repo="$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")"
RUN_DIR="$(mktemp -d -t "review-plan-${repo}-XXXXXXXX")"
echo "scratch dir: $RUN_DIR"
```

Write `$RUN_DIR/context.json`:

```json
{
  "mode": "single-section | whole-plan",
  "plan_dir": "plans/foo",
  "target_section": "plans/foo/section-03.md | null",
  "run_dir": "<absolute path from mktemp>"
}
```

Capture `$RUN_DIR` in orchestrator state and pass it to every sub-agent dispatch below as the `{RUN_DIR}` placeholder.

## Steps 2–5 and 7+8 — Sequential Sub-agent Dispatch

Invoke five Agents in order (Steps 2, 3, 4, 5, then 7+8 — Step 6 is inline, see below). Each dispatch follows the same shape; only `description`, the step `.md` file, and model differ. After each Agent returns, read `$RUN_DIR/{step}.json` for the summary + any escalation payload, then proceed.

**Dispatch template** (substitute `<STEP>`, `<PROTOCOL_FILE>`, `<MODEL>`, and `$RUN_DIR`):

```
Agent({
  description: "review-plan step <STEP>",
  subagent_type: "general-purpose",
  model: "<MODEL>",
  prompt: `
You are the sub-agent for /review-plan Step <STEP>. Read
.claude/skills/review-plan/<PROTOCOL_FILE>
in full and execute it end-to-end.

Your orchestrator-owned scratch dir is: <RUN_DIR absolute path from Step 1>
All read/write paths described in the protocol are relative to that dir.

Read these handoff files first for context:
- <RUN_DIR>/context.json (plan mode, plan_dir, target_section)
{for steps 3+: list <RUN_DIR>/*.json files from prior steps}

Write your own handoff to <RUN_DIR>/<STEP>.json per the protocol's
"Output" section. Never read CLAUDE.md unless the protocol explicitly
requires it (only Step 5 does).

Commits via /commit-push only — never run git commit directly.

Touch only plan docs (plans/**/*.md) unless the protocol explicitly
authorizes other edits. Never edit .rs, .ori, or anything under compiler/,
library/, tests/.

If the protocol says to escalate, return the escalation payload in the
handoff JSON (escalate: true) and stop. The parent handles escalations.
  `
})
```

**Per-step dispatch parameters:**

| Step | `<STEP>` | `<PROTOCOL_FILE>` | `<MODEL>` | Writes |
|---|---|---|---|---|
| 2 | `2-precheck` | `step-2-precheck.md` | `opus` | `$RUN_DIR/precheck.json` |
| 3 | `3-audit` | `step-3-audit.md` | `sonnet` | `$RUN_DIR/audit.json` |
| 4 | `4-blind-spots` | `step-4-blind-spots.md` | `sonnet` | `$RUN_DIR/blind-spots.json` |
| 5 | `5-editor` | `step-5-editor.md` | `opus` | `$RUN_DIR/editor.json` |
| 7+8 | `7-8-verify` | `step-7-8-verify.md` | `sonnet` | `$RUN_DIR/verify.json` |

Step 6 is NOT in this table — it runs inline in main context. Run Step 6 after Step 5 returns and before dispatching Step 7+8. See `## Step 6 — /tpr-review Convergence (inline)` below.

## Step 6 — /tpr-review Convergence (inline, main context)

After Step 5 returns and its escalation (if any) is resolved, run Step 6 inline in main context. Do NOT wrap this in an `Agent({})` sub-agent.

1. Read `.claude/skills/review-plan/step-6-tpr.md` and follow it end-to-end inline.
2. Invoke `/tpr-review` via the Skill tool with `--skill review-plan` plus the scope (`{target_section}` in single-section mode, `{plan_dir}` in whole-plan mode).
3. When `/tpr-review` returns, observe its terminal `exit_reason` (Step 6 runs inline; no file handoff is required from `/tpr-review`) and write `$RUN_DIR/tpr.json` per the branch schemas in `step-6-tpr.md`. The current `status` set — defined by `/tpr-review/SKILL.md §5`'s `exit_reason` values — is `clean` / `iter_cap_reached` / `meta_cap_reached` / `user_accepted` / `escalated` / `both_reviewer_failure`.
4. Apply the same escalation handling described in the next section, reading `$RUN_DIR/tpr.json` as if it came from a sub-agent.
5. Proceed to Step 7+8 once the escalation resolves.

## Escalation handling (MANDATORY)

After each Agent returns (Steps 2–5, 7+8) or after Step 6 writes its inline handoff, read `$RUN_DIR/{step}.json`. If `"escalate": true`:

1. Invoke `AskUserQuestion` with the sub-agent's `question` + `options` verbatim. Never dump as prose.
2. Branch on the selected option using the dispatch table below. The table enumerates every option-key semantic category emitted by the step protocols (`step-2-precheck.md`, `step-5-editor.md`, `step-6-tpr.md`, `step-7-8-verify.md`). Unknown keys MUST re-prompt via `AskUserQuestion` rather than silently no-op.
3. Resume the next step only after the escalation resolves.

### Option-key dispatch table

Each option a step emits falls into one of the categories below. The parent checks the option's **flags and known keys** in order; the first match wins.

The table enumerates every option key emitted by the current step protocols. When a step emits a new key not listed here, add a row AND update the design-log regression list — silent dispatch on an unmapped key is banned.

| Category | Matches on | Known keys | Parent action |
|---|---|---|---|
| **User-accept TPR non-convergence** | option has `applies_user_accepted: true` | `accept-with-findings` (step-6 Branch 2; shared key with `/tpr-review §5` for consistency) | PATCH `$RUN_DIR/{step}.json` — read the handoff, add top-level `"user_accepted": true` and `"user_accepted_option_key": "<selected key>"`, write it back. This signals Step 7+8 to flip `reviewed: true` despite `converged: false`. Record the cap-exit reason in the Step 9 verdict's `### Review Status` row. Resume the pipeline (next step = Step 7+8 when escalation came from Step 6). |
| **Accept-minor / proceed with residual findings** | literal key `accept-minor` | `accept-minor` (step-7-8 verify-non-convergence) | Proceed to Step 8.5 with the current state. No handoff patching (Step 7+8 has already run); the verdict at Step 9 records the residuals under `### Remaining Concerns`. |
| **Invoke a named follow-up skill** | option has a non-null string `next_skill: "<name>"` | (no current key carries a non-null `next_skill`; reserved for future editor / audit escalation options that route to a different skill) | Invoke `Skill: <next_skill>` with the option-provided args (see each option's `next_skill_arg` if present). Resume the pipeline at the step AFTER the one that escalated. |
| **Editor ambiguity resolution** | literal keys `prefer-section-03`, `prefer-section-05`, `split-scope` | step-5 editor Branch B (editor escalated on two contradictory designs) | Record the user's resolution on `$RUN_DIR/editor.json` as `resolution: "<selected key>"` and re-dispatch Step 5 with the resolution hint. The editor re-runs with the chosen direction and its second pass should produce `escalate: false`. If the editor escalates AGAIN with the same ambiguity, abort to Step 9 with a `NEEDS MANUAL ATTENTION` verdict. |
| **Escalate findings to a new plan** | literal key `escalate-to-plan` | `escalate-to-plan` (step-6 Branch 2) | Invoke `Skill: create-plan` with `ever_verified_findings` (from `$RUN_DIR/tpr.json`) as the mission input. Stop the current `/review-plan` pipeline and emit a partial verdict at Step 9 noting the new plan was created (the residual findings are now owned by that plan). |
| **Retry the current step** | literal keys `retry-with-hints`, `retry-immediately`, `dispatch-editor-round-2` | Step 6 Branch 2: `retry-with-hints`; Step 6 Branch 4: `retry-immediately`; Step 7+8 verify-non-convergence: `dispatch-editor-round-2` | Re-dispatch the step that escalated. For `retry-with-hints`, solicit the hint via a follow-up `AskUserQuestion` BEFORE re-invoking `/tpr-review --skill review-plan`. For `retry-immediately`, re-dispatch without additional input. For `dispatch-editor-round-2`, re-dispatch Step 5 (editor) first, THEN re-run Step 7+8. |
| **Triage a transport failure** | literal key `triage-failure` | `triage-failure` (step-6 Branch 4) | Render the `postmortem_dir` path from the handoff and pause. Re-emit the Step 6 Branch 4 `AskUserQuestion` with `triage-failure` removed from the options (user has triaged; now decides retry vs. abandon). |
| **Walk ambiguous cases individually** | literal key `fix-individually` | `fix-individually` (step-2 precheck) | For each item in the step's `escalations` list, re-emit a narrower `AskUserQuestion` letting the user resolve that specific case (keep-as-in-progress / manual-edit / delete). Resume the pipeline only after the full list is drained. |
| **Abort / abandon / leave-as-is** | literal keys `abort`, `abandon-review`, `leave-as-is`, `abort-editor` | `abort` (step-2, step-5, step-6, step-7-8), `abandon-review` (step-6 Branch 4), `leave-as-is` (step-2), `abort-editor` (step-5 Branch B — `next_skill: null` means no follow-up skill, so the option collapses to an abort) | Stop the pipeline and emit a partial verdict at Step 9. For `leave-as-is`, only the current step's proposed changes are discarded — prior steps' fixes are kept. For `abort` / `abandon-review` / `abort-editor`, the remaining pipeline is skipped entirely. |
| **Unknown key (safety fallback)** | none of the above match | (any future option not yet categorized) | Re-emit `AskUserQuestion` with the original `question` + `options` plus a note "Selected option not recognized by handler — please pick again or Abort." Never silently no-op on an unrecognized key; that would lose user input. |

### Which step produced the escalation?

Know this before acting: the action for "retry the current step" or "invoke follow-up skill" depends on WHICH step's handoff carried `escalate: true`. Track `{step}` (the step identifier — `precheck`, `audit`, `blind-spots`, `editor`, `tpr`, or `verify`) alongside the handoff payload; don't try to infer it from the option keys alone. Precheck is the most common source; Steps 3, 5, 6, and 7+8 can also escalate. Step 4 (blind-spots) never escalates.

## Step 8.5 — Cross-Plan Invalidation (parent, Opus)

After Step 7+8 returns clean, run the invalidation detector inline on the parent:

```bash
python3 .claude/skills/plan-audit/plan-invalidate.py {plan_dir} --json > "$RUN_DIR/invalidate.json"
```

Read the output. If `status == "clean"`, skip to Step 9.

If stale sections exist, invoke `AskUserQuestion` with a structured options list — never prose-numbered bullets (§11.5 of `/tpr-review/SKILL.md`, `.claude/rules/ask-user-question.md`):

```
AskUserQuestion(questions=[{
    "question": f"This review changed scope that overlaps with {N} reviewed "
                 f"sections across {M} other plans. How should their "
                 f"reviewed: true fields be handled?",
    "header": "Cross-plan invalidation",
    "multiSelect": False,
    "options": [
        {"key": "invalidate-high-impact",
         "label": "Invalidate high-impact only (weight >= 4) (Recommended)",
         "description": "Recommended because weight-gated invalidation catches "
                        "the sections whose reviewed: true is genuinely at risk "
                        "(significant scope overlap) while leaving weakly-coupled "
                        "reviews intact. This is the canonical middle path: "
                        "stronger than skip, safer than invalidate-all.",
         "recommended": True},
        {"key": "invalidate-all",
         "label": f"Invalidate all {N} overlapping sections",
         "description": "Flips reviewed: false on every overlapping section. "
                        "Pick only when the scope changes are substantial enough "
                        "that even low-weight overlaps warrant re-review."},
        {"key": "skip-invalidation",
         "label": "Skip — leave reviews as-is",
         "description": "No cross-plan changes. Pick only when the review was "
                        "purely cosmetic/formatting and no downstream reviewer "
                        "would reach a different conclusion after reading it."},
    ],
}])
```

Apply the user's choice:

```bash
# invalidate-all:
python3 .claude/skills/plan-audit/plan-invalidate.py {plan_dir} --apply

# invalidate-high-impact:
python3 .claude/skills/plan-audit/plan-invalidate.py {plan_dir} --apply --min-weight 4

# skip-invalidation: no action
```

Skip this step entirely if the review made only cosmetic/formatting changes (no scope shifts).

## Step 9 — Present Verdict (parent, Opus)

Read the `summary` line from each `$RUN_DIR/*.json` (not the full handoffs) and synthesize:

```
## Plan Review: {plan name}

### Pipeline Summary
- **Pre-check** (Step 2): {N effectively-complete sections corrected}
- **Phase 1** (Step 3, plan-audit.py): {N findings, M auto-fixed, K remaining}
- **Phase 2** (Step 4, /tp-help): {N blind spots}
- **Phase 3** (Step 5, editor): {structural changes + content edits}
- **Phase 4** (Step 6, /tpr-review): {clean on iteration N | max reached}
- **Post-edit verification** (Step 7+8): {CLEAN | N remaining}

### Review Status
| Section | `reviewed` Before | `reviewed` After | Reason |
| ... | ... | ... | ... |

### Cross-Plan Invalidation
{Results or "Skipped — changes were cosmetic."}

### Remaining Concerns
{Human-judgment issues ranked Critical > Major > Minor, or "none"}

---

## Verdict
**{CLEAN | MINOR FIXES APPLIED | SIGNIFICANT REWORK APPLIED | RESTRUCTURED | NEEDS MANUAL ATTENTION}**

{2-3 sentences summarizing total edits across all phases.}
```

**Verdict definitions:**
- **CLEAN** — no issues found
- **MINOR FIXES APPLIED** — small corrections
- **SIGNIFICANT REWORK APPLIED** — substantial edits (reordered, added missing sections, fixed incorrect assumptions)
- **RESTRUCTURED** — structure fundamentally changed
- **NEEDS MANUAL ATTENTION** — issues requiring human judgment

## Critical Rules

1. **Sequential phases** — each step completes before the next starts (Steps 2–5, 7+8 are Agents; Step 6 is inline). Handoffs flow via `$RUN_DIR/*.json` files (the orchestrator-owned scratch dir created in Step 1).
2. **`reviewed` flip is LAST** — only in single-section mode, only after Step 6 either converges clean OR the user explicitly accepts a cap-exit via an `applies_user_accepted: true` option (see `step-6-tpr.md` Branch 2/3 and the `user_accepted == true` flip branch in `step-7-8-verify.md §Step 7`). Handled by Step 7+8's agent, never inside the editor.
3. **Whole-plan mode never touches `reviewed`** — not even to add missing ones.
4. **NEVER scope down — always expand** — grow the plan if it doesn't fulfill its mission.
5. **`/tpr-review` is MANDATORY** — Step 6 runs it unconditionally.
6. **Never dismiss TPR findings as "unrelated"** — per CLAUDE.md, no "pre-existing" / "out of scope".
7. **AskUserQuestion on escalation** — never dump escalation payloads as prose.

## Files in this skill

- `SKILL.md` (this file) — parent dispatcher + Steps 1, 8.5, 9.
- `step-2-precheck.md` — Step 2 protocol (effectively-complete detection).
- `step-3-audit.md` — Step 3 protocol (plan-audit.py orchestration).
- `step-4-blind-spots.md` — Step 4 protocol (/tp-help dispatch + distill).
- `step-5-editor.md` — Step 5 protocol (4-lens editor).
- `step-6-tpr.md` — Step 6 protocol (/tpr-review convergence loop — read and executed inline by the parent, NOT dispatched as an Agent).
- `step-7-8-verify.md` — Steps 7+8 protocol (reviewed flip + audit verify loop).

None of `step-*.md` files are registered as skills. They are reference documents: `step-2` through `step-5` and `step-7-8` are read by dispatched Agents; `step-6-tpr.md` is read by the parent inline in main context.
