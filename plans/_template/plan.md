# Plan Template

Use this template when creating new plans in `plans/`.

---

## Directory Layout

```
plans/{plan-name}/
+-- index.md           # Keyword clusters for quick finding
+-- 00-overview.md     # Mission, architecture, dependencies, phasing, metrics
+-- section-01-*.md    # First section
+-- section-02-*.md    # Second section
+-- ...
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

\`\`\`
{Optional: show the information/data flow chain if applicable.
E.g., how input events flow through the system, or how grid state
reaches the GPU.}
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
  +-- {section}: {task description}

Phase 1 - Foundation
  +-- {section.subsection}: {task}
  +-- {section.subsection}: {task}

Phase 2 - Core implementation
  +-- {section.subsection}: {task}
  Gate: {testable condition proving this phase is complete}

Phase 3 - Integration  [CRITICAL PATH]
  +-- {section.subsection}: {task}
  Gate: {testable condition}

Phase N - Verification
  +-- {section}: {comprehensive testing}
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

| Module | Production LOC | Test LOC | Total |
|--------|---------------|----------|-------|
| `{module}` | ~{N} | ~{N} | ~{N} |
| **Total** | **~{N}** | **~{N}** | **~{N}** |

## Estimated Effort

| Section | Est. Lines | Complexity | Depends On |
|---------|-----------|------------|------------|
| {NN} {Title} | ~{N} | Low/Medium/High | — |
|   -> {NN.X} {Subsection} | ~{N} | Low | — |
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
- `name` — short display name (e.g., "GPU Refactor")
- `full_name` — full display name (e.g., "GPU Renderer Refactor")
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
reviewed: true/false             # true for Section 01; false for all others
third_party_review:
  status: none                   # none | findings | resolved
  updated: null                  # YYYY-MM-DD when findings were last triaged
goal: "{One-line measurable goal}"
inspired_by:             # Reference implementations studied
  - "{Emulator} {pattern} ({file path})"
depends_on: ["{NN}"]     # Other sections required first
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
    title: "Build & Verify"
    status: not-started
---

# Section {NN}: {Title}

**Status:** Not Started
**Goal:** {Expanded goal — what must be true when this section is complete.
Not "implement X" but "X works correctly under conditions A, B, C with
no regressions in Y."}

**Production code path:** {Which specific production code path does this
section modify? Name the function, the file, the loop, the handler.
E.g., "GpuRenderer::draw_frame() cell loop", "WindowRoot::dispatch_event()",
"Grid::scroll_up()". If you can't name one, this section is too abstract —
restructure it.}

**Observable change:** {What will be different in the running terminal after
this section lands? E.g., "Cursor blinks at the configured rate",
"Wide characters render at 2-cell width", "Tab bar shows close buttons on
hover". If the answer is "nothing visible yet", this section is wrong.}

**Context:** {Why this section exists. What pain point, bug, or
architectural gap motivated it. Cite specific debugging sessions,
test failures, or design flaws. 2-4 sentences.}

**Reference implementations:**
- **{Emulator}** `{file path}`: {pattern name} — {what we learn from it}
- **{Emulator}** `{file path}`: {pattern name} — {what we learn from it}

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

### {Sub-topic within the subsection}

**Discovery:** {What was learned during investigation that changes
the approach or adds requirements.}

**Implementation steps:**
1. {Specific, numbered, actionable step with file path}
2. {Step referencing specific functions to modify}
3. {Validation step — what test to run, what output to expect}

**Reference implementations:**
- **{Emulator}** `{file}`: {what it does} — {what we adopt from it}

**Co-implementation requirement with Section {NN} ({topic}):**
{Why this subsection and another section's work must land together.
What breaks if only one lands. Be specific about the failure mode.}

---

## {NN}.R Third Party Review Findings

Track findings from independent review runs here. Leave the block in place even
when empty so tooling has a stable anchor.

- None.

When findings exist, use this format:

- [ ] `[TPR-{NN}-001][high]` `path/to/file.rs:123` — Concrete finding summary.
  Validation: {How the reviewer proved it.}

- [x] `[TPR-{NN}-002][medium]` `path/to/file.rs:456` — Concrete finding summary.
  Resolved: Accepted and integrated into {NN}.2 on YYYY-MM-DD.

- [x] `[TPR-{NN}-003][low]` `path/to/file.rs:789` — Concrete finding summary.
  Resolved: Rejected on YYYY-MM-DD after validation. {Why the issue does not
  actually exist.}

Rules:
- Only reject findings that are factually incorrect.
- Do not delete historical findings; mark them resolved with rationale.
- If unchecked findings exist, set `third_party_review.status: findings`.
- If all findings are resolved, set `third_party_review.status: resolved`.
- If the block contains only `- None.`, set `third_party_review.status: none`.

---

## {NN}.N Build & Verify

Every section ends with this gate. No exceptions.

- [ ] `./build-all.sh` passes
- [ ] `./clippy-all.sh` passes
- [ ] `./test-all.sh` passes
- [ ] New tests exist proving this section's changes work
- [ ] No `#[allow(dead_code)]` on new items — everything has a production caller
- [ ] {Concrete, verifiable behavioral item — not "implement X" but "X is visible/measurable in the running terminal"}

