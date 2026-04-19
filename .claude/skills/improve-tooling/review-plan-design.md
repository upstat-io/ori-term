# /review-plan — Design Log

## Purpose + Context

`/review-plan <plan-path>` runs a 6-phase pipeline (Steps 2, 3, 4, 5, 6, 7+8) over a plan directory or a single section file. Its job is to pre-check effectiveness, audit structure, look for blind spots, apply editor changes, run `/tpr-review` to convergence, verify, and flip `reviewed: true` on a clean single-section pass. The parent orchestrator owns path normalization (Step 1), scratch-dir creation, Step 6 TPR convergence (inline), cross-plan invalidation (Step 8.5), and the final verdict (Step 9).

Canonical files: `.claude/skills/review-plan/SKILL.md` plus the six `step-*.md` protocol files.

## §1 Core Design Philosophy

1. **Parent-as-orchestrator, steps as protocols.** The parent SKILL.md is the dispatcher. Each `step-*.md` is a reference protocol — not a registered skill — describing exactly what one phase does. The parent chooses whether to dispatch that phase as an `Agent({})` sub-agent or run it inline, based on the phase's interactivity requirements.
2. **Sub-agents for pure read-compute-write, main context for interactive phases.** A phase that does file reads + JSON writes + deterministic compute can safely run as a sub-agent. A phase that itself invokes another interactive skill, streams output to the user, or needs to round-trip `AskUserQuestion` **must** run inline in main context. Crossing the `Agent({})` boundary drops mid-run user prompts on the floor.
3. **Scratch-dir handoffs.** Every phase writes `${RUN_DIR}/<step>.json` with a fixed schema. The parent reads it to branch, escalate, or proceed. Downstream phases read prior phases' JSON for context — never reconstruct from the plan file.
4. **Single writer at a time.** Steps are strictly sequential. Never parallel-dispatch review-plan phases; invalidation and file-state assumptions break.
5. **`reviewed: true` flip is terminal and gated.** Only Step 7+8 flips it, only in single-section mode, only after Step 6 converges (or the user explicitly accepts a cap-exit via `applies_user_accepted: true`).

## §2 Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|---|---|
| Step 6 runs **inline in main context**, not as an `Agent({})` sub-agent. | `/tpr-review` streams foreground output, runs multi-round loops with per-round AskUserQuestion cap-exits, and may itself spawn nested skills. When wrapped in a Sonnet sub-agent, all of that interactivity is invisible to the parent — the sub-agent either returns before the user can answer, or hangs waiting on input the harness never surfaces. Inline execution lets the main context handle `/tpr-review`'s native prompts directly. |
| Steps 2, 3, 4, 5, 7+8 run as **sub-agents**, not inline. | These are pure read-compute-write phases that produce substantial intermediate context (plan audits, blind-spot enumerations, editor reasoning traces). Running them inline pollutes the parent's context window before Step 9 verdict synthesis. Sub-agent wrapping keeps only the summary line visible to the parent. |
| Step 6's handoff file `${RUN_DIR}/tpr.json` must exist before Step 7+8 dispatches. | Step 7+8 reads `tpr.json` to know whether convergence happened and whether `user_accepted: true` was set. A missing file would force Step 7+8 to guess or re-run `/tpr-review`, either of which is a correctness hazard. |
| Step 6 inline invocation uses `/tpr-review --skill review-plan`. | Without `--skill review-plan`, /tpr-review loads default reviewer preambles, not the review-plan-specific ones. The round-log attribution is also wrong, which breaks post-hoc debugging of reviewer behavior. |
| `{RUN_DIR}/tpr.json` is captured inline by Step 6 from `/tpr-review`'s in-memory state (no file handoff from `/tpr-review` itself). Fields: `status` (one of `clean` / `iter_cap_reached` / `meta_cap_reached` / `user_accepted` / `escalated` / `both_reviewer_failure` — the autonomous exit_reasons `autonomous_accept_at_*`, `autonomous_exit_substantive_at_*`, `autonomous_transport_failure`, `autonomous_spec_gate_violation`, `autonomous_ambiguous_input` all collapse into these same six `status` buckets per `step-6-tpr.md`'s exit-reason classification table), `exit_reason`, `iterations`, `converged`, `user_accepted`, `final_findings`, `survivor_mode_rounds`. | Because Step 6 runs inline, `/tpr-review` does not need to persist a summary file — the orchestrator observes its state directly. If Step 6 is ever re-wrapped as a sub-agent (banned per the inline-execution invariant above), a file-based handoff would have to be re-introduced AND this row would need to document the wire format. The `status` bucket set is stable across interactive and autonomous modes; only the `exit_reason` string distinguishes them. |

## §3 File Inventory

| File | Lines | Role |
|---|---|---|
| `.claude/skills/review-plan/SKILL.md` | ~210 | Parent dispatcher. Steps 1, 8.5, 9 inline. Sub-agent dispatch table. Step 6 inline section. Escalation handling. Verdict template. |
| `.claude/skills/review-plan/step-2-precheck.md` | — | Effectively-complete section detection. Run as Opus sub-agent. |
| `.claude/skills/review-plan/step-3-audit.md` | — | plan-audit.py orchestration. Sonnet sub-agent. |
| `.claude/skills/review-plan/step-4-blind-spots.md` | — | /tp-help dispatch + distill. Sonnet sub-agent. |
| `.claude/skills/review-plan/step-5-editor.md` | — | 4-lens editor. Opus sub-agent. |
| `.claude/skills/review-plan/step-6-tpr.md` | ~180 | /tpr-review convergence loop. **Inline in main context, NOT a sub-agent.** |
| `.claude/skills/review-plan/step-7-8-verify.md` | — | reviewed-flip + audit verify loop. Sonnet sub-agent. |

