# Per-Round Reviewer Prompt Composition

Read by the `/tpr-review` orchestrator (Opus, main context) at the start of every round, before dispatching the reviewer sub-agents. Not a registered skill. Not read by sub-agents.

## Why this file exists

Composing a reviewer prompt is **editorial work** — it requires understanding what the prior round found, where reviewers disagreed, and what this round needs to push on harder. That judgment belongs to the orchestrator (Opus), not to a Sonnet transport sub-agent. Sonnet sub-agents have zero role in composing, translating, or reinterpreting review content.

This file is the orchestrator's protocol for building ONE reviewer prompt per round. It produces a single file on disk:

- `{SCRATCH_DIR}/prompt.md`

Both reviewer sub-agents (codex and gemini) read the same file. The shared prompt is identity-neutral — it does not name a specific reviewer and does not state a trust tier. Each sub-agent injects a 2-line identity header when invoking its CLI (see `tp_agent_prompt.md` Step 2). Writing one shared prompt per round keeps composition at O(rounds) instead of O(2 × rounds), restoring the "Opus composes ONCE" intent of the 2026-04-17 morning refactor.

## Inputs the orchestrator tracks across rounds

- `round_n` — 0-indexed round number (0 is the first dispatch).
- `objective` — the review objective (custom-mode ARGS, or a mode-derived objective for `review-work` / `review-plan`). In help mode this carries the user's question + caller-supplied context.
- `scope` — files/paths under review (git range, plan directory, or free-text scope). In help mode this is typically the literal string `(help mode — no scope)` or empty.
- `prior_verified_fixed` — findings from prior rounds that the orchestrator verified AND fixed. These should NOT be re-raised.
- `prior_verified_outstanding` — findings verified but filed as `- [ ]` (user-accepted at cap exit, or handed off to `/create-plan`). These are knowns; reviewers should not spend attention on them.
- `prior_disagreements` — findings raised by only one reviewer last round (not cross-confirmed). Round N should ask the other reviewer to evaluate them specifically.
- `prior_thin_signal` — `true` if the last round produced zero actionable findings but exhibited asymmetric depth (one reviewer read full files, the other skimmed; one consulted rules, the other did not).
- `help_mode` — `true` when invoked with `--help-mode` (see `SKILL.md §1`). Switches the shared body from the findings template to the help-mode template below. Help mode never tracks prior-round state (it is one-shot by contract), so `prior_*` inputs are always empty when `help_mode == true`.

## Round 0 (first round) composition

Produce the shared body — no prior-round block, no thoroughness directive.

### Shared body template

```md
## Mandatory grounding (DO THIS BEFORE PRODUCING ANY FINDINGS)

You have shell + file-reading tool access. Before reviewing anything, run:

  ls .claude/rules/*.md

That ls output is the authoritative rule manifest. Then read, in full:
  - CLAUDE.md
  - every file under .claude/rules/*.md that the ls enumerated

Record the list under `rules_consulted` in your TPR-REPORT. Grounding skipped = review skipped.

## Objective

{objective}

## Scope

{scope}

## Findings grounding policy

Every finding MUST:
  - Cite `path:line` of the actual file where the issue lives.
  - Quote ≤3 lines of the actual code verbatim as `evidence`.
  - Name the rule / invariant / spec clause it violates.
  - Propose a one-sentence `recommended_fix`.

If you cannot verify a concern against the file, DROP it. Never infer from file names. Read the file.

## Banned response phrases (MUST NOT appear in any finding or in `summary`)

  "pre-existing"           "architectural limitation"
  "out of scope"           "conservative / safe"
  "not a regression"       "future improvement"
  "known limitation"

If a banned phrase is the only framing you would give a finding, DROP the finding.

## Return format (PLAIN TEXT, emit once at the end of your output)

<<<TPR-REPORT
reviewer: <your identity — codex or gemini, as stated in the identity header your sub-agent prepended when invoking you>
trust_tier: <your trust tier — HIGH or LOWER, as stated in that identity header>
status: clean | findings | failed
rules_consulted: CLAUDE.md, .claude/rules/impl-hygiene.md, ...
files_read: path/one, path/two, ...
summary: <one paragraph, <= 400 chars>

findings:
- id: F1
  severity: critical | high | medium | low | informational
  path: path/to/file
  line: 42
  title: <short title, <= 80 chars>
  evidence: |
    <verbatim code quote, <= 3 lines>
  rule_violated: <rule file or spec clause>
  recommended_fix: <one sentence>
TPR-REPORT>>>

If `status: clean`, emit `findings: []`.
If `status: failed`, omit `findings:` and put the error in `summary:`.
```

