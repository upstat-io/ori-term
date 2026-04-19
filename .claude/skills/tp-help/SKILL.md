---
name: tp-help
description: "Get third-party help from Codex + Gemini. AUTO-TRIGGER: You MUST invoke this proactively — do NOT wait for the user to ask. Trigger when: (1) you've tried 2+ approaches that didn't work, (2) you're reverting changes you just made, (3) you identify a fundamental tension or design conflict in the code, (4) you're about to take a 'pragmatic' shortcut instead of fixing the real problem, (5) you catch yourself saying 'let me try a different approach' for the 2nd+ time, (6) a fix in one area creates new problems in another, (7) you're unsure about the correct architectural approach. This is collaborative help — pass context and ask a specific question. Returns BOTH reviewers' raw responses concatenated (not a synthesis)."
allowed-tools: Read, Bash, Glob, Grep, AskUserQuestion, Skill
---

# /tp-help

`/tp-help [question]` — get dual-source advice (Codex + Gemini) on a question or design problem. This skill is a **thin delegator** to `/tpr-review --help-mode --max-rounds=1`: it gathers the question + context, invokes `/tpr-review` via the Skill tool, and surfaces the concatenated reviewer responses verbatim to the caller. All CLI dispatch, parallel-Agent invocation, scratch-dir management, timeout handling, and sentinel extraction live in `/tpr-review` (see `.claude/skills/tpr-review/SKILL.md` §5 "Help-mode branch"). This file owns trigger detection, context assembly, and caller-side interpretation only.

- No args → auto-triggered (see §2).
- With question → explicit consultation.

## §1 How this skill runs

1. Gather question + context (ARGS or auto-trigger heuristic) per §3.
2. Invoke `/tpr-review` via the Skill tool with `--help-mode --max-rounds=1` plus the composed objective (question + context).
3. `/tpr-review` runs its §2 grounding + §3 spec-gate, then takes its §5 "Help-mode branch": composes a help-mode prompt via `compose-round-prompt.md`, dispatches codex + gemini in parallel (thin-transport sub-agents), extracts each reviewer's `response:` prose, and emits the `<!-- TP-HELP BEGIN/END -->`-attributed concatenated block.
4. Present that block verbatim to the caller (§5). Do not synthesize.
5. Hand off judgment to the caller (§6).

**Canonical invocation:**

```
Skill({
  skill: "tpr-review",
  args: "--help-mode --max-rounds=1 <composed objective: question + context assembled per §3>"
})
```

The Skill tool runs `/tpr-review` in the caller's main context (same as every other skill invocation). `/tpr-review` handles foreground parallel Agent dispatch internally; this file does NOT emit Agent calls of its own.

## §2 MANDATORY Auto-Trigger — Do NOT Wait for User

Invoke IMMEDIATELY when ANY of these are true:

1. **Multiple failed approaches** — 2+ approaches tried on the same problem, none worked cleanly.
2. **Reverting your own changes** — undoing work you just did because it caused new problems.
3. **Fundamental tension identified** — design conflict where fixing one thing breaks another.
4. **Pragmatic retreat** — catching yourself about to take a shortcut instead of solving the real problem.
5. **Approach cycling** — saying "let me try a different approach" for the 2nd+ time.
6. **Fix interference** — a fix in one subsystem creates new failures in another.
7. **Architectural uncertainty** — unsure which of two+ fundamental approaches is correct.
8. **Stuck without forward progress** — working on the same problem and not making visible progress.

### Do NOT trigger for

- Simple bugs with obvious fixes.
- First attempt at an approach (try it first, ask if it fails).
- Questions about Ori syntax or spec (read the spec instead).
- Minor implementation details with clear precedent in the codebase.

### Exception — Design Consensus Mode (called by `/fix-bug` Phase 1.75)

