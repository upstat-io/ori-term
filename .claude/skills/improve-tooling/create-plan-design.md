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

7. **No Sonnet subagents for plan content.** Section files, `00-overview.md`, and `index.md` are all authored inline by the main agent (Opus). Subagents run only for research/orchestration (Steps 3–6 research passes, Step 13 cohesion read-only scan). Plan content is judgment-writing: each section resolves architectural ambiguities that surface only while writing — sync-point enumeration, cross-section dependency phrasing, semantic-pin wording. Delegating to a Sonnet subagent produced scope drift (pre-implementing described code into sibling repos) and template drift (hallucinated paths, mis-referenced prior sections). Context pressure is handled by Read-on-demand of prior sections, not by subagent delegation.

8. **Opus writes; Sonnet reads.** Architecture synthesis (Step 7), `00-overview.md` authorship (Step 8), mission expansion (Step 1B), fork-gate evaluation (Phase 0), reviewer-finding triage (Phase 1D), section authorship (Step 11), overview/index update (Step 12), self-check (Step 14), reroute setup (Step 18) all run in the main agent. Sonnet subagents are read-only research passes (breadth scan, deep read, pattern study, prior-art study, cohesion check). Any rewrite that re-introduces a Sonnet subagent for a task that mutates plan files is a regression.

## §2 — Load-Bearing Invariants

| # | Invariant | Why (which failure mode it prevents) |
|---|-----------|--------------------------------------|
| I1 | Section files, `00-overview.md`, and `index.md` MUST be authored inline by the main agent (Opus) — no Sonnet subagent dispatch for plan content | Section-writer subagents drifted scope twice (2026-04-17: Section 04 subagent committed 905 LOC to `lang_intelligence/master`; recurring template drift from hallucinated paths). Inlining eliminates the dispatch boundary where drift occurred. Context pressure is handled by Read-on-demand of prior sections, not subagent delegation. |
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

### 2026-04-19: §08.3 retrospective — scoped-patch reversal post-commit form + path-list coverage

