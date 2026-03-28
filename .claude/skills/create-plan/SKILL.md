---
name: create-plan
description: Create a new plan directory with index and section files using the standard schema
argument-hint: "<name> [description]"
---

# Create Plan Command

Create a new plan directory with index and section files using the standard plan schema. **Research-first, architecture-second, sections-last**: deeply understand the existing codebase, design the architecture, then write sections sequentially.

**Schema**: `.claude/skills/create-plan/plan-schema.md` — the single source of truth for plan structure, frontmatter fields, section format, status conventions, and writing principles.

## Usage

```
/create-plan <name> [description]
/create-plan <add xyz to roadmap>
```

- `name`: Directory name for the plan (kebab-case, e.g., `gpu-refactor`, `mux-architecture`)
- `description`: Optional one-line description of the plan's goal
- **Roadmap mode**: If the name/description indicates adding to the roadmap (e.g., "add tab bar to roadmap", "roadmap: selection"), this command operates in **Roadmap Mode** — see the dedicated section below.

---

## Mode Detection

**New Plan Mode** (default): The argument names a new plan directory. Creates `plans/{name}/` from scratch.

**Roadmap Mode**: The argument indicates adding a section to the existing roadmap. Detected when the input contains "roadmap" or references an existing roadmap section. Operates on `plans/roadmap/` instead of creating a new directory.

Both modes follow the SAME research rigor, the SAME iterative deepening, the SAME sequential writing discipline. The difference is the target: a new plan vs. an existing one.

---

## Design Principles

These principles govern the entire plan creation process. When in doubt, consult these:

1. **Research depth > research breadth** — One agent that reads 15 files thoroughly beats 5 agents that scan 50 files superficially. Understanding invariants, control flow, and edge cases matters more than listing type signatures.

2. **Architecture before sections** — The overview isn't boilerplate. It's the load-bearing design document. Sections are *implementations of* the architecture, not independent documents. Design first, detail second.

3. **Sequential section writing is non-negotiable** — Sections depend on each other. Section 3 references decisions made in Section 2. Parallel writing forces each section to *guess* what other sections decided, producing contradictions. Write one section at a time, in order.

4. **User checkpoints at design-level decisions** — Don't ask the user to review 8 completed sections. Ask them to review the architecture *first*, then write sections they've already conceptually agreed to.

5. **Iterative deepening over parallel breadth** — Start wide, then go deep on what matters. Each research pass builds on the findings of the prior pass.

6. **Incremental design** — Every section must touch the real system. No section should build types, traits, abstractions, or infrastructure in isolation. Every section starts from the production code path, modifies it, and produces an observable, verifiable change in the running application.

---

## Phase 1: Prerequisites

### Step 0: Read CLAUDE.md (ABSOLUTE FIRST — NO EXCEPTIONS)

**Before doing ANYTHING else**, read the ENTIRE CLAUDE.md file — every single word, top to bottom:

```
Read file: CLAUDE.md
```

This is mandatory. Do not skip, skim, or partially read. The rules in CLAUDE.md govern ALL behavior in this command. Proceed to Step 1 only after reading the complete file.

### Step 1: Gather Initial Scope

If not provided via arguments, use `AskUserQuestion` to ask:

1. **Plan name** — kebab-case directory name
2. **Plan title** — Human-readable title (e.g., "GPU Renderer Refactor")
3. **Goal** — One-line description of what this plan accomplishes
4. **Rough scope** — Which parts of the codebase does this touch? (crates, subsystems, features)

Do NOT ask for sections yet. Sections emerge from research, not from guessing.

### Step 2: Read the Template & Hygiene Rules

Read `.claude/skills/create-plan/plan-schema.md` for the structure reference.

The full rule set is embedded below (source of truth files — do not maintain separate copies). Use these rules when structuring plan sections to ensure plans account for module boundary discipline, file size limits, rendering pipeline purity, and other hygiene requirements from the start.

**Implementation Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
@.claude/rules/impl-hygiene.md

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
@.claude/rules/code-hygiene.md

