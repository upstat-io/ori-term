---
description: Create a new plan directory with index and section files using the standard template
---

# Create Plan Command

Create a new plan directory with index and section files using the standard template. **Research-first**: deeply explore the existing codebase before writing anything. **Zero assumptions**: every claim in the plan must be grounded to actual code. **Interactive**: ask the user follow-up questions after research, not before.

## Incremental Design — Non-Negotiable

**Every section must touch the real system.** No section should build types, traits, abstractions, or infrastructure in isolation. Every section starts from the production code path, modifies it, and produces an observable, verifiable change in the running application.

**The anti-pattern (banned):** Design an entire type hierarchy / trait system / abstraction layer across sections 01-05, then wire it into the actual render loop / event loop / terminal in section 06 and hope it all works. This produces thousands of lines of dead code behind `#[allow(dead_code)]`, zero observable behavior, and inevitable reverts.

**The correct pattern:** Each section:
1. **Starts from the production code path** that needs to change (the render loop, the event handler, the grid operation — the real thing)
2. **Builds only what's needed** to make that specific change work
3. **Wires it in immediately** — new code has a production caller in the same section
4. **Produces observable behavior** — you can see it working, measure it, test it
5. **Ends with build + clippy + test verification** — `./build-all.sh`, `./clippy-all.sh`, `./test-all.sh` all pass, plus new tests proving the change works

**Concretely:**
- If a section introduces a new type, that type must have a production caller by section end — no `#[allow(dead_code)]`
- If a section introduces a new trait, something must implement and use it by section end
- If a section is "infrastructure" with no observable terminal behavior change, it's wrong — restructure it to start from the behavior change and pull in only the infrastructure needed
- A section that can't be independently verified against the running application is too abstract — split it so each piece touches reality

## Usage

```
/create-plan <name> [description]
```

- `name`: Directory name for the plan (kebab-case, e.g., `gpu-refactor`, `mux-architecture`)
- `description`: Optional one-line description of the plan's goal

**Arguments:** `$ARGUMENTS`

---

## Workflow

### Phase 1: Initial Scoping (Brief)

If not provided via arguments, ask the user:

1. **Plan name** — kebab-case directory name
2. **Plan title** — Human-readable title (e.g., "GPU Renderer Refactor")
3. **Goal** — One-line description of what this plan accomplishes

**Do NOT ask for sections yet.** Sections are determined by research, not guesswork. Do NOT ask the user to design the plan structure — that's YOUR job after research.

### Phase 2: Deep Codebase Research (Parallel Agents)

**This is the critical phase.** Before writing a single line of the plan, you MUST deeply understand the existing codebase in the areas the plan will touch. Launch **4-6 parallel research agents** to investigate different aspects simultaneously. Each agent should be thorough — reading actual source files, tracing call chains, mapping types and their usages.

Tell the user: "Launching N parallel research agents to investigate the codebase..."

**MANDATORY: Every research agent must report its findings with specific file paths, line numbers, function signatures, type definitions, and call chains. No vague summaries. No "the code probably does X." Every finding must be grounded to a specific location in the code.**

#### Research Agent 1: Production Code Paths

```
You are researching the ori_term codebase to prepare for creating a plan about: {goal}.

Your job is to find and map the PRODUCTION CODE PATHS that this plan will need to modify.

INSTRUCTIONS:
1. Identify which production code paths are relevant to {goal}
2. For EACH code path, trace it end-to-end:
   - Entry point (which function, which file, which line)
   - Key intermediate calls (function → function chain)
   - Data types flowing through the path
   - Where state is stored and mutated
   - Where side effects happen (rendering, I/O, events)
3. Read the actual source files — do not guess based on names
4. Document the CURRENT behavior precisely:
   - What does this code path do TODAY?
   - What types does it use?
   - What are its inputs and outputs?
   - What constraints does it operate under?
5. Identify which specific functions/lines would need to change for {goal}

Report your findings with EXACT file paths, line numbers, function signatures, and type definitions.
Do NOT make assumptions — if you can't find something, say so explicitly.
```

#### Research Agent 2: Type & Module Architecture