**Context:** Plan `empty-container-typeck-phase-contract/section-08-codegen-poly-lambda.md` §08.3 specified a §08.2 negative pin via scoped-patch reversal (capture working-tree diff → `git apply -R` → re-run tests → `git apply` restore). Executed during `/roadmap-work` post-commit (§08.3's impl landed in commit `9eae468d` before the negative pin ran). Two authoring defects surfaced.

**Finding 1 — Silent no-op post-commit.** The plan's form was `git diff crates/types/src/pool/re_intern/ src/test/runner/llvm_backend.rs > /tmp/section-08-3.patch` (working-tree vs HEAD). Post-commit, that diff is empty; `git apply -R` on an empty patch is a no-op, and re-run tests pass trivially — a FALSE-POSITIVE "pin passed" signal with no reversal ever occurring. Detection was accidental during `/roadmap-work` execution (Claude noticed the semantic and adapted to `git diff HEAD~1 HEAD -- <paths>`). Without the adaptation, the pin would have been silently meaningless.

**Finding 2 — Path-list coverage gap.** The plan's path list (`re_intern/` + `llvm_backend.rs`) omitted `pool/mod.rs:27`, where §08.3 bundled a re-export line for the new `re_intern_type_with_var_remap` + `re_intern_sig_with_var_remap` functions. The reverse-apply therefore produced a COMPILE FAILURE (re-export referring to removed symbols) rather than the predicted test-count regression (35→25). The compile failure is a STRONGER negative-pin signal than test-count regression, but the plan's predicted mode was wrong — plan authors writing scoped-patch reversal pins must enumerate every file touched by the implementing commit, not just the plan's "primary surface" paths. `git show --stat <commit>` is the ground truth.

**Fixes applied (this session):**
- This §4 entry + §5 regression guards below.
- Plan `§08.3` item 2 text annotated inline with the adapted `HEAD~1 HEAD` form and the stronger-than-expected compile-fail outcome (`plans/empty-container-typeck-phase-contract/section-08-codegen-poly-lambda.md` line 318).

**No script created.** Filter check: scoped-patch reversal is a niche negative-pin technique (~1x per plan that uses it); a `scripts/scoped-patch-reversal.sh` wrapper would add infrastructure for a marginal use. Documentation-only improvement passes the filter; helper script does not.

### 2026-04-19: Step 11 inlined — retire Sonnet section-writer subagent

**Context:** User directive via `/improve-tooling`: "make sure sonnet is not writing any plan content in /create-plan, if it is, that needs to be removed from sub-agent and inlined." Audit found Step 11 (Write Sections Sequentially via Sonnet Subagents) dispatched a `model: "sonnet"` Agent that called Write on `section-{NN}-*.md` files — plan content. Steps 12, 13, 15 were not writing plan content (12 was already main-agent; 13 is read-only cohesion scan; 15 is a progress summary).

**Root cause of the prior design:** Context-window conservation. Section text is thousands of tokens × 8+ sections; the original rationale was "Opus holds only the architecture; Sonnet holds the per-section text." But the dispatch boundary is also where scope drift surfaced (2026-04-17 Section 04: 905 LOC committed to `lang_intelligence/master`). The context saving did not justify the drift cost.

**Fix applied:** Step 11 rewritten as main-agent inline writing (Step 11a: gather context via Read; Step 11b: select template by PLAN TYPE; Step 11c: Write the section file; Step 11d: self-verify). Context pressure is now handled by Read-on-demand of prior sections rather than subagent delegation — the main agent does not need to retain earlier section text in active context to cross-reference it; it re-Reads only when needed.

**Invariants changed:** §1.7 rewritten ("No Sonnet subagents for plan content" replaces "Subagent prompts MUST contain scope fences"). §1.8 rewritten ("Opus writes; Sonnet reads" replaces the previous Opus-for-architecture / Sonnet-for-sections split). §2 I1 rewritten to forbid subagent dispatch for plan content rather than mandate a scope fence. §5 regressions flipped to guard against re-introducing Sonnet dispatch.

**Scope:** Research passes (Steps 3–6) remain Sonnet subagents — they READ code and return findings, not plan content. Step 13 (cohesion check) remains Sonnet — same rationale (read-only, returns findings).

## §5 — Regressions To Watch For

Check each before editing `SKILL.md` or `plan-schema.md`:

- [ ] Step 11 MUST stay main-agent inline. Do NOT re-introduce a `model: "sonnet"` Agent dispatch for section writing, even with a "better scope fence". The dispatch boundary is what drifts; removing the boundary removes the drift.
- [ ] Step 12 (Update Overview and Index) MUST stay main-agent inline. Same reasoning — `00-overview.md` and `index.md` are plan content.
- [ ] The Section File Template's completion checklist MUST branch by plan type (compiler / skill-infra-docs / spec-grammar) — do NOT collapse back into a single unconditional template "for simplicity".
- [ ] `reviewed:` frontmatter default MUST vary by plan type. Never hard-code `reviewed: false` or `reviewed: true` as the unconditional default.
- [ ] The `§NN.R Third Party Review Findings` subsection MUST be conditional on plan type. Compiler plans include it; skill/infra/docs plans omit it.
- [ ] Phase 1B MUST classify plan type before Phase 2 research. A "classify after the fact" retrofit always leaves drift.
- [ ] `/review-plan` (Step 16) MUST be listed as optional for skill/infra/docs plans. Mandating dual-source review on skill changes creates circular dependency (reviewing the review infrastructure).
- [ ] `00-overview.md` must be authored by main-context Opus, not a Sonnet subagent. If a rewrite moves overview authorship to a subagent, architecture-synthesis gets lost.
- [ ] Step 18 Reroute Lifecycle MUST remain mandatory and surface the current queue before asking. Silently skipping is a workflow-invisibility bug.
- [ ] Scoped-patch reversal pins using `git diff <paths>` (no refs) are a hazard POST-COMMIT — the diff is empty and the reversal is a silent no-op producing a false-positive "pin passed" signal. Plan authors MUST write the post-commit form `git diff <pre-impl-ref> <post-impl-ref> -- <paths>` (e.g., `HEAD~1 HEAD`) whenever the pin runs after the implementing commit has landed. See §4 2026-04-19 `§08.3 retrospective` entry.
- [ ] Scoped-patch reversal pins MUST enumerate every file touched by the implementing commit, not just the plan's "primary surface" paths. Bundled re-export edits in sibling `mod.rs` / `lib.rs` / similar files will produce compile-fails (not test-count regressions) when the pin runs. Plan authors: run `git show --stat <impl-commit>` against the implementing commit to ground-truth the path list before writing the pin.

## §6 — Improvement Log

### Open items

- [ ] **[p2]** Consider adding a `/create-plan --type=skill-infra` flag that explicitly selects the reduced template at invocation time, bypassing Phase 1B classification. Useful when the user already knows the plan type. (Surfaced 2026-04-17 — would have shortened this session by ~5 min of manual strip work if the template had branched automatically.)
- [ ] **[p2]** Consider a "plan type inference" heuristic in Phase 0: if the user's scope description touches only `.claude/`, `scripts/`, `diagnostics/`, `lang_intelligence/`, the skill can pre-fill the plan-type question with `skill-infra-docs`. User can still override. (Surfaced 2026-04-17.)
- [ ] **[p3]** The Phase 0 fork gate's three-criterion structure could be renamed to "Phase 0: Classify plan type + scope" to make plan-type classification a first-class part of the gate rather than a Phase 1B retrofit. Deferred because the current structure is already live across many completed plans; changing naming is a doc-migration cost.

### Recently closed

- [x] **2026-04-19** — §08.3 retrospective: documented scoped-patch reversal post-commit form (`git diff HEAD~1 HEAD -- <paths>` vs working-tree `git diff <paths>` silent-no-op hazard) + path-list coverage requirement (`git show --stat <impl-commit>` as ground truth for enumerating bundled sibling-file edits) as §4 Lessons + two §5 regression guards. Helper script filtered out — niche negative-pin pattern, doc-only fix is sufficient. Commit: `127531c2`.
- [x] **2026-04-19** — Retired Sonnet section-writer subagent. Step 11 rewritten as main-agent inline writing (Step 11a gather → 11b select template → 11c Write → 11d self-verify). Model Policy table, §1.7, §1.8, §2 I1, §5 regressions all updated. Context-pressure relief now handled by Read-on-demand of prior sections. Commit: pending (this `/improve-tooling` session).
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