---

## Phase 2: Multi-Pass Research (MANDATORY — NO SHORTCUTS)

**THIS IS THE MOST IMPORTANT PHASE.** You MUST deeply understand the existing codebase before designing architecture or writing sections. Every claim in the plan must be grounded to actual code — no assumptions, no guessing.

Research uses **iterative deepening** — four sequential passes, each building on the findings of the prior pass. Passes 1 and 2 may use parallel agents for breadth. Passes 3 and 4 are focused, sequential deep-dives.

### Step 3: Pass 1 — Breadth Scan (parallel agents)

Launch **2-4 parallel agents** to build an inventory of everything relevant. This pass answers: **what exists?**

**Every agent MUST be instructed to:**
- Read actual source files (not just file names)
- Report exact file paths, line numbers, function signatures, type definitions
- Report what EXISTS today — not what they think should exist
- Flag anything ambiguous or surprising as `UNCLEAR: {what}`
- NO assumptions — if something is unclear, say so rather than guessing

Tailor agents to the specific plan topic. Standard agents:

#### Agent 1: Implementation & Boundary Survey

```
You are researching the ori_term codebase for plan creation. Your job is to build a complete inventory of everything related to: {topic/scope}.

Read CLAUDE.md first.

PART A — Implementation Inventory:
1. Find ALL files, types, functions, traits, and modules related to {topic}
   - Use Glob to find files by name patterns
   - Use Grep to find type/function/trait definitions
   - READ the actual source code of every file you find (not just names)
2. For each relevant file, report:
   - Full path
   - Line count (total, production, test)
   - Key types/structs/enums defined (with field signatures)
   - Key functions (with full signatures)
   - Imports and dependencies (what does this file depend on?)
   - Exports (what does this file expose to other crates?)
3. Report ALL existing tests for this area:
   - Test file locations and what each test covers
   - Any #[ignore] tests and their reasons
   - Gaps in test coverage you notice

PART B — Integration Points & Boundaries:
1. Identify every crate that {topic} touches or will need to touch
2. For each crate boundary:
   - What types cross the boundary? (Read the actual pub types)
   - What functions are called across the boundary? (Read actual call sites)
   - What registration/sync points exist? (enums, match arms, if-chains that must stay in sync)
3. Map the full pipeline flow for {topic}:
   - oriterm_core (terminal emulation) → oriterm_ui (widgets/layout) → oriterm_mux (pane server) → oriterm (app shell/GPU)
   - At each stage, what representation does {topic} have?
   - Where are the hand-off points?
4. Check for registration sync requirements:
   - Enum variants that must be added in multiple places
   - Match arms that must stay in sync
   - Test arrays/lists that enumerate all variants
   - Registry entries that must be updated

OUTPUT FORMAT:
For each file:
  PATH: {full path}
  LINES: {count}
  KEY TYPES: {list with signatures}
  KEY FUNCTIONS: {list with signatures}
  DEPENDENCIES: {what it imports}
  EXPORTS: {what it exposes}
  TESTS: {test file path and coverage summary}
  NOTES: {anything surprising, unclear, or noteworthy}

Then:
  CRATES_TOUCHED: {list}
  BOUNDARY_TYPES: {for each boundary, the types that cross it}
  PIPELINE_FLOW: {stage-by-stage representation}
  SYNC_POINTS: {every enum/match/registry that must stay in sync}
  UNCLEAR: {list of anything you couldn't determine}
  EXISTING_BUGS: {any bugs or issues you noticed while reading}
```

#### Agent 2: Tests, Hygiene & Constraint Audit

