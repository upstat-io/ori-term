---
name: tpr-review
description: "Dual-source third-party review (codex + gemini) in parallel, with verification-against-code, iterative fix-and-re-run until both reviewers return clean. Reviews code, plans, skills, docs, or any custom objective. TRIGGER proactively after completing ANY non-trivial work: bug fixes, features, refactors, multi-file changes, compiler changes, codegen changes, test additions, plan implementations, or anything touching correctness-sensitive code. When in doubt, run it."
allowed-tools: Read, Write, Edit, Bash, Glob, Grep, Agent, AskUserQuestion, Skill
---

# /tpr-review

Dispatches the **codex** and **gemini** CLIs as parallel sub-agents per round. Orchestrator (Opus, main context) composes the per-round reviewer prompt — tailored to prior rounds' findings to push toward consensus — and writes it to each reviewer's scratch dir BEFORE dispatch. Sub-agents (Sonnet) are thin CLI transports: read the pre-composed prompt, invoke the CLI, `sed`-extract the `<<<TPR-REPORT>>>` block, return ONLY that block (not the full transcript, never translated or reinterpreted). Orchestrator verifies every finding against actual code, fixes accepted findings, and re-runs until both reviewers return clean or a stop gate fires.

**Separation of concerns:** Opus owns all interpretation (prompt composition, finding verification, convergence judgment). Sonnet sub-agents own transport mechanics only (Bash invocation, extraction, byte-identical return). No Sonnet role includes "understanding" review content. See `compose-round-prompt.md` (orchestrator protocol) and `tp_agent_prompt.md` (transport protocol).

## §1 — Invocation

Inspect the first token of `ARGS`:

- empty or `--skill review-work` → **work mode**. Scope: current working-tree changes.
- `--skill review-plan <section-path>` → **plan mode**. Scope: the named plan section. Findings file into the section's `third_party_review` frontmatter per §10.
- any other value → **custom-objective mode**. The entire `ARGS` becomes the review objective.

Ambiguous input — interactive mode: `AskUserQuestion` with mode candidates; do not guess. Autonomous mode: exit immediately with `exit_reason = "autonomous_ambiguous_input"` and a final round summary listing the received `ARGS` verbatim. Never guess and never hang on a prompt no user will answer.

### Composable flags (stripped before mode detection)

ARGS parse order is mandatory: **first strip every composable flag from the ARGS string (setting the corresponding orchestrator state), then apply the mode detection above on the remaining tokens.** Flags can appear anywhere in ARGS — first token, last token, interleaved. Consumers pass each as a separate token (whitespace-delimited); do not accept `--flag=value` forms for boolean flags, do not quote values.

Supported flags:

- `--autonomous` (boolean) — sets `autonomous_mode = True`. See autonomous-mode semantics below.
- `--max-rounds=N` (integer, optional, default `3`) — sets `max_rounds = N`. The `iteration_counter < max_rounds` check in §5 uses this value. Valid range: `1 ≤ N ≤ 10`. Values outside the range are clamped with a warning in the final round summary. The `meta_only_streak` cap stays at `2` regardless of `max_rounds` (it tracks thin-signal convergence, not iteration count).
- `--help-mode` (boolean) — sets `help_mode = True`. Swaps the semantic contract from "find bugs" to "provide advice." When set:
  - `max_rounds` is forced to `1` (no convergence loop; help queries are one-shot consultations). An explicit `--max-rounds=N > 1` is clamped to `1` with a warning in the final summary.
  - §4 verify-against-code is SKIPPED — reviewers return prose advice, not file:line claims to verify.
  - §6 meta classification is SKIPPED — advice is not a "finding."
  - §7 fix-and-commit is SKIPPED — the coordinator never edits code in help mode.
  - §10 plan-TPR integration is SKIPPED — help responses are not filed to plan frontmatter or bug-tracker.
  - §5 cap-exit `AskUserQuestion` is SKIPPED — the coordinator emits a concatenated-responses block and exits.
  - §3 spec-gate still runs (spec governance has no mode exemption; help mode cannot bypass it).
  - §2 grounding still runs (reviewers still `ls .claude/rules/*.md` to ground advice in project conventions).
  - compose-round-prompt.md produces the help-mode body (advice contract + `response: |` return schema) instead of the findings body.
  - The canonical consumer is `/tp-help`, which invokes `/tpr-review --help-mode --max-rounds=1 <question + context>` via the Skill tool and renders the returned concatenated block verbatim.

Canonical use cases:

- Default (3 rounds): `/tpr-review <args>` — normal convergence loop.
- One-shot second opinion: `/tpr-review --max-rounds=1 <custom-objective>` — dispatch both reviewers once, fix any actionable findings, return without a convergence loop.
- Help/advice consultation: `/tpr-review --help-mode --max-rounds=1 <question>` — `/tp-help`'s canonical invocation. Returns raw concatenated reviewer prose; no finding filing, no code edits.
- Nightly unattended: `/tpr-review --autonomous --max-rounds=2 <scope>` — both flags compose; autonomous mode + a tighter cap for faster batches.
- Deep review for correctness-sensitive work: `/tpr-review --max-rounds=6 <scope>` — extended cap up front (equivalent to the interactive "run-more" extension default).

Without the strip-first normalization, custom-objective mode's "any other value → the entire `ARGS` becomes the review objective" rule would absorb `--autonomous` or `--max-rounds=N` into the objective string, leaving the flags ignored. The normalization happens once at §1 parse time; every downstream check reads the normalized state (`autonomous_mode`, `max_rounds`). Autonomous mode replaces every `AskUserQuestion` the coordinator would otherwise emit with a best-judgement auto-decision that emits a distinct `exit_reason` so the calling parent can triage outcomes in bulk. See §5 (cap-exit), §9 (both-reviewer-failure), §3 (spec-gate), and §11.5 (user-interaction discipline) for the per-point auto-decision rules.

**Who passes this flag.** `/tpr-review` is invoked by autonomous parent skills that run without a human at the keyboard (cron-triggered batches, nightly pipelines). The canonical consumer today is `/sync-docs` ("Nightly-ready, fully automated"). Any parent that runs unattended MUST append `--autonomous` to its `ARGS` when invoking `/tpr-review`; conversely, interactive callers (`/review-plan`, `/fix-bug`, direct user invocation) MUST NOT pass the flag, because the best-judgement auto-decisions are strictly worse than a human answer when a human is available.