### Help-mode body (used when `help_mode == true` — replaces the findings body above)

When the orchestrator sets `help_mode = true`, produce this body INSTEAD of the findings body. It swaps the reviewer contract from "find bugs" to "provide advice," removes the file:line-citation requirement, and changes the return schema to `response: |` prose instead of `findings:` list. Everything above the body (grounding block) is unchanged — reviewers still `ls .claude/rules/*.md` and read the rule manifest, because advice must be grounded in project conventions.

```md
## Mandatory grounding (DO THIS BEFORE ANSWERING)

You have shell + file-reading tool access. Before answering, run:

  ls .claude/rules/*.md

That ls output is the authoritative rule manifest. Then read, in full:
  - CLAUDE.md
  - every file under .claude/rules/*.md that the ls enumerated

Ground your answer in the project's conventions and invariants. If your
recommendation conflicts with a rule, name the rule and explain why you
still recommend the action.

## Question

{objective}

## How to answer

- Lead with your recommendation in one paragraph.
- Then give the reasoning: what tradeoffs, what you considered, what
  you ruled out and why.
- If you would do something different from what the caller proposed, say so
  plainly.
- If the question has multiple reasonable answers, surface the tradeoff
  instead of picking arbitrarily.
- If you do not have enough information, say what is missing — do not fill
  gaps with assumptions.
- Cite files and lines for every specific code reference.
- Keep it under 800 words unless complexity genuinely requires more.

## Banned response phrases (MUST NOT appear in your answer)

  "it depends"            "conservatively"
  "for safety"            "pre-existing"
  "out of scope"          "future improvement"

Take a position. Do not hedge. Do not restate the question back. Answer it.

## Do NOT

- Do not write code for the caller to paste in — give guidance, not
  patches, unless the caller explicitly asked for a diff.
- Do not produce a structured findings list. This is advice, not a bug
  review. Use the `response: |` prose format below, not `findings:`.

## Return format (PLAIN TEXT, emit once at the end of your output)

<<<TPR-REPORT
reviewer: <your identity — codex or gemini, as stated in the identity header your sub-agent prepended when invoking you>
trust_tier: <your trust tier — HIGH or LOWER, as stated in that identity header>
mode: help
status: advice | failed
rules_consulted: CLAUDE.md, .claude/rules/impl-hygiene.md, ...
summary: <one-sentence gist of your recommendation, <= 200 chars>
response: |
  <multi-line prose advice — lead with recommendation, then reasoning,
   then tradeoffs. Cite path:line for any specific code reference. No
   structured findings list; this is help, not a review.>
TPR-REPORT>>>

If the question cannot be answered (missing context, CLI failure, etc.),
emit `status: failed` and put the error in `summary:`. Do NOT invent
advice to fill the gap.
```

The return-schema discriminator is `mode: help` + `status: advice` + `response: |` (prose) — distinct from review-mode's `status: clean | findings | failed` + `findings: [...]`. The coordinator branches on `mode: help` to render the `response:` text into the §5 help-mode output block; review-mode parsing is untouched.

### Identity is NOT in the shared prompt

The shared body does NOT name the reviewer and does NOT state a trust tier. Each sub-agent injects its own 2-line identity header at CLI-invocation time:

```
You are {REVIEWER}. Your trust tier in the consuming orchestrator is {TRUST_TIER}.
```

See `tp_agent_prompt.md` Step 2. The CLI fills the `reviewer:` / `trust_tier:` slots in the return block based on this header. The orchestrator never writes reviewer-specific text into the shared prompt.

## Round N>0 composition (prior-state feedback)

For rounds after the first, prepend a **Prior-Round State** block above the shared body. The orchestrator fills it from the inputs above.

### Prior-Round State block template

