---
name: review-plan
description: Review and improve a plan via a 6-phase pipeline. Steps 2, 3, 5 and 7+8 run as sub-agents via Agent({}); Step 4 (/tp-help blind-spots) and Step 6 (/tpr-review) run inline in main context via the Skill tool; the parent handles path normalization, cross-plan invalidation, and the final verdict.
argument-hint: "<plan-path>"
---

# Review Plan

`/review-plan <plan-path>` — review and improve a plan. `plan-path` is a plan directory (whole-plan mode) or a single section file (single-section mode).

The parent does Step 1 inline, dispatches Steps 2, 3, 5 and 7+8 as independent `Agent({})` sub-agents reading step-specific `.md` protocol files, runs Step 4 (/tp-help blind-spots) and Step 6 (/tpr-review) inline in main context via the Skill tool, then handles Steps 8.5 and 9 inline.

## Reviewed-field semantics

- **Single-section mode** (file path): after the FULL pipeline clean pass, Step 7+8 flips `reviewed: true` on `{target_section}`. If unresolved issues remain, leaves `reviewed: false`.
- **Whole-plan mode** (directory path): NEVER touch `reviewed` fields. Fix content issues only.

## Step 1 — Path Normalization + Scratch Dir + Resume Detection (parent, inline)

Inspect `$ARGUMENTS`:

- **Single file** (e.g. `plans/foo/section-03.md`): `mode = "single-section"`, `plan_dir = "plans/foo"`, `target_section = "plans/foo/section-03.md"`.
- **Directory** (e.g. `plans/foo/`): `mode = "whole-plan"`, `plan_dir = "plans/foo"`, `target_section = null`.

If the path does not exist, stop and report.

### Step 1a — Resume detection (plan file is the SSOT)

The **plan file is the single source of truth for pipeline state.** Every step writes a `review_pipeline:` marker block to the target section's frontmatter recording `stage`, `next_step`, and `updated`. On invocation, probe for an existing marker BEFORE creating a scratch dir or dispatching any step.

**Single-section mode:** read the section file's YAML frontmatter. Look for a top-level `review_pipeline:` block:

```yaml
review_pipeline:
  stage: <stage-name>           # precheck-done | audit-done | blind-spots-done | editor-done | tpr-done
  next_step: <int>              # 3, 4, 5, 6, or 7
  updated: <YYYY-MM-DD>
  # --- Step-6-only fields (present any time Step 6 has run ≥1 round) ---
  rounds_completed: <N>         # cumulative TPR rounds done; /tpr-review initializes iteration_counter from this on resume
  last_round_commit: <sha>      # SHA of the last round's fix commit (`/tpr-review §7 fix-and-commit`)
  last_round_findings: <N>      # verified findings count from the last round (all dispositions)
  note: <freeform>              # optional — e.g. "paused mid-loop" or "<exit_reason>" for cap-exits
```

- `rounds_completed` / `last_round_commit` / `last_round_findings` are MANDATORY any time Step 6 has run ≥1 round (clean, paused, or cap-exit). Absent ≡ Step 6 hasn't started ≡ `iteration_counter` starts at 0 on next entry.
- Omitting these after a round ran loses round-count provenance → duplicate work on resume (2026-04-19 pause-without-round-count incident).

**Whole-plan mode:** read `<plan_dir>/.review-pipeline-state.yaml` (a plan-owned dotfile with the same schema). Whole-plan mode uses this file because it never touches section frontmatters (§Reviewed-field semantics).

If a marker is present AND `next_step` is valid, this is a resumable pipeline — invoke Step 1b. Otherwise it's a fresh run — invoke Step 1c.

### Step 1b — Resume path

Invoke `AskUserQuestion` per `.claude/rules/ask-user-question.md`:

