# Plan Schema

The single source of truth for plan structure. All plans in `plans/` and `plans/completed/` must conform to this schema. Referenced by `/create-plan` (creation) and `/continue-roadmap` (validation).

---

## Directory Layout

```
plans/{plan-name}/
├── index.md           # Keyword clusters for quick finding
├── 00-overview.md     # Mission, architecture, dependencies, phasing, metrics
├── section-01-*.md    # First section
├── section-02-*.md    # Second section
└── ...
```

---

## Overview File Template (`00-overview.md`)

The overview is the master document. It answers: **what** is the goal, **why** does it matter, **how** do the pieces fit together, and **in what order** should they be built?

```markdown
---
plan: "{plan-name}"
title: "{Plan Title}: Exhaustive Implementation Plan"
status: not-started
supersedes:             # Plans this replaces (if any)
  - "plans/{old-plan}/"
references:             # Design docs, proposals, prior art
  - "plans/{related-doc}.md"
---

# {Plan Title}: Exhaustive Implementation Plan

## Mission

{1-2 sentences. What is this plan accomplishing and why? Not "implement X" but "complete X as one cohesive system: from A through B to C." Establish scope and intent.}

## Mission Success Criteria

{The mission is complete when ALL of these are true. Each criterion must be concrete, testable, and verifiable — not "X works" but "X produces Y when Z is run." Section success criteria are the building blocks — when every section meets its own criteria, the mission criteria must follow. Every mission criterion must be traceable to at least one section that delivers it.}

- [ ] {Criterion 1 — specific, measurable, verifiable condition}
- [ ] {Criterion 2 — with command or test that proves it}
- [ ] {Criterion 3 — connects to section(s) that deliver it}
- [ ] `./test-all.sh` green — no regressions
- [ ] All section success criteria met

## Architecture

\`\`\`
{ASCII diagram showing the pipeline/system being built or modified.
Show the flow of data through stages, the key types at each boundary,
and where this plan's sections fit in.}
\`\`\`

## Design Principles

{Name the core architectural principle(s) driving this plan's design.
Explain WHY these matter — cite concrete bugs or pain points that
motivated the principle. 2-3 principles max.}

\`\`\`
{Optional: show the information/data flow chain if applicable.
E.g., how each stage enriches data for the next stage.}
\`\`\`

## Section Dependency Graph

\`\`\`
{ASCII graph showing section dependencies.
Use arrows to show what depends on what.
Note which sections are independent (parallelizable).}
\`\`\`

{Prose explanation:}
- Sections {X-Y} are independent and can be worked in any order.
- Section {Z} requires {X}. Section {W} requires all.

**Cross-section interactions (must be co-implemented):**
- **{Section A} + {Section B}**: {Why these must land together. Cite the
  specific bug or invariant that breaks if only one lands.}

## Implementation Sequence

{Resolve the dependency graph into a concrete build order. Each phase
gates the next; items within a phase can be parallelized.}

\`\`\`
Phase 0 - Prerequisites
  └─ {section}: {task description}

Phase 1 - Foundation
  └─ {section.subsection}: {task}
  └─ {section.subsection}: {task}

Phase 2 - Core implementation
  └─ {section.subsection}: {task}
  Gate: {testable condition proving this phase is complete}

Phase 3 - Integration  [CRITICAL PATH]
  └─ {section.subsection}: {task}
  Gate: {testable condition}

Phase N - Verification
  └─ {section}: {comprehensive testing}
\`\`\`

**Why this order:**
- Phase 0-1 are pure additions — no behavioral changes.
- Phase 2 must precede Phase 3 because {reason}.
- Phase 3 is the critical path because {reason}.

**Known failing tests (expected until plan completion):**

{List tests that are expected to fail and WHY. Prevents wasted effort
investigating "failures" that are symptoms of missing infrastructure.
Include root causes tied to specific phases.}

- **`test_name`** — {symptom}. Root cause: {Phase N} ({missing infrastructure}).

Do NOT attempt to fix these tests individually. They share infrastructure
dependencies that must be built bottom-up through Phases {X-Y}.

## Metrics (Current State)

{Baseline measurements before implementation begins. Establishes the
starting point so progress and regressions can be measured.}

| Crate | Production LOC | Test LOC | Total |
|-------|---------------|----------|-------|
| `{crate}` | ~{N} | ~{N} | ~{N} |
| **Total** | **~{N}** | **~{N}** | **~{N}** |

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| {NN} {Title} | ~{N} | Low/Medium/High | — |
|   ↳ {NN.X} {Subsection} | ~{N} | Low | — |
| **Total new** | **~{N}** | | |
| **Total deleted** | **~{N}** | | |

## Known Bugs (Pre-existing)

{Bugs discovered during investigation that affect multiple sections.
Track root causes, fix locations, and status so they don't get lost.}

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| {Description} | {Root cause analysis} | Section {NN} | Not Started / Fixed / Guarded |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | {Title} | `section-01-{name}.md` | Not Started |
| 02 | {Title} | `section-02-{name}.md` | Not Started |
```