```
You are researching the ori_term codebase for plan creation. Your job is to understand the test landscape, constraints, and hygiene state for {topic/scope}.

Read CLAUDE.md first, then read .claude/rules/impl-hygiene.md, .claude/rules/code-hygiene.md, .claude/rules/test-organization.md, and .claude/rules/crate-boundaries.md.

PART A — Tests & Constraints:
1. Find ALL existing tests related to {topic}:
   - Rust unit tests (sibling tests.rs files)
   - Architecture tests (oriterm/tests/architecture/)
   - Widget tests using WidgetTestHarness
   - Read the actual test code, not just file names
2. Check existing plans:
   - Read plans/ directory for related or superseded plans
   - Report any existing plan items that overlap with this topic
   - Report any completed plan items that this plan builds on
3. Check performance invariants:
   - oriterm_core/tests/alloc_regression.rs
   - oriterm/src/app/event_loop_helpers/tests.rs
   - Are there hot paths this plan touches?
4. Check cross-platform requirements:
   - Does this plan need platform-specific code?
   - What existing #[cfg(target_os)] blocks exist?
5. Check CLAUDE.md and memory for relevant context

PART B — Hygiene Audit:
1. Find all files that will likely be touched based on the scope: {topic}
2. For EACH file, report:
   - Full path and line count
   - Whether it exceeds the 500-line limit
   - Any existing TODOs, FIXMEs, HACKs, WORKAROUNDs
   - Any dead code or stale comments you notice
   - Any registration sync points that are already out of sync
3. Check for crate boundary violations:
   - Does any file import from a crate it shouldn't?
   - Is internal state leaking through boundary types?
4. Check test file conventions:
   - Are tests in sibling tests.rs files (not inline)?
   - Any #[cfg(test)] mod tests blocks that should be extracted?
5. Produce a hygiene summary:
   - Clean files (no issues)
   - Files with issues (categorized: BLOAT/WASTE/DRIFT/EXPOSURE/LEAK/STYLE)
   - Priority files that need splitting before the plan can proceed

OUTPUT FORMAT:
  EXISTING_TESTS: {list with paths and coverage}
  RELATED_PLANS: {existing plans that overlap}
  PERFORMANCE_INVARIANTS: {hot paths, alloc constraints}
  PLATFORM_REQUIREMENTS: {cross-platform considerations}
  FILES_TOUCHED: {list with line counts}
  OVER_LIMIT: {files > 500 lines}
  HYGIENE_ISSUES: {categorized findings with file:line}
  SYNC_VIOLATIONS: {any already-broken sync points}
  PRIORITY_SPLITS: {files that must be split before work begins}
  UNCLEAR: {anything ambiguous}
  EXISTING_BUGS: {bugs found in tests or hygiene}
```

### Step 4: Pass 2 — Deep Read (sequential, focused)

**After Pass 1 agents complete**, identify the **10-15 most critical files** from their findings. These are the files where the plan's core logic lives — not periphery.

**You (the main agent) or a single focused agent MUST now read these files thoroughly.** Not scan for signatures — read the actual logic. Understand:

1. **Invariants**: What properties does this code maintain? What `debug_assert!`s exist? What would break if those invariants were violated?
2. **Control flow**: How does execution actually flow through this code? What are the error paths? What are the edge cases?
3. **State mutations**: What state changes? Where? In what order? What are the pre/post conditions?
4. **Why it works this way**: Look for comments explaining design decisions. Look at git blame for recent changes. Understand the *reasoning*, not just the *structure*.
5. **What would break**: If you changed X, what else would need to change? What tests would fail? What invariants would be violated?

**Output**: For each critical file, write a paragraph (not a list) explaining how the code works, what invariants it maintains, and what would break if changed. This understanding is what grounds the plan.

**This step cannot be parallelized.** Each file read may inform what to look for in the next file.

### Step 5: Pass 3 — Pattern Study (single focused agent)

Launch **one agent** to trace 2-3 analogous features end-to-end through the codebase. These are features that already exist and follow the same structural pattern that the new plan will need.