The "simple bugs" and "first attempt" non-triggers DO NOT apply when `/tp-help` is invoked by `/fix-bug` at Phase 1.75. In that calling context, `/tp-help` is used for **design consensus** — a pre-emptive pressure-test of a proposed fix approach before tests or implementation are written. Design consensus runs for EVERY bug that reaches `/fix-bug` Phase 1.75.

See `.claude/skills/fix-bug/SKILL.md` § Phase 1.75.

### Legacy triggers (still valid)

- Stuck on a bug and can't figure out root cause.
- Unsure which of two implementation approaches is better.
- Just wrote something tricky and want a sanity check.
- Test failing and can't see why.
- Need help understanding unfamiliar code.
- Want to validate reasoning before committing to an approach.
- About to make a significant architectural decision.

## §3 Context Assembly (before dispatch)

Build the composed objective passed to `/tpr-review --help-mode`. The objective must be a single string; structure it with clear labeled sections so both reviewers parse it consistently.

```
## Question
<the user's question, or the caller's design-consensus prompt>

## Context
- Recent attempts: <approaches tried, what failed, error messages>
- Relevant files: <paths + line ranges of code under discussion>
- Constraints: <project rules that bind the solution space; cite
  `CLAUDE.md §The One Rule`, `impl-hygiene.md §SSOT`, etc. as relevant>
- Caller mode: <"design consensus — pressure-test the proposed approach
  before implementation" when invoked from /fix-bug Phase 1.75, else omit>
```

Keep the context concise. The reviewers read `CLAUDE.md` + rules themselves during `/tpr-review` §2 grounding; the composed objective is for problem-specific material only.

The entire composed string becomes the `<custom-objective>` argument after the flags — `/tpr-review` treats it as the objective since the ARGS don't start with `--skill review-work` or `--skill review-plan`. Help-mode's `compose-round-prompt.md` drops the objective under the `## Question` heading.

## §4 Delegation (canonical)

Single Skill invocation, foreground:

```
Skill({
  skill: "tpr-review",
  args: "--help-mode --max-rounds=1 " + composed_objective
})
```

That's the entire dispatch. `/tpr-review` handles everything downstream:
- `--help-mode` switches its §5 to the help-mode branch (skips verify/classify/fix/file).
- `--max-rounds=1` caps iteration (enforced by help-mode too; redundant but explicit).
- The spec-gate (§3) still runs — help mode cannot bypass spec governance.
- Both reviewers are dispatched in parallel as thin-transport Agent sub-agents.
- The coordinator emits the concatenated `<!-- TP-HELP BEGIN/END -->`-attributed block and returns.

**Do NOT:**
- Do not emit `Agent()` calls from this skill — `/tpr-review` owns dispatch.
- Do not create scratch dirs — `/tpr-review` §8 step 8a owns scratch-dir creation.
- Do not invoke codex/gemini CLIs directly — the thin-transport sub-agent pattern in `tp_agent_prompt.md` is the single canonical invocation path.
- Do not re-parse the concatenated block — it is the raw reviewer output, ready to present to the caller.

## §5 Output

The Skill tool returns the output of `/tpr-review`. In help mode, `/tpr-review` emits exactly this block (§5 §11.H):

```md
<!-- TP-HELP BEGIN codex -->

{codex response verbatim}

<!-- TP-HELP END codex -->

<!-- TP-HELP BEGIN gemini -->

{gemini response verbatim}

<!-- TP-HELP END gemini -->
```

`/tp-help` surfaces this block to its caller **unchanged** — no reformatting, no synthesis, no extra commentary. The attribution-sentinel format is the stable contract that existing callers (`/fix-bug` Phase 1.75, `/create-plan` Step 6B/8B, `/review-plan` Step 4) parse.

### Failure surface