---

## Index File Template (`index.md`)

The index enables keyword-based discovery across all sections. If this plan is a
**reroute** (a parallel track alongside the main roadmap), add frontmatter to make
it discoverable:

```yaml
---
reroute: true
name: "{Short Name}"
full_name: "{Full Plan Name}"
status: queued
order: N
---
```

- `reroute: true` — marks this plan as a reroute (omit for non-reroute plans)
- `name` — short display name (e.g., "GPU Fixes")
- `full_name` — full display name (e.g., "GPU Pipeline Fixes")
- `status` — `active | queued | resolved`
- `order` — queue priority; lower value = promoted first when active reroute completes (default 999 if omitted)
- `key` and `dir` are derived at load time from the directory name

```markdown
# {Plan Name} Index

> **Maintenance Notice:** Update this index when adding/modifying sections.
> **Supersedes:** `plans/{old-plan}/` (if applicable)

## How to Use

1. Search this file (Ctrl+F) for keywords
2. Find the section ID
3. Open the section file

---

## Keyword Clusters by Section

### Section 01: {Title}
**File:** `section-01-{name}.md` | **Status:** Not Started

\`\`\`
keyword1, keyword2, keyword3
formal term, common alias, abbreviation
file_path.rs, function_name, TypeName
reference implementation term, prior art concept
\`\`\`

---

### Section 02: {Title}
**File:** `section-02-{name}.md` | **Status:** Not Started

\`\`\`
keywords here
\`\`\`

---

## Quick Reference

| ID | Title | File |
|----|-------|------|
| 01 | {Title} | `section-01-{name}.md` |
| 02 | {Title} | `section-02-{name}.md` |
```

---

## Section File Template

Each section file follows this structure. Sections range from focused (single subsection) to comprehensive (5+ subsections with deep analysis).

