---
name: tpr-review
description: "Dual-source third-party review (codex + gemini) in parallel, with verification-against-code, iterative fix-and-re-run until both reviewers return clean. Reviews code, plans, skills, docs, or any custom objective. TRIGGER proactively after completing ANY non-trivial work: bug fixes, features, refactors, multi-file changes, compiler changes, codegen changes, test additions, plan implementations, or anything touching correctness-sensitive code. When in doubt, run it."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent, AskUserQuestion, Skill
---

# /tpr-review

Dispatches the **codex** and **gemini** CLIs as parallel sub-agents per round. Reads their plain-text `<<<TPR-REPORT>>>` output in the main context. Verifies every finding against actual code. Fixes accepted findings. Re-runs until both reviewers return clean or a stop gate fires.

## §1 — Invocation

Inspect the first token of `ARGS`:

- empty or `--skill review-work` → **work mode**. Scope: current working-tree changes.
- `--skill review-plan <section-path>` → **plan mode**. Scope: the named plan section. Findings file into the section's `third_party_review` frontmatter per §10.
- any other value → **custom-objective mode**. The entire `ARGS` becomes the review objective.

Ambiguous input → `AskUserQuestion`. Do not guess.

## §2 — Grounding (MANDATORY before round 1)

```
Read: CLAUDE.md
Bash: ls .claude/rules/*.md
Read: every file the ls enumerated
```

The `ls` output is the authoritative rule manifest. Do not hand-select. Reviewers perform their own grounding inside their sub-agents (see `tp_agent_prompt.md`).

## §3 — Spec/grammar gate (MANDATORY, all modes, pre-round-1)

```
Bash: git diff --name-only HEAD -- docs/spec/ docs/spec/grammar.ebnf
```

If the output is **non-empty**, check `docs/ori_lang/proposals/approved/` for a proposal whose body mentions any of the touched files. If no matching proposal is found, emit a synthetic pre-round-0 finding and prepend it to the round-0 verified set:

```
{
  id: "SPEC-GATE-001",
  severity: "critical",
  path: "<first spec/grammar file touched>",
  line: 1,
  title: "Spec/grammar modified without approved proposal",
  evidence: "<git diff output listing all touched spec/grammar files>",
  rule_violated: ".claude/rules/spec.md §Enforcement",
  recommended_fix: "Either revert the spec/grammar changes OR create + approve a proposal under docs/ori_lang/proposals/approved/ and reference it in the commit message"
}
```

The finding is CRITICAL, NOT meta, and verified-by-construction (git diff is ground truth). The user must fix it (revert or approve a proposal) before the round's reviewers can complete.

## §4 — Trust tiers (verification posture)

Every finding — codex or gemini — is verified against actual code before acting. Reviewer claims are hypotheses. Trust tier sets depth.

- **Codex — HIGH trust.** For each codex finding: Read the cited file around the cited line (±20 lines). Confirm the quoted evidence exists verbatim. If it matches, accept for classification (§6). If the quote doesn't match, drop silently.
- **Gemini — LOWER trust.** For each gemini finding: Read the cited file IN FULL. Trace the code path end-to-end. Confirm the claimed behavior matches the code. Drop any finding that fails verification. Gemini URL citations are never authoritative — verify the underlying claim against the actual code.

Verification happens BEFORE classification. An unverified finding never reaches the classifier.

## §5 — Round loop

Two counters bound the loop:

- `iteration_counter` — finding-fixing rounds completed. Cap: **5**.
- `meta_only_streak` — consecutive rounds where every verified finding was meta. Cap: **2**.

Stop conditions (check in order):

1. Both reviewers `status: clean` AND `verified` is empty → exit clean.
2. `meta_only_streak == 2` → exit (juice not worth the squeeze).
3. `iteration_counter == 5` → exit (iteration cap).
4. Both-reviewer failure twice → §9 escalation.