```
You are studying implementation patterns in the ori_term codebase. Your job is to trace analogous features end-to-end to discover the exact implementation pattern that {topic/scope} should follow.

Read CLAUDE.md first.

INSTRUCTIONS:
1. Identify 2-3 features ALREADY IMPLEMENTED in ori_term that are structurally similar to {topic}. Examples:
   - If adding a new widget: trace how an existing widget (Button, Toggle) was implemented
   - If adding a new interaction pattern: trace how HoverController or ClickController works
   - If modifying the grid: trace how selection or search was implemented
   - If adding GPU rendering: trace how an existing render pass works

2. For EACH analogous feature, trace the COMPLETE implementation through every relevant layer:
   a. Core types: What types in oriterm_core? (grid, cell, palette, etc.)
   b. UI framework: What widget/controller in oriterm_ui? (widget trait, layout, interaction)
   c. Mux layer: What pane/PTY handling in oriterm_mux? (if applicable)
   d. App shell: What wiring in oriterm? (event loop, GPU, session)
   e. Tests: What test files and patterns? (sibling tests.rs, WidgetTestHarness, architecture tests)

3. For each layer, READ THE ACTUAL CODE. Report:
   - Exact file path and function/type names
   - How data enters and leaves that layer
   - What registration/sync points were needed
   - What the implementation pattern is (not just "it exists" but "here's how it works")

4. Synthesize the pattern:
   - What is the exact sequence of files to create/modify?
   - What is the exact sequence of types/enums/match-arms to add?
   - What is the order of operations? (What must come first?)
   - Where did the analogous feature deviate from the expected pattern, and why?

OUTPUT FORMAT:
For each analogous feature:
  FEATURE: {name}
  LAYER TRACE:
    CORE: {file, types, how it works}
    UI: {file, widgets/controllers, how it works}
    MUX: {file, pane handling, how it works}
    APP: {file, wiring, how it works}
    TESTS: {files, patterns, coverage}
  SYNC_POINTS: {all registration points that had to stay in sync}
  ORDER_OF_OPERATIONS: {what was built first, second, third}
  DEVIATIONS: {where this feature broke the expected pattern}

Then:
  RECOMMENDED_PATTERN: {the pattern the new plan should follow}
  RECOMMENDED_ORDER: {the order in which layers should be implemented}
  PATTERN_RISKS: {where the new feature might need to deviate from the pattern}
```

### Step 6: Pass 4 — Prior Art Study (single focused agent)

Launch **one agent** to study reference terminal emulators for the specific design decisions this plan will face.

```
You are studying prior art in reference terminal emulator implementations. Your job is to find how other terminal emulators handle the specific design decisions that {topic/scope} will face.

Read CLAUDE.md first for reference repo locations.

INSTRUCTIONS:
1. Identify the 2-4 specific DESIGN DECISIONS this plan will need to make. Examples:
   - "Should X use damage tracking or full redraws?"
   - "Should X live in oriterm_ui or oriterm_core?"
   - "How should X interact with the GPU pipeline?"
   - "What data structure should X use for storage?"

2. For EACH design decision, check the reference repos at ~/projects/reference_repos/console_repos/:
   - Alacritty (Rust, OpenGL, 4-crate workspace)
   - WezTerm (Rust, WebGPU, 69-crate monorepo)
   - Ghostty (Zig, Metal+OpenGL)
   - Ratatui (Rust, widget framework)
   - Crossterm (Rust, terminal abstraction)
   - tmux (C, canonical multiplexer)
   Also check ~/projects/reference_repos/gui_repos/ if the plan involves UI/widgets:
   - egui, iced, zed/GPUI, druid, masonry, makepad

3. For each reference implementation you find:
   - Read the ACTUAL CODE (not just file names)
   - Understand their design choice and WHY they made it
   - Note the trade-offs they accepted
   - Note any bugs or limitations in their approach

4. Synthesize design recommendations:
   - For each design decision, recommend an approach with evidence
   - Cite specific files and patterns from reference implementations
   - Explain which reference implementation's approach best fits ori_term's constraints

OUTPUT FORMAT:
For each design decision:
  DECISION: {what needs to be decided}
  REFERENCE IMPLEMENTATIONS:
    {Project}: {file path} — {their approach and why}
    {Project}: {file path} — {their approach and why}
  RECOMMENDATION: {what ori_term should do}
  EVIDENCE: {why, citing specific reference impl trade-offs}
  RISKS: {what could go wrong with this approach}
```

