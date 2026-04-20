# Per-Round Reviewer Prompt Composition

Read by the `/tpr-review` orchestrator (Opus, main context) at the start of every round, before dispatching the reviewer sub-agents. Not a registered skill. Not read by sub-agents.

## Why this file exists

Composing a reviewer prompt is **editorial work** — it requires understanding what the prior round found, where reviewers disagreed, and what this round needs to push on harder. That judgment belongs to the orchestrator (Opus), not to a Sonnet transport sub-agent. Sonnet sub-agents have zero role in composing, translating, or reinterpreting review content.

This file is the orchestrator's protocol for building ONE reviewer prompt per round. It produces a single file on disk:

- `{SCRATCH_DIR}/prompt.md`

Both reviewer sub-agents (codex and gemini) read the same file. The shared prompt is identity-neutral — no reviewer name, no trust tier. Each sub-agent injects a 1-line identity header at CLI-invocation time (see `tp_agent_prompt.md` Step 2). One shared prompt per round; O(rounds), not O(2 × rounds).

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
## Mandatory grounding — READ IN FULL, EVERY LINE, NOT OPTIONAL

You have shell + file-reading tool access. Before reviewing ANY code and before drafting ANY finding, execute this sequence. It is non-negotiable.

1. Run `ls .claude/rules/*.md` — capture the authoritative rule manifest.
2. Read `CLAUDE.md` IN FULL — every line, first to last. No skimming. No skipping. No "scan for the relevant part." The entire file.
3. Read EVERY file under `.claude/rules/*.md` that the `ls` enumerated — each one IN FULL, every line. ALL of them. Not a sample. Not the ones that look relevant. Not the ones whose titles match the scope. ALL rule files, every line, from first to last.
4. Record every file read under `rules_consulted` in your TPR-REPORT — the list MUST equal `CLAUDE.md` plus the full `ls` output.

### Why full reads are mandatory

  - Rule files carry load-bearing invariants (AIMS Five Load-Bearing Invariants, cross-phase contracts, SSOT rules, banned-phrase lists, INVERTED-TDD prohibitions, The One Rule, Ownership & Deferral, Matrix Testing). A skimmed rule file produces a review that misses the exact violation the rule was written to catch.
  - Rule files are dense and short. Reading every file in full typically costs 5–15 minutes of wall-time — the cheapest step in the round. Skipping it to save time is strictly worse than skipping the review entirely.
  - Prior-conversation or training-data memory of these rules is STALE by definition. Read them in THIS invocation.

### Banned grounding shortcuts

  - "Reading only the rules that seem relevant." — FORBIDDEN. Read all of them.
  - "Skimming for keywords." — FORBIDDEN. Read every line in order.
  - "Relying on what I remember from training or prior sessions." — FORBIDDEN. Read the file now.
  - Using file-read tools with an offset/limit that truncates content. — FORBIDDEN. Read without truncation.
  - "Reading only CLAUDE.md and inferring the rules from it." — FORBIDDEN. Each rule file carries its own content; CLAUDE.md is not a substitute for the rule files it references.

A `rules_consulted` list shorter than the `ls` output plus `CLAUDE.md` is evidence of skipped grounding. The orchestrator WILL reject the round as thin-signal and the next round will fire a mandatory Thoroughness Re-Review Directive. Grounding skipped = review skipped.

## Objective

{objective}

## Scope

{scope}

## Read and run, do NOT write — ABSOLUTE

You run with elevated tool access (`--full-auto` for codex, `--approval-mode yolo` for gemini). That access is for **investigation only**.

**ALLOWED** (use freely to verify findings):
  - Read / view file contents.
  - Grep / search the codebase.
  - Bash for tests, builds, intelligence-graph queries: `cargo test`, `cargo b`, `cargo c`, `cargo test --all`, `cargo clippy --all -- -D warnings`, _(intel-query not available in this project; use Grep/Glob)_, `git log`, `git diff`, `git show`, `git status`, any read-only diagnostic.

