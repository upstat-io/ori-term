# Step 4 — /tp-help blind-spot analysis

Read by the **parent inline in main context** from `/review-plan` — NOT dispatched as an `Agent({})` sub-agent. Not a registered skill.

**Why inline:** `/tp-help` spawns Codex + Gemini CLIs concurrently; each reviewer runs 20–45 minutes wall-clock with streaming output. A sub-agent wrapping cannot hold the Skill-tool invocation open across that wall-clock, and its monitoring loops are invisible across the `Agent({})` boundary (same root-cause failure that moved Step 6 inline on 2026-04-17; see `.claude/skills/improve-tooling/review-plan-design.md §4`). Running inline lets the parent's Opus context hold the call open and synthesize richer blind-spot distillation than Sonnet would produce.

## Input

The parent already has `{RUN_DIR}` in scope from Step 1. Read `{RUN_DIR}/context.json` for `mode`, `plan_dir`, `target_section` (even though the parent already has them in memory — the read keeps this protocol self-contained and resumable).

Read the plan to build context for /tp-help:

- `{plan_dir}/00-overview.md` — mission, section list with goals and statuses
- `{plan_dir}/index.md` (if exists) — full_name, description
- In single-section mode, also read `{target_section}` body

## Dispatch /tp-help

```
Skill: tp-help
```

Provide a prompt that contains:

- **The plan's mission/goal** (from `00-overview.md`)
- **The section list** (id, title, goal, status)
- **Plan scope** (which crates, which subsystems)
- **Review mode** (single-section vs whole-plan)
- **Specific questions:**
  - "What are the most likely failure modes this review should watch for?"
  - "What architectural risks or blind spots would you flag?"
  - "Are there cross-cutting concerns that might fall between section boundaries?"

Wait for /tp-help to complete.

## Distill the response

The /tp-help return contains BOTH reviewers' raw text concatenated. Extract:

- **Blind spots** — bullet list, max 10 items, merged across reviewers (drop duplicates)
- **Architectural risks** — ≤5 bullets
- **Cross-cutting concerns** — ≤5 bullets

Each bullet must be ≤200 characters and reference something specific (a file, a section, a named risk). Reject vague bullets like "consider more testing" — if both reviewers only produce vague output, note it in the `summary` field and return an empty list.

## Output

Write `{RUN_DIR}/blind-spots.json`:

```json
{
  "blind_spots": [
    "Section 04's Tag::Var propagation rule contradicts Section 02's validate_body_types behavior when …"
  ],
  "architectural_risks": [
    "Bodies-pass integration risks breaking #[ignore] tests that assumed silent passthrough"
  ],
  "cross_cutting": [
    "impl-hygiene tests.md matrix testing rules not woven into Section 03 checklist"
  ],
  "summary": "Phase 2: N blind spots, M risks, K cross-cutting concerns",
  "escalate": false
}
```

Never escalate — this phase is advisory. The editor phase uses these as discovery pointers, not authoritative claims.

## Do NOT

- Make plan edits in this phase (that's the editor's job)
- Cite /tp-help results as authoritative — they're discovery
- Truncate the tp-help context aggressively (reviewers need context to produce useful blind spots); only the distilled bullets are bounded