**Note**: Passes 3 and 4 CAN run in parallel with each other (they are independent), but both MUST wait for Passes 1-2 to complete (they depend on knowing what files and code are relevant).

---

## Phase 3: Architecture Design (REQUIRED BEFORE SECTION WRITING)

This phase synthesizes all research into a cohesive architecture. **No sections are written until the architecture is designed and the user approves it.**

### Step 7: Synthesize Research into Architecture

After ALL research passes complete, synthesize findings into a structured architecture. Compile:

1. **Complete file inventory** — every file that will be touched, with line counts and current state
2. **Deep understanding summary** — for each critical file, how the code works, what invariants it maintains, what would break (from Pass 2)
3. **Implementation pattern** — the exact pattern that analogous features follow, and how this plan should follow it (from Pass 3)
4. **Design decisions** — for each decision, the recommended approach with evidence from prior art (from Pass 4)
5. **All sync points** — every enum, match, registry that must be updated together
6. **Test strategy** — existing coverage AND planned test requirements per section: what tests exist (from Pass 1-2), what harness patterns (WidgetTestHarness, sibling tests.rs, architecture tests), what edge cases
7. **All unclear items** — things the research couldn't determine
8. **All existing bugs found** — bugs discovered during research (these go into the plan)
9. **Hygiene pre-scan** — files that need splitting or cleanup
10. **Dependency chain** — what must be built first, what gates what, what can be parallelized

### Step 8: Write `00-overview.md` FIRST

The overview is the **load-bearing design document**. It is NOT boilerplate filled in after sections are written — it is the architectural blueprint that DRIVES section content.

Write `00-overview.md` following the template in `.claude/skills/create-plan/plan-schema.md`, grounding every element in research:

- **Mission**: Based on the actual problem discovered during research
- **Architecture diagram**: Based on the actual data flow map from Pass 2's deep read
- **Design principles**: Based on patterns observed in analogous features (Pass 3) and prior art (Pass 4)
- **Section dependency graph**: Based on actual crate dependencies and sync points found in Pass 1
- **Implementation sequence**: Based on the analogous feature pattern from Pass 3
- **Design decisions**: Include the key design decisions from Pass 4 with recommended approaches and evidence
- **Known bugs**: Include ALL bugs found during research passes
- **Metrics**: Use actual line counts from the hygiene pre-scan

**Also create `index.md`** with keyword clusters using REAL keywords from the research (actual type names, function names, file names — not placeholders).

### Step 9: User Review of Architecture (MANDATORY — DO NOT SKIP)

**You MUST use `AskUserQuestion` here.** Present the architecture and get explicit buy-in before writing sections.

Present:
1. **The architecture**: Summarize the design from `00-overview.md`
2. **The proposed sections**: List each section with its goal, what files it touches, and what it depends on
3. **Design decisions**: For each key design decision, present the recommended approach with evidence
4. **Analogous pattern**: "Feature X follows this pattern: {pattern}. This plan will follow the same pattern."
5. **Resolve unclear items**: For every `UNCLEAR` item from research, ask the user
6. **Report existing bugs**: "During research, I found these existing issues: {list}. Per zero-deferral, these will be included in the plan."
7. **Scope adjustments**: If research revealed the scope is larger or smaller than expected, propose adjustments

**Do NOT proceed to Phase 4 until the user responds and approves the architecture.**

---

## Phase 4: Sequential Section Writing (MANDATORY SEQUENTIAL — NO PARALLELISM)

**CRITICAL RULE: Write sections ONE AT A TIME, IN ORDER.** Do NOT launch parallel agents to write sections.

### Step 10: Create Directory Structure

Create the plan directory:

```
plans/{name}/
├── index.md           # Already created in Step 8
├── 00-overview.md     # Already created in Step 8
├── section-01-*.md    # Written sequentially starting here
├── section-02-*.md    # Written after section-01 is complete
└── section-NN-*.md    # Written after all prior sections are complete
```

