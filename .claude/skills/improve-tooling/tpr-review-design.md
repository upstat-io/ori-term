# `/tpr-review` + `/tp-help` — Design Notes and Improvement Log

**Purpose of this file.** This is the institutional memory for the 2026-04-16 rewrite of `/tpr-review` and `/tp-help`. It captures the **design philosophy** (so future edits don't regress the architecture), the **load-bearing invariants** (things you must NOT change without very good reason and a concrete plan), and a **running log of improvements and bugs** found during real use.

**Context.** On 2026-04-16 the dual-TPR surface was rewritten from ~12,800 lines (envelope/polling/merger/sub-agent-triage infrastructure) to ~850 lines using a main-context orchestrator + parallel-Agent dispatch pattern. See `plans/use-plan-tool-to-sprightly-cat.md` for the approved plan (held at `/home/eric/.claude/plans/use-plan-tool-to-sprightly-cat.md`).

**When to update this file.** Any time you encounter a bug, surprise, or improvement idea while running `/tpr-review`, `/tp-help`, or any of their consumers (`/fix-bug`, `/review-work`, `/review-plan`, `/roadmap-work`, `/rosetta-test`, `/continue-roadmap`). Add a `- [ ]` item under §Improvement Log. When you find design-violating behavior in the wild, add it to §Regressions To Watch For.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Main-context orchestrator.** `/tpr-review`'s round loop runs in the caller's main conversation. No `context: fork`. Every tool call, every finding, every edit is visible to the user in real time. The old system hid this behind envelope/polling/triage-sub-agent; the rewrite's whole point was visibility.

2. **Parallel Agent dispatch per reviewer.** Each round: ONE assistant message with TWO `Agent({subagent_type: "general-purpose", model: "sonnet", prompt: <filled tp_agent_prompt.md>})` calls. They run concurrently per https://code.claude.com/docs/en/sub-agents. Wall-clock = `max(codex, gemini)`. No per-tool completion callbacks (undocumented); treat results as batch-complete.

3. **Plain-text transport.** Reviewers emit a fenced `<<<TPR-REPORT … TPR-REPORT>>>` block at the end of their CLI output. The orchestrator parses it inline. No JSON envelope, no schema, no merger script, no polling, no background process.

4. **Grounding by reference, not by embedding.** The inner prompt passed to codex/gemini tells the CLI to `ls .claude/rules/*.md` and `cat` each one itself. Do NOT embed `CLAUDE.md` + 28 rule files into the prompt argv — that's ~700KB and blows past Linux `ARG_MAX` (~128KB) with `Argument list too long`.

5. **Verification-against-code discipline.** Every reviewer finding is a **hypothesis**, not a fact. The orchestrator (main context) reads the cited file, confirms the quote exists verbatim, and drops unverifiable findings before classification. Trust tier sets verification **depth**, not pass/fail:
   - Codex HIGH → spot-check (read ±20 lines of cited range)
   - Gemini LOWER → full-verify (read cited file in full, trace behavior end-to-end)

6. **One main-context edit path.** When actionable findings land, the orchestrator edits code directly. No hand-off to a fix sub-agent, no separate edit phase. Feedback loop stays tight and visible.

7. **No `@`-includes in the new skill files.** `@`-includes are undocumented per the Claude Code skills reference. The rewrite deliberately avoided them. Exception: `compose-intel-summary.md` is still `@`-included by 20 OTHER consumers — that's legitimate pre-existing usage outside the rewrite's scope.

8. **Per-invocation unique scratch dirs.** Use `mktemp -d -t <prefix>-XXXXXXXX` in reviewer sub-agents. Hard-coded `/tmp/...` subdirs COLLIDE across parallel Claude sessions (the user routinely runs concurrent sessions). This is load-bearing — see §Regressions To Watch For.

9. **CLAUDE.md-compliant timeouts.** `timeout: 2700000` (45 min) on the foreground Bash call to the reviewer CLI. The `.claude/hooks/block-banned-commands.sh` hook blocks any `timeout` under `1200000` ms on codex/gemini commands. Never set `600000` or lower.

10. **Always-on spec/grammar gate.** `/tpr-review` §3 runs in ALL modes (work / plan / custom-objective). The rule it enforces — `.claude/rules/spec.md §Enforcement` — has no mode exemption. The gate is cheap (one git diff + one ls) and gating it by mode creates a coverage hole that custom-objective reviews can drive through.

## §2 — Load-Bearing Invariants

Changing any of these without a concrete plan risks re-introducing the bugs the dogfood runs surfaced. Re-read §4 ("Lessons from the dogfood runs") before changing.

| # | Invariant | Why (which failure mode it prevents) |
|---|-----------|--------------------------------------|
| I1 | Per-invocation `mktemp -d -t "tpr-round-${repo}-XXXXXXXX"` scratch dirs (repo name = basename of git worktree root) | Parallel-session collision; user runs concurrent sessions routinely, across multiple repos. Without the `${repo}` prefix, `/tmp/tpr-round-*` listings mix scratch dirs from `ori_lang`, `warpkit`, `upstat`, etc., making triage of stranded-report recovery (I14) hard. With it, every dir self-identifies. |
| I2 | Foreground Bash, `timeout: 2700000` | Hook-blocked under 1,200,000 ms; `run_in_background` breaks the return-in-same-turn contract |
| I3 | NO embedded CLAUDE.md/rules in reviewer prompt | ARG_MAX (~128KB) causes `Argument list too long` on Linux |
| I4 | `<<<TPR-REPORT … TPR-REPORT>>>` sentinels exact | Orchestrator parser is sentinel-based; changing them breaks extraction |
| I5 | Single assistant message, two Agent calls | Parallel-concurrency pattern; multiple messages serialize them |
| I6 | Orchestrator inherits caller's model, not pinned | Callers use Opus for judgment; skill-level `model:` is undocumented binding |
| I7 | Sub-agents dispatched with `model: "sonnet"` explicitly | Documented Agent-tool override; keeps reviewer dispatch cheap |
| I8 | Spec gate runs in ALL modes (not mode-gated) | Custom-objective reviews can still touch spec; `spec.md §Enforcement` has no exemption |
| I9 | `ever_verified_findings` accumulates across all rounds for plan-mode frontmatter write | Last-round-only write loses earlier-round findings |
| I10 | Render round summary AFTER `fix_and_commit`, BEFORE state-branching | The render contract mandates `Fix commit: {sha}`; pre-fix render has no sha |
| I11 | Banned-response phrases enforced in both orchestrator and reviewer prompt | "pre-existing", "out of scope", etc. — reasoning out of findings is banned per CLAUDE.md §One Rule |
| I12 | Verify findings BEFORE meta-classification | Can't classify a finding as meta if its quote doesn't exist — that's confabulation, drop it |
| I13 | User-facing choice points use `AskUserQuestion`, never plain-text numbered options | Prose options look identical to round-summary narration; the harness's structured-choice UI only engages on `AskUserQuestion` — prose asks force the user to type and invite them to skip reading. User flagged this after post-Round-1 dogfood. |
| I14 | Orchestrator owns the scratch dir (created via `mktemp -d` in §8 step 8a, passed to sub-agent as `{SCRATCH_DIR}`); sub-agent tees CLI stdout to `$scratch/{REVIEWER}-stdout.txt` AND writes the extracted TPR-REPORT block to `$scratch/{REVIEWER}-report.txt` | Dual-path transport. Prevents stranded-report failure mode where a fully-completed CLI invocation produces a valid TPR-REPORT that never reaches the orchestrator because the sub-agent auto-backgrounded its Bash call and can't be resumed (SendMessage is not universally available). Orchestrator can recover from disk without retrying. Surfaced 2026-04-16 dogfood session via `/improve-tooling` on a custom-objective review that burned ~20 min on sub-agent CLI invocations producing no retrievable report. |
| I15 | Cap-exit terminal branch (iter_cap / meta_cap / max_iterations_reached / max_thoroughness_rejections_reached) MUST offer the user an explicit `accept-with-findings` / `accept-remaining` / `accept-best-effort` option via `AskUserQuestion` that (a) leaves findings filed as `- [ ]` items in §NN.R or bug-tracker and (b) flips `reviewed: true` (plan mode). The option MUST be labeled with the concrete side-effect ("flip reviewed: true with a note") — not vague phrases like "continue to verify". The parent consumer (`/review-plan` Escalation handling) MUST patch `/tmp/review-plan-tpr.json` with `user_accepted: true` before resuming to Step 7+8. Step 7+8's flip condition is `converged == true OR user_accepted == true` (plus the editor-escalate guard). | Prevents "dead-end" UX where a multi-round TPR with no consensus leaves the user with no way to mark the plan reviewed and continue — they had to manually edit frontmatter or abandon the review. User-surfaced 2026-04-17 via `/improve-tooling` feedback: "After a large amount of tpr reviews and no consensus you never give me the option to mark reviewed true and continue." The accept path is NOT deferral (findings stay tracked as `- [ ]` items; the plan's own completion gates own them) — see `tpr-review/SKILL.md §7` for the distinction from banned "dismissal" phrases. |

