# Steps 7 + 8 — reviewed-flip + audit verify loop

Read by a Sonnet sub-agent dispatched from `/review-plan`. Not a registered skill.

## Input

The parent orchestrator passed the scratch-dir path as `{RUN_DIR}`. Read:

- `{RUN_DIR}/context.json` — `mode`, `plan_dir`, `target_section`
- `{RUN_DIR}/editor.json` — Step 5 editor handoff. The `escalate` field indicates whether the editor surfaced unresolvable human-judgment issues; if absent, treat as `false`.
- `{RUN_DIR}/tpr.json` — flip condition is `"converged": true` OR `"user_accepted": true`. The `user_accepted: true` flag is written by the parent orchestrator (see `review-plan/SKILL.md §Escalation handling` item 4) when the user explicitly selects an `applies_user_accepted: true` option at Step 6's cap-exit AskUserQuestion (`step-6-tpr.md` Branch 2 `accept-with-findings` — the key aligned with `/tpr-review §5` to keep the two prompts consistent). When neither is true, this sub-skill does NOT flip `reviewed: true` and reports the reason.

## Step 7 — Flip `reviewed` field (single-section mode ONLY)

Flip condition: `mode == "single-section"` AND `editor.escalate != true` AND (`tpr.converged == true` OR `tpr.user_accepted == true`).

If flip condition holds:

- Set `reviewed: true` in `{target_section}`'s frontmatter via the Edit tool.
- Record `reviewed_flipped: true` in the output.
- If the flip is by `tpr.user_accepted == true` (not `tpr.converged`), ALSO append a line to the section's `third_party_review.notes` frontmatter field recording the cap-exit reason + `user_accepted_option_key` value from the patched handoff JSON, e.g. `notes: "user-accepted via accept-with-findings at iter_cap_reached — 2 findings filed as - [ ] items in §NN.R"`. Status names come from `/tpr-review/SKILL.md §5`'s current `exit_reason` set (`iter_cap_reached` / `meta_cap_reached`). The audit trail lives in the plan file itself, not just `$RUN_DIR/*.json` (the orchestrator-owned scratch dir).

If `mode == "whole-plan"`: skip this step entirely. Never touch `reviewed` fields in whole-plan mode.

If `editor.escalate == true`: leave `reviewed: false` regardless of `tpr.*` — an editor-escalated plan has unresolved human-judgment issues that block flip even if TPR converged. Record `editor-escalated` as the reason.

If `tpr.converged == false` AND `tpr.user_accepted != true`: leave `reviewed: false` and record the reason as `tpr-non-convergence` (user declined the accept option, chose retry/abort/escalate-to-plan instead).

## Step 8 — Post-edit audit verify loop

Run plan-audit.py in verify mode until clean (max 5 iterations — escalate if it doesn't converge):

```bash
python3 .claude/skills/plan-audit/plan-audit.py {plan_dir} --verify --json \
  > "{RUN_DIR}/plan-audit-verify.json" 2>&1
```

Read results. If `critical > 0` OR `major > 0`:

1. Apply the fixes via Edit (trust plan-audit.py's recommendations — they're deterministic)
2. Re-run the audit
3. Repeat until clean (zero critical + zero major) OR 5 iterations reached

Commit any fixes via `Skill: commit-push` with message `chore(plans): post-review audit verify fixes`.

## Output

Write `{RUN_DIR}/verify.json`. On a clean converge:

```json
{
  "reviewed_flipped": true,
  "reviewed_flipped_section": "plans/foo/section-03.md",
  "reviewed_flipped_reason": "clean | user-accepted-tpr-non-convergence | tpr-non-convergence | whole-plan-mode | editor-escalated",
  "verify_iterations": 2,
  "verify_converged": true,
  "remaining_critical": 0,
  "remaining_major": 0,
  "remaining_minor": 2,
  "summary": "Step 7+8: reviewed flipped on section-03.md, verify converged in 2 iterations (0 critical, 0 major, 2 minor remaining)",
  "escalate": false
}
```

If the verify loop does not converge within 5 iterations, include `question` + `options` inside the handoff object so the parent can pass them verbatim to `AskUserQuestion`:

```json
{
  "reviewed_flipped": false,
  "reviewed_flipped_section": "plans/foo/section-03.md",
  "reviewed_flipped_reason": "verify-non-convergence",
  "verify_iterations": 5,
  "verify_converged": false,
  "remaining_critical": 0,
  "remaining_major": 3,
  "remaining_minor": 2,
  "summary": "Step 7+8: verify loop did NOT converge in 5 iterations (3 major, 2 minor remaining)",
  "escalate": true,
  "question": "Post-edit audit verify reached the 5-iteration cap with 3 major findings still open. How do you want to proceed?",
  "options": [
    {"key": "dispatch-editor-round-2",
     "label": "Run a second editor pass (Recommended)",
     "description": "Recommended because 3 major findings are too many to accept silently — a second editor pass on the same inputs typically clears structural issues the first pass missed. Cost is one more editor Agent round; benefit is converging without manual intervention.",
     "recommended": true},
    {"key": "accept-minor",
     "label": "Accept remaining minor findings and finish review",
     "description": "Pick only if the majors have been re-classified as non-blocking after re-reading. Default is to resolve majors before flipping reviewed: true."},
    {"key": "abort",
     "label": "Abort — findings need manual attention",
     "description": "Exits the review loop entirely; leaves reviewed: false with no follow-up anchor. Pick only if the findings surfaced plan-state that editor tooling cannot resolve."}
  ]
}
```

`question` and `options` MUST live inside the JSON handoff object when `escalate: true`. Never emit `options` as a sibling code block outside the handoff schema — the parent reads the fields from the JSON and passes them directly to `AskUserQuestion`.

## Step 8.5 — Clear the `review_pipeline` marker

**MANDATORY on ANY successful terminal exit** — clean converge, user-accepted, or whole-plan-mode (no-op flip). The marker's purpose is to record mid-pipeline state; once the pipeline reaches a terminal outcome, the marker MUST be removed so future `/review-plan` invocations on this section start fresh.

In single-section mode: `Edit` the section file to delete the entire `review_pipeline:` YAML block from the frontmatter.

In whole-plan mode: delete `<plan_dir>/.review-pipeline-state.yaml`.

On escalation (`escalate: true` return path with `verify-non-convergence`): DO NOT clear the marker. Leave it set to `stage: tpr-done, next_step: 7` so the escalation-handling path can resume cleanly — the user's `dispatch-editor-round-2` / `accept-minor` / `abort` choice determines whether the marker gets cleared later (on accept/abort) or re-advanced (on dispatch-editor-round-2 which returns to Step 5).

## Do NOT

- Flip `reviewed: true` in whole-plan mode (hard rule)
- Flip `reviewed: true` when tpr-review didn't converge AND the user did not explicitly accept the remaining findings via the `applies_user_accepted: true` option at Step 6's cap-exit prompt (hard rule — the user-accepted path is the ONLY non-convergence route to flip)
- Edit `.rs` / `.ori` files (plan-docs only)
- Exceed 5 verify iterations without escalating
- Leave the `review_pipeline` marker in place on a terminal exit (Step 8.5 above — prevents future /review-plan runs from falsely detecting a mid-pipeline state)
