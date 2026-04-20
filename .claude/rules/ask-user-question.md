---
paths:
  - "**"
---

# AskUserQuestion — Recommended-Option Rule (SSOT)

**Scope.** Every `AskUserQuestion` invocation — from any skill, command, or the
base agent — MUST present a recommended option **as the top (first) choice**,
with an explicit rationale explaining WHY it is the recommended pick. No
exceptions.

Rationale for the rule: when a user is interrupted, decision latency and
decision quality both matter. A top-positioned, justified recommendation
lets a trusting user proceed quickly (just hit Enter / accept default) AND
lets a skeptical user redirect with full context. A bare option list offloads
the judgment Claude has already done back onto the user, producing slower,
less-informed choices.

## The Rule

Every `AskUserQuestion` options array (2–4 options per question per the tool
schema):

1. **Position.** The recommended option is index `0` — the very first entry.
   Never middle, never last, never alphabetized into a different slot.
2. **Label marker.** The recommended option's `label` ends with
   `(Recommended)`. This is the exact literal the AskUserQuestion tool
   description prescribes.
3. **Rationale.** The recommended option's `description` field (required by
   the tool schema) MUST explain WHY it is recommended — not merely what it
   does. Frame around the most likely user goal, the safest default, or the
   cost of the alternative.
4. **Non-recommended options.** Still have clear `description` fields
   explaining what they do and their trade-off relative to the recommended
   path. Never have more than ONE option marked `(Recommended)`.
5. **"Other" field.** The AskUserQuestion harness always appends a built-in
   "Other" option for custom text. Do not add your own "other" / "something
   else" option — it is redundant.

## Canonical example (direct `AskUserQuestion` call)

```python
AskUserQuestion(questions=[{
    "question": "TPR loop exited at iter_cap after 10 rounds with 2 verified "
                "findings still open. How do you want to proceed?",
    "header": "TPR cap exit",
    "multiSelect": False,
    "options": [
        {
            "label": "Accept with findings filed (Recommended)",
            "description": "Recommended because 2 findings is a small residual "
                           "tail; they stay filed as - [ ] items under §NN.R "
                           "and the plan's own completion gates own them. "
                           "Flipping reviewed: true here matches the common "
                           "path — further rounds rarely converge past iter_cap."
        },
        {
            "label": "Run 3 more rounds",
            "description": "Spend another ~20–40 min of reviewer wall-clock. "
                           "Pick this only if the open findings are in a single "
                           "tight cluster that one more pass could resolve."
        },
        {
            "label": "Escalate to /create-plan",
            "description": "Spin up a new plan that takes ownership of the 2 "
                           "findings. Worth it only when the findings are "
                           "structural (architectural) rather than local."
        },
        {
            "label": "Abort — leave reviewed: false",
            "description": "Leaves the plan un-reviewed with no follow-up hook. "
                           "Least-preferred — equivalent to silent deferral."
        }
    ]
}])
```

Note: the recommended option is **index 0**, its label ends with
`(Recommended)`, and its `description` opens with `Recommended because …`
followed by the reason. The three alternatives have descriptions that frame
their trade-off against the recommended path.

## Canonical example (JSON-handoff schema, e.g. `/review-plan` Step outputs)

Skills that produce JSON handoffs consumed by a parent which *then* calls
`AskUserQuestion` (e.g. `.claude/skills/review-plan/step-*-*.md`,
`.claude/skills/continue-roadmap/roadmap_scan.py`) use this shape:

```json
{
  "escalate": true,
  "question": "Precheck found 3 ambiguous sections. How do you want to resolve them?",
  "options": [
    {
      "key": "fix-individually",
      "label": "Walk through each ambiguous section and decide (Recommended)",
      "description": "Recommended because ambiguity usually reflects real plan-state conflicts that need per-section judgment; bulk actions lose that signal.",
      "recommended": true
    },
    {
      "key": "leave-as-is",
      "label": "Leave ambiguous sections in-progress",
      "description": "Punts the ambiguity forward; only pick if you plan to resolve these sections out-of-band in the same session.",
      "recommended": false
    },
    {
      "key": "abort",
      "label": "Abort review and fix manually",
      "description": "Exits the review entirely; choose only if the precheck surfaced plan-state that review tooling can't untangle.",
      "recommended": false
    }
  ]
}
```

