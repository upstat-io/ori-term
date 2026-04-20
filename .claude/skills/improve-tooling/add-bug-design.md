# /add-bug — Design Log

## Purpose + Context

`/add-bug` files an entry in `plans/bug-tracker/section-{NN}-*.md` for a bug out-of-scope for the current task. Converted from sub-agent dispatch to inline execution on 2026-04-19 after a hook-wall race incident during §08.2 close — prior design invoked a foreground Sonnet Agent that ran its own `/commit-push`.

## §1 Core Design Philosophy

1. **Inline execution in main context.** The workflow is ~3 tool calls (Read target section, Edit append, grep for cross-refs). Agent dispatch overhead (thousands of tokens of context setup) dominates the actual work and buys nothing.
2. **No auto-commit.** Leave the markdown change unstaged. The caller decides when to commit — bundled with their next commit (common mid-workflow case) or standalone when the user is filing at the terminal.
3. **No nested skill invocation for commit.** Do not call `/commit-push` from inside `/add-bug`. A nested `/commit-push` creates hook-wall races with the caller's own commit and produces opaque failure modes.
4. **Plan-doc only.** Touch `plans/bug-tracker/*.md`. Never edit source, tests, or other plan directories.
5. **Minimal research at add-time.** Confirm existence, note approximate location, capture repro. Do NOT deep-dive — the code may change before fix time, and deep research belongs in `/fix-bug` Phase 1.
6. **Canonical source-of-provenance field.** Every entry carries exactly one `Source: <value>` from the enumerated list in `workflow.md` Step 5. No ad-hoc provenance.
7. **Caller resumes immediately.** After filing, return a single confirmation block. Do NOT prompt, do NOT pause, do NOT stop to ask for next action.

## §2 Load-Bearing Invariants

