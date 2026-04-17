# `/create-plan` — Design Notes and Improvement Log

**Purpose of this file.** Institutional memory for `/create-plan`'s evolution. Captures the design philosophy (so future edits don't regress the architecture), the load-bearing invariants (things you must NOT change without very good reason), and a running log of improvements and bugs surfaced during real use.

**Context.** `/create-plan` is the Phase-0-through-Phase-5 heavy-path workflow for generating `plans/<name>/` directories with overview + sections. It also supports a narrow "light path" (inline `ExitPlanMode` checklist) when Phase 0's three-criterion gate opens. Canonical files: `.claude/skills/create-plan/SKILL.md`, `.claude/skills/create-plan/plan-schema.md`.

**When to update this file.** Any time you encounter drift, drag, or surprise while running `/create-plan`, or while any consumer (`/continue-roadmap`, `/review-plan`, `/fix-bug` scope-escalation) interacts with plans it produced. Add a `- [ ]` item under §6 Open.

---

## §1 — Core Design Philosophy (KEEP THIS)

1. **Research-first, architecture-second, sections-last.** Overview (`00-overview.md`) is the load-bearing design document; sections are *implementations of* the architecture, not independent documents. Never write sections before Phase 3 user-approved architecture.

2. **Sequential section writing is non-negotiable.** Section N depends on decisions made in Section N-1. Parallel writing forces each section to *guess* what siblings decided, producing contradictions.

3. **External consultations are SEQUENTIAL and FOREGROUND.** Every `/tp-help` call (Phase 1D consensus, Phase 2 Step 6B, Phase 3 Step 8B) runs foreground, blocking, one at a time. No `run_in_background`. No parallel dispatch across phases.

4. **Rules are woven in, not assumed.** Plans must not assume the implementer has CLAUDE.md or `.claude/rules/*.md` loaded. Every section's checklist items embed the specific rules that govern its work — inline as tasks, not as a "rules to follow" appendix.

5. **Phase 0 fork gate keeps ambitious ceremony scoped to correctness-critical work.** All THREE criteria (non-compiler scope + no correctness invariants + prior `/tpr-review` consensus) must hold to take the light path. Otherwise heavy path.

6. **Plan TYPE is a first-class branch of the template.** Compiler-correctness plans, skill/infra/docs plans, and spec/grammar-proposal plans all need proper structure, but their completion rigor differs. The template branches on plan type at Phase 1B classification time, not at Phase 5 cleanup time. (Added 2026-04-17 — see §4.)

7. **Subagent prompts MUST contain explicit scope fences.** Section-writer Sonnet subagents receive implementation-rich task descriptions (code examples, exact file paths, Cypher queries). Without an explicit "write ONE markdown file, do NOT edit any other file, do NOT run git add/commit" preamble, they may pre-implement the code they're describing. (Added 2026-04-17 after Section 04 drift — see §4.)

8. **Sonnet for section writing; Opus for architecture.** Architecture synthesis (Step 7), `00-overview.md` authorship (Step 8), mission expansion (Step 1B), fork-gate evaluation (Phase 0), reviewer-finding triage (Phase 1D) are judgment-writing → Opus. Section template expansion (Step 11), index bookkeeping (Step 12), cohesion scanning (Step 13) are mechanical-writing → Sonnet. Any rewrite that flips this ratio is probably regressing.

## §2 — Load-Bearing Invariants