## §3 — File Inventory (canonical)

Active files (2026-04-16):

| Path | Lines (~) | Role |
|------|-----------|------|
| `.claude/skills/tpr-review/SKILL.md` | 300 | Main orchestrator contract (13 sections) |
| `.claude/skills/tpr-review/tp_agent_prompt.md` | 180 | Reviewer sub-agent prompt template |
| `.claude/skills/tp-help/SKILL.md` | 160 | Help orchestrator with parallel Agent dispatch |
| `.claude/skills/tp-help/tp_help_prompt.md` | 150 | Help sub-agent prompt template |
| `.claude/skills/review-work/SKILL.md` | 50 | Thin delegator to `/tpr-review` |
| `.claude/skills/query-intel/compose-intel-summary.md` | 280 | Intel-query SSOT (moved from `dual-tpr/`) — used by 20 OTHER consumers |

Deleted on 2026-04-16 (~62 files total):

- `.claude/skills/dual-tpr/` — entire tree (30 scripts, 21 fixtures, 7 docs, `__pycache__`)
- `.claude/skills/tpr-review/step-1-round-setup.md`, `step-2-round-triage.md`, `step-3-final-report.md`
- `.claude/skills/tp-help/workflow.md`

## §4 — Lessons from the Dogfood Runs (2026-04-16)

