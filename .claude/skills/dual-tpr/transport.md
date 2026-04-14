# Dual-TPR Transport — Wrapper Invocation Pattern

This document specifies the wrapper invocation pattern that all four
dual-source review skill wrappers (Sections 04-07) use to launch
both reviewers and parse their output via the shared transport
utility.

## Wrapper invocation structure

Every dual-source review wrapper follows this pattern:

1. Build the prompt from the user's request + starting packet (scope
   hint, plan section name, recent git activity). The packet is
   INFORMATIONAL, not authoritative — reviewers expand as they see fit.

2. Write the prompts to per-run scratch files:
   - `$RUN/codex.prompt.md` — codex-side prompt
   - `$RUN/gemini.prompt.md` — gemini-side prompt

   The codex and gemini prompts share the same evidence packet but
   differ in their activation preamble (see below).

3. Invoke the transport launcher with retry:
   ```bash
   .claude/skills/dual-tpr/scripts/dual-invoke-with-retry.sh \
       --run "$RUN" \
       --skill {skill-name} \
       --codex-prompt "$RUN/codex.prompt.md" \
       --gemini-prompt "$RUN/gemini.prompt.md" \
       --schema .claude/skills/dual-tpr/findings-schema.json
   ```

4. On success, parse both envelopes (already cached by the transport):
   - `$RUN/codex.envelope.json`
   - `$RUN/gemini.envelope.json`

5. Merge findings with reviewer tagging:
   ```bash
   .claude/skills/dual-tpr/scripts/merge-findings.py \
       --codex "$RUN/codex.envelope.json" \
       --gemini "$RUN/gemini.envelope.json" \
       --section {section-number} \
       --out "$RUN/merged.json"
   ```