**Exit Criteria:** {Paragraph describing the measurable, testable condition
that proves this section is complete. Include specific commands, test names,
metric thresholds. Not "X works" but "X produces Y output when Z command
is run, with 0 regressions in test suite."}
```

### The `reviewed` Field

- **Section 01**: Always `reviewed: true` — it's the plan's starting point, already vetted during creation.
- **All other sections**: `reviewed: false` — they must be re-reviewed before implementation.

**Why:** As you implement sections sequentially, reality diverges from the original plan. You discover new constraints, make architectural decisions, and deviate from assumptions that later sections depend on. A section written when the plan was first created may reference files that were renamed, assume types that were redesigned, or propose approaches that conflict with decisions made during earlier sections. The `reviewed: false` gate forces `/review-plan` to run on each section right before implementation, when the actual codebase state is known — catching stale assumptions before they waste work.

**`/review-plan` marks only the reviewed section as `reviewed: true`**, not all sections. Each section gets its own review checkpoint.

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

## {NN}.2 Behavioral Equivalence (if applicable)

Verify that the new path produces identical results to the existing path.

- [ ] Build a test harness comparing outputs: {description}
- [ ] Apply to all relevant tests
- [ ] Track and investigate every mismatch
- [ ] Create a CI-runnable script

---

## {NN}.3 Visual Regression (if applicable)

For rendering changes, verify visual output is correct.

- [ ] Capture reference screenshots for key scenarios
- [ ] Compare before/after for grid rendering, selection, search highlights
- [ ] Verify color accuracy (256-color palette, true color, theme switching)
- [ ] Test at multiple DPI/scale factor combinations

---

## {NN}.4 Performance Validation

- [ ] **{Metric 1}:** Measured {what} ({conditions}):
  - {Workload A}: ~{value}
  - {Workload B}: ~{value}

- [ ] **{Metric 2}:** {comparison}:
  - {result with concrete numbers}

- [ ] **Frame time budget:** 60fps target = 16.6ms per frame
  - Typical frame: ~{N}ms
  - Worst case (full redraw + atlas miss): ~{N}ms

---

## {NN}.5 Documentation

- [ ] Update superseded plans to point to this plan
- [ ] Update CLAUDE.md if new commands/paths/patterns introduced
- [ ] Update relevant .claude/rules/*.md files
- [ ] Add architecture overview to key module docs

---

## {NN}.6 Completion Checklist

- [ ] Test matrix covers all features (every checkbox in {NN}.1)
- [ ] Visual regression verified (if applicable)
- [ ] Performance baselined
- [ ] All documentation updated
- [ ] `./test-all.sh` green
- [ ] `./clippy-all.sh` green
- [ ] `./build-all.sh` green

**Exit Criteria:** {Final measurable proof. Include test counts, metric
thresholds, and the specific commands that demonstrate completion.}
```

---

## Status Conventions

| YAML Status | Meaning | Notes |
|-------------|---------|-------|
| `not-started` | No work done | |
| `in-progress` | Partial completion | Include date + current state in header |
| `complete` | All done | Include completion date in header |

Subsection status uses `not-started`, `in-progress`, `done`.

**Progress tracking conventions:**
- `[x]` — completed (include date: `(2026-03-05)`)
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

### Measurable Exit Criteria
"Implement X" is not an exit criterion. "{Command} produces {output}
with 0 failures across {N} tests" is. Every section ends with a
testable, verifiable condition.

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

### Reference Implementations
Cite specific files from reference terminal emulators. Not "Alacritty does
this" but "Alacritty's `alacritty_terminal/src/grid/storage.rs` uses the
`Storage` pattern where {description}." Include the path so the
reference can be consulted.

---

## Performance-Sensitive Plans

For plans touching **performance-critical components** (GPU rendering, VTE parsing, grid operations, key encoding), include profiling checkpoints:

### When to Profile

| Component | Profile? | Method |
|-----------|----------|--------|
| GPU renderer (`gpu/`) | Yes | Frame time measurement |
| VTE handler (`term_handler.rs`) | Yes | Throughput benchmark |
| Grid operations (`grid/`) | Yes | Resize/reflow timing |
| Key encoding (`key_encoding.rs`) | Maybe | Manual profiling |
| Config, tab bar, drag | No | Not perf-critical |

### Adding Performance Checkpoints

In sections that modify hot paths, add:

```markdown
## X.N Performance Validation

- [ ] Measure frame time before changes (record baseline)
- [ ] Measure frame time after changes
- [ ] No regressions >5% vs baseline
- [ ] Document any intentional tradeoffs
```

Only add this for sections that:
1. Modify hot code paths (render loop, VTE input, grid cell access)
2. Change data structures (Cell layout, instance buffers, atlas)
3. Add new algorithmic complexity

**Do NOT add benchmarks for**: config loading, tab bar cosmetics, drag state machine, non-hot-path features.

---

## Reference

See `plans/roadmap/` for a complete example:
- `plans/roadmap/index.md` — Keyword index with sections
- `plans/roadmap/section-*.md` — Individual section files
- `plans/roadmap/00-overview.md` — High-level overview