Three rounds of `/tpr-review` run against the rewrite itself. **14 real issues surfaced** across the rounds. Capture these so future changes don't re-introduce them.

### Round 0 findings (codex 6, gemini clean)

1. **Spec gate short-circuited to `AskUserQuestion`** instead of emitting a CRITICAL TPR finding through normal flow. Violated `spec.md §Enforcement`. Fixed: §3 now emits a synthetic finding.
2. **Round summary rendered BEFORE `fix_and_commit`**, so the mandatory `Fix commit: {sha}` field was always empty. Fixed: pseudocode reordered (fix → render → branch).
3. **Plan-mode frontmatter write** only passed the LAST round's `verified` list, losing earlier-round findings. Fixed: added `ever_verified_findings` accumulator.
4. **`/tp-help` §6 contradicted §7** — "Claude's interpretation" in §6 vs "No synthesis" in §7. Fixed: §6 rewritten as raw-plus-attribution-only.
5. **`compose-intel-summary.md` header said "24 consumers"** after the file was moved but the count was stale (actual: 20). Fixed: recounted + updated.
6. **`review-work/SKILL.md` said "12-section orchestrator contract"** but actual is 13. Fixed.

### Round 1 findings (codex 4, gemini 4, plus user-flagged critical)

7. **`/tmp/tpr-inner/` and `/tmp/tphelp-inner/` race across parallel Claude sessions** (user-flagged, critical). Fixed: `mktemp -d -t <prefix>-XXXXXXXX` per-invocation.
8. **`timeout: 600000` (10 min) violates CLAUDE.md REVIEW/AGENT TIMEOUTS** (forbids <1,200,000 ms). Both reviewers agreed. Fixed: raised to `2700000` (45 min). **Critical: would have been hook-blocked by `.claude/hooks/block-banned-commands.sh` — why the first dispatches "succeeded" anyway is still partially a mystery; the sub-agents may have auto-promoted to background. Future bug to watch for.**
9. **Spec gate skipped for custom-objective mode** — custom reviews that touch spec files bypassed the mandatory gate. Fixed: §3 now "runs in ALL modes".
10. **Stale consumer rosters at `compose-intel-summary.md:247` and `:109`** still referenced dropped consumers. Fixed: collapsed to single 20-consumer roster.

### Round 2 findings (codex 5, gemini clean)

11. **Explanatory prose still cited `/tmp/tpr-inner/` literal** after the mktemp fix. Fixed: reworded to "Shared `/tmp` subdirectories".
12. **Step 3 text still said "10-minute timeout"** in tp_help_prompt.md after the fix raised it to 45. Fixed.
13. **Example failure text said "Bash 600s timeout exceeded"** in tp_agent_prompt.md. Fixed.
14. **Step F registry still listed `/tp-help` as active SSOT consumer** after it dropped its `@`-include. Fixed: marked as dropped with historical note.

### Round 3 — Dogfood session 2026-04-16 (evening): stranded-report failure mode confirmed

User invoked `/tpr-review` in custom-objective mode to gap-audit `impl-hygiene.md` + `/impl-hygiene-review`. Both sub-agents were dispatched via parallel `Agent({})` calls with the standard `tp_agent_prompt.md` template. Observed symptoms:

- **codex sub-agent** (`a6fd5050aca24e099`, duration_ms ~634000 ≈ 10 min 34s): returned a final message stating "Since the bash command went to background, let me wait for its completion notification rather than polling." No `<<<TPR-REPORT>>>` block.
- **gemini sub-agent** (`ab3eb2cde3478f402`, duration_ms ~692000 ≈ 11 min 32s): hit `MODEL_CAPACITY_EXHAUSTED` 429 retries on `gemini-3.1-pro-preview`, then returned a similar "waiting for Bash task to complete" message. No `<<<TPR-REPORT>>>` block.
- **External monitor event `burm3n2xx`** independently observed `tpr-report-found-in-stdout` for the gemini sub-agent's output — confirming a valid report had reached stdout somewhere, just not the orchestrator's context.
- The scratch dirs (`/tmp/tpr-review-FTQKkl6a/` codex, `/tmp/tpr-review-tZWdyO7e/` gemini) contained only the inner prompts — no stdout, no report — because the old template had sub-agents run `mktemp -d` themselves and capture CLI output in-conversation without persisting to disk.

**Root cause**: sub-agents auto-backgrounded their internal Bash call despite the template's explicit "Foreground only / single Bash call with `timeout: 2700000`" directive (absolute rules #4). Once backgrounded, the sub-agent returned early with narration and the captured stdout was stranded in the paused sub-agent conversation. The orchestrator had no recovery path because:
1. `SendMessage` to resume the sub-agent is not universally available in the orchestrator's toolset.
2. The old scratch-dir pattern (sub-agent owns the dir) meant the orchestrator didn't know where the dir was, even if output had been written there.
3. The old Step 2 ran the CLI without `tee`, so even the sub-agent's own bash never persisted stdout to disk — there was nothing to recover.