```
iteration_counter = 0
meta_only_streak = 0
last_actionable_count = None
ever_verified_findings = []

while iteration_counter < 5 and meta_only_streak < 2:
    template = Read(".claude/skills/tpr-review/tp_agent_prompt.md")
    codex_prompt  = fill(template, REVIEWER=codex,  TRUST_TIER=HIGH,
                         OBJECTIVE=OBJ, SCOPE=SCOPE)
    gemini_prompt = fill(template, REVIEWER=gemini, TRUST_TIER=LOWER,
                         OBJECTIVE=OBJ, SCOPE=SCOPE)

    [codex_out, gemini_out] = dispatch_parallel(codex_prompt, gemini_prompt)

    codex_report  = parse_tpr_report(codex_out)
    gemini_report = parse_tpr_report(gemini_out)

    if codex_report.status == "failed":  codex_report  = retry_or_survivor(codex)
    if gemini_report.status == "failed": gemini_report = retry_or_survivor(gemini)

    all_findings = codex_report.findings + gemini_report.findings
    verified     = [f for f in all_findings if verify_against_code(f)]  # §4
    meta         = [f for f in verified if classify_meta(f)]             # §6
    actionable   = [f for f in verified if f not in meta]
    ever_verified_findings.extend(verified)                              # §10

    commit_sha = None
    if len(actionable) > 0:
        commit_sha = fix_and_commit(actionable)                          # §7
        meta_only_streak = 0
        last_actionable_count = len(actionable)
    elif len(verified) > 0:
        meta_only_streak += 1
        last_actionable_count = 0

    print_round_summary(iteration_counter, codex_report, gemini_report,
                        verified, meta, actionable, commit_sha)           # §11

    if len(verified) == 0 and codex_report.status == "clean" and gemini_report.status == "clean":
        exit_reason = "clean"
        break

    iteration_counter += 1

else:
    cap_reason = "meta_cap_reached" if meta_only_streak >= 2 else "iter_cap_reached"
    # Cap fired without clean consensus. Escape hatch: ask the user what to do.
    # Prose-options are banned here (§11.5) — use AskUserQuestion with four options.
    # Note: when invoked from /review-plan, the outer dispatcher owns this prompt via
    # its own Escalation handling (see review-plan/step-6-tpr.md Branch 2/3 options);
    # this block runs for STANDALONE invocations of /tpr-review (mode != "review-plan"
    # via /review-plan's Step 6).
    user_choice = AskUserQuestion(
        "Loop exited at {cap_reason} after {iteration_counter} rounds "
        "with {len(ever_verified_findings)} verified findings filed as - [ ] items. "
        "Reviewers are still producing findings (non-clean consensus). How do you want to proceed?",
        options=[
            {"key": "run-more", "label": "Run up to 3 more rounds (extend cap)"},
            {"key": "accept-with-findings",
             "label": "Accept current state — findings stay filed as - [ ] items in "
                      "§NN.R (plan mode) or bug-tracker (non-plan); "
                      "in plan mode also flip reviewed: true with a note explaining the cap exit"},
            {"key": "escalate-to-plan",
             "label": "Escalate outstanding findings to /create-plan (a new plan "
                      "takes ownership of the findings)"},
            {"key": "abort", "label": "Abort — leave everything as-is (reviewed stays false)"},
        ],
    )

    if user_choice.key == "run-more":
        # Extend the caps locally and continue the outer loop from the top.
        iteration_cap_extension = 3
        meta_cap_extension      = 1
        continue_outer_loop(extend_iter=iteration_cap_extension,
                            extend_meta=meta_cap_extension)
    elif user_choice.key == "accept-with-findings":
        exit_reason = f"user_accepted_at_{cap_reason}"
    elif user_choice.key == "escalate-to-plan":
        exit_reason = f"escalated_to_plan_at_{cap_reason}"
        # Parent will invoke /create-plan with ever_verified_findings as mission input.
    else:  # "abort"
        exit_reason = cap_reason  # Preserve the original cap reason; no flip.

emit_final_report(exit_reason, iteration_counter, last_actionable_count)

if mode == "review-plan":
    write_plan_frontmatter(section_path, exit_reason, ever_verified_findings)  # §10
```

**Semantics of the four choices at cap exit:**

| Choice | `exit_reason` | `third_party_review.status` (plan mode) | `reviewed:` flip (plan mode, non-review-plan-parent) |
|---|---|---|---|
| `run-more` | loop continues | — (not yet at exit) | — |
| `accept-with-findings` | `user_accepted_at_{iter\|meta}_cap_reached` | `findings` | `true` (see §10) |
| `escalate-to-plan` | `escalated_to_plan_at_{iter\|meta}_cap_reached` | `escalated` | `false` (the new plan owns the findings) |
| `abort` | `iter_cap_reached` / `meta_cap_reached` | `escalated` | `false` |