**What best-judgement means.** Autonomous mode NEVER silently accepts substantive findings, NEVER auto-extends the iteration cap, and NEVER "just proceeds" on a real error. Each auto-decision maps a specific cap/failure condition to a distinct `exit_reason` the parent batch can collect into an end-of-run report. The review still ran, findings still file, downstream workflows still own them — the coordinator just hands the decision latency back to the parent instead of pausing on a prompt no user will ever answer.

## §2 — Grounding (MANDATORY before round 1)

Orchestrator-side grounding is MINIMAL. Reviewer sub-agents self-ground (see `tp_agent_prompt.md`). Orchestrator reads rule files JIT during §4 verification, only the files a finding cites.

```
Read: CLAUDE.md                         # general guardrails
Bash: ls .claude/rules/*.md             # manifest for JIT lookups — do NOT Read upfront
```

Rules:
- Do NOT pre-load rule files beyond `CLAUDE.md`.
- Do NOT hand-select a "core subset" of rules.
- JIT-on-cite: when a finding's `rule_violated` or `evidence` references `.claude/rules/<name>.md`, Read THAT file during §4 verification.
- Full-manifest read is the documented waste pattern (see `tpr-review-design.md §5 — orchestrator rule pre-load regression, 2026-04-19`).

## §3 — Vendored-dep gate (MANDATORY, all modes, pre-round-1)

```
Bash: git diff --name-only HEAD -- crates/vte/ crates/portable-pty/ crates/wgpu-hal/
```

If the output is **non-empty**, a vendored dependency was modified. Emit a synthetic pre-round-0 finding and prepend it to the round-0 verified set:

```
{
  id: "VENDOR-GATE-001",
  severity: "critical",
  path: "<first vendored file touched>",
  line: 1,
  title: "Vendored crate modified without documented rationale",
  evidence: "<git diff output listing all touched vendored files>",
  rule_violated: ".claude/rules/crate-boundaries.md §Vendored crates",
  recommended_fix: "Either revert the vendored change OR document the patch + upstream intent in the commit message, and record the divergence in the corresponding per-crate rules file under .claude/rules/."
}
```

The finding is CRITICAL, NOT meta, and verified-by-construction (git diff is ground truth). The user must fix it (revert or document the patch) before the round's reviewers can complete.

**Autonomous-mode vendored-dep gate handling.** When `autonomous_mode == True`, the gate CANNOT pause for a user decision. If a real VENDOR-GATE finding fires (vendored crate diff touched without documented rationale), exit immediately with `exit_reason = "autonomous_vendor_gate_violation"`, emit a final round summary listing the touched files, and return. The parent batch MUST treat this status as a blocking alarm — never silently proceed, never continue the batch on the violating file. Vendored changes without documentation are correctness-sensitive in a way autonomous mode cannot sign off on.

## §4 — Trust tiers (verification posture)

Every finding — codex or gemini — is verified against actual code before acting. Reviewer claims are hypotheses. Trust tier sets depth.

- **Codex — HIGH trust.** For each codex finding: Read the cited file around the cited line (±20 lines). Confirm the quoted evidence exists verbatim. If it matches, accept for classification (§6). If the quote doesn't match, drop silently.
- **Gemini — LOWER trust.** For each gemini finding: Read the cited file IN FULL. Trace the code path end-to-end. Confirm the claimed behavior matches the code. Drop any finding that fails verification. Gemini URL citations are never authoritative — verify the underlying claim against the actual code.

Verification happens BEFORE classification. An unverified finding never reaches the classifier.

## §5 — Round loop

### Help-mode branch (executes before the main round loop when `help_mode == True`)

When `help_mode == True` (set by `--help-mode` in §1), the coordinator dispatches ONE round, extracts each reviewer's `response:` prose from the TPR-REPORT envelope, and emits a concatenated-responses block. Verification (§4), classification (§6), fix-and-commit (§7), plan-TPR integration (§10), and the cap-exit `AskUserQuestion` (this section's terminal `else`) are ALL skipped — help mode is a one-shot consultation, not a review loop.

```
if help_mode:
    # §2 grounding and §3 spec-gate already ran above — both are mandatory in all modes.
    protocol = Read(".claude/skills/tpr-review/compose-round-prompt.md")
    prompt_text = compose_round(
        protocol,
        round_n = 0,
        objective = OBJ,        # the help question + context, as the custom objective
        scope = SCOPE,           # empty string or "(help mode — no scope)" when caller is /tp-help
        help_mode = True,        # switches compose-round-prompt.md to the help-mode body
        prior_verified_fixed = [], prior_verified_outstanding = [],
        prior_disagreements = [], prior_thin_signal = False,
    )
    Write($scratch + "/prompt.md", prompt_text)
    [codex_out, gemini_out] = dispatch_parallel_thin_transports($scratch)

    codex_report  = parse_tpr_report(codex_out)    # mode: help, status: advice, response: |
    gemini_report = parse_tpr_report(gemini_out)

    # Survivor mode: if one reviewer failed, emit the surviving response with an
    # attribution note for the failed side. Both failed → §9 failure handling
    # (which in help mode exits immediately with no AskUserQuestion — the parent
    # caller /tp-help surfaces the failure in its own wrapper).
    codex_response  = codex_report.response  if codex_report.status  == "advice" else f"(codex failed: {codex_report.summary})"
    gemini_response = gemini_report.response if gemini_report.status == "advice" else f"(gemini failed: {gemini_report.summary})"

    print_help_output(codex_response, gemini_response)   # §11.H rendering
    exit_reason = "help_mode_complete"
    # Return; no iteration counter, no fix-and-commit, no plan frontmatter write.
    return
```

**Help-mode output rendering (§11.H).** The coordinator emits exactly this block and nothing else (no round summary, no findings table):

```md
<!-- TP-HELP BEGIN codex -->

{codex_response verbatim}

<!-- TP-HELP END codex -->

<!-- TP-HELP BEGIN gemini -->

{gemini_response verbatim}

<!-- TP-HELP END gemini -->
```

The attribution-sentinel format matches `/tp-help`'s pre-refactor contract so existing callers (`/fix-bug` Phase 1.75, `/create-plan` Step 6B/8B, `/review-plan` Step 4, proactive auto-triggers) continue to parse the output unchanged.