**Fix (committed this session via `/improve-tooling fix`)**: dual-path transport. Orchestrator owns the scratch dir (`{SCRATCH_DIR}` placeholder, `mktemp -d` in §8 step 8a, BEFORE dispatch). Sub-agent tees CLI stdout to `$scratch/{REVIEWER}-stdout.txt` (live line-buffered) and writes the extracted TPR-REPORT block to `$scratch/{REVIEWER}-report.txt` in Step 3. Orchestrator §9 stranded-report recovery checks disk BEFORE declaring a reviewer failed. Inline return stays the primary path; disk is the backup — they agree when both succeed, and recovery works when only one does. Invariant I14 added. Regression line added to §5.

**Insight**: this failure mode was predicted in Round 1 F8's footnote ("the sub-agents may have auto-promoted to background. Future bug to watch for") and partially captured in the [p2] sub-agent status visibility open item. This session is its first confirmed recurrence. The stranded-report half of the [p2] item is now closed; the residual half (mid-stream visibility during long reviews) remains open as a separate concern.

### Dogfood session 2026-04-17 (morning): cap-exit dead-end UX confirmed

User surfaced via `/improve-tooling`: "After a large amount of tpr reviews and no consensus you never give me the option to mark reviewed true and continue." Investigation mapped the exact control flow:

- `/review-plan` Step 6 (`step-6-tpr.md` Branch 2 `max_iterations_reached` and Branch 3 `max_thoroughness_rejections_reached`) already emitted `accept-remaining` / `accept-best-effort` options inside the escalation schema, so the user's prompt HAS shown those options.
- However: the option labels were vague — "Accept remaining findings and continue to verify" / "Accept the last round as a best-effort clean pass (informed override)". Neither said anything about flipping `reviewed: true`, so the user couldn't tell what the options actually did.
- AND: `/review-plan` Step 7+8 (`step-7-8-verify.md`) hard-coded the flip condition as `tpr.converged == true`. When the user DID select "accept-remaining", the parent orchestrator "honored the choice" by proceeding to Step 7+8 — which then read `converged: false` from `/tmp/review-plan-tpr.json` and refused to flip `reviewed: true`. The user's selection was silently lost.
- Meanwhile, `/tpr-review`'s own §5 terminal branch for standalone (non-`/review-plan`) plan-mode invocations had NO accept option at all — the `else` clause just set `exit_reason = "meta_cap_reached" | "iter_cap_reached"` and exited. §11.5 item 5 described the "final-report escalation" should offer next-step choices via `AskUserQuestion`, but the §5 pseudocode never implemented the prompt.

**Fix (committed this session via `/improve-tooling` on three surfaces):**

1. **`/tpr-review` §5 terminal branch**: replaced the single-line cap-assignment with an explicit 4-option `AskUserQuestion` (`run-more` / `accept-with-findings` / `escalate-to-plan` / `abort`). Added a semantics table mapping each choice to `exit_reason`, `third_party_review.status`, and `reviewed:` flip behavior. Documented the no-double-prompt rule: when invoked via `/review-plan` Step 6, the outer parent owns the escalation and `/tpr-review` does not render this `AskUserQuestion`.
2. **`/tpr-review` §10**: added step 3a — when `exit_reason` starts with `user_accepted_at_`, the frontmatter write ALSO flips `reviewed: true` AND appends a `third_party_review.notes` line recording the cap type + round count.
3. **`/tpr-review` §11.5 item 5**: replaced the vague one-line description with the concrete 4-option spec.
4. **`/tpr-review` §7**: added a banned-phrase carve-out — `accept-with-findings` at cap exit is NOT deferral because findings remain as `- [ ]` items with concrete artifacts; the carve-out distinguishes it from "pre-existing" / "future improvement" dismissal patterns.
5. **`/review-plan` `step-6-tpr.md` Branch 2/3**: relabeled `accept-remaining` / `accept-best-effort` options to be explicit about the flip (no more "continue to verify" — says "flip reviewed: true with a note"). Added `applies_user_accepted: true` flag to the option schema. Added Branch 2 option `escalate-to-plan` (was missing). Updated Invariants section to document the flag semantic.
6. **`/review-plan/SKILL.md` §Escalation handling**: added item 4 — when the user picks an `applies_user_accepted: true` option, the parent PATCHES `/tmp/review-plan-tpr.json` to add `user_accepted: true` + `user_accepted_option_key` before resuming. This is the only way the downstream Step 7+8 can know the user explicitly accepted.
7. **`/review-plan/step-7-8-verify.md`**: flip condition is now `converged == true OR user_accepted == true`. `reviewed_flipped_reason` enum gained `user-accepted-tpr-non-convergence`. Added a sub-step — on user-accepted flip, also append a line to the section's `third_party_review.notes` for the audit trail. Updated the "Do NOT" list accordingly.