The `accept-with-findings` choice is NOT deferral: all findings remain as `- [ ]` items in §NN.R (plan mode) or in the bug-tracker (non-plan modes). The user is acknowledging that rounds are no longer converging AND consciously owning the remaining findings via tracked checkbox items — this is distinct from the banned "pre-existing" / "future improvement" patterns in §7, which dismiss findings WITHOUT filing them. See §7 for the finding-handling contract that still applies.

**When invoked from `/review-plan` (via `step-6-tpr.md`):** this `AskUserQuestion` block is NOT rendered by `/tpr-review` itself — the outer `/review-plan` parent owns the escalation UI per its `review-plan/SKILL.md` §Escalation handling. In that case `/tpr-review` writes its `final-report.json` with `status: max_iterations_reached` (or `max_thoroughness_rejections_reached`), `step-6-tpr.md` translates it into `{RUN_DIR}/tpr.json` (the orchestrator-owned scratch dir `/review-plan` created in its Step 1) with an `escalate: true` + `options` payload, and the parent invokes `AskUserQuestion` there. The flow and semantics are identical; only the owner of the prompt differs.

## §6 — Meta classification (AFTER verification)

A finding is **meta** if and only if ALL of these apply:

- Category is purely wording/phrasing, cosmetic/formatting, already-documented-elsewhere, or an exact duplicate of a prior-round finding.
- Does NOT touch: correctness, invariants (AIMS / SSOT / phase boundaries / registry drift), tests, security, spec conformance, error paths, API contracts, memory safety.
- `recommended_fix` is a pure-doc edit, rename, or whitespace change.

Any doubt → NOT meta.

`meta_only_streak` increments ONLY when the entire verified-findings set of the round is meta. A single non-meta finding resets the streak to 0.

## §7 — Finding-handling policy (ABSOLUTE)

There is NO circumstance under which the orchestrator may dismiss, rationalize, scope-note, or defer an actionable finding. Tied to `CLAUDE.md §The One Rule`.

Valid dispositions:

1. **Fix NOW** — edit in main context, run affected tests, commit via `/commit-push`.
2. **Create a plan and execute it** — if too large for inline fix, run `/create-plan` and implement the resulting sections.
3. **`AskUserQuestion`** — genuinely blocked on a user decision or missing domain knowledge.

**BANNED phrases** (MUST NOT appear in round summaries, commit messages, plan updates):

- "pre-existing" / "was already broken"
- "architectural limitation" / "requires major refactor"
- "out of scope" / "not a §NN deliverable"
- "conservative/safe" / "only precision loss"
- "not a regression" / "not introduced by this work"
- "future improvement" / "tracked for later"
- "known limitation"

The size of the fix is irrelevant. Cross-crate refactoring across 10 files is the work, not a reason to defer.

**`accept-with-findings` at cap exit (§5) is NOT deferral.** The cap-exit escape hatch accepts current state only when (a) every verified finding has already been filed as a `- [ ]` item — in §NN.R for plan mode or the bug-tracker for non-plan modes — and (b) the user made the acceptance choice explicitly via `AskUserQuestion` (not buried in prose). The findings are TRACKED with concrete artifacts that downstream workflows (plan completion checklist, `/review-bugs`) will sweep. This is distinct from dismissing findings with the banned phrases above: dismissal silently drops findings; `accept-with-findings` files them AND records the cap-exit reason in `third_party_review.notes` (§10 step 3a) as a durable audit trail. Plan-mode `reviewed: true` in this case means "the plan's design is validated against implementation; open findings are owned by the plan's own completion gates" — it does NOT mean "zero findings outstanding."

**Filing.** Plan-owned findings → append `- [ ]` items to that section's `## {NN}.R Third Party Review Findings` block, tagged `[TPR-{NN}-{ordinal}-{reviewer}][severity]`. Unowned findings → file to `plans/bug-tracker/` under the appropriate subsystem using `BUG-{NN}-{ordinal}` (no reviewer suffix — reviewer provenance in the body). Agreement findings file ONE bug entry, not two.

## §8 — Parallel dispatch (canonical template)