- **One reviewer failed** — `/tpr-review` help-mode runs §9 survivor-mode policy: the surviving reviewer's response is rendered normally; the failed reviewer's block contains `(codex failed: …)` or `(gemini failed: …)` in place of prose.
- **Both reviewers failed** — `/tpr-review` help-mode retries once per §9. If the retry also fails, the concatenated block is emitted with both halves showing `(… failed: …)` text. Unlike review mode, help-mode does NOT emit the `AskUserQuestion` escalation — the block-with-failures IS the result. The caller decides whether to retry, fall back to own reasoning, or ask the user.
- **Skill tool failure** (Skill itself errored before `/tpr-review` ran) — if the Skill invocation returns an error rather than a concatenated block, escalate via `AskUserQuestion`:

```
AskUserQuestion(questions=[{
    "question": "The /tpr-review --help-mode invocation failed before reviewers could respond. How do you want to proceed?",
    "header": "tp-help dispatch failure",
    "multiSelect": False,
    "options": [
        {"key": "retry",
         "label": "Retry the /tp-help invocation (Recommended)",
         "description": "Recommended because dispatch failures are almost always transient (rate limit, cold-start, harness blip); a single retry resolves most cases without sacrificing reasoning quality.",
         "recommended": True},
        {"key": "proceed-without",
         "label": "Proceed without help (I'll answer based on my own reasoning)",
         "description": "Skip the consultation and use my own reasoning. Pick when the question is time-sensitive and you trust my analysis without the second opinion."},
        {"key": "pause",
         "label": "Pause here, clear context, resume with /continue-roadmap",
         "description": "Fresh session; the roadmap picks up where this help query was invoked from. Pick when context pressure may be contributing to the failure."},
        {"key": "abort",
         "label": "Abort and ask the user for clarification",
         "description": "Exit /tp-help entirely and request guidance from the user before continuing."},
    ],
}])
```

## §6 Applying the Output

- **User (interactive)** — print the concatenated block directly, unedited. A one-line attribution header ("codex said X paragraphs, gemini said Y paragraphs") is permitted, but no synthesis, no interpretation, no merged summary.
- **`/fix-bug` Phase 1.75 (design consensus)** — the caller reads both raw responses, looks for agreement on the proposed approach, and blocks the fix if either reviewer flags a design-level concern. `/tp-help` does not apply judgment.
- **Proactive (Claude stuck)** — the caller reads both responses and decides how to proceed, doing the judgment work in its own turn, visible to the user.

**Do NOT blindly apply either reviewer's suggestions.** Use the advisories as second opinions, not as instructions.

**Where reviewers AGREE** — strong signal. Evaluate the shared recommendation against `CLAUDE.md` rules before applying.

**Where reviewers DISAGREE** — the disagreement often surfaces the real tradeoff. Read both carefully before choosing.

## §7 Callers

- `/fix-bug` Phase 1.75 (design consensus — mandatory on every bug).
- `/create-plan` Step 6B/8B (pressure-test design before committing).
- `/review-plan` Step 4 (design consensus on the plan's architecture).
- Proactive auto-trigger per §2.

Callers invoke `/tp-help` as a slash command in prose — they do NOT need to know about the `/tpr-review --help-mode` delegation. The slash-command contract (name, ARGS, concatenated-block output) is preserved; only the implementation switched from "own parallel dispatch" to "thin wrapper over `/tpr-review`."

## §8 Files

- `SKILL.md` (this file) — auto-trigger, context assembly, delegation to `/tpr-review --help-mode`, caller-side interpretation.

Canonical dispatch + prompt composition lives in:
- `.claude/skills/tpr-review/SKILL.md` §1 `--help-mode` flag, §5 "Help-mode branch".
- `.claude/skills/tpr-review/compose-round-prompt.md` "Help-mode body".
- `.claude/skills/tpr-review/tp_agent_prompt.md` — unchanged; serves both review and help modes (it is identity-neutral and extracts whatever TPR-REPORT block the CLI emits).

**Deleted in the 2026-04-17 help-mode refactor:** `tp_help_prompt.md` — the sub-agent prompt template was redundant once `compose-round-prompt.md` gained a help-mode body. See `.claude/skills/improve-tooling/tpr-review-design.md` §4 for the retrospective.