| # | Invariant | Why (which failure mode it prevents) |
|---|-----------|--------------------------------------|
| I1 | Section-writer subagent prompts MUST include an explicit scope fence ("write ONE file at <path>, do NOT modify any other file, do NOT run git") | Prevents section writers from pre-implementing code and committing to sibling repos. Surfaced 2026-04-17: Section 04's subagent committed 905 LOC to `lang_intelligence/master` because the prompt described implementation details richly and didn't fence scope. |
| I2 | `plan-schema.md` Section File Template's completion checklist branches by plan type | Compiler rigor (TPR checkpoints, matrix testing, semantic/negative pins, §NN.R blocks, /impl-hygiene-review) MUST NOT be hard-coded into skill/infra/docs plans. Those plans then need post-creation stripping (234 lines last session) that should have been template-time branching. |
| I3 | Phase 1B mission expansion MUST classify plan type (`compiler` vs `skill-infra-docs` vs `spec-grammar`) before Phase 2 research begins | Drives template selection in Step 8 (overview) + Step 11 (sections). Classification after-the-fact forces post-hoc cleanup. |
| I4 | `reviewed: false` default is CORRECT for compiler plans (gates pre-implementation re-review) | But `reviewed: true` is correct for skill/infra/docs plans (no pre-implementation gate — the work is low-correctness-risk). Never make the default unconditional either way. |
| I5 | `00-overview.md` is authored by main-context Opus, not a Sonnet subagent | Architecture is judgment-writing. Delegating it loses the synthesis step that makes sections cohere. |
| I6 | Sequential section order (01 → 02 → ... → N) | Parallel section writing produces contradictions because section N references section N-1 decisions that don't exist yet. |
| I7 | Phase 0 fork gate's third criterion (`/tpr-review` consensus reached) must cite a concrete artifact (run directory, commit sha, or summary) — not "reviewers agreed verbally" | Without a citation, the light-path trail is broken; future readers can't verify the design was actually reviewed. |
| I8 | Mission Success Criteria in `00-overview.md` must trace down to at least one section's `success_criteria`; every section criterion must trace up to at least one mission criterion | Gap check catches dangling mission goals (no section delivers them) and orphaned section criteria (not contributing to the mission). |
| I9 | `/review-plan` (Step 16) is optional for skill/infra/docs plans | Applying dual-source `/review-plan` to skill/infra work creates circular dependencies (review infrastructure reviewing review-infrastructure changes) and wastes cycles on low-correctness-risk work. Compiler plans: mandatory. Skill/infra: best-effort. |
| I10 | Reroute setup (Step 18) is MANDATORY and NEVER silently skipped (except for `plans/roadmap/` direct edits) | The reroute queue is load-bearing for `/continue-roadmap` dispatch. A missed reroute setup makes the plan invisible to the main workflow. |

## §3 — File Inventory (canonical)

| Path | Lines (~) | Role |
|------|-----------|------|
| `.claude/skills/create-plan/SKILL.md` | ~900 | Main orchestrator: Phase 0 fork, Phase 1-5 workflow, model policy table, subagent prompt patterns |
| `.claude/skills/create-plan/plan-schema.md` | ~1060 | SSOT for plan structure, frontmatter, templates, status conventions, writing principles |

Note: the skill uses inline markdown content for subagent prompts rather than separate template files (unlike `/tpr-review` which uses `tp_agent_prompt.md`). Subagent prompts are assembled in Step 11b each time.

## §4 — Lessons from Dogfood / Production Runs

### 2026-04-17: `plan-bug-dag-ingestion` creation (skills/infra plan)

**Context:** Created a plan for ingesting plan/bug corpus into Neo4j. Scope touched `scripts/plan_corpus/`, `~/projects/lang_intelligence/`, `.claude/rules/`, `.claude/skills/` — zero compiler source. No prior `/tpr-review` → heavy path by default.

**Findings (ordered by severity):**

1. **CRITICAL — Subagent scope drift.** Section 04's Sonnet subagent was prompted with rich implementation detail (Cypher queries, Python handler patterns, test MagicMock templates). It interpreted the prompt as an implementation task and committed 905 LOC to `~/projects/lang_intelligence/master` (commit `bd66560`, unpushed). The prompt had no explicit "write ONE markdown file, do NOT modify any other file" clause. Mitigation: manually added that clause to the Section 05 + 06 prompts; no drift recurred. Root cause: the subagent prompt template in Step 11b doesn't enumerate the scope fence; each caller is on their own to include it.

2. **HIGH — Compiler rigor in skill/infra template.** The `plan-schema.md` Section File Template hard-codes `/tpr-review`, `/impl-hygiene-review`, `§NN.R Third Party Review Findings` blocks, TPR checkpoints, matrix testing dimensions, semantic/negative pins, `/improve-tooling` retrospectives + section-close sweeps, `/sync-claude` retrospectives + section-close sweeps. For a skill-based plan all of these are ceremony overkill. Post-creation cleanup removed 234 lines across 6 sections — that work should have been template-time branching by plan type.

3. **MEDIUM — `reviewed: false` default wrong for skill/infra plans.** Sections all shipped with `reviewed: false`, which is correct for compiler plans (gates `/continue-roadmap` pre-implementation re-review) but wrong for skill/infra plans (no pre-implementation review gate; the work IS low-stakes). User had to manually flip all 6 sections to `reviewed: true`.

4. **MEDIUM — `§06.6 /review-plan final consensus` hard-coded in verification section.** The verification section template for plan-schema.md mandates a `/review-plan` final-consensus subsection. For skill/infra plans this is ceremony; for spec-grammar plans it is essential. Should branch by plan type.

5. **LOW — Narrow light-path gate.** Phase 0 fork gate criterion 3 (prior `/tpr-review` consensus) is itself heavy ceremony. Most skill/infra plans don't have it and fall through to heavy path by default, inheriting compiler-shaped defaults. A middle option — "heavy structure, skill/infra rigor" — doesn't exist; it's either full-heavy or inline-light. User ended up with "heavy path then manual strip" which is strictly worse than either endpoint.

