# Step 6 — /tpr-review convergence loop

Read and executed inline by the `/review-plan` parent orchestrator in main context. NOT dispatched as an `Agent({})` sub-agent. Not a registered skill.

## Input

The parent already has `RUN_DIR`, `mode`, `plan_dir`, and `target_section` in its own state from Step 1. Read `{RUN_DIR}/context.json` only if the values are not already in scope.

## Dispatch /tpr-review

Use the Skill tool with `--skill review-plan` so that `/tpr-review`:

- Uses `review-plan` activation preambles (codex: `Run the /review-plan skill in envelope-only mode.`; gemini: `Activate the review-plan skill and follow its instructions exactly.`)
- Passes `--skill review-plan` to the transport (correct `round.log` attribution)
- Launches Codex and Gemini in parallel using the `review-plan` reviewer skill
- Merges findings from both reviewers
- Fixes actionable findings directly
- Re-runs until both reviewers return zero actionable findings, OR a cap fires (see `.claude/skills/tpr-review/SKILL.md §5` for the current caps: `iteration_counter` cap of 3 rounds, `meta_only_streak` cap of 2 rounds)

```
Skill: tpr-review
Args: --skill review-plan
```

In single-section mode, scope the review to `{target_section}` (pass as the path arg to `/tpr-review`). In whole-plan mode, pass `{plan_dir}`.

**Wait for `/tpr-review` to complete fully.** Do not return partial results.

**No double-prompt.** When invoked via `--skill review-plan`, `/tpr-review`'s §5 terminal cap-exit `AskUserQuestion` is suppressed — the outer `/review-plan` parent owns the escalation UI per this file + `review-plan/SKILL.md §Escalation handling`. `/tpr-review` exits with its terminal `exit_reason` observable to the main-context orchestrator; this file translates that state into `{RUN_DIR}/tpr.json` with the appropriate escalation payload.

## Parse /tpr-review output

`/tpr-review` does not write a separate report file. Because Step 6 runs inline in main context, the orchestrator observes `/tpr-review`'s terminal state directly: the `exit_reason` string set at §5's terminal branch, the round count, the accumulated `ever_verified_findings` list, and (if survivor-mode fired) the `survivor_mode` flag per round. Capture these into `{RUN_DIR}/tpr.json` per the schema below.