```
You are researching the ori_term codebase to prepare for creating a plan about: {goal}.

Your job is to map the TYPE SYSTEM and MODULE ARCHITECTURE relevant to this plan.

INSTRUCTIONS:
1. Find all types, traits, enums, and structs that are relevant to {goal}
2. For each type, document:
   - Full definition (fields, variants, methods)
   - Where it's defined (file:line)
   - Where it's used (all call sites — grep for it)
   - What crate it lives in (oriterm_core, oriterm_ui, oriterm_mux, oriterm)
   - Its public API surface
3. Map module boundaries:
   - Which modules own which types?
   - What are the dependency relationships between modules?
   - Which modules are in which crate?
   - What crosses crate boundaries?
4. Check for existing patterns that the plan should follow:
   - How are similar features currently implemented?
   - What conventions exist for this kind of change?
   - Are there existing abstractions that should be reused vs. new ones needed?
5. Look at existing tests in the relevant areas:
   - What test patterns exist?
   - What's the testing infrastructure like?
   - What testing utilities are available?

Report with EXACT definitions, paths, and line numbers. Quote actual type signatures.
```

#### Research Agent 3: Related Existing Functionality

```
You are researching the ori_term codebase to prepare for creating a plan about: {goal}.

Your job is to find ALL EXISTING FUNCTIONALITY that is related to or overlaps with {goal}.

INSTRUCTIONS:
1. Search the codebase broadly for anything related to {goal}:
   - Existing implementations (even partial or broken)
   - TODO/FIXME/HACK comments mentioning related topics
   - Dead code or commented-out code related to this area
   - Old implementations in _old/ that show prior art
   - Existing plan sections in plans/ that touch this area
2. For each finding:
   - What does it do currently?
   - Is it active (called from production code) or dead?
   - How mature is it (stub, partial, complete)?
   - What can be reused vs. what needs replacing?
3. Check for potential conflicts:
   - Will this plan conflict with in-progress work in other plans?
   - Are there shared state or shared types that multiple systems depend on?
   - Are there performance invariants or regression tests that constrain changes?
4. Read the existing plans in plans/ to understand:
   - What has already been planned that relates to {goal}?
   - What assumptions do existing plans make about this area?
   - Are there dependency relationships with existing plans?

Report EVERYTHING you find, with file paths and line numbers. Especially flag existing code that does part of what the plan needs — this is critical for avoiding duplication.
```

#### Research Agent 4: Reference Implementations

```
You are researching reference terminal emulators to find prior art for: {goal}.

INSTRUCTIONS:
1. Search the reference repos in ~/projects/reference_repos/console_repos/ for how established terminal emulators handle this:
   - Alacritty (Rust, 4-crate workspace)
   - WezTerm (Rust, 69-crate monorepo)
   - Ghostty (Zig)
   - Ratatui (Rust widget framework)
   - Crossterm (Rust terminal abstraction)
   - tmux (C)
2. Also check ~/projects/reference_repos/gui_repos/ if the plan involves UI/widgets:
   - egui, iced, zed/GPUI, druid, masonry, makepad
3. For each reference implementation found:
   - Which file(s) implement it?
   - What's the design pattern?
   - What are the key data structures?
   - What trade-offs did they make?
   - What can we learn from or adopt?
4. Note areas where reference implementations DISAGREE — this often indicates a design decision the plan needs to make explicitly

Report with specific file paths within the reference repos and concrete patterns, not vague descriptions.
```

#### Research Agent 5: Constraints & Invariants

```
You are researching the ori_term codebase to find CONSTRAINTS AND INVARIANTS that a plan about {goal} must respect.

INSTRUCTIONS:
1. Read the CLAUDE.md files for project rules that affect this plan:
   - /home/eric/projects/ori_term/CLAUDE.md
   - .claude/rules/impl-hygiene.md
   - .claude/rules/code-hygiene.md
   - .claude/rules/test-organization.md
2. Find performance invariants that constrain this change:
   - Check oriterm_core/tests/alloc_regression.rs
   - Check oriterm/src/app/event_loop_helpers/tests.rs
   - Are there hot paths this plan touches?
3. Find cross-platform requirements:
   - Does this plan need platform-specific code?
   - What existing #[cfg(target_os)] blocks exist in the relevant areas?
   - What platform abstractions already exist?
4. Find crate boundary constraints:
   - Which crate(s) does this plan modify?
   - Does it respect the allowed dependency direction?
   - Will it need new cross-crate APIs?
5. Check file size limits — are any relevant files already near 500 lines?
6. Check for existing tests that verify current behavior (regression risk)

Report every constraint with its source (file path, rule name, test name).
```

**Additional research agents:** If the plan's goal suggests specific additional areas to research (e.g., GPU rendering → research the shader pipeline, PTY handling → research the mux layer), launch additional targeted agents. Use your judgment — more research is always better than less. The goal is to have COMPLETE knowledge of the relevant codebase before writing anything.