### Step 11: Write Sections Sequentially

For each section, in order from 01 to N:

**Before writing the section**, re-read:
- The `00-overview.md` architecture (to stay aligned with the design)
- ALL previously written sections (to reference their decisions and avoid contradictions)
- The relevant research findings for this section's scope

**Write the section** following the template in `.claude/skills/create-plan/plan-schema.md`. Every section must be grounded:

- **File paths**: Use EXACT paths from research (verified to exist)
- **Type signatures**: Use EXACT signatures from research (copy from source)
- **Function references**: Use EXACT function names from research
- **Registration sync points**: List ALL sync points from research
- **Analogous pattern**: Reference the analogous feature's implementation pattern
- **Code examples**: Show target implementation based on actual code patterns found during research
- **Test strategy**: Every section that modifies code MUST include testing requirements:
  - **Test harness**: Specify which pattern — WidgetTestHarness for widgets, sibling tests.rs for unit tests, architecture tests for crate boundaries
  - **Edge cases**: Unicode, CJK, emoji, platform differences, error paths
  - **TDD ordering**: "Write failing tests BEFORE implementation" as the section's FIRST checklist item
- **Dependencies on prior sections**: Explicitly reference what earlier sections provide
- **What this section provides to later sections**: State what downstream sections will depend on

**Frontmatter includes:**
- Section ID, title, status: not-started, goal
- `reviewed` field (see rules below)
- `inspired_by` with actual reference implementations found
- `depends_on` based on actual crate dependency chain AND section content dependencies
- `third_party_review: { status: none, updated: null }`
- `## {NN}.R Third Party Review Findings` block (empty, with `- None.`) before the completion checklist
- Completion checklist at the end

**`reviewed` field rules:**
- **Section 01**: `reviewed: true` — validated during plan creation against the research findings.
- **All other sections (02+)**: `reviewed: false` — not yet validated against implementation reality.

**After writing each section**, briefly verify:
- File paths referenced exist
- Type/function names referenced exist
- References to prior sections are accurate
- No contradictions with prior sections

Then proceed to the next section.

### Step 12: Update Overview and Index

After all sections are written:
- Update `00-overview.md` with the final section list and any adjustments
- Update `index.md` with complete keyword clusters for all sections

---

## Phase 5: Cohesion Review & Finalization

### Step 13: Cohesion Check

Launch **one agent** to read the ENTIRE plan front-to-back and check for internal coherence:

```
You are reviewing a newly created plan for internal coherence. Read EVERY file in the plan directory: {plan_dir}/

Check for:
1. CONTRADICTIONS: Does Section X say one thing and Section Y say another?
2. GAPS: Is there work that falls between sections?
3. REDUNDANCY: Do multiple sections do the same work?
4. BROKEN REFERENCES: Does Section X reference a type/file/function from Section Y that Section Y doesn't actually define?
5. ORDERING ISSUES: Does Section X depend on work described in Section Y, but X comes before Y?
6. SYNC POINT COMPLETENESS: Are ALL sync points accounted for across all sections?
7. OVERVIEW ALIGNMENT: Does the overview still match what the sections actually describe?

For each issue found, report:
  ISSUE TYPE: {contradiction/gap/redundancy/broken-ref/ordering/sync-gap/overview-drift}
  SECTIONS: {which sections are involved}
  DETAILS: {what the issue is}
  FIX: {how to resolve it}
```

Fix all issues found before proceeding.

### Step 14: Self-Check Before Review

Do a quick self-audit:

1. **Every file path in the plan** — verify it exists in the codebase (use Glob)
2. **Every function/type reference** — verify it exists (use Grep)
3. **Every registration sync point** — verify the list is complete
4. **No placeholder content** — no "TBD", no "placeholder keywords", no "to be determined"
5. **No assumptions** — every technical claim traces to research
6. **No contradictions** — cohesion check passed clean
7. **Test strategy per section** — every code-modifying section has test requirements with correct harness patterns

