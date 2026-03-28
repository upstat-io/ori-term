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
- **{Section A} + {Section B}**: {Why these must land together.}

## Implementation Sequence

{Resolve the dependency graph into a concrete build order.}

\`\`\`
Phase 0 - Prerequisites
  └─ {section}: {task description}

Phase 1 - Foundation
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

**Known failing tests (expected until plan completion):**

- **`test_name`** — {symptom}. Root cause: {Phase N} ({missing infrastructure}).

## Metrics (Current State)

| Crate | Production LOC | Test LOC | Total |
|-------|---------------|----------|-------|
| `{crate}` | ~{N} | ~{N} | ~{N} |
| **Total** | **~{N}** | **~{N}** | **~{N}** |

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| {NN} {Title} | ~{N} | Low/Medium/High | — |
| **Total new** | **~{N}** | | |

## Known Bugs (Pre-existing)

| Bug | Root Cause | Fix Location | Status |
|-----|-----------|-------------|--------|
| {Description} | {Root cause analysis} | Section {NN} | Not Started |

## Quick Reference

| ID | Title | File | Status |
|----|-------|------|--------|
| 01 | {Title} | `section-01-{name}.md` | Not Started |
| 02 | {Title} | `section-02-{name}.md` | Not Started |
```

---

## Index File Template (`index.md`)

The index enables keyword-based discovery across all sections. If this plan is a
**reroute** (a parallel track alongside the main roadmap), add frontmatter:

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
- `full_name` — full display name
- `status` — `active | queued | resolved`
- `order` — queue priority; lower value = promoted first

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

Each section file follows this structure.

```markdown
---
section: "{NN}"
title: "{Title}"
status: not-started
reviewed: false
goal: "{One-line measurable goal}"
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
---

# Section {NN}: {Title}

**Status:** Not Started
**Goal:** {Expanded goal — what must be true when this section is complete.}

**Context:** {Why this section exists. What pain point, bug, or
architectural gap motivated it. 2-4 sentences.}

**Reference implementations:**
- **{Project}** `{file path}`: {pattern name} — {what we learn from it}

**Depends on:** Section {NN} ({why}).

---

## {NN}.1 {Subsection Title}

**File(s):** `{file path(s) being modified}`

{Context paragraph: what this subsection does, what problem it solves.}

- [ ] {Task description with enough detail to implement without ambiguity}
  \`\`\`rust
  // Code example showing the target design
  \`\`\`

- [ ] {Another task}
  - [ ] {Sub-task with specific file + function to modify}

- [ ] {Validation task — how to verify this subsection works}

---

## {NN}.2 {Subsection with Design Decisions}

**File(s):** `{file path(s)}`

**Context:** {The problem requiring a design decision.}

**Fix approach — {N} options:**

**(a) {Recommended approach}** (recommended — {why}):
{Detailed description with code examples.}

**Why this is best:** {Justify against alternatives.}

**(b) {Alternative approach}**:
**Downside:** {Why this is worse than (a).}

**Recommended path:** Option (a) for {reason}.

---

## {NN}.R Third Party Review Findings

<!-- Reserved for Codex or other external reviewers.
If unresolved findings exist here:
- section frontmatter `status` must be `in-progress`
- `third_party_review.status` must be `findings`
-->

- None.

---

## {NN}.N Completion Checklist

- [ ] {Concrete, verifiable item — not "implement X" but "X passes test Y"}
- [ ] {Behavioral verification: `test_name` passes without modification}
- [ ] {Regression check: `./test-all.sh` green}
- [ ] {Build check: `./build-all.sh` green, `./clippy-all.sh` green}
- [ ] `/tpr-review` passed — independent Codex review found no critical or major issues (or all findings triaged)

**Exit Criteria:** {Paragraph describing the measurable, testable condition
that proves this section is complete.}
```

---

## Verification Section Template

Every plan should include a verification section (typically the last section).

```markdown
## {NN}.1 Test Matrix

Build a comprehensive test matrix covering every feature.

- [ ] **{Feature category}:** ({date started})
  - {Sub-feature} — {status: covered (file.rs) | FIXED (date) | gap: reason}

### {NN}.1.1 Discovered Gaps

| Gap | Roadmap Location | Test | Severity |
|-----|-----------------|------|----------|
| {Description} | {Section reference} | `test_name` | CRITICAL / Medium / Low |

---

## {NN}.2 Performance Validation (if applicable)

- [ ] **{Metric 1}:** Measured {what} ({conditions}):
  - {Workload A}: ~{value}
  - Script: `{script path}`

- [ ] **Zero idle CPU beyond cursor blink** — verified by `compute_control_flow()` tests
- [ ] **Zero allocations in hot render path** — verified by alloc regression tests

---

## {NN}.3 Build & Verify

- [ ] `./build-all.sh` green (all platforms)
- [ ] `./clippy-all.sh` green (no warnings)
- [ ] `./test-all.sh` green (all tests pass)
- [ ] Architecture tests pass (`cargo test -p oriterm --test architecture`)

---

## {NN}.4 Documentation

- [ ] Update superseded plans to point to this plan
- [ ] Update CLAUDE.md if new commands/paths/patterns introduced
- [ ] Update relevant .claude/rules/*.md files

---

## {NN}.5 Completion Checklist

- [ ] Test matrix covers all features
- [ ] Performance validated (if applicable)
- [ ] All builds green
- [ ] All documentation updated
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `/tpr-review` passed clean

**Exit Criteria:** {Final measurable proof.}
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

### Plan-Level Status (`index.md`)

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
- Commit references: `(committed c1c1b534)` for traceability
- Strikethrough `~~text~~` for gaps that were fixed (preserves history)

---

## Writing Principles

### Context Over Brevity
Each section should be self-contained enough that someone can understand
WHY the work exists, not just WHAT to do.

### Measurable Exit Criteria
"Implement X" is not an exit criterion. "{Command} produces {output}
with 0 failures across {N} tests" is.

### Design Decisions with Trade-offs
When there are multiple approaches, document all of them with pros/cons.
Mark the recommended approach and explain why.

### Cross-References
Link sections that interact. When Section A depends on Section B,
explain the specific failure mode if only one lands.

### Root Cause Analysis
When a bug or design flaw motivated a section, include the root cause
chain.

### Reference Implementations
Cite specific files from reference projects. Not "Alacritty does
this" but "Alacritty's `alacritty/src/display/damage.rs` uses the
damage tracking pattern where {description}."

---

## Reference

See the roadmap (`plans/roadmap/`) as a working example of this schema in use.