**Fixes applied 2026-04-17 (this session's `/improve-tooling`):**
- Added plan-type classification at Phase 1B (new Step 1B.1 "Classify plan type: compiler | skill-infra-docs | spec-grammar")
- Added "Skill/Infra/Docs Plan Variant" section to `plan-schema.md` with reduced completion checklist template
- Made Section File Template's completion checklist mode-aware (compiler vs skill/infra/docs vs spec-grammar)
- Added mandatory scope fence to Step 11b subagent prompt template
- Clarified `reviewed:` default behavior per plan type
- Documented `/review-plan` (Step 16) as optional for skill/infra plans

## §5 — Regressions To Watch For

Check each before editing `SKILL.md` or `plan-schema.md`:

- [ ] Section-writer subagent prompt MUST retain the "CRITICAL SCOPE CONSTRAINT" preamble — do NOT remove it to "clean up" the prompt template. Its removal is what caused the 2026-04-17 §04 drift.
- [ ] The Section File Template's completion checklist MUST branch by plan type (compiler / skill-infra-docs / spec-grammar) — do NOT collapse back into a single unconditional template "for simplicity".
- [ ] `reviewed:` frontmatter default MUST vary by plan type. Never hard-code `reviewed: false` or `reviewed: true` as the unconditional default.
- [ ] The `§NN.R Third Party Review Findings` subsection MUST be conditional on plan type. Compiler plans include it; skill/infra/docs plans omit it.
- [ ] Phase 1B MUST classify plan type before Phase 2 research. A "classify after the fact" retrofit always leaves drift.
- [ ] `/review-plan` (Step 16) MUST be listed as optional for skill/infra/docs plans. Mandating dual-source review on skill changes creates circular dependency (reviewing the review infrastructure).
- [ ] `00-overview.md` must be authored by main-context Opus, not a Sonnet subagent. If a rewrite moves overview authorship to a subagent, architecture-synthesis gets lost.
- [ ] Step 18 Reroute Lifecycle MUST remain mandatory and surface the current queue before asking. Silently skipping is a workflow-invisibility bug.

## §6 — Improvement Log

### Open items

- [ ] **[p2]** Consider adding a `/create-plan --type=skill-infra` flag that explicitly selects the reduced template at invocation time, bypassing Phase 1B classification. Useful when the user already knows the plan type. (Surfaced 2026-04-17 — would have shortened this session by ~5 min of manual strip work if the template had branched automatically.)
- [ ] **[p2]** Consider a "plan type inference" heuristic in Phase 0: if the user's scope description touches only `.claude/`, `scripts/`, `diagnostics/`, `lang_intelligence/`, the skill can pre-fill the plan-type question with `skill-infra-docs`. User can still override. (Surfaced 2026-04-17.)
- [ ] **[p3]** The Phase 0 fork gate's three-criterion structure could be renamed to "Phase 0: Classify plan type + scope" to make plan-type classification a first-class part of the gate rather than a Phase 1B retrofit. Deferred because the current structure is already live across many completed plans; changing naming is a doc-migration cost.

### Recently closed

- [x] **2026-04-17** — Added plan-type classification to Phase 1B; added scope-fence preamble to Step 11b subagent prompt; branched Section File Template completion checklist by plan type; documented optional `/review-plan` for skill/infra/docs plans. Commit: pending (this session's `/improve-tooling` retrospective).

## §7 — How To Use This File In Future Sessions

**When to open it:**
- Before making any structural change to `.claude/skills/create-plan/SKILL.md` or `.claude/skills/create-plan/plan-schema.md`
- When debugging a plan that turned out wrong-shaped for its work type
- When a section-writer subagent misbehaves (scope drift, wrong rigor level, template drift)
- When considering a new plan-type category (e.g., "pure-docs plans")

**When to update it:**
- Immediately after any change to `/create-plan`'s SKILL.md or plan-schema.md (add `- [x]` under §6 Recently closed)
- When a plan produced by `/create-plan` needed significant post-creation cleanup (add `- [ ]` under §6 Open with the friction)
- When a subagent misbehavior surfaces (add to §5 Regressions To Watch For)
- When an invariant from §2 proves wrong or needs to change (document the new failure mode in §4, flip the §2 row, update SKILL.md to match)

**What to update:**
- §2 invariants when a new failure mode is discovered
- §5 regressions when a regression pattern appears in the wild
- §6 with every `/improve-tooling` session on `/create-plan`
- §4 Lessons with every dogfood run that surfaces findings