**Step 8a — Create orchestrator-owned scratch dirs (ONE per reviewer) BEFORE dispatching.** The orchestrator owns these dirs so it can recover reviewer output from disk even when a sub-agent's return message is truncated or the sub-agent auto-backgrounds its internal Bash call. Per-invocation `mktemp -d` prevents cross-session collision (invariant I1). The prefix embeds the **repo name** (basename of the git worktree root) so parallel sessions running in different repos produce visually distinguishable scratch dirs when listing `/tmp/` — e.g., `tpr-round-ori_lang-a1b2c3d4` vs. `tpr-round-warpkit-e5f6g7h8`.

```
Bash: repo="$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")"
Bash: codex_scratch="$(mktemp -d -t "tpr-round-${repo}-XXXXXXXX")"; echo "$codex_scratch"
Bash: gemini_scratch="$(mktemp -d -t "tpr-round-${repo}-XXXXXXXX")"; echo "$gemini_scratch"
```

The `git rev-parse --show-toplevel` fallback to `pwd` covers non-git cwds (should not happen for a `/tpr-review` invocation, but the fallback keeps the command total and makes the template portable).

**Step 8b — Dispatch BOTH reviewers in a SINGLE assistant message.** Foreground only on the Agent call itself — never `run_in_background: true`. Pass the scratch-dir paths from step 8a as the `{SCRATCH_DIR}` placeholder in each prompt.

```
Agent({
  subagent_type: "general-purpose",
  model: "sonnet",
  description: "tpr-review codex reviewer round {N}",
  prompt: <contents of tp_agent_prompt.md with {REVIEWER}=codex,
           {TRUST_TIER}=HIGH, {OBJECTIVE}=<obj>, {SCOPE}=<scope>,
           {SCRATCH_DIR}=<codex_scratch from step 8a>>
})

Agent({
  subagent_type: "general-purpose",
  model: "sonnet",
  description: "tpr-review gemini reviewer round {N}",
  prompt: <contents of tp_agent_prompt.md with {REVIEWER}=gemini,
           {TRUST_TIER}=LOWER, {OBJECTIVE}=<obj>, {SCOPE}=<scope>,
           {SCRATCH_DIR}=<gemini_scratch from step 8a>>
})
```

**Step 8c — Remember the paths.** Keep `codex_scratch` and `gemini_scratch` in orchestrator state across the round so §9 stranded-report recovery can read `$scratch/{REVIEWER}-report.txt` if a sub-agent returns without an inline TPR-REPORT block.

## §9 — Failure handling

**Stranded-report recovery (check BEFORE classifying a reviewer as failed).** A sub-agent's return message is the PRIMARY transport for its TPR-REPORT, but it is not the only one. Before declaring a reviewer `failed`, attempt disk recovery from the scratch dir created in §8 step 8a:

1. Parse the reviewer's return message. If it contains a `<<<TPR-REPORT … TPR-REPORT>>>` block, use it directly — no recovery needed.
2. If the return message has no report block (sub-agent truncated, auto-backgrounded its Bash call, or returned early for any reason), check `$scratch/{REVIEWER}-report.txt` on disk. The sub-agent is contractually required to write this file during Step 3 of `tp_agent_prompt.md`.
3. If the disk file exists and contains a valid sentinel-delimited block, use it AS IF the sub-agent had returned it inline. Log the recovery path in the round summary so the coverage gap stays visible.
4. If the disk file is missing or empty, fall back to `$scratch/{REVIEWER}-stdout.txt` (the teed CLI stdout) and attempt sentinel extraction directly. If that also lacks a sentinel, THEN the reviewer is `failed`.
5. Only after all three recovery paths fail does the reviewer's `status` become `failed` and the retry/survivor policy below applies.

The dual-path transport (inline return + disk persistence) eliminates the "stranded report" failure mode where a fully-completed reviewer CLI invocation produced a valid report that never reached the orchestrator because the sub-agent couldn't return inline. Recovery is NOT a retry — it's reading output that already exists.

**One reviewer `status: failed`.** Retry that reviewer once (single tool call in a follow-up message; partner already completed). If retry fails, **survivor mode**: use only the surviving report, set `survivor_mode: true` in the round summary, continue.

**Both reviewers `status: failed`.** Retry ONCE (parallel dispatch again). If both fail a second time, escalate:

```
AskUserQuestion:
  "Both reviewers failed twice. What should I do?"
    1. Retry once more
    2. Abort this /tpr-review invocation (code unchanged, no findings filed)
    3. Pause here, clear context, resume with /continue-roadmap (fresh session;
       the roadmap picks up where this review was invoked from)
    4. Proceed without review (NOT RECOMMENDED)
```