**Help-mode both-reviewer failure.** If BOTH reviewer reports return `status: failed` in help mode, retry ONCE (re-dispatch parallel per §9's "Both reviewers `status: failed`" policy). If the retry also fails: in interactive mode, emit the concatenated failure block (both sides show `(codex failed: …)` / `(gemini failed: …)`) and return — the caller (`/tp-help`) surfaces the failure to its own consumer per its §5 escalation path. Do NOT invoke the §9 `AskUserQuestion` — help mode's contract is "return raw responses, even if both are failures." In autonomous mode, exit with `exit_reason = "help_mode_transport_failure"`.

### Main round loop (executes when `help_mode == False`)

Two counters bound the loop:

- `iteration_counter` — finding-fixing rounds completed. Cap: **`max_rounds`** (default `3`, overridable via `--max-rounds=N` flag in §1; valid range 1-10). On resume, callers MAY pass `--resume-from-rounds=N` (integer, default `0`) to initialize `iteration_counter = N` — used by `/review-plan` Step 6 to honor `rounds_completed` read from the plan's `review_pipeline:` marker. Valid range: `0 ≤ N < max_rounds` (clamped with warning on out-of-range).
- `meta_only_streak` — consecutive rounds where every verified finding was meta. Cap: **2** (always; not user-configurable).

**Exit-surface contract (§5.E) — MANDATORY on every terminal exit.** When control returns to a main-context caller (inline invocation via Skill tool, e.g. `/review-plan` Step 6), surface this structured state alongside `exit_reason`:

| Field | Value | Required on |
|---|---|---|
| `exit_reason` | `clean` / `iter_cap_reached` / `meta_cap_reached` / `user_accepted_at_*` / `autonomous_accept_at_*` / `user_pause_and_resume` / `both_reviewer_failure` / `escalated_to_plan_at_*` / `autonomous_exit_substantive_at_*` / `autonomous_transport_failure` / `autonomous_spec_gate_violation` / `autonomous_ambiguous_input` | every exit |
| `rounds_completed` | final value of `iteration_counter` at exit | every exit |
| `last_round_commit` | SHA from most recent `fix_and_commit` in the loop (`None` if no round produced actionable fixes) | every exit where ≥1 round ran |
| `last_round_findings` | `len(verified)` from most recent round | every exit where ≥1 round ran |
| `ever_verified_findings` | accumulated list across all rounds (§10 uses this for plan filing) | every exit |

Callers consume this to update external state (e.g. `/review-plan` writes `rounds_completed` / `last_round_commit` / `last_round_findings` into the plan's `review_pipeline:` marker per `review-plan/SKILL.md §Step 1d` dispatch table). Help mode skips this surface (it does not iterate).

Stop conditions (check in order):

1. Both reviewers `status: clean` AND `verified` is empty → exit clean.
2. `meta_only_streak == 2` → exit (juice not worth the squeeze).
3. `iteration_counter == max_rounds` → exit (iteration cap) → pause and give a full findings report via `AskUserQuestion` with an adaptive recommendation (see terminal `else:` block below). When `max_rounds == 1`, this fires after the first round with findings — the caller explicitly requested one-shot semantics, so there's no convergence loop; findings get fixed if possible, then cap-exit.
4. Both-reviewer failure twice → §9 escalation.

```
iteration_counter = 0
meta_only_streak = 0
last_actionable_count = None
ever_verified_findings = []

# Prior-round state tracked by the orchestrator across rounds —
# used by compose-round-prompt.md to tailor each subsequent round's
# prompt so reviewers don't re-raise fixed findings and sharpen focus
# on the areas where the prior round disagreed.
prior_verified_fixed = []        # findings verified + fixed in prior rounds (don't re-raise)
prior_verified_outstanding = []  # findings verified but filed as - [ ] (known, tracked)
prior_disagreements = []         # findings raised by ONE reviewer only (cross-check next round)
prior_thin_signal = False        # prior round was zero-finding but asymmetric depth

while iteration_counter < max_rounds and meta_only_streak < 2:
    # ── Opus-owned prompt composition (main context) ───────────────────
    # The orchestrator reads compose-round-prompt.md and composes ONE
    # shared, identity-neutral prompt per round. Sub-agents do NOT compose
    # prompts; they inject their own 2-line identity header at CLI-invocation
    # time (see tp_agent_prompt.md Step 2). One Write per round, not two.
    protocol = Read(".claude/skills/tpr-review/compose-round-prompt.md")
    prompt_text = compose_round(
        protocol,
        round_n = iteration_counter,
        objective = OBJ,
        scope = SCOPE,
        prior_verified_fixed = prior_verified_fixed,
        prior_verified_outstanding = prior_verified_outstanding,
        prior_disagreements = prior_disagreements,
        prior_thin_signal = prior_thin_signal,
    )
    Write($scratch + "/prompt.md", prompt_text)

    # ── Parallel dispatch (thin-transport sub-agents, Sonnet) ──────────
    # Sub-agents are pure CLI wrappers. They read $SCRATCH_DIR/prompt.md
    # (same shared file for both), prepend a 2-line identity header,
    # invoke the CLI, sed-extract the <<<TPR-REPORT>>> block, and return
    # ONLY that block — never the full CLI transcript. No translation,
    # no reinterpretation. See tp_agent_prompt.md.
    [codex_out, gemini_out] = dispatch_parallel_thin_transports($scratch)

    # ── Shadow-edit absorption (§8e) ──────────────────────────────────
    # Reviewer CLIs run with file-write authority (--full-auto / yolo)
    # so they can run cargo test / cargo test --all against actual behavior.
    # The prompt forbids file edits, but prompts can be ignored — this
    # step enforces the contract by diffing the working tree against the
    # pre-dispatch snapshot taken in §8 step 8a. Any shadow edits become
    # SHADOW-FIX-{ordinal} synthesized findings prepended to the verified
    # set BEFORE reviewer findings flow through §4. NEVER discard via
    # bulk git checkout/restore/reset — would clobber the user's
    # parallel-session work (per CLAUDE.md "Never destructive git").
    shadow_findings = detect_and_synthesize_shadow_edits($scratch)  # §8e

    codex_report  = parse_tpr_report(codex_out)
    gemini_report = parse_tpr_report(gemini_out)

    if codex_report.status == "failed":  codex_report  = retry_or_survivor(codex)
    if gemini_report.status == "failed": gemini_report = retry_or_survivor(gemini)

    all_findings = shadow_findings + codex_report.findings + gemini_report.findings
    verified     = [f for f in all_findings if verify_against_code(f)]  # §4
    meta         = [f for f in verified if classify_meta(f)]             # §6 (shadow_findings NEVER classified as meta — see §8e)
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

    # ── Update prior-round state for the NEXT round's prompt ───────────
    # compose-round-prompt.md reads these on the next iteration.
    prior_verified_fixed.extend(
        [{"path": f.path, "line": f.line, "title": f.title, "commit": commit_sha}
         for f in actionable]
    )
    prior_verified_outstanding.extend(
        [{"path": f.path, "line": f.line, "title": f.title}
         for f in meta]  # meta findings aren't fixed inline; they're filed
    )
    prior_disagreements = findings_raised_by_one_reviewer_only(
        codex_report.findings, gemini_report.findings, verified
    )
    prior_thin_signal = (
        len(verified) == 0
        and has_asymmetric_depth(codex_report, gemini_report)
    )

    print_round_summary(iteration_counter, codex_report, gemini_report,
                        verified, meta, actionable, commit_sha)           # §11

    if len(verified) == 0 and codex_report.status == "clean" and gemini_report.status == "clean":
        exit_reason = "clean"
        break

    iteration_counter += 1

else:
    cap_reason = "meta_cap_reached" if meta_only_streak >= 2 else "iter_cap_reached"

    # ── Adaptive recommendation ──────────────────────────────────────────
    # Cap fired without clean consensus. Pause and give the user a FULL
    # findings report with an Opus-judged recommendation. Classify the
    # remaining (unfixed) verified findings:
    #   - "meta" = wording/phrasing/line-number/spelling/already-documented,
    #              nothing code-related or correctness-sensitive (see §6).
    #   - "substantive" = anything touching correctness, invariants, tests,
    #              security, API contracts, memory safety.
    # Recommendation:
    #   - If ALL remaining findings are meta → recommend accept-with-findings
    #     (more rounds won't change anything important).
    #   - If ANY remaining finding is substantive → recommend run-more
    #     (there is real unresolved signal worth another round to converge).
    remaining = [f for f in ever_verified_findings if f not in prior_verified_fixed]
    remaining_meta        = [f for f in remaining if classify_meta(f)]
    remaining_substantive = [f for f in remaining if f not in remaining_meta]
    all_meta = len(remaining) > 0 and len(remaining_substantive) == 0

    # Build the full findings report to render INSIDE the AskUserQuestion
    # question body. Prose-options are banned (§11.5) — use structured options.
    report = render_full_findings_report(
        remaining_substantive,   # rendered first — most important
        remaining_meta,          # rendered after — context
        iteration_counter,       # how many rounds ran
        cap_reason,              # which cap fired
    )

    # Build the option definitions. Each key has a stable description;
    # (Recommended) label + recommended:true flag get attached below, and
    # the recommended option is always moved to index 0 per
    # .claude/rules/ask-user-question.md §The Rule.
    # Each option carries a stable neutral body describing WHAT the option
    # does (side-effects, downstream behavior). The recommendation RATIONALE
    # is computed separately below and prepended only to the recommended
    # option's description, so stable bodies MUST NOT contain "Recommended
    # because ..." phrasing (otherwise the dynamic prepend duplicates it).
    # Labels include the concrete side-effect per I15 (cap-exit accept path
    # must communicate the reviewed:true flip explicitly).
    option_defs = {
        "accept-with-findings": {
            "key": "accept-with-findings",
            "label": "Accept with findings filed, flip reviewed: true (plan mode)",
            "description": (
                "Findings stay tracked as - [ ] items (§NN.R in plan mode, "
                "bug-tracker otherwise); in plan mode the flip lands with a "
                "third_party_review.notes line recording the cap-exit reason. "
                "Not deferral — the plan's own completion gates own the open "
                "findings per §7."
            ),
        },
        "run-more": {
            "key": "run-more",
            "label": "Run up to 3 more rounds (extend cap)",
            "description": (
                "Extends the iteration cap by 3 rounds and the meta-cap by 1 "
                "and re-enters the round loop. Reviewer wall-clock on the "
                "order of ~20–40 min per extension round."
            ),
        },
        "escalate-to-plan": {
            "key": "escalate-to-plan",
            "label": "Escalate outstanding findings to /create-plan",
            "description": "Opens a new plan that takes ownership of the "
                           "findings. Best when residual findings are "
                           "structural/architectural rather than local.",
        },
        "abort": {
            "key": "abort",
            "label": "Abort — leave everything as-is",
            "description": "Leaves reviewed: false with no follow-up anchor. "
                           "Equivalent to silent deferral.",
        },
    }

    # Pick the recommended key, build a rationale that explains WHY it is
    # recommended (never redundant with the stable body), prepend it, add
    # the (Recommended) marker + flag, and move the option to index 0 per
    # .claude/rules/ask-user-question.md §The Rule.
    recommended_key = "accept-with-findings" if all_meta else "run-more"
    rec = option_defs.pop(recommended_key)
    rec["label"] = rec["label"] + " (Recommended)"
    rationale = (
        "Recommended because every remaining finding is meta "
        "(wording/phrasing/line-number/spelling, nothing code-related), so "
        "more rounds will not surface new signal. "
        if all_meta else
        f"Recommended because {len(remaining_substantive)} substantive "
        "finding(s) remain that touch correctness or invariants — a 4th "
        "round has a reasonable chance of converging them. "
    )
    rec["description"] = rationale + rec["description"]
    rec["recommended"] = True
    # Stable tail order for the non-recommended options (max 4 total per
    # the AskUserQuestion schema; recommended-option-at-index-0 preserved).
    tail_order = [k for k in ["run-more", "accept-with-findings",
                              "escalate-to-plan", "abort"]
                  if k != recommended_key]

    if autonomous_mode:
        # No user prompt — substitute best-judgement auto-decision.
        # Rules:
        #   all_meta       → accept (same as interactive (Recommended))
        #   substantive    → exit with distinct status so the parent batch
        #                    can collect + report; do NOT auto-accept
        #                    (would mask real issues across a nightly batch)
        # run-more and escalate-to-plan are NEVER auto-selected:
        #   run-more       → autonomous mode is cap-bounded; extending
        #                    requires a user
        #   escalate-to-plan → requires /create-plan (interactive)
        auto_decision = "accept-with-findings" if all_meta else "exit-substantive"
        class _AutoChoice:
            pass
        user_choice = _AutoChoice()
        user_choice.key = auto_decision
    else:
        user_choice = AskUserQuestion(
            question = (
                f"Loop exited at {cap_reason} after {iteration_counter} rounds. "
                f"{len(remaining_substantive)} substantive finding(s) and "
                f"{len(remaining_meta)} meta finding(s) remain unresolved.\n\n"
                f"FULL FINDINGS REPORT:\n\n{report}\n\n"
                f"My recommendation: "
                + (
                    "accept-with-findings — remaining findings are all meta "
                    "(wording/phrasing/line-number/spelling, no code correctness). "
                    "More rounds will not produce new signal."
                    if all_meta else
                    f"run-more — {len(remaining_substantive)} substantive "
                    f"finding(s) remain that touch correctness or invariants. "
                    f"A 4th round has a reasonable chance of converging."
                )
                + "\n\nHow do you want to proceed?"
            ),
            options = [rec] + [option_defs[k] for k in tail_order],
        )

    if user_choice.key == "run-more":
        # Extend by 3 more rounds and continue the outer loop from the top.
        # NEVER reachable in autonomous_mode — autonomous auto-decisions pick
        # only accept-with-findings or exit-substantive.
        iteration_cap_extension = 3
        meta_cap_extension      = 1
        continue_outer_loop(extend_iter=iteration_cap_extension,
                            extend_meta=meta_cap_extension)
    elif user_choice.key == "accept-with-findings":
        # Interactive: user accepted. Autonomous: auto-accept (all_meta branch).
        # Both produce user_accepted_at_* so §10 applies the same plan-mode
        # reviewed:true flip + notes line.
        exit_reason = (f"autonomous_accept_at_{cap_reason}"
                       if autonomous_mode
                       else f"user_accepted_at_{cap_reason}")
    elif user_choice.key == "exit-substantive":
        # Autonomous-only branch. Substantive findings remain at cap exit,
        # but auto-accepting would mask them across a nightly batch.
        # Exit cleanly with a distinct status; findings are already filed
        # as - [ ] items (§NN.R plan mode or bug-tracker) per §7. Parent
        # batch collects findings and reports at end-of-run.
        exit_reason = f"autonomous_exit_substantive_at_{cap_reason}"
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

**When invoked from `/review-plan` (via `step-6-tpr.md`):** this `AskUserQuestion` block is NOT rendered by `/tpr-review` itself — the outer `/review-plan` parent owns the escalation UI per its `review-plan/SKILL.md` §Escalation handling. Because Step 6 of `/review-plan` runs inline in main context (per `review-plan-design.md §2`), there is no file-based handoff from `/tpr-review`: this skill's orchestrator exits §5 with its terminal `exit_reason` (one of `iter_cap_reached`, `meta_cap_reached`, etc.) directly observable to the main-context caller. `step-6-tpr.md` captures that state into `{RUN_DIR}/tpr.json` with an `escalate: true` + `options` payload and the parent invokes `AskUserQuestion` there. The flow and semantics are identical; only the owner of the prompt differs.

## §6 — Meta classification (AFTER verification)

A finding is **meta** if and only if ALL of these apply:

- Category is purely wording/phrasing, cosmetic/formatting, already-documented-elsewhere, or an exact duplicate of a prior-round finding.
- Does NOT touch: correctness, invariants (SSOT / module boundaries / frame-budget / damage-region / VT-conformance), tests, security, protocol conformance, error paths, API contracts, memory safety.
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

**Step 8a — Create the orchestrator-owned scratch dir AND snapshot the working tree state (ONE per round, shared by both reviewer sub-agents) BEFORE dispatching.** The orchestrator owns this dir so it can recover reviewer output from disk even when a sub-agent's return message is truncated or the sub-agent auto-backgrounds its internal Bash call. Per-invocation `mktemp -d` prevents cross-session collision (invariant I1). Output files inside the shared dir are namespaced by reviewer (`codex-stdout.txt` / `gemini-stdout.txt` / `codex-report.txt` / `gemini-report.txt` / `codex-stderr.txt` / `gemini-stderr.txt`) so both sub-agents can write into the same dir without collision. The prefix embeds the **repo name** (basename of the git worktree root) so parallel sessions running in different repos produce visually distinguishable scratch dirs when listing `/tmp/` — e.g., `tpr-round-ori_lang-a1b2c3d4` vs. `tpr-round-warpkit-e5f6g7h8`.

The pre-dispatch snapshot (`pre-dispatch-status.txt` + `pre-dispatch-head.txt`) is the baseline §8e diffs against to detect any source-tree edits made by the reviewer CLIs (codex `--full-auto` and gemini `--approval-mode yolo` both have file-write authority — the prompt forbids it per `compose-round-prompt.md` "Read and run, do NOT write" but prompts can be ignored; §8e is the enforcement).

```
Bash: repo="$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")"
Bash: scratch="$(mktemp -d -t "tpr-round-${repo}-XXXXXXXX")"; echo "$scratch"
Bash: git status --porcelain > "$scratch/pre-dispatch-status.txt"
Bash: git rev-parse HEAD > "$scratch/pre-dispatch-head.txt"
```

The `git rev-parse --show-toplevel` fallback to `pwd` covers non-git cwds (should not happen for a `/tpr-review` invocation, but the fallback keeps the command total and makes the template portable).

**Step 8b — Compose the per-round reviewer prompt (orchestrator-owned, main context / Opus).** Sub-agents DO NOT compose prompts. Read `.claude/skills/tpr-review/compose-round-prompt.md` inline and follow its protocol to produce the shared prompt file on disk BEFORE dispatching. Additionally, in review mode only (not help mode), copy the canonical Gemini depth appendix to the scratch dir so the Gemini sub-agent can concatenate it at CLI-invocation time:

```
Write: $scratch/prompt.md

# Review mode only — skip in help mode:
Bash: cp .claude/skills/tpr-review/gemini-depth-appendix.md \
         "$scratch/prompt-gemini-depth.md"
```

The shared `prompt.md` is identity-neutral — it does not name a specific reviewer and does not state a trust tier. Both sub-agents read the same file. Each sub-agent prepends its own 1-line identity header (`You are {REVIEWER}.`) when invoking its CLI, per `tp_agent_prompt.md` Step 2. Trust tier is orchestrator-only metadata (codex → HIGH, gemini → LOWER); it NEVER ships to the reviewer, header, prompt, or return schema — priming causes self-anchoring bias. Writing ONE shared prompt per round (instead of one per reviewer) halves orchestrator output tokens and restores the "Opus composes ONCE" principle.

The Gemini depth appendix `prompt-gemini-depth.md` is the single permitted per-reviewer prompt artifact — it is a SEPARATE file (never merged into the shared `prompt.md`) and carries depth guidance only (time budget, self-fact-check protocol, disqualifier list). See `compose-round-prompt.md §Gemini depth appendix` for the contract and `tp_agent_prompt.md` Step 2 for the transport-layer concatenation.

For rounds N>0, the orchestrator prepends a Prior-Round State block (findings already fixed, findings filed as `- [ ]`, single-reviewer findings to cross-check) and — if the prior round was thin — a Thoroughness Re-review Directive above the shared body. See `compose-round-prompt.md` for the exact template and prepend order.

This is editorial work — it requires judgment about what the prior round found and where reviewers should sharpen focus this round. Keeping it in main context (Opus) keeps the judgment at the right model tier. Pushing it into a Sonnet sub-agent loses round-over-round convergence pressure.

**Step 8c — Dispatch BOTH reviewers in a SINGLE assistant message.** Foreground only on the Agent call itself — never `run_in_background: true`. Both sub-agents get the SAME `{SCRATCH_DIR}` (the per-round shared dir from step 8a); they differ only in their `{REVIEWER}` identity value. Trust tier is orchestrator-only metadata (codex HIGH, gemini LOWER) and MUST NOT appear as a placeholder, in the header, in the prompt, or in the return schema — per invariant I16. Sub-agents are thin CLI transports — they read `$SCRATCH_DIR/prompt.md` (which you wrote in step 8b), prepend their 1-line identity header (`You are {REVIEWER}.`), and — if reviewer is gemini AND `$SCRATCH_DIR/prompt-gemini-depth.md` exists — concatenate that depth appendix after the shared prompt. Then invoke the CLI, `sed`-extract the TPR-REPORT block, and return ONLY that block. They do NOT translate, reinterpret, or summarize findings. See `tp_agent_prompt.md`.

```
Agent({
  subagent_type: "general-purpose",
  model: "sonnet",
  description: "tpr-review codex reviewer round {N}",
  prompt: <contents of tp_agent_prompt.md with {REVIEWER}=codex,
           {SCRATCH_DIR}=<scratch from step 8a>>
})

Agent({
  subagent_type: "general-purpose",
  model: "sonnet",
  description: "tpr-review gemini reviewer round {N}",
  prompt: <contents of tp_agent_prompt.md with {REVIEWER}=gemini,
           {SCRATCH_DIR}=<scratch from step 8a>>
})
```

Note: the sub-agent prompt carries ONLY `{REVIEWER}` and `{SCRATCH_DIR}` placeholders. `{OBJECTIVE}` / `{SCOPE}` live in `$SCRATCH_DIR/prompt.md`, which the orchestrator wrote in step 8b. `{TRUST_TIER}` is orchestrator-only and never reaches the sub-agent prompt (I16).

**Step 8d — Remember the path.** Keep the single `scratch` value in orchestrator state across the round so §9 stranded-report recovery can read `$scratch/codex-report.txt` or `$scratch/gemini-report.txt` if a sub-agent returns without an inline TPR-REPORT block. Both sub-agents write into the same scratch dir with per-reviewer file namespacing (`{codex,gemini}-{stdout,stderr,report}.txt`) so there is no collision. The scratch-dir inventory for a review-mode round is:

  - `prompt.md` — shared, identity-neutral, read by BOTH sub-agents.
  - `prompt-gemini-depth.md` — Gemini-only depth appendix (review mode only; absent in help mode).
  - `pre-dispatch-status.txt` / `pre-dispatch-head.txt` / `post-dispatch-status.txt` / `shadow-edit-diff.txt` / `shadow-edit-stat.txt` / `shadow-edits.patch` — §8a/§8e shadow-edit detection artifacts.
  - `{codex,gemini}-{stdout,stderr,report}.txt` — per-reviewer CLI capture artifacts.

**Step 8e — Detect shadow edits (post-dispatch, BEFORE §4 verification).** Reviewer CLIs run with file-write authority (`codex --full-auto`, `gemini --approval-mode yolo` — both required so reviewers can run `cargo test` / `cargo test --all` / _(intel-query not available in this project; use Grep/Glob)_ to verify their findings against actual behavior). The prompt forbids file edits per `compose-round-prompt.md` "Read and run, do NOT write", but prompts can be ignored. This step enforces the contract.

```
Bash: git status --porcelain > "$scratch/post-dispatch-status.txt"
Bash: diff -u "$scratch/pre-dispatch-status.txt" "$scratch/post-dispatch-status.txt" > "$scratch/shadow-edit-diff.txt" || true
Bash: git diff --stat > "$scratch/shadow-edit-stat.txt"
```

If `$scratch/shadow-edit-diff.txt` is empty AND `git status --porcelain` matches the pre-dispatch snapshot byte-for-byte, no shadow edits occurred. Continue to §4.

If shadow edits are detected, the orchestrator MUST absorb them through the documented pipeline — they are NEVER silently kept and NEVER destructively discarded (the user runs parallel sessions with uncommitted work; `git checkout --` / `git restore` / `git reset --hard` are forbidden per CLAUDE.md "Never destructive git").

**Absorption protocol:**

1. **Capture the diff** to `$scratch/shadow-edits.patch` for forensic review:
   ```
   Bash: git diff > "$scratch/shadow-edits.patch"
   ```

2. **Synthesize a SHADOW-FIX finding** for each modified file and PREPEND it to the round's verified set BEFORE §4 runs against the reviewer-emitted findings. The finding is verified-by-construction (the diff is ground truth). It is NEVER meta — shadow edits ALWAYS bypass §4-§7 and ALWAYS warrant orchestrator review:

   ```
   {
     id: "SHADOW-FIX-{ordinal}",
     severity: "high",
     path: "<modified file>",
     line: 1,
     title: "Reviewer made out-of-band edit bypassing TPR pipeline",
     evidence: "<truncated git diff for this file, <= 20 lines>",
     rule_violated: "compose-round-prompt.md §Read and run, do NOT write — ABSOLUTE",
     recommended_fix: "Orchestrator: review the diff. If correct → keep + commit with shadow-fix attribution. If wrong or out-of-scope → revert this file's hunks via `git checkout -- <file>` (scoped to ONLY the shadow-edited file paths in the diff, NEVER `git checkout -- .`). Either way, classify as actionable and feed through §7 commit discipline."
   }
   ```

3. **Run §4 verification on the shadow-fix finding.** The diff is ground truth — verification is automatic — but the orchestrator still inspects the diff content to decide keep-or-revert. Reading the diff is mandatory; trusting the reviewer's judgment is banned.

4. **Decide keep-or-revert per shadow-edited file:**
   - **Keep**: the edit is correct AND in scope for the review objective AND would have been an accepted finding's `recommended_fix` anyway. The orchestrator treats this as the round's fix for that finding (no double-fix in §7 for the same file). Attribution: prepend `Shadow-fix from {reviewer} reviewer (absorbed by orchestrator):` to the commit message body for that file's hunks.
   - **Revert**: the edit is wrong OR out-of-scope OR speculative OR breaks other tests. Revert ONLY the shadow-edited file paths via `git checkout -- <enumerated paths>` (NEVER bulk `git checkout -- .` — would clobber the user's parallel-session work). The shadow-fix finding stays in the round summary as `Disposition: shadow-edit reverted: <reason>`.
   - **Re-apply differently**: the intent is correct but the implementation is wrong. Revert the shadow edit (per Revert above), then apply the orchestrator's correct version through §7 normally. Cite both in the round summary.

5. **The shadow-fix findings appear in the round summary (§11)** with explicit `[SHADOW-FIX-N]` IDs and per-file dispositions. They count toward `actionable` for `meta_only_streak` accounting (a round with shadow edits is NEVER a "meta-only" round — by definition the reviewer wrote code).

6. **Append a §11 trailing line** for any round where shadow edits occurred: `**Shadow edits**: {N} file(s) absorbed (kept: {K}, reverted: {R}, re-applied: {A}). Patch archived at $scratch/shadow-edits.patch.`

This protocol turns a contract violation into a documented contribution: the user always sees what the reviewer did, the orchestrator always owns the decision, and `git checkout -- <enumerated paths>` is the ONLY destructive operation permitted (scoped to known shadow-edited paths, NEVER bulk).

## §9 — Failure handling

**Liveness probe (check BEFORE stranded-report recovery, BEFORE retry, BEFORE abort).** Silence from a reviewer sub-agent is ambiguous — it can mean the CLI hung OR that the reviewer is mid-investigation and waiting on a slow Bash / Read / Grep it dispatched. Treating every silence as death biases the orchestrator toward shallow reviewer work and kills agents doing deep analysis. Before any retry/abort decision, consult the liveness probe:

```
Bash (foreground, timeout: 30000):
  diagnostics/tpr-liveness.sh "$scratch" codex --human
  diagnostics/tpr-liveness.sh "$scratch" gemini --human
```

The probe returns a three-state verdict by inspecting `$scratch/{REVIEWER}-stdout.txt` (the tee'd CLI output guaranteed by invariant I14):

- **`alive` (exit 0)** — the reviewer is still emitting, OR its tail contains a `tool_call` / `thinking` / `<<<TPR-REPORT` marker indicating in-flight work. DO NOT retry, DO NOT abort, DO NOT dispatch a replacement Agent. Wait at least one more probe cycle (~60-120s) before reassessing.
- **`quiet` (exit 1)** — borderline; the file grew within the last `[grace, 2*grace)` window (default `[5min, 10min)`) OR an `[2*grace, 4*grace)` window with an in-flight `tool_call` signal (slow Bash suspected, e.g. `cargo build`, `cargo test --all`). Consult the probe once more ~60s later before deciding. Do NOT retry on the first `quiet` verdict.
- **`dead` (exit 2)** — no emission for >= 2*grace seconds AND no strong alive signal in the tail. The CLI is genuinely hung or crashed. Stranded-report recovery (below) and the retry/survivor policy apply.

The probe's 45-min worst-case ceiling is bounded externally by `block-banned-commands.sh` (upper-bound `timeout` on `codex exec` / `gemini -p`). The probe never extends that ceiling — it only prevents premature retry within it.

**Stranded-report recovery (check AFTER the liveness verdict classifies a reviewer as `dead`, BEFORE classifying as failed).** A sub-agent's return message is the PRIMARY transport for its TPR-REPORT, but it is not the only one. Before declaring a reviewer `failed`, attempt disk recovery from the scratch dir created in §8 step 8a:

1. Parse the reviewer's return message. If it contains a `<<<TPR-REPORT … TPR-REPORT>>>` block, use it directly — no recovery needed.
2. If the return message has no report block (sub-agent truncated, auto-backgrounded its Bash call, or returned early for any reason), check `$scratch/{REVIEWER}-report.txt` on disk. The sub-agent is contractually required to write this file during Step 3 of `tp_agent_prompt.md`.
3. If the disk file exists and contains a valid sentinel-delimited block, use it AS IF the sub-agent had returned it inline. Log the recovery path in the round summary so the coverage gap stays visible.
4. If the disk file is missing or empty, fall back to `$scratch/{REVIEWER}-stdout.txt` (the teed CLI stdout) and attempt sentinel extraction directly. If that also lacks a sentinel, THEN the reviewer is `failed`.
5. Only after all three recovery paths fail does the reviewer's `status` become `failed` and the retry/survivor policy below applies.

The dual-path transport (inline return + disk persistence) eliminates the "stranded report" failure mode where a fully-completed reviewer CLI invocation produced a valid report that never reached the orchestrator because the sub-agent couldn't return inline. Recovery is NOT a retry — it's reading output that already exists.

**One reviewer `status: failed`.** Consult the liveness probe for that reviewer FIRST. If `alive` or `quiet`, wait — do not retry. Only when the probe returns `dead` AND stranded-report recovery produces nothing do you retry that reviewer once (single tool call in a follow-up message; partner already completed). If retry fails, **survivor mode**: use only the surviving report, set `survivor_mode: true` in the round summary, continue.

**Both reviewers `status: failed`.** Consult the probe for BOTH reviewers. If either is `alive` or `quiet`, do NOT retry — wait and reassess. Only when BOTH probes return `dead` AND neither has a stranded report on disk do you retry the parallel dispatch ONCE. If both fail a second time:

- **Autonomous mode** (`autonomous_mode == True`): exit with `exit_reason = "autonomous_transport_failure"`, preserve the per-round scratch dir as the postmortem artifact, emit a final round summary, and return to the parent batch. No `AskUserQuestion` — no user present to answer. The parent collects the status and reports the failure in its end-of-run summary.
- **Interactive mode**: escalate via `AskUserQuestion` — NEVER render these as prose-numbered bullets (§11.5 item 3):

```
AskUserQuestion(questions=[{
    "question": "Both reviewers failed twice in a row. What should I do?",
    "header": "TPR both-reviewer failure",
    "multiSelect": False,
    "options": [
        {"key": "pause-and-resume",
         "label": "Pause here, clear context, resume with /continue-roadmap (Recommended)",
         "description": "Recommended because back-to-back dual-failures are almost always transient "
                        "infra problems (rate limits, cold-starts, network flakes) compounded by "
                        "context pressure. A fresh session both retries AND resets — preserving the "
                        "work this session already committed (prior rounds' fixes stay) while "
                        "restarting the review itself in a clean context the roadmap can pick up.",
         "recommended": True},
        {"key": "retry-once-more",
         "label": "Retry once more (third attempt, parallel dispatch)",
         "description": "Spends another ~25-40 min on a third parallel dispatch. Pick if you have "
                        "specific evidence (e.g. rate-limit window has cleared) that the failure "
                        "was transient and won't reproduce. Otherwise pause-and-resume is safer."},
        {"key": "abort",
         "label": "Abort /tpr-review — code unchanged, no findings filed",
         "description": "Exits the review entirely. Findings from prior rounds stay fixed and "
                        "committed; the remaining review loop is skipped. Pick when the failure is "
                        "reviewer-specific (auth lapse, CLI outage) and you want to proceed with "
                        "downstream work without a review."},
    ],
}])
```

Never silently exit without producing either a round summary or an escalation. The banned fourth option "Proceed without review (NOT RECOMMENDED)" was removed — it maps to `abort` plus ignoring the missing-review signal, which is a `/tpr-review §7` finding-dismissal pattern.

Exit-reason assignment for each `user_choice.key` in interactive mode:

- `pause-and-resume` → exit cleanly with `exit_reason = "user_pause_and_resume"`; the parent or a fresh session picks up. Do NOT emit a `third_party_review.status: escalated` — this is not a failure, it's a planned pause. (Interactive-only; not reachable in autonomous mode.)
- `retry-once-more` → re-dispatch the parallel-reviewer pair. If THAT retry also fails, the loop bottoms out with `exit_reason = "both_reviewer_failure"`.
- `abort` → `exit_reason = "both_reviewer_failure"`; render round summary; `third_party_review.status: escalated`; `reviewed: false` in plan mode.

**Never insert a mid-loop context-pressure pause.** The orchestrator MUST NOT emit an `AskUserQuestion` between rounds based on "context has grown", "round count ≥ 3", "long transcript", or any other session-length heuristic. Dispatch the next round directly. Mid-loop pause-for-context is banned in both interactive and autonomous modes — the `/clear` + `/continue-roadmap` round-trip it invites costs more context than it saves, and a fresh session restarts `/tpr-review` at round 0 rather than resuming mid-loop.

## §10 — Plan-TPR integration (plan mode only)

After the loop terminates when `ARGS` began with `--skill review-plan <section-path>`:

1. Read the section file's YAML frontmatter.
2. Set `third_party_review.status`:
   - `clean` if `exit_reason == "clean"` and zero verified findings across all rounds.
   - `findings` if any verified findings occurred (even if all were fixed inline), OR if `exit_reason` starts with `user_accepted_at_` (user explicitly accepted the non-converged state; findings remain filed as `- [ ]`), OR if `exit_reason` starts with `autonomous_accept_at_` (autonomous mode auto-accepted an all-meta cap exit; findings remain filed as `- [ ]`).
   - `escalated` if `exit_reason` was `meta_cap_reached`, `iter_cap_reached`, `both_reviewer_failure`, `escalated_to_plan_at_*`, `autonomous_exit_substantive_at_*`, `autonomous_transport_failure`, `autonomous_spec_gate_violation`, or `autonomous_ambiguous_input` (§5 / §9 / §3 / §1 terminal branches where no accept decision landed).
3. Set `third_party_review.updated` to today's date (YYYY-MM-DD).
3a. **When `exit_reason` starts with `user_accepted_at_` or `autonomous_accept_at_`:** also set the section's top-level `reviewed: true` in the same Edit pass, AND append a `third_party_review.notes` line recording the cap type + round count + decision provenance, e.g.:

- Interactive: `notes: "user-accepted at iter_cap_reached after 3 rounds; 7 findings filed as - [ ] in §NN.R"`
- Autonomous: `notes: "autonomous-accepted at meta_cap_reached after 3 rounds (all_meta); 4 findings filed as - [ ] in §NN.R"`

The autonomous-accept flip is only reached when `all_meta == True` at cap exit (§5's autonomous auto-decision never accepts substantive findings — those exit with `autonomous_exit_substantive_at_*` which lands `reviewed: false`). Downstream `/review-plan` Step 7+8 MUST no-op when it sees `reviewed: true` already set (see `review-plan/step-7-8-verify.md`). Rationale: an autonomous batch signing off on a plan with only meta findings remaining is equivalent to the interactive "Recommended" accept-with-findings path — those findings are owned by the plan's own `- [ ]` checklist, not by the review pipeline.

3b. **When `exit_reason` is `autonomous_exit_substantive_at_*`, `autonomous_transport_failure`, `autonomous_spec_gate_violation`, or `autonomous_ambiguous_input`:** set `third_party_review.status: escalated` and LEAVE `reviewed: false`. Append a `third_party_review.notes` line recording the status and directing follow-up to the parent batch's end-of-run report. The plan is NOT reviewed; the autonomous batch bubbled the open question up to its parent which must surface it to a human.
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

**Autonomous-mode carve-out.** When `autonomous_mode == True` (see §1 Autonomous-mode flag), EVERY `AskUserQuestion` listed in items 1-5 above is replaced with a best-judgement auto-decision that emits a distinct `exit_reason`:

| Choice point | Interactive action | Autonomous auto-decision |
|---|---|---|
| §5 cap-exit (iter_cap / meta_cap) | `AskUserQuestion` with 4 options | `all_meta` → `autonomous_accept_at_*`; else → `autonomous_exit_substantive_at_*` |
| §9 both-reviewer-failure-twice | `AskUserQuestion` with 3 options | Exit with `autonomous_transport_failure` |
| §3 SPEC-GATE finding (critical) | Finding filed; user fixes before continuing | Exit with `autonomous_spec_gate_violation` |
| §1 ambiguous input | `AskUserQuestion` with mode interpretations | Exit with `autonomous_ambiguous_input` (ARGS parsing couldn't resolve a mode after stripping `--autonomous`); emit final round summary with the received `ARGS` verbatim — never guess |

The autonomous auto-decisions are NOT interactive-mode shortcuts — they are distinct terminal exit points with distinct `exit_reason` values. Parent batches MUST branch on the `autonomous_*` statuses in their end-of-run aggregation; silently treating them as "clean" or "retry" defeats the policy. Interactive callers MUST NOT pass `--autonomous` because the auto-decisions are strictly inferior to a human answer when a human is available — best-judgement defaults trade decision quality for throughput, and that trade is only correct when no human is watching.

## When to Trigger — Bias Toward Running

Run after ANY of:

- Bug fixes (any severity), new features, refactors, multi-file changes (2+ files).
- Changes to workspace crates (`oriterm_core`, `oriterm_ui`, `oriterm_mux`, `oriterm_ipc`, `oriterm`).
- Test matrix additions, stdlib changes, registry changes, diagnostics changes.
- Plan section implementations, docs touching invariants.

**Also run when** unsure whether a change warrants review, the change touches code paths shared across subsystems, or a fix surfaced interfering behavior elsewhere.

**Run with a custom objective when** iterating on any artifact (skill, doc, design) with multi-agent consensus.

**Skip only for** single-line typo fixes, comment edits, or formatting-only changes. When in doubt, run it.