```markdown
---
section: "{NN}"
title: "{Title}"
status: not-started
reviewed: false
goal: "{One-line measurable goal}"
success_criteria:        # Concrete conditions proving this section is done
  - "{Criterion 1 — testable, verifiable}"
  - "{Criterion 2 — with command or observable result}"
inspired_by:             # Reference implementations studied
  - "{Project} {pattern} ({file path})"
depends_on: ["{NN}"]     # Other sections required first
third_party_review:
  status: none           # none | findings | resolved
  updated: null          # YYYY-MM-DD when last touched
sections:
  - id: "{NN}.1"
    title: "{Subsection}"
    status: not-started
  - id: "{NN}.2"
    title: "{Subsection}"
    status: not-started
  - id: "{NN}.R"
    title: "Third Party Review Findings"
    status: not-started
  - id: "{NN}.N"
    title: "Completion Checklist"
    status: not-started
# ── TPR Checkpoint Placement ──
# For sections with 3+ implementation subsections, add intermediate
# `/tpr-review` checkpoints after every 2-3 completed subsections.
# Mark these as `- [ ] **TPR checkpoint** — ...` items at the END of
# the last subsection in each group. The final `/tpr-review` in the
# Completion Checklist ({NN}.N) still runs — it catches integration
# issues across the full section.
#
# Example for a 6-subsection section:
#   {NN}.1  Implementation A
#   {NN}.2  Implementation B
#   {NN}.3  Implementation C  ← TPR checkpoint here (covers .1-.3)
#   {NN}.4  Implementation D
#   {NN}.5  Implementation E  ← TPR checkpoint here (covers .4-.5)
#   {NN}.R  Third Party Review Findings
#   {NN}.N  Completion Checklist  ← final TPR (full section)
---

# Section {NN}: {Title}

**Status:** Not Started
**Goal:** {Expanded goal — what must be true when this section is complete.
Not "implement X" but "X works correctly under conditions A, B, C with
no regressions in Y."}

**Success Criteria:**
{Concrete, testable conditions that prove this section's work is done.
Each criterion should be independently verifiable. Together, these criteria
contribute to one or more mission success criteria in `00-overview.md`.}

- [ ] {Criterion — specific behavioral outcome with verification method}
- [ ] {Criterion — test name, command, or observable result}
- [ ] {Criterion — connects upward to mission criterion: "Satisfies mission criterion N"}

**Context:** {Why this section exists. What pain point, bug, or
architectural gap motivated it. Cite specific debugging sessions,
test failures, or design flaws. 2-4 sentences.}

**Reference implementations:**
- **{Project}** `{file path}`: {pattern name} — {what we learn from it}
- **{Project}** `{file path}`: {pattern name} — {what we learn from it}

**Depends on:** Section {NN} ({why}).

---

## {NN}.1 {Subsection Title}

**File(s):** `{file path(s) being modified}`

{Context paragraph: what this subsection does, what problem it solves,
and how it fits into the section's overall goal.}

- [ ] {Task description with enough detail to implement without ambiguity}
  \`\`\`rust
  // Code example showing the target design (types, signatures, key logic).
  // This is the SPEC — the implementation should match this.
  \`\`\`

- [ ] {Another task}
  - [ ] {Sub-task with specific file + function to modify}
  - [ ] {Sub-task}

- [ ] {Validation task — how to verify this subsection works}

---

## {NN}.2 {Subsection with Design Decisions}

**File(s):** `{file path(s)}`

**Context:** {The problem requiring a design decision.}

{Detailed analysis of the problem — what was tried, what failed, why.
Include debugging traces, root cause analysis, data from experiments.}

**Fix approach — {N} options:**

**(a) {Recommended approach}** (recommended — {why}):
{Detailed description with code examples.}

\`\`\`rust
// Target implementation
\`\`\`

**Why this is best:** {Justify against alternatives. Cite the
architectural principle it upholds.}

**Trade-off:** {What this approach costs or complicates.}

**(b) {Alternative approach}** ({characterization}):
{Description with code.}
**Downside:** {Why this is worse than (a).}

**(c) {Least recommended}** (not recommended):
{Brief description.}
**Downside:** {Why.}

**Recommended path:** Option (a) for {reason}, with option (b) as
acceptable interim if {condition}.

- [ ] **TPR checkpoint** — `/tpr-review` covering {NN}.1–{NN}.2 implementation work
  <!-- For sections with 3+ implementation subsections, place intermediate
       TPR checkpoints after every 2-3 completed subsections. This catches
       design drift, missed edge cases, and hygiene issues BEFORE they
       compound across the remaining subsections. The checkpoint item lives
       at the end of the last subsection in the group. -->

### {Sub-topic within the subsection}

**Discovery:** {What was learned during investigation that changes
the approach or adds requirements.}

**Implementation steps:**
1. {Specific, numbered, actionable step with file path}
2. {Step referencing specific functions to modify}
3. {Validation step — what test to run, what output to expect}

**Reference implementations:**
- **{Project}** `{file}`: {what it does} — {what we adopt from it}

**Co-implementation requirement with Section {NN} ({topic}):**
{Why this subsection and another section's work must land together.
What breaks if only one lands. Be specific about the failure mode.}

---

## {NN}.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers.
If unresolved findings exist here:
- section frontmatter `status` must be `in-progress`
- `third_party_review.status` must be `findings`

When all findings are triaged:
- accepted findings are integrated into the relevant implementation subsection(s)
- rejected findings are closed with rationale
- all items in this block are marked resolved
- `third_party_review.status` becomes `resolved` or `none`
-->

- None.

---

## {NN}.N Completion Checklist

- [ ] {Concrete, verifiable item — not "implement X" but "X passes test Y"}
- [ ] {Item with specific command to verify: `cargo test -p oriterm_core -- test_name` passes}
- [ ] {Behavioral verification: `test_name` passes without modification}
- [ ] {Regression check: `./test-all.sh` green}
- [ ] {Build check: `./build-all.sh` green, `./clippy-all.sh` green}
- [ ] {No spurious warnings in normal compilation}
- [ ] Plan annotation cleanup: all temporary scaffolding (TPR, CROSS, BUG, §, Phase, section- refs) removed from `.rs` files
- [ ] All intermediate TPR checkpoint findings resolved (see checkpoint items in subsections above)
- [ ] **Plan sync** — update plan metadata to reflect this section's completion:
  - [ ] This section's frontmatter `status` → `complete`, subsection statuses updated
  - [ ] `00-overview.md` Quick Reference table status updated for this section
  - [ ] `00-overview.md` mission success criteria checkboxes updated (check off any now satisfied)
  - [ ] `index.md` section status updated
  - [ ] Cross-links to other plans updated if this section resolved external blockers (`<!-- resolved-by: ... -->`)
  - [ ] Next section's `depends_on` verified — no stale assumptions from this section's work
- [ ] `/tpr-review` passed (final, full-section) — independent Codex review found no critical or major issues (or all findings triaged)
- [ ] `/impl-hygiene-review last commit` passed — implementation hygiene review found no critical or major findings (or all findings triaged and fixed). MUST run AFTER `/tpr-review` is clean.

**Exit Criteria:** {Paragraph describing the measurable, testable condition
that proves this section is complete. Include specific commands, test names,
metric thresholds. Not "X works" but "X produces Y output when Z command
is run, with 0 regressions in test suite (N tests)."}
```

