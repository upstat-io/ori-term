---
name: add-bug
description: File a bug via tracker path (`plans/bug-tracker/`) or inline path (`--inline <plan-section>` when the bug blocks plan completion); Step 0 big-picture gate decides the routing, and the skill MUST be invoked proactively on ANY bug encountered during ANY work (if in doubt, file it).
allowed-tools: Read, Grep, Glob, Edit, Write, Bash, AskUserQuestion, Skill
argument-hint: "[--inline <plan-section-path>] [subsystem=X severity=Y title=\"...\" context=\"...\"]"
---

# Add Bug

File a bug via one of two routes:

| Route | When | Destination |
|---|---|---|
| **Tracker** (default) | Bug has independent lifecycle; plan completes without this fix | `plans/bug-tracker/section-{NN}-*.md` |
| **Inline** (`--inline <path>`) | Bug blocks plan completion — plan cannot reach its stated goal | New subsection in `<plan-section-path>` with full `/fix-bug` template |

Routing decided at **Step 0: Big-Picture Analysis** in `workflow.md` — every invocation answers: "Will the rest of the plan complete without this bug being fixed?"

## Usage

```
/add-bug [description]                                       — tracker path (after Step 0 confirms)
/add-bug subsystem=core-terminal severity=high title="..."   — structured tracker form
/add-bug --inline plans/foo/section-04-X.md [...]            — inline path, explicit (skips Step 0 question)
```

## Proactive Triggering — MANDATORY

This skill MUST be invoked proactively whenever you encounter a bug that is **not part of your current task**. Do NOT:
- Gloss over it as "not related"
- Note it mentally and move on
- Say "this is a separate issue" without filing
- Assume someone else will catch it
- Skip it because you're "in the middle of something"

**If in doubt, file it.** Verification happens when bugs are reviewed (`/review-bugs`). A false positive costs nothing; a missed bug costs everything.

### When to trigger (non-exhaustive)
- You see a test failure unrelated to your current work
- You notice suspicious behavior while reading code
- A code journey or exploration reveals unexpected output
- You encounter an edge case that probably doesn't work
- You find a TODO/FIXME/HACK comment that describes an unfixed bug
- A rendered frame, widget paint, or escape-sequence handler produces the wrong output
- A platform-specific `#[cfg(target_os = ...)]` branch is missing on one of the supported targets
- Any test is `#[ignore]`-d and the reason looks fixable
- A performance invariant (zero idle CPU, zero allocations in hot render path, stable RSS) appears to be violated

## Usage

Execute the workflow inline in the main context. The full protocol lives in `workflow.md` — read it via `@` include and follow it end-to-end.

Args: `<ARGS>` from the user (subsystem, severity, title, optional context).

@.claude/skills/add-bug/workflow.md

## Rules

- **Plan-doc only, two allowed write targets:**
  - Tracker path: touch `plans/bug-tracker/*.md` ONLY.
  - Inline path: touch the single `<plan-section-path>` passed via `--inline` (or chosen in Step 0). No other plan files. No source, tests, or terminal changes.
- Never edit `.rs` or anything under `oriterm_core/`, `oriterm_ui/`, `oriterm_mux/`, `oriterm_ipc/`, `oriterm/`, `tests/`.
- Do NOT dispatch a sub-agent. Execution happens inline in main context.
- Do NOT invoke `/commit-push`. Leave the markdown change in the working tree. The caller owns committing.
  - Mid-workflow invocation (common case): the caller's next commit bundles the bug entry / inline subsection with their in-flight work.
  - Standalone invocation: the caller sees the unstaged change in `git status` and commits when ready.
  - Avoids the hook-wall race documented in `.claude/skills/improve-tooling/add-bug-design.md` §4 (2026-04-19 incident).
- Step 0 is the ONLY place `AskUserQuestion` is permitted. Steps T1–T7 and I1–I5 MUST NOT pause for input — the caller resumes immediately on return.
- Inline path MUST NOT assign a `BUG-XX-NNN` ID — the plan subsection IS the tracking artifact. No bug-tracker entry is created for pure plan-blocker inlines (though a cross-ref may be added to an existing tracker entry per Step I4).

## Files in this skill

- `SKILL.md` (this file) — caller-facing entry point.
- `workflow.md` — full protocol (Steps 1–7 executed inline by the caller).

## Design log

`.claude/skills/improve-tooling/add-bug-design.md` — philosophy, load-bearing invariants, lessons from dogfood, regressions to watch for.