**Insight**: the old design was almost right — the options existed, the parent escalation machinery existed, but the flip condition and option labels were disconnected. The fix is plumbing between already-built pieces, not a new feature. The design-log invariant I15 now guards this end-to-end: "the accept path must be labeled explicitly, must be honored downstream, must flip reviewed:true, must record the audit trail in the plan file." Any future edit that weakens one link in that chain regresses the user-facing dead-end UX.

### Cross-round patterns (generalizable)

- **Architectural fixes land cleanly; doc drift lingers.** Round 0 fixed architecture (gemini clean). Round 1 caught the *next* architectural issues (gemini agreed on timeout/spec-gate). Round 2 found only doc-text drift (gemini clean again). The sequence of findings tracks the code's actual maturation.
- **Gemini LOWER-trust is worth it.** Gemini was clean 2/3 rounds, which is a strong convergence signal rather than a weakness. When both reviewers agree on a finding, it's almost certainly a real architectural issue. When only codex flags, it's usually doc consistency.
- **Trust-tier aware verification is load-bearing.** Codex cited accurate line numbers in 100% of findings across 3 rounds. Gemini's citations (when it had any) also matched — but the LOWER-trust discipline of "re-read file in full" caught drift in the round-1 timeout issue that codex's spot-check alone would have missed on a different codebase.
- **Doc-string drift is the long tail.** After the architectural fixes, the remaining findings (rounds 2) were all doc text that referenced the OLD behavior (10-min timeout, old `/tmp` path). When fixing an architectural invariant, grep the ENTIRE file for references to the old value and update them all at once.

## §5 — Regressions To Watch For

When editing `tp_agent_prompt.md`, `tp_help_prompt.md`, `tpr-review/SKILL.md`, or `tp-help/SKILL.md`, check for these in order:

- `- [ ]` Any `/tmp/tpr-inner/` or `/tmp/tphelp-inner/` hard-coded literal → should be `$(mktemp -d -t ...-XXXXXXXX)`
- `- [ ]` Any `timeout: 600000` (or <1,200,000) → should be `2700000`
- `- [ ]` Any "10-minute timeout" / "10 min" in prose → should be "45-minute" / "45 min"
- `- [ ]` Any "Bash 600s" / "600s timeout" in example text → should be "2700s" / "45 min"
- `- [ ]` Any `cat CLAUDE.md >> prompt.md` or loop over rule files embedding content → DELETE; tell CLI to read them itself
- `- [ ]` Any `run_in_background: true` on the reviewer Bash call → REMOVE; must be foreground
- `- [ ]` Any "context: fork" in `tpr-review/SKILL.md` frontmatter → REMOVE; orchestrator runs in main context
- `- [ ]` Any `@.claude/skills/dual-tpr/...` reference anywhere → path is dead; the whole directory was deleted
- `- [ ]` Any `@.claude/skills/` in the new tpr-review or tp-help files → the new files deliberately have NO `@`-includes; inline the policy
- `- [ ]` Any `model:` in `tpr-review/SKILL.md` or `tp-help/SKILL.md` frontmatter → REMOVE; skill-level binding is undocumented; sub-agents use Agent-tool `model: "sonnet"` instead
- `- [ ]` Any render-before-fix ordering in `§5 round loop pseudocode` → fix must come first (Round 0 F2)
- `- [ ]` Any "work mode and plan mode only" on the spec gate → should be "runs in ALL modes" (Round 1 F9)
- `- [ ]` Any CLI invocation in `tp_agent_prompt.md` Step 2 without `| tee "$RUN/{REVIEWER}-stdout.txt"` → stdout persistence is mandatory; a CLI call without `tee` breaks dual-path transport (I14) and re-opens the stranded-report failure mode (Round 3)
- `- [ ]` Any `RUN="$(mktemp -d ...)"` in `tp_agent_prompt.md` Step 1 → the orchestrator owns the scratch dir via `{SCRATCH_DIR}`; sub-agent must use `RUN="{SCRATCH_DIR}"` without its own `mktemp` (I14, Round 3)
- `- [ ]` Any missing Step-3 `sed` extraction to `$RUN/{REVIEWER}-report.txt` in `tp_agent_prompt.md` → the orchestrator's §9 stranded-report recovery reads this file; a sub-agent that doesn't write it leaves the orchestrator unable to recover (I14, Round 3)
- `- [ ]` Any `tpr-review/SKILL.md §8` that dispatches Agents WITHOUT a preceding `mktemp -d -t "tpr-round-${repo}-XXXXXXXX"` step → orchestrator needs to own the scratch-dir paths before dispatch (I14, Round 3), AND the `${repo}` component (basename of git worktree root) must be present so multi-repo parallel sessions remain distinguishable in `/tmp/` listings (I1, Round 3 user-surfaced refinement). A prefix without `${repo}` regresses I1's multi-repo visibility rationale.
- `- [ ]` Any cap-exit terminal branch in `tpr-review/SKILL.md §5` (iter_cap / meta_cap) OR `review-plan/step-6-tpr.md` Branch 2/3 (max_iterations_reached / max_thoroughness_rejections_reached) WITHOUT an explicit `accept-with-findings` / `accept-remaining` / `accept-best-effort` option → I15 regression; the user has no escape hatch after a long non-converging TPR and must manually edit frontmatter to unblock.
- `- [ ]` Any cap-exit accept option labeled vaguely (e.g. "Accept remaining findings and continue to verify", "informed override", "proceed") WITHOUT explicit "flip reviewed: true" in the label → I15 regression on the clarity half; the option exists but the user can't tell what it does, so they don't pick it.
- `- [ ]` Any `step-6-tpr.md` accept option missing `applies_user_accepted: true` → the parent orchestrator has no way to know it should patch `/tmp/review-plan-tpr.json` with `user_accepted: true`, and Step 7+8 will refuse the flip (I15 regression on the plumbing half).
- `- [ ]` Any `review-plan/SKILL.md §Escalation handling` that drops the "patch handoff JSON on `applies_user_accepted: true`" step → Step 7+8's flip condition will never see `user_accepted: true` and the accept path will silently fail (I15 regression).
- `- [ ]` Any `step-7-8-verify.md` flip condition that hardcodes `tpr.converged == true` alone (without the `OR user_accepted == true` branch) → the user's accept selection is silently ignored (I15 regression — the original user-reported bug).
- `- [ ]` Any `step-7-8-verify.md` Do-NOT list item that says "Flip `reviewed: true` when tpr-review didn't converge (hard rule)" WITHOUT the user-accept carve-out → reads as banning the accept path entirely (I15 regression on the contract documentation).
- `- [ ]` Any `tpr-review/SKILL.md §10` that doesn't flip `reviewed: true` on `user_accepted_at_*` exit_reason for standalone plan-mode invocations → I15 regression on the `/tpr-review` standalone path (direct invocation without `/review-plan`).