---

## Verification Section Template

Every plan should include a verification section (typically the last section). This proves the system works as one cohesive whole.

```markdown
## {NN}.1 Test Matrix

Build a comprehensive test matrix covering every feature through the
pipeline being built/modified.

- [ ] **{Feature category}:** ({date started})
  - {Sub-feature} — {status: covered (file.rs) | FIXED (date) | gap: reason (#[ignore])}
  - {Sub-feature} — {status}

### {NN}.1.1 Discovered Gaps

| Gap | Roadmap Location | Test | Severity |
|-----|-----------------|------|----------|
| {Description} | {Section reference} | `test_name` | CRITICAL / Medium / Low |

---

## {NN}.2 Performance Validation

- [ ] **{Metric 1}:** Measured {what} ({conditions}):
  - {Workload A}: ~{value}
  - Script: `{script path}`

- [ ] **Zero idle CPU beyond cursor blink** — verified by `compute_control_flow()` tests
- [ ] **Zero allocations in hot render path** — verified by alloc regression tests
- [ ] **Stable RSS under sustained output** — verified by scrollback bounds

---

## {NN}.3 Safety Verification (if applicable)

- [ ] **{Safety property}:** {How it's verified, what tool/technique}
- [ ] **Stress test:** {Scale — N cells, N scrollback lines, N resize cycles}
- [ ] **Cross-platform:** {Verified on macOS, Windows, Linux}

---

## {NN}.4 Build & Verify

- [ ] `./build-all.sh` green (all platforms)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `./test-all.sh` green (all tests pass)
- [ ] Architecture tests pass (`cargo test -p oriterm --test architecture`)

---

## {NN}.5 Documentation

- [ ] Update superseded plans to point to this plan
- [ ] Update CLAUDE.md if new commands/paths/patterns introduced
- [ ] Update relevant .claude/rules/*.md files
- [ ] Add architecture overview to key module docs

---

## {NN}.6 Completion Checklist

- [ ] Test matrix covers all features (every checkbox in {NN}.1)
- [ ] Performance validated (zero idle CPU, zero hot-path allocs, stable RSS)
- [ ] All builds green
- [ ] All documentation updated
- [ ] Plan annotation cleanup: all temporary scaffolding removed from `.rs` files
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `/tpr-review` passed — independent Codex review clean
- [ ] `/impl-hygiene-review last commit` passed — hygiene review clean. MUST run AFTER `/tpr-review` is clean.

**Exit Criteria:** {Final measurable proof. Include test counts, metric
thresholds, and the specific commands that demonstrate completion.}
```

---

## Status Conventions

### Section and Subsection Status (section files, `00-overview.md`)

| YAML Status | Meaning | Notes |
|-------------|---------|-------|
| `not-started` | No work done | |
| `in-progress` | Partial completion | Include date + current state in header |
| `complete` | All done | Include completion date in header |

Sections AND subsections use the same values: `not-started`, `in-progress`, `complete`. Do NOT use `done` — always use `complete`.

### Plan-Level Status (`index.md` — website-facing)

| YAML Status | Meaning |
|-------------|---------|
| `active` | Currently being worked on |
| `queued` | Waiting in queue (lower `order` = promoted first) |
| `resolved` | Completed and archived |

Do NOT use `done` or `complete` in `index.md` — always use `resolved` for finished plans.

### Completed Plans

When all sections are `complete`, the plan is archived:
1. Set `index.md` status to `resolved`
2. Set `00-overview.md` status to `complete`
3. Move to `plans/completed/` via `git mv`

**Progress tracking conventions:**
- `[x]` — completed (include date: `(2026-02-24)`)
- `[ ]` — not started
- `**FIXED** (date)` — a bug discovered and fixed during implementation
- `#[ignore]` — test exists but is skipped due to known gap
- Commit references: `(committed c1c1b534)` for traceability
- Strikethrough `~~text~~` for gaps that were fixed (preserves history)