### Phase 3: Research Synthesis & Interactive Q&A

**After ALL research agents complete**, synthesize their findings into a structured research summary. Present this to the user and ask follow-up questions.

#### Step 3a: Present Research Findings

Show the user a structured summary:

```
## Research Findings for: {plan title}

### Existing Code Paths
{List the production code paths that will be modified, with file:line references}

### Existing Types & APIs
{Key types, traits, modules that are relevant, with definitions}

### Related Existing Functionality
{What already exists that overlaps — partial implementations, dead code, old code in _old/}

### Reference Implementations
{How Alacritty/WezTerm/Ghostty/etc. handle this, with specific patterns}

### Constraints
{Performance invariants, platform requirements, crate boundaries, file size limits}

### Key Design Decisions Needed
{Questions that the research revealed but couldn't answer — architecture choices, scope decisions}
```

#### Step 3b: Ask Follow-Up Questions

Ask the user about design decisions revealed by the research. These should be SPECIFIC questions grounded in what you found, not generic. Examples:

- "The codebase already has {X} in {file}. Should the plan extend this or replace it?"
- "Alacritty does {A} but WezTerm does {B}. Which approach fits ori_term better?"
- "{Type} in {crate} is currently {N} lines. Adding {feature} would push it past 500 lines. Should we split the module first?"
- "There's an existing plan at plans/{name} that touches the same area. Should this plan supersede it or coordinate with it?"

**Ask ALL questions you're uncertain about.** Never assume. If you're unsure about scope, approach, ordering, or integration points — ASK.

**Wait for the user's answers before proceeding.** If answers raise more questions, ask those too. This phase is iterative until you have clarity on all design decisions.

### Phase 4: Plan Design (Still No Files Yet)

Based on research findings + user answers, design the plan structure. Before writing files, present the proposed plan outline to the user:

```
## Proposed Plan Structure: {title}

### Section 01: {name}
- Production code path: {specific function/file}
- Observable change: {what will be different}
- Key tasks: {brief list}

### Section 02: {name}
- Production code path: {specific function/file}
- Observable change: {what will be different}
- Key tasks: {brief list}

...

### Section NN: Verification
- Test matrix, visual regression, performance validation
```

Ask: "Does this plan structure look right? Any sections to add, remove, or reorder?"

**Wait for approval before writing files.**

### Phase 5: Read Template & Hygiene Rules

Read `plans/_template/plan.md` for the structure reference.

Read the following rule files and use them when structuring plan sections:

**Implementation Hygiene Rules** -- read `.claude/rules/impl-hygiene.md`:
!`cat .claude/rules/impl-hygiene.md`

**Code Hygiene Rules** -- read `.claude/rules/code-hygiene.md`:
!`cat .claude/rules/code-hygiene.md`

### Phase 6: Write Plan Files

Now — and ONLY now — create the plan directory and files. Every claim, file path, type reference, and function name in the plan must come from the research findings. **No fabricated paths. No guessed type names. No assumed module structures.**

#### Step 6a: Create Directory Structure

```
plans/{name}/
+-- index.md           # Keyword index for discovery
+-- 00-overview.md     # High-level goals and section summary
+-- section-01-*.md    # First section
+-- section-02-*.md    # Additional sections...
+-- section-NN-*.md    # Final section
```

#### Step 6b: Generate index.md

Create the keyword index with:
- **Reroute frontmatter** (if this is a reroute plan):
  ```yaml
  ---
  reroute: true
  name: "{Short Name}"
  full_name: "{Full Plan Name}"
  status: queued
  order: N
  ---
  ```
- Maintenance notice at the top
- How to use instructions
- Keyword cluster for each section (with REAL keywords from research — actual file names, type names, function names, not placeholders)
- Quick reference table

#### Step 6c: Generate 00-overview.md

Create overview with ALL sections from the template. **Ground every claim in research:**
- Mission: cite the specific production code paths being modified
- Architecture: show the ACTUAL current architecture (from research), then the target
- Design Principles: cite the specific constraints discovered in research
- Section Dependency Graph: based on actual code dependencies found in research
- Implementation Sequence: ordered by actual crate/module dependency direction
- Metrics: actual line counts from the relevant modules (measured, not estimated)
- Known Bugs: any bugs discovered during research

#### Step 6d: Generate Section Files

For each section, create `section-{NN}-{name}.md` with:

- YAML frontmatter (section ID, title, status: not-started, goal, `reviewed`, `third_party_review: { status: none, updated: null }`)
- **`reviewed: true` for Section 01 ONLY**
- **`reviewed: false` for ALL other sections**
- **`Production code path:`** — MANDATORY. Name the specific function, file, and line. From research, not guessed.
- **`Observable change:`** — MANDATORY. What will be different in the running terminal.
- **`Context:`** — Why this section exists, grounded in research findings
- **`Reference implementations:`** — Specific patterns from reference repos found during research
- **Subsections with `- [ ]` checkboxes** — Each task must reference a specific file and function from the research
- **Code examples** — Show the CURRENT code (from research) and the target code
- **`## {NN}.R Third Party Review Findings` block** initialized with `- None.`
- **Mandatory Build/Verify/Test gate** as the final subsection of EVERY section:
  ```markdown
  ## {NN}.N Build & Verify
  - [ ] `./build-all.sh` passes
  - [ ] `./clippy-all.sh` passes
  - [ ] `./test-all.sh` passes
  - [ ] New tests exist proving this section's changes work
  - [ ] No `#[allow(dead_code)]` on new items — everything has a production caller
  ```

**Grounding enforcement:** If you find yourself writing a file path, type name, or function name that did NOT appear in the research findings, STOP. Go back and verify it exists. If you can't verify it, flag it as `<!-- UNVERIFIED: {claim} -->`.

### Phase 7: Run /review-plan

After ALL plan files are written, run the actual `/review-plan` command on the plan directory. This is NOT optional — every plan gets reviewed before delivery.

**Do NOT recreate the review logic.** Run `/review-plan plans/{name}` which runs its own 4 sequential review agents with fresh eyes.

### Phase 8: Report Summary

Show the user:
- Files created (with paths)
- Key findings from the /review-plan verdict
- Any remaining concerns or decisions flagged by the review
- Whether the plan is ready for implementation or needs user attention

---

## Example

**Input:** `/create-plan gpu-refactor "Restructure GPU rendering pipeline for per-window renderers"`

**What happens:**
1. User provides name + goal
2. 5+ parallel research agents explore: GPU renderer code paths, wgpu types, atlas management, shader pipeline, reference renderer architectures, performance invariants
3. Research synthesis presented: "Current renderer is single-window at `oriterm_gpu/src/renderer.rs:47`, atlas at `oriterm_gpu/src/atlas.rs`, uses `wgpu::Surface` per window..."
4. Follow-up questions: "Should each window have its own atlas or share one? WezTerm shares, Alacritty uses one per window."
5. User answers questions
6. Plan outline presented for approval
7. Plan files written with grounded references
8. `/review-plan` runs automatically
9. Final report delivered

**Creates:**
```
plans/gpu-refactor/
+-- index.md
+-- 00-overview.md
+-- section-01-renderer-architecture.md
+-- section-02-atlas-management.md
+-- section-03-pipeline-consolidation.md
```

---

## Section Naming Conventions

| Section Type | Naming Pattern |
|--------------|----------------|
| Setup/Infrastructure | `section-01-setup.md` |
| Core Implementation | `section-02-core.md` |
| Integration | `section-03-integration.md` |
| Testing | `section-04-testing.md` |
| Documentation | `section-05-docs.md` |

---

## After Creation

Remind the user to:
1. Review the `/review-plan` verdict and address any flagged concerns
2. **If performance-sensitive** (GPU rendering, VTE parsing, grid operations): Verify benchmark/profiling checkpoints in relevant sections

## Performance-Sensitive Plans

For plans touching hot paths, include a "Performance Validation" section in `index.md`:

```markdown
## Performance Validation

Use profiling after modifying hot paths.

**When to benchmark:** [list specific sections]
**Skip benchmarks for:** [list non-perf sections]
```

See `plans/_template/plan.md` for full guidance.

---

## What This Command MUST NOT Do

1. **Must NOT skip research.** The parallel research phase is mandatory, not optional.
2. **Must NOT make assumptions.** If a file path, type name, or function isn't verified by reading the actual code, it doesn't go in the plan.
3. **Must NOT skip user questions.** When research reveals design decisions, ASK the user. Don't pick defaults silently.
4. **Must NOT write plan files before research completes.** The plan is the OUTPUT of research, not the starting point.
5. **Must NOT replace /review-plan with inline agents.** Use the actual /review-plan command.
6. **Must NOT ask the user to design the plan structure before research.** You figure out the right structure from the research, then propose it.

## Template Reference

The command uses `plans/_template/plan.md` as the structure reference. See that file for:
- Complete index.md template
- Section file template
- Status conventions
- The roadmap (`plans/roadmap/`) as a working example