## §6 — Improvement Log

Add items here as you encounter them in real use. Format: `- [ ] [priority] <description>. Context: <where/when you hit it>. Suggested fix: <one sentence>.`

Priorities: `p0` (blocks all future runs), `p1` (frequent user-visible bug), `p2` (nice-to-have), `p3` (cosmetic).

### Open items

- [ ] **[p1] Finish skill cleanup: remove remaining timeframe + model-editorial + doc-URL references from skill files.** Context: user flagged on 2026-04-16 that timeframes and model-policy commentary in skills bias judgment toward skipping process, and doc-URL citations ("per https://code.claude.com/docs/...") + "is undocumented per docs" rationale are meta-documentation that muddies instructions. First pass cleaned the most visible surfaces (all skill `description` frontmatter fields that show in the skills list; `/tpr-review`, `/tp-help`, `/verify-tpr`, `/fix-bug` runtime-expectation lines; `/impl-hygiene-review`, `/roadmap-work`, `/review-plan` descriptions + top sections; both prompt templates rewritten clean). Remaining work: (a) `fix-bug/SKILL.md` body paragraphs still carry "(Sonnet)"/"(Opus)" phase labels at ~lines 25, 27, 33, 114, 116, 120, 464, 465; (b) `fix-next-bug/SKILL.md` line 23 "entirely in the parent (Opus) context"; (c) `create-plan/SKILL.md` lines 66-97 contain a whole model-policy assignment table that is essentially editorial; (d) `improve-tooling/SKILL.md` has "10 minutes" heuristics at lines ~263, 296, 343; (e) `impl-hygiene-review/SKILL.md` body has "(Sonnet)"/"(Opus)" phase labels beyond the description; (f) `roadmap-work/SKILL.md` body has "Opus"/"Sonnet" commentary at lines 140, 141, 158; (g) any remaining `https://code.claude.com/docs/...` URL citations; (h) `CLAUDE.md §Commands — REVIEW/AGENT TIMEOUTS` paragraph has wall-clock rationale ("20-45 min in practice", "cold-starts of 8-10 min") that should be trimmed to just the hook's numeric bounds. Rule of thumb: skills contain instructions only ("do X", "MUST Y"); rationale belongs in this design log. `model:` in Agent dispatch code blocks is a technical parameter and stays; `model:` in skill frontmatter is a harness directive and stays.

- [ ] **[p2] Sub-agent MID-STREAM visibility during long reviews.** Context: the stranded-report half of this item (sub-agent returns without a TPR-REPORT after the CLI completed) was closed 2026-04-16 evening via the dual-path-transport fix (invariant I14, §9 stranded-report recovery, `tee` persistence, orchestrator-owned scratch dir). What remains is heartbeat visibility *during* the 20–45-min review window — the user can't tell if the CLI is making progress, cold-starting, stuck, or done. The old design had 5-min polling with `status-check.sh`; the new design has none. Consider: (a) the orchestrator spawns a lightweight background `tail -f "$scratch/{REVIEWER}-stdout.txt"` task to render heartbeats (the tee now persists stdout line-by-line, so this is free), or (b) the orchestrator emits a `## Round {N} in flight` narration with periodic refreshes via `Read` on the growing stdout file. Both need prototyping; both benefit from the I14 groundwork.

