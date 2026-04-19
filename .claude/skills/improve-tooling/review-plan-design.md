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
| Every pipeline state change (Step 2/3/4/5/6 complete) is persisted to the target section's `review_pipeline` frontmatter marker (or `<plan_dir>/.review-pipeline-state.yaml` in whole-plan mode) BEFORE the step yields. Step 7+8 on successful terminal exit removes the marker. | Without a persistent marker, the ONLY cross-session signal that /review-plan was ever invoked on a section is `reviewed: true` (for clean exits) — `reviewed: false` is indistinguishable from "never reviewed." `/clear` + `/continue-roadmap` therefore fires the `unreviewed_plan` gate on any paused mid-pipeline section and recommends "Run /review-plan now" — which restarts the pipeline from Step 2, re-dispatching Step 4's ~20–45 min /tp-help reviewer wall-clock needlessly. Observed twice on the same section (empty-container §04) on 2026-04-18 before this invariant landed. Transient scratch dirs in `/tmp/` do NOT count — they are (a) lost across reboots, (b) not discovered by `/continue-roadmap`, and (c) re-created via `mktemp -d` on every new invocation. The plan file is the single source of truth for pipeline state. |
| Steps **4 and 6** run **inline in main context**, not as `Agent({})` sub-agents. | Both invoke long-running, streaming, interactive-within-the-round Skill calls (`/tp-help` for Step 4, `/tpr-review` for Step 6). `/tp-help` dispatches Codex + Gemini CLIs concurrently for 20–45 minutes wall-clock and streams partial output; `/tpr-review` streams foreground output, runs multi-round loops with per-round AskUserQuestion cap-exits, and may itself spawn nested skills. When wrapped in a Sonnet sub-agent, all of that interactivity is invisible to the parent — the sub-agent returns early (observed twice for Step 4 on 2026-04-18 before this invariant covered Step 4), or hangs waiting on input the harness never surfaces. Inline execution lets the main context hold the Skill invocation open until both reviewers complete, and uses the parent's Opus context for synthesis (richer distillation than Sonnet). |
| Steps 2, 3, 5, 7+8 run as **sub-agents**, not inline. | These are pure read-compute-write phases that produce substantial intermediate context (plan audits, editor reasoning traces, verify-loop transcripts). Running them inline pollutes the parent's context window before Step 9 verdict synthesis. Sub-agent wrapping keeps only the summary line visible to the parent. Step 4 is the exception — its `/tp-help` call is intrinsically interactive/long-running and cannot tolerate sub-agent wrapping (see row above). |
| Step 6's handoff file `${RUN_DIR}/tpr.json` must exist before Step 7+8 dispatches. | Step 7+8 reads `tpr.json` to know whether convergence happened and whether `user_accepted: true` was set. A missing file would force Step 7+8 to guess or re-run `/tpr-review`, either of which is a correctness hazard. |
| Step 6 inline invocation uses `/tpr-review --skill review-plan`. | Without `--skill review-plan`, /tpr-review loads default reviewer preambles, not the review-plan-specific ones. The round-log attribution is also wrong, which breaks post-hoc debugging of reviewer behavior. |
| `{RUN_DIR}/tpr.json` is captured inline by Step 6 from `/tpr-review`'s in-memory state (no file handoff from `/tpr-review` itself). Fields: `status` (one of `clean` / `iter_cap_reached` / `meta_cap_reached` / `user_accepted` / `escalated` / `both_reviewer_failure` — the autonomous exit_reasons `autonomous_accept_at_*`, `autonomous_exit_substantive_at_*`, `autonomous_transport_failure`, `autonomous_spec_gate_violation`, `autonomous_ambiguous_input` all collapse into these same six `status` buckets per `step-6-tpr.md`'s exit-reason classification table), `exit_reason`, `iterations`, `converged`, `user_accepted`, `final_findings`, `survivor_mode_rounds`. | Because Step 6 runs inline, `/tpr-review` does not need to persist a summary file — the orchestrator observes its state directly. If Step 6 is ever re-wrapped as a sub-agent (banned per the inline-execution invariant above), a file-based handoff would have to be re-introduced AND this row would need to document the wire format. The `status` bucket set is stable across interactive and autonomous modes; only the `exit_reason` string distinguishes them. |

## §3 File Inventory

| File | Lines | Role |
|---|---|---|
| `.claude/skills/review-plan/SKILL.md` | ~210 | Parent dispatcher. Steps 1, 8.5, 9 inline. Sub-agent dispatch table. Step 6 inline section. Escalation handling. Verdict template. |
| `.claude/skills/review-plan/step-2-precheck.md` | — | Effectively-complete section detection. Run as Opus sub-agent. |
| `.claude/skills/review-plan/step-3-audit.md` | — | plan-audit.py orchestration. Sonnet sub-agent. |
| `.claude/skills/review-plan/step-4-blind-spots.md` | — | /tp-help dispatch + distill. **Inline in main context, NOT a sub-agent** (fixed 2026-04-18). |
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

### 2026-04-18 — /clear + /continue-roadmap silently restarts paused pipelines from Step 2 (marker absent)