The `recommended: true` flag is the JSON-handoff convention. The parent skill
reads the flag, places the option first (it already is), appends
`(Recommended)` to the label (already in the label above — belt-and-suspenders
is fine), and passes the `description` verbatim to the real
`AskUserQuestion` call.

**Canonical exemplar in-tree:**
`.claude/skills/continue-roadmap/roadmap_scan.py` — search for `"recommended": True` to see this pattern applied across every gate (yaml-parse, stale-review, TPR-status, relevant-bugs, dirty-tree).

## When NOT to recommend

You still MUST pick and justify one, even when the choice feels balanced. If
a decision is genuinely a coin-flip, that itself is the rationale: say so.
Example `description`:

> Recommended because both paths are roughly equivalent in cost and risk;
> this option keeps more reversibility, which is the tie-break default.

"No recommendation" is not a permitted output — if you can't articulate
*any* reason to prefer one option, you haven't thought hard enough to be
asking the user yet. Go gather more context first.

## Banned patterns

- **Bare option list.** Four options with `description` fields that only
  describe what they do, no ranking signal.
- **Recommendation in the wrong slot.** Labeling option 2 or 3 as
  `(Recommended)` and expecting the user to scan past the default path.
- **Rationale-less recommendation.** A `(Recommended)` suffix on option 0
  with no explanation in `description`. The user cannot redirect
  intelligently without the why.
- **Multiple recommendations.** Labeling two options `(Recommended)` —
  collapses the point of the rule.
- **Asking when you should decide.** If you can articulate the recommendation
  AND you are authorized (local/reversible action, clear context), just do
  it. `AskUserQuestion` is for decisions that need human judgment or for
  actions outside your authorized scope.

## Relation to other rules

- **CLAUDE.md §Ownership & Deferral** and §"Future Improvement" both tell you
  to use `AskUserQuestion` when genuinely blocked. This rule specifies *how*
  to structure the question.
- **CLAUDE.md §Executing actions with care** (Output Style) tells you to
  confirm risky/shared-state actions. When those confirmations surface as
  `AskUserQuestion`, this rule applies — pre-pick the safer option as the
  recommended default.

## Consumers

Every skill/command that calls `AskUserQuestion` (directly or via a JSON
handoff consumed by a parent) is a consumer. Non-exhaustive live list
(grep-verifiable via `grep -l AskUserQuestion .claude -r`):

- Skills with direct calls: `/tpr-review`, `/fix-bug`, `/fix-next-bug`,
  `/review-plan`, `/continue-roadmap` (Step 3 block-gates + Step 4 pacing
  only — Step 6 subsection loop forbids `AskUserQuestion` except for
  architectural decisions per its tight definition), `/tp-help`,
  `/rosetta-test`, `/impl-hygiene-review`, `/create-plan`,
  `/create-draft-proposal`, `/review-draft-proposal`,
  `/design-pattern-review`, `/add-bug`, `/commit-push`.
- Commands: `/independent-review`, `/review-bugs`.
- **Explicitly NOT consumers** (autonomous — forbid `AskUserQuestion`):
  `/code-journey`, `/sync-docs`. Grep will surface `AskUserQuestion`
  literal matches in these files, but every match is a negative
  guard ("NEVER ask the user") rather than a call. Exclude from the
  live-count via `grep -L "NEVER ask" | xargs grep -l AskUserQuestion`
  or a similar filter that drops negative-mention files.
- JSON-handoff producers (parent builds the real call):
  `.claude/skills/review-plan/step-2-precheck.md`,
  `.claude/skills/review-plan/step-5-editor.md`,
  `.claude/skills/review-plan/step-6-tpr.md`,
  `.claude/skills/review-plan/step-7-8-verify.md`,
  `.claude/skills/continue-roadmap/roadmap_scan.py`.

Drift-check: if a consumer's `AskUserQuestion` options list does not have
the recommended option at index 0 with a rationale-bearing description, it
is a rule violation. The consumer file must be patched.