```
AskUserQuestion(questions=[{
    "question": f"Section {target_section_name} is mid-pipeline at stage '{marker.stage}' (last updated {marker.updated}). How do you want to proceed?",
    "header": "Resume mid-pipeline",
    "multiSelect": False,
    "options": [
        {"label": f"Resume from Step {marker.next_step} (Recommended)",
         "description": f"Recommended because the plan file records the pipeline reached stage '{marker.stage}' on {marker.updated}. Re-running completed steps is wasted work: Step 4 (/tp-help) alone costs ~20–45 min of reviewer wall-clock. Resume dispatches Step {marker.next_step} immediately against the plan's current state (Steps 2..{marker.next_step - 1} already edited the plan; their outputs are baked into what you see now).",
         "recommended": True},
        {"label": "Start over fresh (clear the marker)",
         "description": "Clear the review_pipeline marker and re-run every step from Step 2. Pick when the plan section changed materially since the prior run (e.g., the mission was rewritten) OR when you suspect the prior pipeline's Step 5 edits introduced correctness issues that need re-validation from scratch."},
    ],
}])
```

On **resume**: proceed directly to the dispatch block for `marker.next_step`. Emit a one-line header:

```
## Resuming /review-plan on {target_section_name} from Step {next_step} (prior stage: {marker.stage}, updated: {marker.updated})
```

On **start-over**: remove the `review_pipeline` block from the section frontmatter (or delete `.review-pipeline-state.yaml` in whole-plan mode), then proceed as Step 1c fresh.

**Edge case — step 5 resume without intermediate JSONs:** if `next_step == 5` (editor), the Step 5 agent reads Steps 2/3/4 handoff summaries to inform its edits. Those summaries are NOT in the plan file; the transient scratch dir from the prior session is gone. The agent re-runs Steps 2-4 internally to regenerate those summaries — documented in `step-5-editor.md`. This is the only resume point where work is duplicated; all other resume points (Steps 6, 7+8) read the plan directly and do not need prior-step JSONs.

### Step 1c — Fresh-run path + scratch dir

Create a per-invocation scratch dir for transient intermediate JSONs (handoff payloads between Steps 2 and 5 consumed by the editor; not required for resume):

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

The scratch dir is ephemeral by design — `/tmp/` is fine because the **plan file carries resume state**, not the scratch. This matches the simpler "plan is SSOT" model from the 2026-04-18 fix.

Capture `$RUN_DIR` in orchestrator state and pass it to every sub-agent dispatch below as the `{RUN_DIR}` placeholder.

### Step 1d — Per-step marker-write contract (MANDATORY)

Every step (2, 3, 4, 5, 6) MUST update the `review_pipeline` marker on the plan file before yielding. Step 7+8 on clean exit (reviewed: true) removes the marker entirely. The marker write is an `Edit` on the section file (single-section) or on `<plan_dir>/.review-pipeline-state.yaml` (whole-plan):

| After step | `stage` value | `next_step` value |
|---|---|---|
| 2 (precheck) | `precheck-done` | `3` |
| 3 (audit) | `audit-done` | `4` |
| 4 (blind-spots) | `blind-spots-done` | `5` |
| 5 (editor) | `editor-done` | `6` |
| 6 (tpr) | `tpr-done` | `7` |
| 7+8 (verify + reviewed flip) | remove `review_pipeline` block entirely | — |

**Why this matters:** `/clear` + `/continue-roadmap` has ONLY the plan file to go on. Without this marker, /continue-roadmap sees `reviewed: false` and fires the `unreviewed_plan` gate, recommending a fresh `/review-plan` invocation — which then re-runs every step including Step 4's ~20–45 min /tp-help. This pattern has burned two prior sessions; closing it is the entire point of the 2026-04-18 fix (see `review-plan-design.md §4`).

Sub-agents for Steps 2, 3, 5, 7+8 receive the marker-write requirement as part of their dispatch prompt (below). Inline Steps 4 and 6 perform the marker update in the parent's own `Edit` call after writing their handoff JSON.

## Steps 2, 3, 5 and 7+8 — Sequential Sub-agent Dispatch

Invoke four Agents in order (Steps 2, 3, 5, then 7+8 — Steps 4 and 6 run inline, see below). Each dispatch follows the same shape; only `description`, the step `.md` file, and model differ. After each Agent returns, read `$RUN_DIR/{step}.json` for the summary + any escalation payload, then proceed.

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