6. Write merged findings to the target location (plan section TPR
   block, bug-tracker, or direct presentation to user — depending on
   the wrapper's loop semantics).

## Codex prompt preamble

The codex prompt MUST include the literal keyword `envelope-only`
somewhere in its first 500 characters. This triggers the Step 0 mode
branch in `.codex/skills/review-work/SKILL.md` or
`.codex/skills/review-plan/SKILL.md` and dispatches to envelope-only
mode.

Recommended preamble (first line of the prompt):

    Run the /review-work skill in envelope-only mode. Emit the JSON
    envelope per .claude/skills/dual-tpr/findings-schema.json; do NOT
    write findings to plan files.

(Substitute `review-plan` for `review-work` as appropriate.)

## Gemini prompt preamble — EXPLICIT ACTIVATION REQUIRED

Per Phase 2 empirical research, gemini skills are discovered from
`.gemini/skills/<name>/SKILL.md` but are NOT auto-activated by
description matching. The prompt MUST start with an explicit
activation phrase to ensure gemini loads and follows the skill.

MANDATORY first line of every gemini prompt:

    Activate the review-work skill and follow its instructions exactly.

For plan-review invocations, the mandatory first line is:

    Activate the review-plan skill and follow its instructions exactly.

(Sections 04/05 wrappers use the review-work phrasing; Section 06
review-plan wrapper uses the review-plan phrasing. Both literal
strings are reference templates for wrapper implementation.)

Do NOT rely on gemini noticing the skill on its own — the activation
phrase is load-bearing and MUST be present on every invocation.

## Reviewer Hygiene Preamble (MANDATORY — gemini; recommended — codex)

Beyond the activation phrase, every gemini prompt MUST include hygiene
guidance. Empirically (from the 2026-04-14 §08.5 round-11 TPR run),
gemini without these constraints will (a) dump scratch files in the
repo root, tripping the worktree-guard, and (b) attempt to read
end-to-end multi-thousand-line diff files, stalling past the watchdog
threshold. Codex is less prone to these behaviors but including the
same guidance costs nothing.

MANDATORY gemini hygiene preamble (place between the activation phrase
and the grounding block):

    Do NOT create scratch files (diff.txt, scope_diff.txt, etc.) in
    the repo root — use `/home/eric/.gemini/tmp/ori-term/` or `/tmp`
    for intermediate work. The worktree-guard will fail the round if
    tracked files are modified or untracked files appear in the repo
    root during review.

    Keep review focused. For non-trivial commit ranges:
    - Run `git diff <range> --stat` first to see the file list, NOT
      the full diff. Read targeted hunks via `git show <commit>`
      or ranged file reads.
    - Skip redundant workspace gates — `./build-all.sh`,
      `./clippy-all.sh`, `./test-all.sh`, and
      `cargo build --target x86_64-pc-windows-gnu` have already been
      verified at commit time by lefthook. A `cargo test -p <crate>`
      smoke on the crates the commit touches is sufficient.

**Why this is load-bearing:** the first round-11 iteration consumed
an infra retry because gemini wrote `diff.txt` + `diff_core.txt` +
`scope_diff.txt` to the repo root and the worktree-guard rejected
the envelope. The second iteration's gemini-side stall was caused
by reading a 3500-line diff file across two 2000-line read_file
calls, then running a cargo build that was already verified, then
entering a composition phase that never completed before the 23-min
watchdog fired. Both are fully prevented by the preamble above.

## Plan/Code Consistency Verification (MANDATORY — both reviewers)

Every reviewer prompt MUST ask the reviewer to verify that plan text
describing the changed behavior matches the shipped code. Without this
explicit instruction, reviewers default to code-only review and plan
metadata drift is caught only by side effects (e.g., codex happened to
grep for old strings during expansion). The §08.5 round-11 run
surfaced three separate metadata-drift findings
(`[TPR-08-005/006/007]`) that could have been caught in a single
earlier iteration had the prompt asked.

MANDATORY plan-consistency clause (append to the "What to focus on"
or "Scope" section of every reviewer prompt):

    Plan/code consistency: the commits in scope change observable
    behavior (e.g., remove a feature, flip a semantic, rename an
    invariant). Verify that the owning plan's success criteria,
    subsection titles, N-checklist bullets, resolved-finding notes,
    and any catalog rows the commits cite still describe the CURRENT
    behavior, NOT the superseded one. Historical TPR resolution
    notes may describe superseded intermediate states — those are
    allowed as long as a "Superseded by round N" annotation is
    present. File a finding for any surface that describes behavior
    the code no longer implements.

This clause is independent of the grounding block; grounding scopes
the finding vocabulary, plan-consistency scopes the surface of
review. Both are required.

## Mandatory Grounding Block (both reviewers)

**Every reviewer prompt — codex and gemini — MUST contain a "Grounding
— read these files FIRST" section between the activation preamble and
the scope hint.** The grounding block is identical for both reviewers
and lists the project rule files that scope the finding vocabulary.

Canonical grounding template:

    ## Grounding — read these files FIRST before reviewing

    Before you look at any of the changed code, read these files in
    full so your findings are scoped to the project's actual rules.
    Every finding must use the finding categories and architectural
    vocabulary defined in impl-hygiene.md (LEAK, DRIFT, GAP, WASTE,
    EXPOSURE, BLOAT, NOTE).

    1. CLAUDE.md (project root)
    2. .claude/rules/impl-hygiene.md
    3. .claude/rules/code-hygiene.md
    4. .claude/rules/tests.md
    5. .claude/rules/test-organization.md
    6. .claude/rules/crate-boundaries.md
    7. Any per-crate rule file under .claude/rules/oriterm*.md
       whose `paths:` glob covers the files under review
       (oriterm_core.md, oriterm_ui.md, oriterm_mux.md,
       oriterm_ipc.md, oriterm.md — the live inventory can be
       discovered with `ls .claude/rules/*.md`).

**Why this is load-bearing:** Without grounding, reviewers produce
findings against unknown conventions — generic "this looks odd"
noise instead of precise category-tagged findings that match the
project's actual rules. Grounded reviewers emit findings like
`LEAK:scattered-knowledge at dual-invoke-with-retry.sh:99`; ungrounded
reviewers emit findings like "this function could be clearer".

Wrappers that skip the grounding block should be treated as buggy
and their envelopes treated with extra scrutiny by the consuming
Claude instance.

## Finding Verification Contract (Claude-side)

**Reviewer findings are hypotheses, not facts.** When the wrapper's
consuming Claude instance receives merged findings from the
transport, it MUST independently verify EVERY actionable finding
against the actual code before acting on it — regardless of which
reviewer produced it.

**Trust tiers (set verification depth, not pass/fail):**

- **Codex: HIGH trust.** Citations and line numbers tend to match
  reality. Spot-check each finding: read the cited lines, confirm
  the specific claim, move on if it holds.
- **Gemini: LOWER trust.** More prone to confabulation — invented
  line numbers, misquoted code, reframed-as-finding positive
  observations. Every gemini finding needs FULL verification: read
  the cited file in full, trace the code path end-to-end, confirm
  the claim against what the code actually does.

Both reviewers can be wrong. Agreement amplifies the hypothesis but
does not substitute for verification. The verification step is
codified in `.claude/skills/tpr-review/SKILL.md` §5 "Classify merged
findings (and VERIFY each one independently)" — all consuming
wrappers of this transport must implement an equivalent step.

## Scripts consumed by wrappers

All wrappers consume the same set of transport scripts from Section 02:
- `.claude/skills/dual-tpr/scripts/scratch-dir.sh` — per-run scratch dir
- `.claude/skills/dual-tpr/scripts/dual-invoke-with-retry.sh` — launcher + retry
- `.claude/skills/dual-tpr/scripts/parse-codex.py` — codex parser
- `.claude/skills/dual-tpr/scripts/parse-gemini.py` — gemini parser
- `.claude/skills/dual-tpr/scripts/validate-envelope.py` — standalone validator
- `.claude/skills/dual-tpr/scripts/worktree-guard.sh` — git worktree safety
- `.claude/skills/dual-tpr/scripts/merge-findings.py` — reviewer-tagged merger

See Section 02 (`section-02-transport.md`) for the full scripts contract.

## Failure handling

The transport layer (Section 02) handles infra retries internally —
3 retries per reviewer per round with exponential backoff (1s, 2s, 4s).
After 3 retries, `dual-invoke-with-retry.sh` exits non-zero and prints
the failure category and postmortem directory path.

Wrappers should:
- On success: proceed to parse + merge + write
- On failure: surface the failure category and postmortem path to the
  user via AskUserQuestion, including the `$RUN` directory where the
  JSONL streams and error messages are retained for inspection
- NEVER consume a semantic iteration of the wrapper's outer loop on
  infra failure — the 10-iteration loop is for finding-fixing rounds,
  not transport failures

## Wrapper loop semantics

`/tpr-review` and `/review-work` use the 10-iteration find+fix+rerun
loop. Each iteration:
1. Runs the dual-source transport (both reviewers per round, max
   3 infra retries per reviewer)
2. Claude reads the merged findings
3. If zero actionable findings: clean pass, exit loop
4. Otherwise: Claude fixes findings, commits, re-runs (increment
   semantic iteration counter)
5. After 10 iterations: surface remaining findings to user via
   AskUserQuestion

`/review-plan` does NOT loop — it emits proposed edits once per
invocation. The wrapper applies them (or presents them for user
approval) and does not re-invoke.

`/tp-help` does NOT loop and does NOT use the findings schema — it
emits raw concatenated responses from both reviewers (see Section 07
for the tp-help-specific envelope).