- [ ] **[p3] Reviewer transcript persistence.** Context: the old `/tpr-review` persisted per-round artifacts to `{run_id}/round-{N}/` for post-hoc forensic review. The new design is transcript-only. If the user ever wants to retroactively inspect "what did gemini actually say in round 1 of that bug fix two weeks ago?", they can't — it's gone with context compaction. Consider: optional `ORI_TPR_ARCHIVE=1` env var that writes each round's raw CLI stdout to `$HOME/.cache/ori-tpr-archive/<date>/round-N/<reviewer>.stdout`.

- [ ] **[p2] Convergence heuristic to exit early.** Context: the current design caps at 5 iteration rounds + 2 meta-only rounds. The old design had a 5-condition "convergence gate" (§6c.1). We dropped it in favor of simpler caps. If in practice rounds 2+ consistently find only doc-hygiene residue (as happened during the dogfood), consider adding back: "exit after round N if all findings are LOW severity + strictly decreasing trajectory + category in {wording, cosmetic}". Only add this if the current caps produce obvious waste.

- [ ] **[p1] `/tp-help` has never been dogfooded with the new pattern.** Context: this rewrite was run through `/tpr-review` three times but `/tp-help` was not directly invoked. The first real `/tp-help` call (via `/fix-bug` Phase 1.75 or ad-hoc) will be the first live test. Suggested fix: on first invocation, watch for the same BUG-A (ARG_MAX) and BUG-B (sub-agent return contract) patterns; if seen, reuse the same fixes as `/tpr-review`.

- [ ] **[p3] `plans/use-plan-tool-to-sprightly-cat.md` lives outside the project** (in `/home/eric/.claude/plans/`). Reviewers get a 404 when instructed to read it as part of the objective. The objective prompts currently say "may 404; skip" but this is fragile. Consider: copy the plan file into the project tree at `plans/rewrites/tpr-review-2026-04-16.md` for local reference, OR stop referencing it in objectives and describe the rewrite inline.

- [ ] **[p2] Spec gate only checks `HEAD`, not staged/unstaged variants.** Context: `.claude/skills/tpr-review/SKILL.md §3` uses `git diff --name-only HEAD`. If a spec file is ONLY modified in the unstaged working tree (not yet added), this diff catches it. But if spec is staged only (added but not committed), does the diff still catch it? Verify with a targeted test; may need `git diff --name-only HEAD --cached --staged` or similar.

- [ ] **[p3] No plan-TPR round-aggregation test.** Context: the F3 fix (`ever_verified_findings` accumulator) has never been exercised with a multi-round plan review that had findings in round 1 but cleaned up by round 3. The plan-mode frontmatter write happens after the loop, consuming `ever_verified_findings` — untested pathway.

### Recently closed (retrospective record)