AFTER writing your handoff JSON, you MUST update the target section's
`review_pipeline` frontmatter block to record the new stage. This is
MANDATORY per SKILL.md §Step 1d — without it, `/clear` + /continue-roadmap
cannot detect the mid-pipeline state on resume, and the pipeline silently
restarts from Step 2 on the next invocation (burning ~20–45 min of Step 4
/tp-help reviewer wall-clock needlessly). Use the Edit tool on the section
file to set:

  review_pipeline:
    stage: <your-step-name>-done        # precheck-done | audit-done | blind-spots-done | editor-done | tpr-done
    next_step: <next-step-number>       # 3 after Step 2, 4 after Step 3, 5 after Step 4, 6 after Step 5, 7 after Step 6
    updated: <today's date YYYY-MM-DD>

If the block does not yet exist in frontmatter, insert it above the
`sections:` field (preserving the YAML document's field ordering). If it
exists (from a prior step), replace the block wholesale with the new
values — do NOT merge or preserve prior fields. In whole-plan mode
(target_section == null), write to <plan_dir>/.review-pipeline-state.yaml
instead (dotfile owned by the plan; same schema).

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
| 5 | `5-editor` | `step-5-editor.md` | `opus` | `$RUN_DIR/editor.json` |
| 7+8 | `7-8-verify` | `step-7-8-verify.md` | `sonnet` | `$RUN_DIR/verify.json` |

Steps 4 and 6 are NOT in this table — they run inline in main context. Run Step 4 after Step 3 returns and before dispatching Step 5. Run Step 6 after Step 5 returns and before dispatching Step 7+8. See `## Step 4 — /tp-help Blind-spots (inline, main context)` and `## Step 6 — /tpr-review Convergence (inline, main context)` below.

## Step 4 — /tp-help Blind-spots (inline, main context)

After Step 3 returns and its handoff is clean, run Step 4 inline in main context. Do NOT wrap this in an `Agent({})` sub-agent.

**Why inline:** `/tp-help` dispatches Codex + Gemini CLIs concurrently; each reviewer runs 20–45 minutes wall-clock and streams partial output mid-run. When Step 4 was wrapped in a Sonnet sub-agent (the prior design), the sub-agent exited prematurely before the reviewers completed — its monitoring loops and the Skill-tool synchronous-wait semantics are invisible across the `Agent({})` boundary (same class of failure that moved Step 6 inline on 2026-04-17). Running inline lets the main context hold the `/tp-help` Skill invocation open until both reviewers return, then synthesize findings using the parent's Opus context (richer blind-spot distillation than Sonnet would produce).

1. Read `.claude/skills/review-plan/step-4-blind-spots.md` and follow it end-to-end inline.
2. Invoke `/tp-help` via the Skill tool with a prompt containing: the plan's mission (from `{plan_dir}/00-overview.md`), the section list with goals/statuses, the scope (crates/subsystems), review mode (single-section vs whole-plan), and the three specific questions listed in `step-4-blind-spots.md §Dispatch /tp-help`. In single-section mode, include the target section body.
3. Wait for `/tp-help` to return (synchronous in-context — the Skill invocation blocks until both reviewers complete).
4. Distill the concatenated reviewer output per `step-4-blind-spots.md §Distill the response` — bounded bullet lists (≤10 blind spots, ≤5 architectural risks, ≤5 cross-cutting concerns), each bullet ≤200 chars and anchored to a specific file/section/risk.
5. Write `$RUN_DIR/blind-spots.json` per the Output schema in `step-4-blind-spots.md`. Always `escalate: false` — this phase is advisory and never escalates.
6. **Update the frontmatter marker** per §Step 1f: set `stage: blind-spots-done`, `next_step: 5`, `updated: <today>` on the target section (single-section) or `<plan_dir>/.review-pipeline-state.yaml` (whole-plan). MANDATORY — without this the pipeline silently restarts on /clear + /continue-roadmap.
7. Proceed to Step 5 dispatch.

## Step 6 — /tpr-review Convergence (inline, main context)

After Step 5 returns and its escalation (if any) is resolved, run Step 6 inline in main context. Do NOT wrap this in an `Agent({})` sub-agent.

1. Read `.claude/skills/review-plan/step-6-tpr.md` and follow it end-to-end inline.
2. Invoke `/tpr-review` via the Skill tool with `--skill review-plan` plus the scope (`{target_section}` in single-section mode, `{plan_dir}` in whole-plan mode). **On resume entry** (Step 6 invoked with `rounds_completed > 0` in the marker): ALSO pass `--resume-from-rounds=<rounds_completed>` so `/tpr-review` initializes `iteration_counter = rounds_completed`. First dispatched round becomes Round `<rounds_completed>`, not Round 0. Flag syntax per `/tpr-review SKILL.md §1` composable flags.
3. When `/tpr-review` returns, observe its terminal `exit_reason` (Step 6 runs inline; no file handoff is required from `/tpr-review`) and write `$RUN_DIR/tpr.json` per the branch schemas in `step-6-tpr.md`. The current `status` set — defined by `/tpr-review/SKILL.md §5`'s `exit_reason` values — is `clean` / `iter_cap_reached` / `meta_cap_reached` / `user_accepted` / `escalated` / `both_reviewer_failure`.
4. **Update the `review_pipeline` marker** per §Step 1d. MANDATORY on EVERY exit_reason. Wholesale-replace the block. In whole-plan mode, write to `<plan_dir>/.review-pipeline-state.yaml`.

   Read rounds state from `/tpr-review`'s exit surface: `iteration_counter` (→ `rounds_completed`), last `commit_sha` from `fix_and_commit` (→ `last_round_commit`), last-round `len(verified)` (→ `last_round_findings`). `/tpr-review §5` MUST surface these in its main-context return.

   Branch by `exit_reason`:

   | `exit_reason` | `stage` | `next_step` | Extra fields |
   |---|---|---|---|
   | `clean` | `tpr-done` | `7` | `rounds_completed: <N>`, `updated: <today>` |
   | `user_accepted_at_*` / `autonomous_accept_at_*` | `tpr-done` | `7` | `rounds_completed: <N>`, `note: "<exit_reason>"`, `updated: <today>` |
   | `user_pause_and_resume` | `editor-done` (Step 6 still pending) | `6` | `rounds_completed: <N>`, `last_round_commit: <sha>`, `last_round_findings: <N>`, `note: "paused mid-loop"`, `updated: <today>` |
   | `iter_cap_reached` / `meta_cap_reached` / `both_reviewer_failure` / `escalated_to_plan_at_*` / `autonomous_exit_substantive_at_*` / `autonomous_transport_failure` | `editor-done` (stays at Step 6) | `6` | `rounds_completed: <N>`, `last_round_commit: <sha>`, `last_round_findings: <N>`, `note: "<exit_reason>"`, `updated: <today>` |

   Resume flag handoff is specified in step 2 above (`--resume-from-rounds=<rounds_completed>`).
5. Apply the same escalation handling described in the next section, reading `$RUN_DIR/tpr.json` as if it came from a sub-agent.
6. Proceed to Step 7+8 once the escalation resolves.

## Escalation handling (MANDATORY)

After each Agent returns (Steps 2, 3, 5, 7+8) or after Step 4 / Step 6 writes its inline handoff, read `$RUN_DIR/{step}.json`. If `"escalate": true`:

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

1. **Sequential phases** — each step completes before the next starts (Steps 2, 3, 5, 7+8 are Agents; Steps 4 and 6 are inline). Handoffs flow via `$RUN_DIR/*.json` files (the orchestrator-owned scratch dir created in Step 1).
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
- `step-4-blind-spots.md` — Step 4 protocol (/tp-help dispatch + distill — read and executed inline by the parent, NOT dispatched as an Agent).
- `step-5-editor.md` — Step 5 protocol (4-lens editor).
- `step-6-tpr.md` — Step 6 protocol (/tpr-review convergence loop — read and executed inline by the parent, NOT dispatched as an Agent).
- `step-7-8-verify.md` — Steps 7+8 protocol (reviewed flip + audit verify loop).

None of `step-*.md` files are registered as skills. They are reference documents: `step-2-precheck.md`, `step-3-audit.md`, `step-5-editor.md`, and `step-7-8-verify.md` are read by dispatched Agents; `step-4-blind-spots.md` and `step-6-tpr.md` are read by the parent inline in main context.