**FORBIDDEN** — under all circumstances, even if you are certain you know the fix:
  - Edit, Write, apply_patch, NotebookEdit, or any tool that modifies file contents.
  - Shell redirection that writes the working tree: `>`, `>>`, `tee` into source paths, `sed -i`, `awk -i inplace`, `mv`, `cp` into a tracked path, `rm` against tracked files, `git checkout --`, `git restore`, `git reset`, `git stash`, `git add`, `git commit`, `git rm`, `git mv`.
  - Patch application: `patch`, `git apply`, `applypatch`.
  - Creating new files anywhere under the repo (other than the scratch dir paths the orchestrator owns: `$RUN/{REVIEWER}-{stdout,stderr,report}.txt`).

Your sole output is the `<<<TPR-REPORT … TPR-REPORT>>>` block. The orchestrator is the **only** entity that edits code; you produce findings, the orchestrator decides whether to fix, verifies fixes, and commits with attribution. If you "helpfully" edit a file, your edit:
  - bypasses the orchestrator's verification step (§4),
  - bypasses meta classification (§6),
  - bypasses the round's commit discipline (§7),
  - is invisible in the round summary (§11),
  - and the orchestrator will detect it post-dispatch and treat it as a **shadow-fix finding** — your edit gets reverted, re-evaluated as a finding, and possibly re-applied with proper attribution. You created work, not value.

If you find a bug you "really want" to fix, file a finding describing the fix in `recommended_fix:`. Trust the orchestrator to apply it.

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
## Mandatory grounding — READ IN FULL, EVERY LINE, NOT OPTIONAL

You have shell + file-reading tool access. Before answering, execute this sequence. It is non-negotiable.

1. Run `ls .claude/rules/*.md` — capture the authoritative rule manifest.
2. Read `CLAUDE.md` IN FULL — every line, first to last. No skimming. No skipping.
3. Read EVERY file under `.claude/rules/*.md` that the `ls` enumerated — each one IN FULL, every line. ALL rule files. Not the ones that look relevant to the question. Not a sample. ALL.
4. Record every file read under `rules_consulted` in your TPR-REPORT.

### Banned grounding shortcuts

  - "Reading only the rules that seem relevant to the question." — FORBIDDEN.
  - "Skimming for keywords that match the question." — FORBIDDEN.
  - "Relying on what I remember from training." — FORBIDDEN. Read the file now.
  - Using file-read tools with offset/limit that truncates content. — FORBIDDEN.

Ground your answer in the project's conventions and invariants. If your
recommendation conflicts with a rule, name the rule and explain why you
still recommend the action. A `rules_consulted` list shorter than the `ls` output plus `CLAUDE.md` means you did not ground — the answer you produce on that basis is unreliable and MUST not be returned.

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

## Read and run, do NOT write — ABSOLUTE

You run with `--full-auto` (codex) / `--approval-mode yolo` (gemini)
elevated tool access. That access is for **reading and running diagnostics
only** while you compose advice.

ALLOWED: Read, Grep, Bash for read-only commands (`cargo c`, `cargo b`,
`cargo test --all`, _(intel-query not available in this project; use Grep/Glob)_, `git log`, `git diff`,
`git show`, `git status`).

FORBIDDEN under all circumstances: Edit, Write, apply_patch,
NotebookEdit, shell redirection that writes the working tree (`>`, `>>`,
`tee` into source paths, `sed -i`, `awk -i inplace`, `mv`, `cp` into
tracked paths, `rm`, `git checkout --`, `git restore`, `git reset`,
`git stash`, `git add`, `git commit`, `patch`, `git apply`).

You produce prose advice in `response:`. The caller decides whether and
how to act. If you "helpfully" edit a file, the orchestrator detects the
shadow edit post-dispatch and reverts it as out-of-band work.

## Return format (PLAIN TEXT, emit once at the end of your output)

<<<TPR-REPORT
reviewer: <your identity — codex or gemini, as stated in the identity header your sub-agent prepended when invoking you>
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