```md
## Prior-Round State — READ BEFORE REVIEWING

This is round {round_n}. Previous rounds produced the following state:

### Findings already fixed (DO NOT re-raise)

{bulleted list of prior_verified_fixed: `path:line — title — fixed in commit {sha}`}
{if empty: "None yet."}

### Findings filed as `- [ ]` but not yet fixed (known, tracked)

{bulleted list of prior_verified_outstanding: `path:line — title — filed at {location}`}
{if empty: "None."}

### Single-reviewer findings from last round (cross-check needed)

The prior round raised the following findings that were verified by the orchestrator but NOT cross-confirmed by the other reviewer. This round, evaluate each one explicitly — agree or refute with evidence:

{bulleted list of prior_disagreements: `path:line — title — raised by {other_reviewer}`}
{if empty: "None."}

### Your job this round

Focus on NEW findings and the cross-check list above. Do NOT re-report the "already fixed" items as new findings; if the fix is incomplete cite the remaining gap specifically. Depth is required — not volume.
```

### Thoroughness Directive (conditional — prepend BEFORE Prior-Round State when `prior_thin_signal == true`)

```md
## THOROUGHNESS RE-REVIEW DIRECTIVE — MANDATORY

The previous round produced zero actionable findings, but the orchestrator rejected the round for insufficient depth. One or more of these signals crossed the threshold where a "no findings" outcome cannot be trusted:

- Asymmetric depth between reviewers (one read full files, the other skimmed).
- Thin `files_read` or empty `rules_consulted` in the prior envelope.
- Wall-time / event-count ratio between reviewers outside tolerance.

This re-review is MANDATORY. You must now meet a deeper investigation standard:

1. READ EVERY FILE IN SCOPE IN FULL — not only the hunks named in the objective. Every file must appear in `files_read`.
2. READ NEIGHBORING CODE required to understand invariants and boundary contracts. If a function calls into another module, read the callee. Add those files to `files_read`.
3. READ THE GROUNDING RULES IN FULL. `rules_consulted` MUST list CLAUDE.md plus every `.claude/rules/*.md` relevant to the scope. Empty `rules_consulted` will be rejected again.
4. TRACE DATA FLOW across at least two layers of call chain beyond the immediate scope. Record traced files in `files_read`.
5. If after this deeper pass you STILL find zero actionable issues, emit at least one `informational`-severity entry describing WHAT you verified and WHY the subject is sound — so the orchestrator can calibrate trust on a no-findings outcome.

A superficial re-review WILL be rejected again.
```

## Composition order (top → bottom of the written prompt file)

**Review mode (`help_mode == false`):**

1. (Conditional) Thoroughness Directive — only when `prior_thin_signal == true`.
2. (Conditional) Prior-Round State block — only when `round_n > 0`.
3. Shared findings body (grounding, objective, scope, findings policy, banned phrases, TPR-REPORT return format with `findings:`).

**Help mode (`help_mode == true`):**

1. Help-mode body (grounding, question, advice contract, banned hedges, TPR-REPORT return format with `mode: help` / `response: |`). No Thoroughness Directive, no Prior-Round State block — help mode is one-shot and stateless.

There is NO per-reviewer prefix in either mode. Sub-agents inject identity at invocation time (see `tp_agent_prompt.md` Step 2).

The orchestrator writes the final assembled text to `{SCRATCH_DIR}/prompt.md`. Both sub-agents read that file verbatim — no further composition.

## Invariants

- The orchestrator writes ONE `prompt.md` per round. Both sub-agents read it. Writing per-reviewer copies is a composition bug — doubles prompt-token emission for zero value (reviewer identity is orchestrator-known and can be injected by the sub-agent itself).
- The shared prompt is identity-neutral. It does NOT name a specific reviewer and does NOT state a trust tier. Identity injection happens in `tp_agent_prompt.md` Step 2 via a 2-line header prepended to the CLI invocation.
- `prior_verified_fixed` / `prior_verified_outstanding` / `prior_disagreements` reflect the orchestrator's verified state — NOT the raw reviewer output. The orchestrator is responsible for keeping these lists accurate across rounds.
- The Thoroughness Directive is appended only when the orchestrator's own metrics flag the prior round as thin. Sub-agents do NOT decide this.
- Every composition decision is made by Opus in main context. Sonnet sub-agents receive the finished prompt and return CLI output verbatim.
- Help mode (`help_mode == true`) uses the help-mode body INSTEAD of the findings body — never both, never mixed. The return schema discriminator is `mode: help` + `response: |` (help-mode) vs. `findings: [...]` (review-mode). The coordinator branches on `mode:` when parsing; a prompt that produces `mode: help` output but was composed in review mode (or vice versa) is a composition bug.