| Invariant | Failure mode it prevents |
|---|---|
| Execute inline, never dispatch a sub-agent | Sub-agent's own `/commit-push` hits the caller's pre-commit hook wall with no coordination → opaque stuck state (incident 2026-04-19 §4.a) |
| Never invoke `/commit-push` from inside `/add-bug` | Two `/commit-push` calls fighting the same wall, each needing independent user approval for bypass |
| Leave markdown change unstaged | Caller can stage it alongside their in-flight work, producing clean single-commit provenance (incident 2026-04-19 showed the bug entry being staged before caller's own work was ready, forcing `MM` dual-status recovery) |
| ID ordinal = "existing bugs + 1" within the section | ID uniqueness per section; parallel sessions that both count from the same snapshot will collide, but this is acceptable at project volume and caught at `/review-bugs` |
| Only touch `plans/bug-tracker/*.md` | Preserves the "minimal research" constraint; prevents `/add-bug` from mutating the compiler or rule files under time pressure |
| Source value from the enumerated list only | SSOT for provenance — `/review-bugs` groups + weighs by source; ad-hoc values break the aggregation |
| Subsystem mapping table is the SSOT for routing | Prevents bug entries landing in the wrong subsystem file, which makes `/review-bugs` miss them |
| Step 7 confirmation is ONE block, not a conversation | Mid-workflow caller needs to resume; a conversational close-out adds friction and tempts the caller to wait for input |
| Step 0 big-picture gate runs on EVERY invocation | Plan-blocker bugs get silently routed into the tracker and produce `Plan → bug → bug → bug` dependency chains that stall forever (CLAUDE.md §Plan-Blocker Bugs Belong IN the Plan) |
| `AskUserQuestion` permitted ONLY in Step 0 | Steps T1–T7 and I1–I5 would turn into conversations; callers in mid-workflow need a single-block return |
| Inline path assigns NO `BUG-XX-NNN` ID | Plan-owned subsection IS the tracking artifact; issuing a tracker ID creates a second artifact that must be kept in sync and tempts future sessions to re-route the fix back to `fix-BUG-XX-NNN.md` (sibling pattern this feature exists to prevent) |
| Inline path writes ONLY the single `<plan-section-path>` + the frontmatter `sections:` list within it | Widening to "any plan file" means `/add-bug` can corrupt plans outside the caller's active context under time pressure |
| `--inline` on a plan section with `status: complete` is rejected in Step I1 | Reopening a closed plan section is a large-blast-radius decision; `/add-bug` is not authorized to make it — caller must re-route to tracker or explicitly reopen first |
| `kind: plan-blocker-inline` marks every inlined subsection in the frontmatter `sections:` list | Distinguishes inlined blockers from planned subsections for reviewers (`/review-plan`, `/tpr-review`) and for the future `/fix-bug` inline-dispatch follow-up (§6 Open) |

## §3 File Inventory

| File | Role | Lines |
|---|---|---:|
| `.claude/skills/add-bug/SKILL.md` | Caller entry point; rules; workflow include via `@` | ~75 |
| `.claude/skills/add-bug/workflow.md` | Inline protocol — Step 0 (big-picture gate) + Tracker path (T1–T7) + Inline path (I1–I5) | ~270 |
| `.claude/skills/improve-tooling/add-bug-design.md` | This file — design log | — |

**Deleted on 2026-04-19 re-inlining:**
- Agent-dispatch boilerplate in SKILL.md (the `Agent({description, subagent_type, prompt})` block)
- `allowed-tools: ..., Agent, ...` — `Agent` removed from the allowlist (the skill never dispatches an Agent now)
- Step 8 "Resume Prior Workflow" in workflow.md — vacuous under inline execution because the caller IS the one executing

## §4 Lessons from Dogfood / Production Runs

### 2026-04-19 — Hook-wall race during §08.2 close

During §08.2 close of `plans/empty-container-typeck-phase-contract/section-08-codegen-poly-lambda.md`, the caller (`/roadmap-work` → `/commit-push`) hit a pre-commit hook failure on `clippy::todo` (caller had introduced `todo!()` stubs that were later re-grounded as `#[cfg(any())]` gates). As part of the close-out retrospective, caller filed a docs-improvement bug via `/add-bug`.

The sub-agent dispatch behaved as follows:
- Parent invoked `Agent({description, subagent_type: general-purpose, model: sonnet, ...})`.
- Sub-agent read `workflow.md`, filed the bug, then invoked its OWN `Skill: commit-push`.
- Sub-agent's `/commit-push` hit the FULL pre-commit `full-check` step — which runs `cargo test --all` unconditionally when `.rs` or `.md` files are staged. The run reported 843 pre-existing interpreter failures (§06.2 wall) + 2 pre-existing AOT TDD pins (sibling `BUG-04-AOT-MONO`).
- Parent received a background Monitor event showing `🥊 lefthook v1.11.5 hook: pre-commit` and misread it as "sub-agent still running".
- User had to interrupt: "the sub-agent is definitely not running".
- Parent then ran `git status` and discovered the sub-agent HAD filed the bug (modified `plans/bug-tracker/section-08-spec-docs.md`) but had either silently failed its commit or returned partial state.
- Parent then had to retry its OWN `/commit-push`, hit the same test wall, ask the user via `AskUserQuestion` for `--no-verify` approval, and commit with full justification.

**Root cause:** sub-agent design placed a second `/commit-push` in a workflow whose CALLER was already mid-`/commit-push`. Both commits fought the same hook wall with no coordination. The massive Agent dispatch overhead (Sonnet cold-start, context rebuild, workflow re-read) added cost for zero structural benefit — the work is literally three tool calls.

**Fix:** re-inline the workflow in main context; drop the `/commit-push` invocation entirely; leave the markdown change unstaged for the caller to bundle. Produces clean single-commit provenance, eliminates the race, eliminates the sub-agent overhead, eliminates the opacity.

**Cross-reference:** `plans/bug-tracker/section-08-spec-docs.md` contains the docs-improvement bug filed during this incident (the one whose filing surfaced the sub-agent defect).

### 2026-04-19 — Big-picture gate + `--inline <plan-section>` feature

**Motivation.** CLAUDE.md §"Plan-Blocker Bugs Belong IN the Plan" has mandated the classifying rule ("Can the plan complete with this bug open?" NO → merge into plan as new subsection; YES → `/add-bug` sibling) since the sibling-chain anti-pattern was first documented. Until now, execution of that rule relied on manual interpretation: recognize the blocker, manually write a new subsection body, manually update the plan's `sections:` frontmatter, manually close any tracker entry. Doing-it-right was harder than doing-it-wrong, so sibling-chain `Plan → fix-BUG-A → fix-BUG-B → fix-BUG-C …` kept forming under time pressure. The user's incident prompt (`.claude/skills/improve-tooling/add-bug-design.md` is born of this retrospective): *"when working on a plan you make the correct decision about when to use /add-bug, you need to pull back and do a big picture analysis."*

**Design.** Split the workflow into:

- **Step 0 (big-picture gate)** — every invocation. `AskUserQuestion` forces "will the rest of the plan complete without this fix?" as a decision that cannot be skipped by omission. Recommended option is "Tracker bug" per `.claude/rules/ask-user-question.md` (the common case), with explicit rationale that "plan blocker" is the correct pick only after pulling back on the full plan.
- **Tracker path (T1–T7)** — renamed from the prior Steps 1–7; behavior unchanged.
- **Inline path (I1–I5)** — new. Writes a `{section}.BLOCKER-{N}` subsection into the caller-specified `<plan-section-path>`. Two edits in one file: append to frontmatter `sections:` list + append body subsection at end-of-file. Body follows a condensed fix-section-template shape (Root Cause / Fix Consensus / TDD Matrix / Plan TPR / Implementation / TPR / Completion Checklist — all marked "pending" for `/fix-bug` to fill). No `BUG-XX-NNN` ID is assigned; the plan subsection IS the tracking artifact (per user decision: "no, because it's no longer a bug tracker bug, it's a plan owned bug").
- **Full `/fix-bug` rigor** is expected on the subsection (per user decision: "the entire /fix-bug rigor"). The current workflow reports "Next: invoke /fix-bug targeting this subsection to execute full rigor." Until `/fix-bug` learns inline-subsection dispatch, the caller applies Phase -1 through Phase 6 manually against the subsection body. See §6 Open.

**What this feature does NOT do** (scope-boxed on purpose):
- Does NOT extend `/fix-bug` itself to accept an inline-subsection target. That is tracked as the load-bearing follow-up (§6 Open). Today, `/fix-bug BUG-XX-NNN` still expects a tracker ID.
- Does NOT auto-detect the active plan from recent edits — Step 0 asks the user explicitly. Auto-detection is deferred; explicit choice is less likely to route the wrong plan under context pressure.
- Does NOT close any existing tracker entry automatically. Step I4 adds a cross-ref to the inline subsection body pointing at a matching tracker entry; `/fix-bug` Phase 5 closure is responsible for flipping the tracker entry to `- [x]` when the inline subsection completes.

**Cross-reference:** the classifying rule lives in project CLAUDE.md §"Plan-Blocker Bugs Belong IN the Plan — NEVER Sibling Fix Files". This feature operationalizes that rule; it does not replace or weaken it.

## §5 Regressions To Watch For

- [ ] A reviewer or skill author re-introduces `Agent({..., subagent_type: ...})` in `SKILL.md` under the rationale "context isolation." The sub-agent does NOT buy context isolation in practice — it forks a 2nd context that duplicates the intel-query SSOT and still needs the caller's task description verbatim.
- [ ] A future edit adds `Skill: commit-push` to `workflow.md` Step 5 / Step 7 / Step 8. The markdown change MUST stay unstaged; the caller owns committing.
- [ ] SKILL.md's `allowed-tools:` frontmatter grows an `Agent` entry. If the skill never dispatches an Agent, the allowlist should not include it — reviewers infer capability from allowlist.
- [ ] workflow.md grows language like "the sub-agent reads this file" or "you are the filing agent" — vestiges of the old design. All such phrasing should read "the main agent executes this inline".
- [ ] A caller's `/add-bug` invocation blocks because the skill prompts for confirmation. Step 7 returns ONE block and returns immediately — do not add an `AskUserQuestion` to Step 5, 6, or 7.
- [ ] Bug entries filed with `Source: <ad-hoc-string>` rather than a value from the enumerated list. `/review-bugs` groups by Source; ad-hoc values break the aggregation silently.
- [ ] Step 0 big-picture gate is silently skipped by a future refactor (e.g., a well-intentioned "fast path for unambiguous tracker bugs" that bypasses `AskUserQuestion`). The point of the gate is that ambiguity is almost always present and requires explicit articulation — skipping reintroduces the `Plan → bug → bug → bug` sibling-chain anti-pattern this feature exists to prevent.
- [ ] `--inline` writes frontmatter `sections:` list entry but forgets the matching `## {id} — ...` body heading (or vice versa). Both MUST be updated in the same invocation per Step I3; reviewers that read only one surface will see divergent state.
- [ ] Inline subsection body drops the `kind: plan-blocker-inline` field, making it indistinguishable from a planned subsection. `/fix-bug` inline-dispatch follow-up (§6 Open) depends on this field to detect inline targets.
- [ ] An inlined subsection grows a `BUG-XX-NNN` ID under a well-intentioned "add a tracker cross-ref for grep-ability" patch. That recreates the dual-artifact problem the user's decision explicitly rejected ("no, because it's no longer a bug tracker bug, it's a plan owned bug"). Cross-refs from EXISTING tracker entries are fine (Step I4); minting NEW ones is not.

## §6 Improvement Log

### Open items

- [ ] `[p2]` Step 0 routing when invoked by `/fix-bug --autopilot` — currently documented as "always take tracker path". Revisit now that the `/fix-bug` inline dispatch has landed (2026-04-19); autopilot may want to inline plan blockers discovered mid-fix directly into the active `/fix-bug`'s fix section. Complicated by the fact that an autopilot `/fix-bug` already owns ONE fix section (tracker or inline) — inlining a blocker into a plan the fix isn't scoped to risks scope creep. Default stance is still "tracker path in autopilot"; revisit if production runs surface a pattern where mid-fix plan-blockers are common.
- [ ] `[p2]` Parallel-session ID collision detection — two concurrent `/add-bug` invocations both count `existing + 1 = N` and produce colliding `BUG-XX-N` entries. Low-frequency at current project volume; caught at `/review-bugs`. Proper fix would require a lock-file or a git-based atomic-append primitive.
- [ ] `[p3]` Intelligence-graph blast-radius queries (workflow.md Step T4/I2 item 4) duplicate whatever the caller already queried. Consider passing a `--skip-intel` flag when the caller's prompt already contains intel-summary output.
- [ ] `[p3]` Auto-detect the `<plan-section-path>` from recently-edited plan files so Step 0's follow-up question prefills a best-guess option instead of asking the user to paste the path.

### Recently closed

- [x] **2026-04-19** `[p1]` **`/fix-bug` inline-subsection dispatch (companion to `/add-bug --inline`).** Paired same-session with the `/add-bug --inline` feature above. `/fix-bug` now accepts `inline:<plan-section-path>#<subsection-id>` (or shorthand `<path>#<id>`) as a target argument. The Sonnet Phase-0 sub-agent detects mode via `workflow.md` Step 0; inline mode reads the named plan section, validates `kind: plan-blocker-inline` on the frontmatter `sections:` entry, extracts the body subsection, and returns a new `[INLINE]` handoff with skeleton-state flags driving Resume-mode decisions. Opus-side workflow adds a §Inline Mode Phase Overrides block that tabulates per-phase deltas: Phase 1.6 (fix-section-file creation) is SKIPPED, Phases 1/1.5/1.75/2/2.5/3/4 edit the subsection body in-place, Phase 5 closure flips the frontmatter `sections:` entry `status:` to `complete` + fills the body `### R/§N` blocks + closes the tracker cross-ref if one exists. Validation gates refuse to operate on non-`plan-blocker-inline` subsections or closed parent sections. Autopilot support is inherited: tracker-mode autopilot rules apply unchanged. Design + invariants + lessons recorded in `.claude/skills/improve-tooling/fix-bug-design.md`. Commit: pending.
- [x] **2026-04-19** `[p0]` **Add Step 0 big-picture gate + `--inline <plan-section>` feature.** Branch workflow.md into Tracker path (T1–T7, unchanged behavior) and Inline path (I1–I5, new). Step 0 asks `AskUserQuestion` "will the rest of the plan complete without this fix?" on every invocation (unless `--inline <path>` was passed explicitly). Inline path appends a `{section}.BLOCKER-{N}` subsection to the caller-specified plan section file — both the frontmatter `sections:` list entry and the body heading — with the full `/fix-bug` template shape for Phases 1 through 5 to fill in. No `BUG-XX-NNN` ID is minted; plan subsection IS the tracking artifact. Updated SKILL.md argument-hint + usage block + rules (two allowed write targets: bug-tracker for tracker path, `<plan-section-path>` for inline path; no source/test/compiler writes in either). Updated §2 Load-Bearing Invariants with 6 new rows, added §4 Lessons entry "Big-picture gate + `--inline` feature", added 5 new §5 regressions. Rationale: CLAUDE.md §"Plan-Blocker Bugs Belong IN the Plan" mandates the classifying rule but relied on manual interpretation; sibling-chain `Plan → bug → bug → bug` patterns kept forming because doing-it-right was harder than doing-it-wrong. Commit: pending.
- [x] **2026-04-19** `[p0]` Convert `/add-bug` from sub-agent dispatch to inline execution. Drop `Agent({...})` invocation in SKILL.md; drop `Skill: commit-push` in workflow.md Step 7; drop Step 8 "Resume Prior Workflow" (vacuous under inline execution); remove `Agent` from `allowed-tools`. Rewrite workflow.md preamble to say "main agent executes inline" instead of "Sonnet sub-agent executes". Rationale: hook-wall race during §08.2 close where sub-agent's `/commit-push` hit the caller's pre-existing test wall with no coordination (see §4). Commit: pending.

## §7 How To Use This File In Future Sessions

**When to open this file:**
- Before editing `SKILL.md` or `workflow.md` — read §1 (philosophy) and §2 (invariants) to understand what must NOT regress.
- When `/add-bug` misbehaves in production — check §5 (regressions) for the specific failure mode, then §4 (lessons) for precedent.
- Before promoting a script-level change or pattern from `/add-bug` to sibling skills — §4 has the concrete incident data.

**When to update this file:**
- After every `/improve-tooling` session that touches `/add-bug`: add a `- [x]` entry to §6 Recently closed with date + description + commit sha.
- Whenever §5 regression is caught in the wild: add a dated note to §4 Lessons describing the incident, and bump the §5 entry's tracking if it recurs.
- Whenever an invariant from §2 is relaxed or removed: add a §4 entry explaining the new failure mode that justifies the change, and flip the §2 row. The design log and the SKILL.md must agree.

**What NOT to update here:**
- Ephemeral conversation context, in-flight session state, or TODO-style scratch. Those belong in the plan file or commit message.
- The skill's behavioral contract — that lives in `SKILL.md` and `workflow.md`. This file is the retrospective lens.