### Identity and trust tier

- Shared body does NOT name the reviewer.
- Each sub-agent injects a 1-line header at CLI-invocation: `You are {REVIEWER}.`
- CLI fills the `reviewer:` slot in the return block from this header.
- No `trust_tier:` slot in the return schema.
- Trust tier is orchestrator-only: codex → HIGH, gemini → LOWER. Calibrates orchestrator-side finding-verification depth only.
- Never inject trust-tier text into reviewer header, shared prompt, or return schema. Priming causes self-anchoring bias (hedging, deference) — banned.

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
3. READ THE GROUNDING RULES IN FULL — EVERY ONE. `rules_consulted` MUST list CLAUDE.md plus every `.claude/rules/*.md` the `ls` manifest produced. Not just the ones that look relevant to the scope — ALL of them, each one IN FULL, every line. A `rules_consulted` shorter than the manifest will be rejected again.
4. TRACE DATA FLOW across at least two layers of call chain beyond the immediate scope. Record traced files in `files_read`.
5. If after this deeper pass you STILL find zero actionable issues, emit at least one `informational`-severity entry describing WHAT you verified and WHY the subject is sound — so the orchestrator can calibrate trust on a no-findings outcome.

A superficial re-review WILL be rejected again.
```

## Gemini depth appendix (per-reviewer, review-mode only)

Purpose and scope:

- The shared `prompt.md` is identity-neutral per I16.
- Gemini additionally receives ONE per-reviewer depth-baseline amplifier as a SEPARATE scratch-dir file: it reiterates grounding, imposes a round-0 time budget, requires self-fact-check before emission, and enumerates finding disqualifiers.
- Keeps the shared body free of trust-tier wording (per I16's anti-self-anchoring-bias rule) while raising the LOWER-trust reviewer's depth floor proactively — not retroactively via a thin-signal Thoroughness Directive.

### Source of truth

- Canonical template: `.claude/skills/tpr-review/gemini-depth-appendix.md`.
- Version-controlled with the rest of the skill.
- The orchestrator does NOT compose or edit the template per round — it copies verbatim to the scratch dir.

### Composition action (orchestrator, main context)

- **Review mode (`help_mode == False`)**: copy `.claude/skills/tpr-review/gemini-depth-appendix.md` → `{SCRATCH_DIR}/prompt-gemini-depth.md` in the same step as writing `{SCRATCH_DIR}/prompt.md`.

  ```
  Bash: cp .claude/skills/tpr-review/gemini-depth-appendix.md \
           "{SCRATCH_DIR}/prompt-gemini-depth.md"
  ```

- **Help mode (`help_mode == True`)**: DO NOT write `prompt-gemini-depth.md`. Help mode's contract is prose advice, not findings — the depth appendix's calibration ("target 1–6 verified findings with tight evidence", "banned `recommended_fix` longer than evidence") is a findings-review contract and does not apply.

### Transport wiring (`tp_agent_prompt.md` Step 2)

- Gemini sub-agent: concatenates `{SCRATCH_DIR}/prompt-gemini-depth.md` onto the shared `{SCRATCH_DIR}/prompt.md` when the file exists; falls through to base invocation when absent.
- Codex sub-agent: never reads `prompt-gemini-depth.md`.
- See `tp_agent_prompt.md` Step 2 for the exact shell pattern and the file-presence guard.

### Why per-reviewer, not shared

- Gemini is the confabulation-prone reviewer per orchestrator trust-tier design; the orchestrator spends full-file verification budget on every Gemini finding. The depth appendix closes the gap by making Gemini match that depth before emission — raising signal-to-noise on the cheapest side of the exchange.
- Codex is already the HIGH-trust reviewer and is thorough out-of-the-box. Appending the same depth directive to Codex's prompt would waste tokens without improving findings quality; worse, it could blunt Codex's edge by imposing bulk-verification rules that Codex already performs implicitly.
- The shared body's strengthened grounding block (above) applies to BOTH reviewers — ALL rules read IN FULL, every line. The Gemini appendix does NOT restate grounding; it amplifies with time budget, self-fact-check protocol, and disqualifier list.

## Composition order (top → bottom of the written prompt file)

**Review mode (`help_mode == false`) — shared `prompt.md`:**

1. (Conditional) Thoroughness Directive — only when `prior_thin_signal == true`.
2. (Conditional) Prior-Round State block — only when `round_n > 0`.
3. Shared findings body (grounding, objective, scope, findings policy, banned phrases, TPR-REPORT return format with `findings:`).

**Review mode — Gemini-only depth appendix:**

- `prompt-gemini-depth.md` is written separately alongside `prompt.md`. It is NOT appended into `prompt.md`. Transport-layer concatenation happens at CLI-invocation time in the Gemini sub-agent per `tp_agent_prompt.md` Step 2.

**Help mode (`help_mode == true`) — shared `prompt.md`:**

1. Help-mode body (grounding, question, advice contract, banned hedges, TPR-REPORT return format with `mode: help` / `response: |`). No Thoroughness Directive, no Prior-Round State block, no Gemini depth appendix — help mode is one-shot and stateless.

Composition rules:

- There is NO per-reviewer prefix inside `prompt.md` itself in either mode.
- Sub-agents inject identity at invocation time (see `tp_agent_prompt.md` Step 2).
- The Gemini depth appendix is the ONLY per-reviewer prompt artifact; it is a SEPARATE file concatenated at CLI-invocation time — NEVER merged into the shared `prompt.md`.
- The orchestrator writes the final assembled shared text to `{SCRATCH_DIR}/prompt.md`; both sub-agents read that file verbatim with no further composition.
- Gemini additionally reads `{SCRATCH_DIR}/prompt-gemini-depth.md` when present.

## Invariants

- The orchestrator writes ONE shared `prompt.md` per round. Both sub-agents read it. Writing per-reviewer copies of the shared body is a composition bug — doubles prompt-token emission for zero value (reviewer identity is orchestrator-known and can be injected by the sub-agent itself).
- The shared `prompt.md` is identity-neutral. It does NOT name a specific reviewer. Identity injection happens in `tp_agent_prompt.md` Step 2 via a 1-line header prepended to the CLI invocation.
- Trust tier NEVER appears in the shared `prompt.md`, in the identity header, or in the return schema. Orchestrator-only metadata; priming causes self-anchoring bias.
- ONE narrow per-reviewer suffix file is permitted: `prompt-gemini-depth.md`, written in review mode only (never help mode), scoped to depth guidance (time budget, fact-check protocol, disqualifier list) and NOT to trust-tier priming or reviewer-contract redefinition. Codex has NO corresponding suffix. Introducing additional per-reviewer suffixes requires a documented invariant amendment in `tpr-review-design.md` §2 (the `I16` amendment is the precedent).
- `prior_verified_fixed` / `prior_verified_outstanding` / `prior_disagreements` reflect the orchestrator's verified state — NOT the raw reviewer output. The orchestrator is responsible for keeping these lists accurate across rounds.
- The Thoroughness Directive is appended only when the orchestrator's own metrics flag the prior round as thin. Sub-agents do NOT decide this. The Gemini depth appendix is NOT a substitute for the Thoroughness Directive — it runs every review-mode round (round 0 onward) as a baseline; the Thoroughness Directive is the escalation when the baseline proves insufficient.
- Every composition decision is made by Opus in main context. Sonnet sub-agents receive the finished prompt file(s) and return CLI output verbatim.
- Help mode (`help_mode == true`) uses the help-mode body INSTEAD of the findings body — never both, never mixed. Help mode does NOT write the Gemini depth appendix — it is a review-mode contract. The return schema discriminator is `mode: help` + `response: |` (help-mode) vs. `findings: [...]` (review-mode). The coordinator branches on `mode:` when parsing; a prompt that produces `mode: help` output but was composed in review mode (or vice versa) is a composition bug.