Fix any issues found.

### Step 15: Report Progress

Show the user:
- Files created (with paths)
- Brief summary of what each section covers
- Any issues found and fixed during cohesion/self-check
- Note: "Running /review-plan for formal review..."

### Step 16: Run /review-plan (MANDATORY — USE THE ACTUAL SKILL)

**CRITICAL: Run the actual `/review-plan` skill using the Skill tool.** Do NOT reimplement the review logic.

```
Skill: review-plan
Args: plans/{name}/
```

### Step 17: Post-Review Summary

After `/review-plan` completes, report to the user:
- The review verdict
- What the review changed
- Any remaining concerns that need human judgement

### Step 18: Ask About Reroute Status

Use `AskUserQuestion` to ask the user whether this plan should be the active reroute. This determines the `reroute` frontmatter in `index.md`.

If the user says **yes**: add reroute frontmatter to `index.md` with `status: active` and `order: 1`.
If the user says **queued**: add reroute frontmatter with `status: queued` and ask for the `order` value.
If the user says **no**: do not add reroute frontmatter (plan is not a reroute).

---

## Example

**Input:** `/create-plan gpu-refactor "Restructure GPU rendering pipeline for damage tracking"`

**Phase 1**: Read CLAUDE.md. Ask user about scope ("Which crates? Grid rendering only or UI too?").

**Phase 2**:
- *Pass 1*: Launch 2 parallel agents — (1) survey `oriterm_gpu`, `oriterm/src/app/` rendering code, all GPU-related files; (2) audit tests, hygiene state, performance invariants.
- *Pass 2*: Deep-read the 12 most critical files. Understand how `GpuRenderer::draw_frame()` works, how the atlas manages glyphs, how damage tracking would integrate.
- *Pass 3*: Trace how the existing cell rendering pipeline works end-to-end — from grid state through GPU submission to screen.
- *Pass 4*: Study Alacritty's damage tracking (`alacritty/src/display/damage.rs`), Ghostty's multi-backend approach, WezTerm's texture atlas. Recommend approaches for ori_term.

**Phase 3**: Design architecture. Write `00-overview.md` with data flow, design decisions. Present to user.

**Phase 4**: After user approves, write sections sequentially.

**Phase 5**: Cohesion check → self-check → report → run `/review-plan plans/gpu-refactor/`.

---

## Section Naming Conventions

| Section Type | Naming Pattern |
|--------------|----------------|
| Setup/Infrastructure | `section-01-setup.md` |
| Core Implementation | `section-02-core.md` |
| Integration | `section-03-integration.md` |
| Testing/Verification | `section-NN-verification.md` |

---

## Anti-Deferral Rule for Plan Items

**Every checklist item in a plan must be implementable by the agent executing that section.** When writing plan items:

- Do NOT use soft language that invites skipping: "bonus", "future", "lower priority", "nice to have", "if time permits", "stretch goal".
- Do NOT label items "requires architectural change" — architectural changes are implementation tasks, not deferrals.
- Do NOT create items that are descriptions of work rather than work itself.
- If an item genuinely cannot be done within the section (blocked by unimplemented feature, needs user decision), use `<!-- blocked-by:X -->` with a concrete blocker reference.
- Every item must pass this test: "Can the implementing agent, with access to the codebase, complete this item in a single session?" If no, break it into items that can.

## Zero Assumptions Rule

**ABSOLUTE — NO EXCEPTIONS.** Every technical claim in the plan must be grounded to something found during research:

- **File paths**: Must exist in the codebase (verified by Glob/Read)
- **Type/function signatures**: Must match actual source (verified by reading the file)
- **Behavior descriptions**: Must match actual code behavior (verified by reading the implementation)
- **Registration sync points**: Must be the complete list (verified by Grep)
- **Patterns to follow**: Must reference actual analogous implementations (verified by reading them)

If you cannot verify a claim, it MUST be flagged as `<!-- UNVERIFIED: {reason} -->` and reported to the user in Step 9.

## Reviewed Field Semantics