Never silently exit without producing either a round summary or an escalation.

**Context-pressure pause (mid-loop, optional).** Between rounds, if the session has accumulated substantial context from earlier rounds' findings, verification reads, and fix edits — enough that a fresh session would review better — the orchestrator MAY insert an AskUserQuestion before dispatching the next round:

```
AskUserQuestion:
  "Round {N} complete. Context has grown substantially across {N+1} rounds
   of findings + verification + fixes. Options:"
    1. Continue to round {N+1} in this session
    2. Pause here, clear context, resume with /continue-roadmap (fresh session;
       the roadmap picks up where this review was invoked from)
    3. Stop here and commit current state (findings fixed through round {N};
       remaining loop exits cleanly)
```

Use this proactively, not reactively — by the time the current session is truly exhausted, the user can't cleanly resume. Trigger signals: round count ≥3, context visibly long (multiple rounds of finding tables + fix diffs already rendered), or the reviewers are still returning substantive findings (indicating more work remains).

## §10 — Plan-TPR integration (plan mode only)

After the loop terminates when `ARGS` began with `--skill review-plan <section-path>`:

1. Read the section file's YAML frontmatter.
2. Set `third_party_review.status`:
   - `clean` if `exit_reason == "clean"` and zero verified findings across all rounds.
   - `findings` if any verified findings occurred (even if all were fixed inline), OR if `exit_reason` starts with `user_accepted_at_` (user explicitly accepted the non-converged state; findings remain filed as `- [ ]`).
   - `escalated` if `exit_reason` was `meta_cap_reached`, `iter_cap_reached`, `both_reviewer_failure`, or `escalated_to_plan_at_*` (§5 terminal branches where the user did NOT accept-with-findings).
3. Set `third_party_review.updated` to today's date (YYYY-MM-DD).
3a. **When `exit_reason` starts with `user_accepted_at_`:** also set the section's top-level `reviewed: true` in the same Edit pass, AND append a `third_party_review.notes` line recording the cap type and round count, e.g. `notes: "user-accepted at iter_cap_reached after 5 rounds; 7 findings filed as - [ ] in §NN.R"`. This flip is authoritative — downstream `/review-plan` Step 7+8 MUST no-op when it sees `reviewed: true` already set (see `review-plan/step-7-8-verify.md`). Rationale: the user made an explicit, audited choice that the plan's design is sound despite open findings; those findings are now owned by the plan's own `- [ ]` checklist, not by the review pipeline.
4. For `findings` status — append each accepted finding as `- [ ]` items under the section's `## {NN}.R Third Party Review Findings` block (create the block if missing):

   ```md
   - [ ] `[TPR-{NN}-{ordinal}-{reviewer}][{severity}]` `{path}:{line}` — {title}.
     Evidence: {evidence}
     Impact: {one-line impact summary}
     Required plan update: {recommended_fix}
     Basis: fresh_verification | direct_file_inspection. Confidence: {high|medium|low}.
   ```

   Agreement findings (same location + title from both reviewers) → file BOTH halves with an `Agreement:` cross-reference line pointing at each other. Single-reviewer findings → file ONE entry noting the reviewer.

5. Write via `Edit`. Section `status` stays `in-progress` while `third_party_review.status: findings`.

## §11 — Coordinator rendering (MANDATORY)

After EVERY round, print a round summary as a direct assistant message. Render AFTER `fix_and_commit` (so `Fix commit: {sha}` is populated) but BEFORE any state-branching (exit-clean, meta-cap, iter-cap, round N+1 dispatch).

Required structure:

```md
### Round {N} Summary

**Dispatch**: codex {codex_findings} / gemini {gemini_findings} / survivor_mode: {true|false}
**Verification**: verified {verified_count} / dropped {dropped_count}
**Classification**: actionable {actionable_count} / meta {meta_count}
**Fix commit**: {sha or "none — no actionable findings this round"}

**Findings this round:**
- `[TPR-{NN}-{ordinal}-{reviewer}][severity]` `path:line` — title. Disposition: {fixed in {sha} | handed off to /fix-bug BUG-XX-NNN | classified meta: {reason} | dropped at verification: {reason}}.
- ... one bullet per verified finding (agreement findings produce ONE bullet cross-referencing both reviewer IDs) ...

**Next round will confirm**: {one sentence — what the next round should verify, or "loop exiting {reason}"}.
```