## §4 Lessons from Dogfood / Production Runs

### 2026-04-17 — Step 6 Sonnet sub-agent could not orchestrate `/tpr-review` rounds

**Finding source:** User report during `/improve-tooling` session. "Sonnet isn't able to handle the orchestration. It needs to let the main context handle the rounds by simply calling /tpr-review skill."

**Symptom:** Step 6 was dispatched as a Sonnet sub-agent via `Agent({subagent_type: "general-purpose", model: "sonnet"})`. Inside that sub-agent, `/tpr-review` was invoked via the Skill tool. The sub-agent could not reliably drive `/tpr-review` through its 10-iteration convergence loop: mid-round cap-exit `AskUserQuestion` prompts became invisible, foreground streaming was swallowed, and the sub-agent tended to terminate early or produce malformed `tpr.json`.

**Root cause:** `Agent({})` boundaries are one-shot request/response. `/tpr-review` is not — it's an interactive multi-round transport with its own escalation points. Wrapping an interactive skill inside a non-interactive sub-agent context-mismatches the harness. The Sonnet model class is not the problem (Opus would have failed the same way); the architectural problem is the sub-agent wrapping itself.

**Fix:** Moved Step 6 to inline execution in main context. Parent reads `step-6-tpr.md` and invokes `/tpr-review` via the Skill tool directly, then writes `${RUN_DIR}/tpr.json` with the same branch schemas as before. Downstream Step 7+8 contract is unchanged — it still reads `tpr.json`.

**Invariant added to §2:** "Step 6 runs inline in main context, not as an `Agent({})` sub-agent."

## §5 Regressions To Watch For

- [ ] Somebody re-adds a Step 6 row to the dispatch table in `SKILL.md` (`| 6 | 6-tpr | step-6-tpr.md | sonnet | $RUN_DIR/tpr.json |`). This would re-introduce the sub-agent wrap that the 2026-04-17 fix removed.
- [ ] `step-6-tpr.md` preamble drifts back to "Read by a Sonnet sub-agent dispatched from `/review-plan`." The preamble must continue to state inline execution.
- [ ] The parent starts running Step 6's TPR loop without first writing `${RUN_DIR}/tpr.json`, leaving Step 7+8 to read a missing file.
- [ ] `/tpr-review` is invoked without `--skill review-plan`, leading to wrong reviewer preambles and wrong round-log attribution.
- [ ] Someone tries to parallel-dispatch Steps 3 and 4 (or any two phases) to save time. All steps are sequential — invalidation and file-state assumptions break otherwise.
- [ ] `step-6-tpr.md` references `.claude/skills/tpr-review/step-3-final-report.md` or `final-report.json`. Both were deleted on 2026-04-16 (see `tpr-review-design.md §3`). Authoritative terminal-state reference is `tpr-review/SKILL.md §5`.
- [ ] `step-6-tpr.md` status names include any of `max_iterations_reached`, `max_thoroughness_rejections_reached`, `transport_failure`, `thoroughness_reject_counter`. Current contract uses `clean` / `iter_cap_reached` / `meta_cap_reached` / `user_accepted` / `escalated` / `both_reviewer_failure`. Iteration cap is 3 (not 10). No thoroughness counter — use `meta_only_streak`.
- [ ] `SKILL.md §Critical Rules` item 2 says the `reviewed` flip requires "converges clean" without the `user_accepted == true` carve-out. This silently contradicts `step-7-8-verify.md §Step 7`'s actual flip condition (I15 regression).
- [ ] `SKILL.md §Escalation handling` branches on a narrow key list (`proceed`/`abort`/`leave-as-is`/`next_skill`/`applies_user_accepted`) WITHOUT a dispatch table covering every semantic category emitted by the step protocols. Unknown-key fallback MUST re-prompt — silent no-op is banned (it loses user selections). The current SKILL.md table lists 9 categories: user-accept-tpr-non-convergence, accept-minor, invoke-named-skill, escalate-to-plan, retry-current-step, triage-transport-failure, walk-ambiguous-cases, abort-like, unknown-key fallback.

## §6 Improvement Log

### Open items

_None currently tracked._

### Recently closed

- [x] 2026-04-17 — Move Step 6 (`/tpr-review` convergence) from Sonnet sub-agent to inline main-context execution. Updated `SKILL.md` (description, top paragraph, dispatch table, inline Step 6 section, escalation-handling preamble, Critical Rule #1, Files-in-this-skill footer) and `step-6-tpr.md` (preamble + Input section). Created this design log. Commit: pending.

## §7 How To Use This File In Future Sessions

Open this file before editing anything in `.claude/skills/review-plan/`. Check §2 for load-bearing invariants — if your change would violate one, STOP and re-examine. Check §5 for the regression checklist. If the change is approved, add a `- [x]` entry to §6 with today's date, a one-line description, and the commit sha. If a new real-use bug surfaces during a `/review-plan` run, add it as a `- [ ]` under §6 Open even if you can't fix it immediately — tracking is non-optional, fixing is best-effort.