- [x] **2026-04-17 — Cap-exit accept-with-findings escape hatch end-to-end.** Addressed user feedback "After a large amount of tpr reviews and no consensus you never give me the option to mark reviewed true and continue." Root cause: the `accept-remaining` / `accept-best-effort` options already existed in `review-plan/step-6-tpr.md` Branch 2/3, but (1) their labels were vague (no mention of the flip), (2) `review-plan/SKILL.md` had no parent-side plumbing to propagate the user's selection downstream, (3) `step-7-8-verify.md` hardcoded `tpr.converged == true` as the ONLY flip condition, silently dropping the user's accept, and (4) `/tpr-review` standalone cap-exit had no accept option at all. Fix spans 7 files: `tpr-review/SKILL.md` §§5/7/10/11.5 (concrete 4-option AskUserQuestion, §10 step 3a frontmatter flip on user-accepted exit, §7 banned-phrase carve-out, §11.5 concretized); `review-plan/step-6-tpr.md` Branch 2/3 (explicit labels + `applies_user_accepted: true` flag, added Branch 2 `escalate-to-plan`); `review-plan/SKILL.md` §Escalation handling (item 4 — patch handoff JSON on user-accept); `review-plan/step-7-8-verify.md` (flip condition, output enum, audit-trail notes, Do NOT carve-out); `improve-tooling/tpr-review-design.md` (I15, §4 dated lesson, §5 7 regression lines). Invariant I15 guards the end-to-end plumbing; §5 regressions cover each link in the chain.
- [x] **2026-04-16 — Context-pressure pause option added.** `/tpr-review §9` (both-reviewer failure escalation), `/tp-help §5` (both-reviewer failure escalation), and `/fix-next-bug` Step 4.A (interactive-mode between-bug prompt) all gained a new option: "Pause here, clear context, resume with /continue-roadmap (fresh session)." Additionally, `/tpr-review §9` documents an optional **context-pressure pause** that the orchestrator MAY insert between rounds when context has grown substantially (≥3 rounds, long transcript of findings+fixes, substantive findings still arriving) — a fresh session reviews better than an exhausted one. Trigger: proactive, not reactive; by the time the session is truly exhausted, the user can't cleanly resume. User-surfaced via `/improve-tooling` session 2026-04-16 after observing multi-round /tpr-review dogfood rounds accumulating significant context.
- [x] **2026-04-16 — Round 0 F1:** spec gate short-circuit → fixed via synthetic finding in §3
- [x] **2026-04-16 — Round 0 F2:** render-before-fix ordering → fixed via pseudocode reorder
- [x] **2026-04-16 — Round 0 F3:** plan-mode loses earlier-round findings → fixed via `ever_verified_findings`
- [x] **2026-04-16 — Round 0 F4:** tp-help synthesis contradiction → fixed by rewriting §6
- [x] **2026-04-16 — Round 0 F5:** SSOT consumer count stale → recounted to 20
- [x] **2026-04-16 — Round 0 F6:** 12 vs 13 section reference → fixed
- [x] **2026-04-16 — Round 1 USER-flagged:** scratch-dir collision → fixed via `mktemp -d`
- [x] **2026-04-16 — Round 1 F7/F8:** timeout 600000 → raised to 2700000
- [x] **2026-04-16 — Round 1 F9 (codex):** spec gate mode-gated → now always-on
- [x] **2026-04-16 — Round 1 F9/F10 (gemini) + F10 (codex):** stale consumer rosters → collapsed
- [x] **2026-04-16 — Round 2 F1–F5:** doc-consistency drift (prose literals, 10-min/600s, Step F registry) → fixed
- [x] **2026-04-16 — Post-rewrite dogfood USER-flagged:** orchestrator emitted prose "1. Continue / 2. Exit / 3. Abort" options at post-Round-1 inflection point instead of using `AskUserQuestion`. User correctly flagged that structured choice points must use the harness's structured-choice UI, not plain prose bullets that look identical to narration. Fixed via new SKILL.md §11.5 "User-interaction discipline (MANDATORY)" enumerating five choice points (post-round inflection, ambiguous input, transport-failure retry, spec-gate resolution, cap-exit escalation) that require `AskUserQuestion`; prose summaries explicitly allowed only for informational renders where the next assistant turn is identical regardless of user reaction. Load-bearing invariant added as I13. Design philosophy principle 11 added.
- [x] **2026-04-16 (evening) — Stranded-report failure mode fixed via dual-path transport.** Round 3 dogfood (see §4 Round 3 entry) confirmed that sub-agents can auto-background their internal Bash call despite `tp_agent_prompt.md` absolute-rule-#4, leaving the completed CLI's TPR-REPORT stranded in the paused sub-agent conversation with no orchestrator-side recovery path. Fixed by: (1) orchestrator-owned scratch dir — `tpr-review/SKILL.md` §8 step 8a now `mktemp -d`s BEFORE dispatch and passes the path as `{SCRATCH_DIR}`; (2) `tp_agent_prompt.md` Step 1 updated to use `RUN="{SCRATCH_DIR}"` (no own `mktemp`); (3) Step 2 `tee`s CLI stdout to `$RUN/{REVIEWER}-stdout.txt` (line-buffered, live); (4) Step 3 writes extracted TPR-REPORT block to `$RUN/{REVIEWER}-report.txt` via `sed`; (5) Step 4 requires `scratch_dir: $RUN` as first line of return message; (6) `tpr-review/SKILL.md` §9 adds "Stranded-report recovery" protocol reading `$scratch/{REVIEWER}-report.txt` → `$scratch/{REVIEWER}-stdout.txt` BEFORE declaring a reviewer failed. Invariant I14 added. Regression lines added to §5 covering missing `tee`, self-`mktemp`, missing Step-3 extraction, and missing §8 `mktemp` step. Open item [p2] narrowed from "sub-agent status visibility" to "mid-stream visibility only" — the stranded-report half closed by I14.

## §7 — How To Use This File In Future Sessions

**On every surprise.** When you run `/tpr-review` or `/tp-help` and something unexpected happens (failure mode, bad output, confusion about what the skill is doing), come here first. Check §Regressions To Watch For — is the surprise a known regression pattern? Check §Improvement Log — is it already filed? If neither, add it.

**Before editing any of the 6 canonical files in §3.** Re-read §Load-Bearing Invariants. Every invariant there is defended by a specific failure mode from the dogfood runs. Breaking an invariant without understanding its failure mode means re-running the dogfood to re-discover what this file already records.

**When `/fix-bug` or `/tpr-review` on a real bug fails.** If the failure is in the *skill* (not the bug being fixed), log it here with the specific invocation that triggered it. This is the correct home for skill-level bugs — it's a file dedicated to this skill pair, inside the improve-tooling skill folder (which itself is the canonical home for "improve the tooling, don't work around it" per `CLAUDE.md §Continuous improvement everywhere`).

**When the design evolves.** If you change §1 Core Design Philosophy or §2 Load-Bearing Invariants, update both SKILL.md files to match AND add a dated entry under §4 Lessons documenting why the invariant changed. Don't silently mutate the design — the whole point of this file is that the design doesn't get lost.