**Rules:**
- Every bullet MUST end with `Disposition:`. A bullet without disposition is a contract violation.
- Agreement findings produce ONE bullet, not two.
- Clean-pass rounds still render the block — `Findings this round:` becomes `(none — both reviewers returned clean)` and `Next round will confirm` becomes `loop exiting clean`.
- Dropped findings appear as `Disposition: dropped at verification: <reason>`.
- Bullets ≤120 characters per line.

## §11.5 — User-interaction discipline (MANDATORY)

**Every user-facing choice point in the coordinator MUST use `AskUserQuestion` — never plain-text numbered options.** Plain prose options force the user to type a full response instead of selecting a structured choice; the harness renders `AskUserQuestion` as tappable options and feeds a clean answer back into the loop. Prose options look identical to narration and invite the user to ignore them as commentary.

**Enumerated choice points in this skill (non-exhaustive — when in doubt, use `AskUserQuestion`):**

1. **Post-round continue/exit inflection** — when the orchestrator renders a round summary and the user could reasonably want to stop before the next round (long-running loop, cost-sensitive session, natural boundary after a large plan revision). The coordinator SHALL emit `AskUserQuestion` with options like: `1. Continue to round N+1 (expected ~{M} min)` / `2. Exit here with current commit(s) clean` / `3. Abort and discard the in-progress state`. Do NOT print these as prose bullets.

2. **Ambiguous input detection** (§1) — if `ARGS` parsing is unclear (mode detection fails, scope can't be inferred), `AskUserQuestion` with candidate mode interpretations. Never guess.

3. **Both-reviewer transport failure second retry** (§9) — already correctly specified via `AskUserQuestion`. Do NOT regress this to prose on refactor.

4. **Spec-gate critical finding resolution** (§3) — when the synthetic SPEC-GATE finding requires a user decision (revert diff vs. approve proposal), use `AskUserQuestion` with `1. Revert the spec diff now` / `2. Pause while I create + approve a proposal` / `3. Cancel this /tpr-review invocation`.

5. **Meta-cap or iter-cap exit escalation** (§5 state machine terminal branches) — when the loop exits without clean consensus, the final-report stage MUST emit the four-option `AskUserQuestion` spelled out in §5's terminal `else` block: `run-more` (extend caps by 3/1), `accept-with-findings` (findings stay filed as `- [ ]`; in plan mode also flip `reviewed: true` per §10 step 3a), `escalate-to-plan` (hand findings to `/create-plan`), `abort` (leave state as-is). Prose options are banned — the harness renders `AskUserQuestion` as tappable options; prose invites the user to skip the prompt entirely. When invoked via `/review-plan`, the outer parent (`review-plan/SKILL.md` §Escalation handling) owns this prompt via `step-6-tpr.md` Branch 2/3 options — `/tpr-review` does not double-prompt in that case. Closed 2026-04-17 — design log entry in `.claude/skills/improve-tooling/tpr-review-design.md` §4.

**Banned pattern**: prose like "1. Continue ... / 2. Exit ... / 3. Abort ..." as bullet text in the assistant message. This looks identical to round-summary prose and bypasses the harness's structured-choice UI.

**Exception**: informational renders (round summaries §11, final reports §5-terminal) remain prose — they describe state, they do not solicit a choice. The distinction: if the next assistant turn depends on which option the user selects, it's a choice point → `AskUserQuestion`. If the assistant will proceed identically regardless of user reaction, it's a summary → prose.

## When to Trigger — Bias Toward Running

Run after ANY of:

- Bug fixes (any severity), new features, refactors, multi-file changes (2+ files).
- Changes to compiler crates (`ori_arc`, `ori_types`, `ori_llvm`, `ori_eval`, `ori_parse`, `ori_patterns`).
- Test matrix additions, stdlib changes, registry changes, diagnostics changes.
- Plan section implementations, docs touching invariants.

**Also run when** unsure whether a change warrants review, the change touches code paths shared across subsystems, or a fix surfaced interfering behavior elsewhere.

**Run with a custom objective when** iterating on any artifact (skill, doc, design) with multi-agent consensus.

**Skip only for** single-line typo fixes, comment edits, or formatting-only changes. When in doubt, run it.