Authoritative reference for the terminal states: `.claude/skills/tpr-review/SKILL.md §5` (the round-loop pseudocode's `exit_reason` assignments + the semantics table below its options block).

### Exit-reason classification

`/tpr-review`'s `exit_reason` maps into a `status` field for the `tpr.json` handoff:

| `/tpr-review` `exit_reason` | `tpr.json` `status` | `converged` | `user_accepted` | `escalate` |
|---|---|---|---|---|
| `"clean"` | `"clean"` | `true` | `false` | `false` |
| `"iter_cap_reached"` | `"iter_cap_reached"` | `false` | `false` | `true` |
| `"meta_cap_reached"` | `"meta_cap_reached"` | `false` | `false` | `true` |
| `"user_accepted_at_iter_cap_reached"` | `"user_accepted"` | `false` | `true` | `false` |
| `"user_accepted_at_meta_cap_reached"` | `"user_accepted"` | `false` | `true` | `false` |
| `"escalated_to_plan_at_iter_cap_reached"` | `"escalated"` | `false` | `false` | `false` |
| `"escalated_to_plan_at_meta_cap_reached"` | `"escalated"` | `false` | `false` | `false` |
| `"both_reviewer_failure"` (or equivalent from §9 escalation) | `"both_reviewer_failure"` | `false` | `false` | `true` |
| `"autonomous_accept_at_iter_cap_reached"` | `"user_accepted"` | `false` | `true` | `false` |
| `"autonomous_accept_at_meta_cap_reached"` | `"user_accepted"` | `false` | `true` | `false` |
| `"autonomous_exit_substantive_at_iter_cap_reached"` | `"escalated"` | `false` | `false` | `true` |
| `"autonomous_exit_substantive_at_meta_cap_reached"` | `"escalated"` | `false` | `false` | `true` |
| `"autonomous_transport_failure"` | `"both_reviewer_failure"` | `false` | `false` | `true` |
| `"autonomous_spec_gate_violation"` | `"escalated"` | `false` | `false` | `true` |
| `"autonomous_ambiguous_input"` | `"escalated"` | `false` | `false` | `true` |

Notes on the table:

- `user_accepted_at_*` and `escalated_to_plan_at_*` mean the user ALREADY made a choice inside `/tpr-review`'s §5 cap-exit `AskUserQuestion` — so Step 6 does NOT re-prompt. `escalate: false` in both cases because the user decision has landed.
- When `/tpr-review` is invoked via `--skill review-plan`, its §5 cap-exit prompt is suppressed, so `user_accepted_at_*` and `escalated_to_plan_at_*` do NOT appear directly here — instead `/tpr-review` exits with `exit_reason = iter_cap_reached | meta_cap_reached` and Step 6 issues the escalation below (Branch 2). If the user then selects an `applies_user_accepted: true` option, `review-plan/SKILL.md §Escalation handling` PATCHES this handoff to `user_accepted: true` (see Invariants below).
- `both_reviewer_failure` is raised when `/tpr-review` §9 ran its own `AskUserQuestion` for both-reviewer-failure-twice AND the user chose either `abort` or `retry-once-more` that failed again. If the user chose `pause-and-resume` instead, `/tpr-review` exits cleanly with no `tpr.json` write needed from this step — the parent handles pipeline shutdown.

### Fields captured from `/tpr-review`'s terminal state

- `exit_reason` — verbatim string from `/tpr-review` §5.
- `iterations` — value of `iteration_counter` at exit.
- `final_findings` — `ever_verified_findings` minus anything committed inline during the loop; remaining findings filed as `- [ ]` items in the plan's §NN.R block OR in the bug-tracker (see `/tpr-review §7`).
- `survivor_mode_rounds` — list of round indices where exactly one reviewer was lost and the survivor's report was used solo.

## Output

Write `{RUN_DIR}/tpr.json`. Exactly one of the four branches below applies, keyed by `status`. The top-level schema is the same; only the `escalate` / `question` / `options` fields differ.

### Branch 1 — `status: "clean"` (no escalation)

```json
{
  "status": "clean",
  "exit_reason": "clean",
  "iterations": 2,
  "converged": true,
  "user_accepted": false,
  "final_findings": [],
  "survivor_mode_rounds": [],
  "summary": "Phase 4: clean on iteration 2 (both reviewers returned zero actionable findings)",
  "escalate": false
}
```

### Branch 2 — `status: "iter_cap_reached"` or `"meta_cap_reached"` (cap fired, no user decision yet)

This branch fires when `/tpr-review` hits its 3-iteration cap OR its 2-round meta-only-streak cap AND was invoked via `--skill review-plan` (so its own §5 cap-exit `AskUserQuestion` was suppressed). The parent re-emits the cap-exit prompt here so `/review-plan` owns the escalation UI.

```json
{
  "status": "iter_cap_reached",
  "exit_reason": "iter_cap_reached",
  "iterations": 3,
  "converged": false,
  "user_accepted": false,
  "final_findings": [/* ...remaining findings filed as - [ ] items in the plan's §NN.R block... */],
  "survivor_mode_rounds": [],
  "summary": "Phase 4: iteration cap (3 rounds) reached with 2 findings remaining",
  "escalate": true,
  "question": "/tpr-review reached its 3-round iteration cap with 2 findings still open. How do you want to proceed?",
  "options": [
    {"key": "accept-with-findings",
     "label": "Accept remaining findings, flip reviewed: true with cap-exit note (Recommended)",
     "description": "Recommended because at the iteration cap with findings still remaining the marginal value of more rounds is low — findings stay filed as - [ ] items under §NN.R and the plan's own completion gates own them (this is NOT deferral per /tpr-review §7). This is the canonical cap-exit path that unblocks downstream work without losing the audit trail. Key matches /tpr-review §5's accept-with-findings for consistency across the two prompts.",
     "applies_user_accepted": true,
     "recommended": true},
    {"key": "retry-with-hints",
     "label": "Retry /tpr-review with user-provided hints (extend cap)",
     "description": "Spend another full review cycle (up to 3 more rounds). Pick only if you can articulate a concrete hint that would change reviewer behavior (e.g. a missed code path); otherwise the retry is likely to re-converge at the same findings."},
    {"key": "escalate-to-plan",
     "label": "Escalate remaining findings to /create-plan (reviewed stays false)",
     "description": "Open a new plan that owns the remaining findings. Best when findings are structural/architectural rather than local fixes the editor can resolve."},
    {"key": "abort",
     "label": "Abort review — findings need manual attention; reviewed stays false",
     "description": "Exits entirely with no follow-up anchor. Least-preferred — equivalent to silent deferral."}
  ]
}
```

For `status: "meta_cap_reached"` the same option set applies — only the `summary` / `question` wording changes to reflect that the last 2 rounds produced only meta-findings (wording / spelling / line-number drift) rather than substantive ones. The `accept-with-findings` option remains (Recommended) because meta-only caps are even stronger signal that more rounds won't help.

### Branch 3 — `status: "user_accepted"` or `"escalated"` (user already decided inside `/tpr-review`)

This branch fires only when `/tpr-review` was invoked standalone (NOT via `--skill review-plan`) — rare from Step 6, but legal if a caller layered direct invocation inside Step 6. The user already answered §5's cap-exit prompt; Step 6 just records the outcome.

```json
{
  "status": "user_accepted",
  "exit_reason": "user_accepted_at_iter_cap_reached",
  "iterations": 3,
  "converged": false,
  "user_accepted": true,
  "user_accepted_option_key": "accept-with-findings",
  "final_findings": [/* ...remaining findings filed as - [ ] items... */],
  "survivor_mode_rounds": [],
  "summary": "Phase 4: user-accepted at iter_cap_reached after 3 rounds; 2 findings filed as - [ ] items",
  "escalate": false
}
```

For `status: "escalated"` (`exit_reason: "escalated_to_plan_at_*"`), same shape with `user_accepted: false` and a `next_plan` field naming the plan the parent's escalation handler should create via `/create-plan`.

### Branch 4 — `status: "both_reviewer_failure"` (infra failure after §9 retries + user chose abort/retry-failed)

This branch fires when `/tpr-review` §9's `AskUserQuestion` for "both reviewers failed twice" surfaced, the user chose `retry-once-more` and the retry failed again (or chose `abort`). In either case `/tpr-review` exits without a review; Step 6 records the failure so the parent can decide next steps.

```json
{
  "status": "both_reviewer_failure",
  "exit_reason": "both_reviewer_failure",
  "iterations": 0,
  "converged": false,
  "user_accepted": false,
  "final_findings": [],
  "survivor_mode_rounds": [],
  "summary": "Phase 4: aborted after both-reviewer failure (user chose retry-once-more which also failed, or chose abort)",
  "escalate": true,
  "failure_category": "<literal category from /tpr-review §9 — e.g. codex_cli_nonzero, gemini_429_capacity_exhausted, stdout_missing_sentinel>",
  "postmortem_dir": "<$scratch path from /tpr-review §8 so the operator can inspect codex-stdout.txt / gemini-stdout.txt / *-stderr.txt / *-report.txt>",
  "question": "/tpr-review aborted because both reviewers failed despite the §9 retry. The postmortem is preserved in the round's scratch dir. How do you want to proceed?",
  "options": [
    {"key": "triage-failure",
     "label": "Triage the failure — open the postmortem dir and inspect stdout / stderr / report files (Recommended)",
     "description": "Recommended because back-to-back dual-failures almost always indicate a real bug (parser regression, CLI contract violation, hook config drift) rather than a transient fault. Triaging first prevents the same failure on an immediate retry and feeds /improve-tooling.",
     "recommended": true},
    {"key": "retry-immediately",
     "label": "Retry /tpr-review immediately",
     "description": "Use sparingly — dual failures usually reflect real bugs that will reproduce on retry. Pick only if you have external evidence (e.g. remote CLI outage already resolved) that the failure was transient."},
    {"key": "abandon-review",
     "label": "Abandon this review — log the failure category in the plan's working notes",
     "description": "Exits without follow-up. Pick only if the failure is in a reviewer you can't run anyway (e.g. auth lapse you can't fix in this session); still file the failure category so it can be investigated later."}
  ]
}
```

### Invariants

- `question` and `options` MUST live INSIDE the JSON handoff object when `escalate: true` — the parent uses them as-is with `AskUserQuestion`. Never emit `options` as a sibling code block outside the handoff schema.
- `status` is MANDATORY — the parent's Step 6 consumer branches on it. An absent or unknown `status` is a contract violation by this step.
- When `status == "clean"`, `escalate` MUST be `false` and `question` / `options` MUST be absent.
- When `escalate == true` AND `status != "clean"`, `question` + `options` MUST be present.
- `exit_reason` MUST be the verbatim string from `/tpr-review §5` — never reword it. Downstream forensic tools read it for audit.
- **`applies_user_accepted` field on an option**: when present and `true`, signals the parent orchestrator that selecting this option means the user has explicitly accepted the non-converged state AND intends to flip `reviewed: true` downstream. The parent MUST patch `{RUN_DIR}/tpr.json` to set `user_accepted: true` AND `user_accepted_option_key: "<selected key>"` before resuming to Step 7+8 (see `review-plan/SKILL.md §Escalation handling` and `step-7-8-verify.md §Step 7`). Options without this field are not user-accept options — they signal retry / escalate-to-plan / triage-failure / abort (see the currently-emitted keys per `review-plan/SKILL.md §Escalation handling` dispatch table), and the parent handles them per that table.

## Do NOT

- Reimplement `/tpr-review` logic inline
- Add polling / foreground / background directives — `/tpr-review` manages its own transport
- Run `/tpr-review` without `--skill review-plan` (wrong reviewer preambles would load, and the §5 cap-exit prompt would fire, producing a double-prompt UX)
- Reference `.claude/skills/tpr-review/step-3-final-report.md` or `final-report.json` — both were deleted on 2026-04-16 (see `.claude/skills/improve-tooling/tpr-review-design.md §3`). The authoritative terminal-state reference is `.claude/skills/tpr-review/SKILL.md §5`.