---

## Writing Principles

### Context Over Brevity
Each section should be self-contained enough that someone can understand
WHY the work exists, not just WHAT to do. Include the bug report, the
debugging session insight, the architectural principle that motivates it.

### Rules Woven In, Not Assumed
Plans cannot assume the implementer has CLAUDE.md or `.claude/rules/*.md`
loaded in context. Every section must embed the specific rules that
govern its work — woven organically into checklist items, constraints,
and callouts. If a rule applies, it appears in the task description
itself: "Add `FooWidget` in `oriterm_ui/src/widgets/foo/mod.rs` —
implement Widget trait, create sibling `tests.rs` with WidgetTestHarness"
rather than "Add widget (check conventions)." The plan is a self-contained
execution document. Relevant rule files: `test-organization.md` (TDD,
sibling tests.rs), `impl-hygiene.md` (module boundaries, file size,
rendering discipline), `code-hygiene.md` (surface cleanliness),
`crate-boundaries.md` (ownership rules).

### Measurable Exit Criteria
"Implement X" is not an exit criterion. "{Command} produces {output}
with 0 failures across {N} tests" is. Every section ends with a
testable, verifiable condition.

### Success Criteria Hierarchy
The plan has mission-level success criteria in `00-overview.md`. Each
section has its own success criteria. Section criteria are the building
blocks — when every section meets its criteria, the mission criteria
must follow. Every mission criterion must trace to at least one section
that delivers it, and every section criterion must trace upward to at
least one mission criterion it contributes to. A section without
success criteria is not executable. A mission criterion that no section
delivers is a gap in the plan.

### Design Decisions with Trade-offs
When there are multiple approaches, document all of them with pros/cons.
Mark the recommended approach and explain why. This prevents re-litigating
decisions and helps future readers understand the reasoning.

### Cross-References
Link sections that interact. When Section A depends on Section B,
explain the specific failure mode if only one lands. Use
"Co-implementation requirement" callouts for hard dependencies.

### Root Cause Analysis
When a bug or design flaw motivated a section, include the root cause
chain. "X broke because Y, which happened because Z, which is
fundamentally caused by W." This prevents surface-level fixes.

### TPR Cadence — Review Early, Review Often
Don't save `/tpr-review` for the very end of a section. For sections
with 3+ implementation subsections, place **intermediate TPR checkpoints**
after every 2-3 completed subsections of finished work. This catches
design drift, hygiene violations, and missed edge cases *before* they
compound across remaining subsections — fixing an issue in subsection 2
is cheap; discovering it after subsection 6 means rework across 4
subsections. The final TPR in the Completion Checklist still runs as a
full-section integration review. Larger, more complex subsections
(high estimated lines, cross-crate changes, new data flow paths) should
trigger a checkpoint sooner rather than later.

### Reference Implementations
Cite specific files from reference projects. Not "Alacritty does
this" but "Alacritty's `alacritty/src/display/damage.rs` uses the
damage tracking pattern where {description}." Include the path so the
reference can be consulted.

---

## TPR Checkpoint Rules

`/tpr-review` should not only run at the end of a section — it should run **mid-section after subsections that produce finished, testable code**. Catching issues early is far cheaper than catching them at the end of a large section.

### When to insert a TPR checkpoint in a subsection

A subsection gets a `- [ ] /tpr-review checkpoint` item when **any** of:

1. **Substantial new code** — the subsection adds or modifies ~100+ lines of production code across multiple files.
2. **New module or public API** — introduces a new module, trait, or public interface that later subsections build on. Catching design issues here prevents cascading rework.
3. **Complex logic** — implements non-trivial algorithms, state machines, or cross-module coordination where subtle bugs hide.
4. **Integration boundary** — wires together components from different crates or layers (e.g., connecting UI to GPU, mux to PTY).

### When NOT to insert a TPR checkpoint

- **Scaffolding-only subsections** — adding type stubs, config fields, or empty trait impls that aren't yet wired in.
- **Small mechanical changes** — renaming, moving files, updating imports, adjusting constants.
- **Test-only subsections** — adding tests for already-reviewed code.

### Placement

The TPR checkpoint is the **last item** in the subsection's task list, after validation tasks. It runs after the subsection's code is finished and tests pass.

### Section completion checklist still required

Mid-section TPR checkpoints do **not** replace the final `/tpr-review` in the `{NN}.N Completion Checklist`. The final TPR covers cross-subsection interactions and the section as a whole.

---

## Reference

See the roadmap (`plans/roadmap/`) as a working example of this schema in use.