The `reviewed: true/false` field in section frontmatter is a **pre-implementation gate**.

**Rules:**
- **Section 01** is always `reviewed: true` at creation — it's the starting point.
- **All other sections** are `reviewed: false` at creation.
- **Single-section review** (`/review-plan plans/foo/section-03.md`): pre-implementation gate. After confirming accuracy, flip to `reviewed: true`.
- **Whole-plan review** (`/review-plan plans/foo/`): improves quality but does NOT change `reviewed` values.
- **`/continue-roadmap`** starting a `reviewed: false` section: triggers a single-section review first.

---

## After Creation

Remind the user to:
1. Review the `/review-plan` verdict and address any flagged concerns
2. **If performance-sensitive** (GPU rendering, VTE parsing, grid operations): Verify profiling checkpoints in relevant sections

## Performance-Sensitive Plans

For plans touching hot paths, include a "Performance Validation" section:

```markdown
## Performance Validation

Profile after modifying hot paths.

**When to benchmark:** [list specific sections]
**Skip benchmarks for:** [list non-perf sections]
```

---

## Roadmap Mode

When the input indicates adding to the roadmap (e.g., `/create-plan add selection to roadmap`), this command operates on `plans/roadmap/` instead of creating a new plan directory.

**Same rigor, different target.** Every phase applies identically.

### Roadmap Mode: How It Differs

#### Phase 1 Differences

- **Step 1**: Instead of asking for a plan name, identify:
  1. **What feature/section** to add to the roadmap
  2. **Where it fits** — after which existing section? What does it depend on?
  3. **What it might affect** — which existing sections reference related code?

- **Step 2**: In addition to the template and hygiene rules, **read the entire roadmap**:
  - `plans/roadmap/00-overview.md` — understand the mission, architecture, dependency graph
  - `plans/roadmap/index.md` — understand the keyword structure and section numbering
  - **Every existing section file** — understand what's already planned

#### Phase 2 Differences

Research adds a roadmap-specific dimension:

- **Pass 1**: Also identify which existing roadmap sections touch the same files/types/crates
- **Pass 2**: Also deep-read the 2-3 existing roadmap sections most related to the new one

#### Phase 3 Differences

- **Step 7**: Synthesis must include impact analysis on existing roadmap
- **Step 8**: **Update** the existing `00-overview.md` and `index.md` instead of creating new ones
- **Step 9**: Present impact on existing sections alongside the new section proposal

#### Phase 4 Differences

- After writing the new section(s), update any existing sections that are affected
- Update `depends_on`, cross-references, `00-overview.md`, `index.md`

#### Phase 5 Differences

- Cohesion check reads the ENTIRE roadmap (all sections, not just new ones)
- Run `/review-plan plans/roadmap/` (the full roadmap)
- Skip the reroute question

### Roadmap Mode: The "Leave It Better" Rule

**You MUST leave the roadmap in better shape than you found it.** When operating in roadmap mode:

1. **Format drift**: If existing sections don't match the current template, update them
2. **Stale content**: Fix stale file paths, outdated type signatures
3. **Missing cross-references**: Add explicit `depends_on` or co-implementation callouts
4. **Overview accuracy**: The overview must accurately reflect the current state after your changes

### Roadmap Mode: Example

**Input:** `/create-plan add tab bar rendering to roadmap`

**Phase 1**: Read CLAUDE.md. Read the entire roadmap. Identify this relates to UI framework work, probably depends on existing widget infrastructure.

**Phase 2**: Survey tab bar code, existing roadmap sections touching UI, reference implementations.

**Phase 3**: Design the new section. Determine where it fits in the dependency graph. Present impact to user.

**Phase 4**: Write section. Update affected existing sections and overview.

**Phase 5**: Cohesion check on full roadmap. Run `/review-plan plans/roadmap/`.

---

## Template Reference

The command uses `.claude/skills/create-plan/plan-schema.md` as the structure reference. See that file for:
- Complete index.md template
- Section file template
- Status conventions
- Writing principles