**Finding source:** User report during live session. "This is a massive issue in the workflow because what you keep doing is telling me to pause and resume. I do that, but you never record anything about being on Step 4, so what happens is it starts the review process over from the beginning for this section. This has happened like 2 times now." And: "I /clear and then re-run /continue-roadmap, that's supposed to work but since you never update the plan it doesn't."

**Symptom:** /review-plan pauses mid-pipeline (e.g., via /tpr-review §9 context-pressure pause between Step 5 and Step 6). User clears context and re-runs /continue-roadmap from a fresh session. /continue-roadmap sees `reviewed: false` on the section, fires the `unreviewed_plan` gate, and surfaces "Run /review-plan now" as the recommended option. User selects it — /review-plan's Step 1 creates a fresh `mktemp -d` scratch dir and restarts from Step 2, re-dispatching Steps 2/3/4/5 including Step 4's ~20–45 min /tp-help reviewer wall-clock. User observed this failure mode on the same section (empty-container §04) at least twice before reporting it.

**Root cause:** /review-plan's pipeline state lived ONLY in an ephemeral `/tmp/review-plan-${repo}-XXXXXXXX` scratch dir created via `mktemp -d` per invocation. The plan file's frontmatter had no marker recording "paused at Step N." Cross-session recovery therefore had no signal to distinguish "never reviewed" from "mid-pipeline paused." /continue-roadmap's `roadmap_scan.py` read only `reviewed:` and `third_party_review.*` frontmatter fields; it had no knowledge of review-plan's internal pipeline state.

Secondary: user's initial attempt at a fix proposed a repo-local `.claude/state/review-plan/<slug>/` state dir alongside the marker. User pushed back ("what a stupid fucking fix? Wouldn't it make more sense to just update the plan with where you left off?") — correctly identifying that the state dir was redundant complexity. The plan file IS the state. Prior-step JSONs are only consumed by Step 5 (editor); Steps 6 and 7+8 read the plan directly. Simplification landed: no state dir, plan-marker-only.

**Fix:** Added a mandatory `review_pipeline:` frontmatter marker (or `<plan_dir>/.review-pipeline-state.yaml` in whole-plan mode) with fields `stage` / `next_step` / `updated` / optional `note`. Every step (2/3/4/5/6) updates the marker atomically with its handoff JSON write; Step 7+8 on clean exit removes the marker entirely. /review-plan Step 1 probes for the marker on entry and offers resume via `AskUserQuestion` when present. /continue-roadmap's `roadmap_scan.py` reads the marker in the `unreviewed_plan` gate and surfaces `"Resume /review-plan from Step N (Recommended)"` instead of `"Run /review-plan now"` when a marker is present. Scratch dir remains ephemeral `/tmp/` — it only holds transient Step 2→5 handoff payloads the editor consumes; resume-at-Step-6+ does not need the scratch dir at all.

**Invariants added to §2:** (1) "Every pipeline state change is persisted to the target section's `review_pipeline` frontmatter marker BEFORE the step yields." (2) `/continue-roadmap`'s scanner reads the marker and surfaces resume option when present.

**Files touched:** `.claude/skills/review-plan/SKILL.md` (Step 1a/b/c/d rewrite, sub-agent dispatch template marker-write instruction, Step 4/6 inline marker-write, Files footer), `.claude/skills/review-plan/step-7-8-verify.md` (new §Step 8.5 marker-clear on terminal exit), `.claude/skills/continue-roadmap/roadmap_scan.py` (gate 1.7 resume branch — reads `review_pipeline` dict from frontmatter, emits resume option), section 04's frontmatter (marker applied for the pending pause: `stage: editor-done, next_step: 6, updated: 2026-04-18`), plus design log updates.

### 2026-04-18 — Step 4 Sonnet sub-agent exited before `/tp-help` reviewers returned (same class of failure)

**Finding source:** User report during a live `/review-plan` session. First Step 4 Agent dispatch returned premature text ("Both monitoring loops are running. I'll wait for notification") without writing the handoff JSON; the parent re-dispatched and the second dispatch was interrupted for the same reason. User: "Blind spots should not be a sonnet wrapper calling /tpr-review, it should be inline just like how we handle /tpr-review and it should be calling the /tpr-review in help mode or whatever it's called for the blindspot analysis which is being done by Opus not Sonnet."

**Symptom:** Step 4 was dispatched as a Sonnet sub-agent via `Agent({subagent_type: "general-purpose", model: "sonnet"})`. Inside that sub-agent, `/tp-help` was invoked via the Skill tool. `/tp-help` spawns Codex + Gemini CLIs concurrently (20–45 min wall-clock each); the Sonnet sub-agent could not hold the Skill invocation open across that duration. The sub-agent exited with a bare "I'll wait for notification" message, leaving `blind-spots.json` unwritten and the pipeline stalled at Step 4.

**Root cause:** Identical to the 2026-04-17 Step 6 failure. `Agent({})` boundaries are one-shot request/response; the sub-agent's context is torn down when the model decides to return, not when child Skill calls complete. Any Skill invocation with wall-clock much larger than the sub-agent's own attention budget (roughly: minutes, not tens of minutes) fails inside a sub-agent wrap. `/tp-help` shares this shape with `/tpr-review` — both are long-running dual-source reviewer dispatchers. The fix for Step 6 should have been extended to Step 4 in the same change; it wasn't, leaving the regression latent until the next /review-plan run surfaced it.

**Fix:** Moved Step 4 to inline execution in main context. Parent reads `step-4-blind-spots.md` and invokes `/tp-help` via the Skill tool directly (Opus context for synthesis), then writes `${RUN_DIR}/blind-spots.json` with the same Output schema as before. Downstream Step 5 contract is unchanged — it still reads `blind-spots.json`. Updated: `SKILL.md` (frontmatter description, intro paragraph, section heading, dispatch-table row removal, new inline-Step-4 section, escalation-handling preamble, Critical Rule #1, Files-in-this-skill footer) and `step-4-blind-spots.md` (preamble + Input section).

**Invariant generalized in §2:** the previous row "Step 6 runs inline..." now reads "Steps 4 and 6 run inline...", covering the whole class of long-running interactive Skill wrappers. Future additions (e.g., a Step N that invokes another long-running dual-source reviewer) default to inline unless proven otherwise.

## §5 Regressions To Watch For

- [ ] Somebody re-adds a Step 4 row to the dispatch table in `SKILL.md` (`| 4 | 4-blind-spots | step-4-blind-spots.md | sonnet | $RUN_DIR/blind-spots.json |`). This would re-introduce the sub-agent wrap that the 2026-04-18 fix removed.
- [ ] `step-4-blind-spots.md` preamble drifts back to "Read by a Sonnet sub-agent dispatched from `/review-plan`." The preamble must continue to state inline execution.
- [ ] Somebody re-adds a Step 6 row to the dispatch table in `SKILL.md` (`| 6 | 6-tpr | step-6-tpr.md | sonnet | $RUN_DIR/tpr.json |`). This would re-introduce the sub-agent wrap that the 2026-04-17 fix removed.
- [ ] `step-6-tpr.md` preamble drifts back to "Read by a Sonnet sub-agent dispatched from `/review-plan`." The preamble must continue to state inline execution.
- [ ] A future Step N is added that wraps another long-running dual-source Skill (`/tpr-review`, `/tp-help`, `/review-work`, `/independent-review`) inside an `Agent({})` sub-agent. These MUST run inline per the generalized §2 invariant. Presence of any such step as a new dispatch-table row is the regression signal.
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

- [x] 2026-04-18 — **Persistent pipeline-state marker on the plan file.** Closes the /clear + /continue-roadmap silent-restart loop. Every pipeline step now writes `review_pipeline: {stage, next_step, updated}` to the section's frontmatter (or `<plan_dir>/.review-pipeline-state.yaml` in whole-plan mode) before yielding; Step 7+8 removes the marker on clean exit. /review-plan Step 1 probes the marker and offers resume via AskUserQuestion. /continue-roadmap's `roadmap_scan.py` gate 1.7 reads the marker and surfaces "Resume from Step N" instead of "Run /review-plan now" when present. No state dir — plan file is the SSOT. Applied to section 04's frontmatter for the current pause (stage: editor-done, next_step: 6). Commit: pending.
- [x] 2026-04-18 — Move Step 4 (`/tp-help` blind-spots) from Sonnet sub-agent to inline main-context execution. Same root cause as the 2026-04-17 Step 6 fix (long-running interactive Skill cannot survive `Agent({})` boundary). Surfaced live during a `/review-plan` run on `empty-container-typeck-phase-contract/section-04-codegen-assertions.md` when two consecutive Step 4 Sonnet sub-agents returned with "I'll wait for notification" before writing `blind-spots.json`. Updated `SKILL.md` (frontmatter description, top paragraph, section heading, dispatch-table row removal, new inline Step 4 section, escalation-handling preamble, Critical Rule #1, Files-in-this-skill footer), `step-4-blind-spots.md` (preamble + Input section), and this design log (§2 invariant generalized, §3 row updated, §4 new lesson, §5 new regression entries). Commit: pending.
- [x] 2026-04-17 — Move Step 6 (`/tpr-review` convergence) from Sonnet sub-agent to inline main-context execution. Updated `SKILL.md` (description, top paragraph, dispatch table, inline Step 6 section, escalation-handling preamble, Critical Rule #1, Files-in-this-skill footer) and `step-6-tpr.md` (preamble + Input section). Created this design log. Commit: pending.

## §7 How To Use This File In Future Sessions

Open this file before editing anything in `.claude/skills/review-plan/`. Check §2 for load-bearing invariants — if your change would violate one, STOP and re-examine. Check §5 for the regression checklist. If the change is approved, add a `- [x]` entry to §6 with today's date, a one-line description, and the commit sha. If a new real-use bug surfaces during a `/review-plan` run, add it as a `- [ ]` under §6 Open even if you can't fix it immediately — tracking is non-optional, fixing is best-effort.
