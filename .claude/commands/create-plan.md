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

---

## Execution Model — Why Order Matters

Plan sections are inherently sequential. Section 02 depends on what section 01 establishes — the types it introduces, the APIs it creates, the architectural decisions it makes. This has two consequences:

1. **Research can be parallel.** Multiple agents exploring different facets of the codebase simultaneously is fine — they're reading, not writing. More agents = more coverage.

2. **Section writing MUST be sequential.** A single agent writes all sections, one after another, in dependency order. Section 02 is written with full knowledge of what section 01 contains. Section 03 knows what 01 and 02 established. This produces sections that form a coherent narrative with real cross-references, not parallel islands of assumption.

**NEVER launch parallel agents to write different sections.** This is the single most important rule in this command. Parallel section writing produces plans where each section makes independent assumptions about the others, leading to contradictions, duplicated types, missed dependencies, and sections that read like they were written by different people who never talked.

---

## Workflow

### Phase 1: Initial Scoping (Brief)

If not provided via arguments, use AskUserQuestion to get:

1. **Plan name** — kebab-case directory name
2. **Plan title** — Human-readable title (e.g., "GPU Renderer Refactor")
3. **Goal** — One-line description of what this plan accomplishes

**Do NOT ask for sections yet.** Sections are determined by research, not guesswork. Do NOT ask the user to design the plan structure — that's YOUR job after research.

### Phase 2: Broad Research Sweep (Parallel Agents)

**This is the landscape mapping pass.** Launch **4-6 parallel research agents** to explore different facets of the codebase simultaneously. The goal is to build a complete map of what exists today in every area the plan will touch.

Tell the user: "Launching N parallel research agents to map the codebase..."

**MANDATORY: Every research agent must report its findings with specific file paths, line numbers, function signatures, type definitions, and call chains. No vague summaries. No "the code probably does X." Every finding must be grounded to a specific location in the code.**

**MANDATORY: Each agent must READ the actual source files it reports on — not just grep for names. Reading a function means understanding what it does, what it calls, what data flows through it, and what constraints it operates under. A grep hit is a starting point, not a finding.**

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
3. READ the actual source files — not just grep results. Read each function you report on.
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
   - Full definition (fields, variants, methods) — READ the file, quote the actual code
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

Report with EXACT definitions, paths, and line numbers. Quote actual type signatures and struct definitions.
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
   - READ the actual source files, don't just report file names
   - What's the design pattern? Describe it concretely.
   - What are the key data structures? Quote their definitions.
   - What trade-offs did they make? What did they get right? What did they get wrong?
   - What can we learn from or adopt?
4. Note areas where reference implementations DISAGREE — this often indicates a design decision the plan needs to make explicitly
5. For the most relevant 2-3 implementations, trace the full data flow through their system — not just "they have a BorderStyle struct" but "BorderStyle is created in X, flows through Y, gets consumed in Z which uses it to do W"

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
5. Check file size limits — READ each relevant file and report its current line count. Flag any file over 400 lines (approaching the 500-line limit).
6. Check for existing tests that verify current behavior (regression risk)

Report every constraint with its source (file path, rule name, test name).
```

**Additional research agents:** If the plan's goal suggests specific additional areas to research (e.g., GPU rendering → research the shader pipeline, PTY handling → research the mux layer), launch additional targeted agents. Use your judgment — more research is always better than less.

### Phase 3: Targeted Deep Dives (Parallel Agents)

**This is the drilling pass.** After the broad sweep completes, you now know *where* the important code is. Launch **2-4 targeted deep-dive agents** that go deep into the specific areas that matter most.

The difference between Phase 2 and Phase 3:
- **Phase 2** answers "what exists and where is it?"
- **Phase 3** answers "how does it actually work, end-to-end, in detail?"

Tell the user: "Broad sweep complete. Launching N targeted deep-dive agents into the critical areas..."

Design each deep-dive agent based on what Phase 2 revealed. Each agent should:

1. **Trace one complete data flow end-to-end** through 3-6 files. Not "this file defines X" but "X is created in file A at line N, passed to file B's function Y, transformed by file C's function Z, and ultimately consumed by file D's function W. Here is how each transformation works."

2. **Read every function it reports on in full.** Not just the signature — the body. Understand what happens at each step. Report the actual logic, not just "it processes the data."

3. **Identify the exact modification points.** For the plan's goal, exactly which lines need to change? What's the before/after? What downstream code would break?

4. **Map the ripple effects.** If we change type X, what breaks? Grep for every usage. Read each usage site. Report whether it's a trivial update or a design change.

Example deep-dive agent prompt (adapt based on Phase 2 findings):

```
You are doing a DEEP DIVE into {specific area} for a plan about: {goal}.

Phase 2 research found that {key finding from broad sweep}. Your job is to trace this in full detail.

INSTRUCTIONS:
1. Start at {entry point found in Phase 2} and trace the complete flow:
   - Read {file A} — understand what it does with {data}
   - Follow the call to {file B} — read the full function body
   - Continue to {file C} — understand the transformation
   - End at {file D} — understand how the result is consumed
2. For each step, report:
   - The function signature and its full body (or key sections if very long)
   - What data flows in and out
   - What invariants it maintains
   - What would need to change for {goal}
3. Map every usage of {key type/function} across the codebase:
   - Grep for it
   - Read each usage site (don't just list file names)
   - Classify each usage: trivial update / needs redesign / unaffected
4. Identify the dependency chain:
   - What must change FIRST (deepest dependency)?
   - What changes NEXT (depends on the first change)?
   - What changes LAST (depends on everything else)?
   - This dependency chain becomes the section ordering.

Report the complete trace with file paths, line numbers, and quoted code.
```

### Phase 4: Research Synthesis (Single Agent)

**This is the most important agent in the entire process.** Launch a SINGLE synthesis agent that receives ALL findings from Phase 2 and Phase 3 and builds a unified model.

**Do NOT attempt synthesis in the main conversation thread.** The synthesis agent needs focused context dedicated entirely to pulling findings together.

```
You are the SYNTHESIS AGENT for a plan about: {goal}.

Below are the findings from {N} research agents who explored the codebase. Your job is to pull these findings into ONE COHERENT MODEL and propose a plan structure.

## Research Findings

{Paste ALL findings from Phase 2 and Phase 3 agents here — the complete text, not summaries}

## Your Tasks

1. BUILD A UNIFIED MODEL of the relevant codebase:
   - What are the key files, types, and functions?
   - How does data flow through the system today?
   - What are the constraints (file size limits, crate boundaries, performance invariants)?
   - Where do the research agents agree? Where do they contradict each other?
   - What did the research miss? (Flag gaps explicitly.)

2. IDENTIFY THE DEPENDENCY CHAIN for the plan's changes:
   - What must change first? (Deepest dependency — usually a type or trait in a library crate)
   - What depends on that change?
   - What depends on THAT change?
   - Continue until you reach the outermost consumer.
   - This chain determines section ordering.

3. PROPOSE A SECTION BREAKDOWN:
   For each proposed section:
   - What production code path does it modify?
   - What observable change does it produce?
   - What does it depend on from prior sections? (Be specific — name the type, API, or change.)
   - What does it establish for later sections? (Name what later sections will use.)
   - Estimated scope (small / medium / large)

4. IDENTIFY DESIGN DECISIONS that need user input:
   - Where do reference implementations disagree?
   - Where are there multiple valid approaches?
   - Where does the research leave uncertainty?
   - What scope questions remain open?

5. FLAG RISKS:
   - Which sections are highest risk (most code change, most dependencies, most unknowns)?
   - Which sections have the most ripple effects?
   - Are there any "this might not work" concerns?

## Output Format

Structure your output as:
- UNIFIED MODEL: {the coherent picture}
- DEPENDENCY CHAIN: {ordered list with reasoning}
- PROPOSED SECTIONS: {numbered list with details}
- DESIGN DECISIONS: {questions for the user}
- RISKS: {ranked list}
```

### Phase 5: Interactive Q&A

**After the synthesis agent completes**, present its findings to the user and ask follow-up questions.

#### Step 5a: Present Research Findings

Show the user the synthesis agent's unified model, reformatted for clarity:

```
## Research Findings for: {plan title}

### How the System Works Today
{Unified model — the data flow, the key types, the architecture, grounded in specific files}

### What Needs to Change
{The dependency chain — what changes first, what depends on it, ordered}

### Reference Implementations
{How established projects handle this, with specific patterns and trade-offs}

### Constraints
{Performance invariants, platform requirements, crate boundaries, file size limits}

### Key Design Decisions Needed
{Questions from the synthesis agent — specific, grounded in findings}
```

#### Step 5b: Ask Follow-Up Questions

Use AskUserQuestion to ask the user about design decisions revealed by the research. These should be SPECIFIC questions grounded in what you found, not generic. Examples:

- "The codebase already has {X} in {file}. Should the plan extend this or replace it?"
- "Alacritty does {A} but WezTerm does {B}. Which approach fits ori_term better?"
- "{Type} in {crate} is currently {N} lines. Adding {feature} would push it past 500 lines. Should we split the module first?"
- "There's an existing plan at plans/{name} that touches the same area. Should this plan supersede it or coordinate with it?"

**Ask ALL questions you're uncertain about.** Never assume. If you're unsure about scope, approach, ordering, or integration points — ASK.

**Wait for the user's answers before proceeding.** If answers raise more questions, ask those too. This phase is iterative until you have clarity on all design decisions.

### Phase 6: Plan Design (Still No Files Yet)

Based on the synthesis + user answers, finalize the plan structure. Present the proposed outline to the user:

```
## Proposed Plan Structure: {title}

### Section 01: {name}
- Production code path: {specific function/file}
- Observable change: {what will be different}
- Depends on: nothing (first section)
- Establishes: {what later sections will use from this}
- Key tasks: {brief list}

### Section 02: {name}
- Production code path: {specific function/file}
- Observable change: {what will be different}
- Depends on: Section 01 ({specifically what — name the type/API})
- Establishes: {what later sections will use from this}
- Key tasks: {brief list}

...

### Section NN: Verification
- Test matrix, visual regression, performance validation
```

Use AskUserQuestion: "Does this plan structure look right? Any sections to add, remove, or reorder?"

**Wait for approval before writing files.**

### Phase 7: Read Template & Hygiene Rules

Read `plans/_template/plan.md` for the structure reference.

The full hygiene rule set is embedded below (source of truth files — do not maintain separate copies). Use these rules when structuring plan sections to ensure plans account for module boundary discipline, file size limits, rendering pipeline purity, and other hygiene requirements from the start.

**Implementation Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
@.claude/rules/impl-hygiene.md

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
@.claude/rules/code-hygiene.md

### Phase 8: Write Plan Files (Single Sequential Agent)

**CRITICAL: This phase uses ONE agent that writes ALL files sequentially.** Do NOT launch parallel agents for different sections.

Launch a single agent with the following structure. This agent writes every plan file in order: `index.md` first, then `00-overview.md`, then `section-01`, then `section-02`, etc. Each section is written with full knowledge of what all previous sections contain.

```
You are the PLAN WRITER for: {goal}.

You will write ALL plan files, ONE AT A TIME, IN ORDER. Each section builds on what came before it. You have full knowledge of the research, the synthesis, the user's design decisions, and the approved structure.

## Context

{Paste: synthesis agent output, user's design decisions, approved plan outline}

## Research Findings

{Paste: ALL research findings from Phase 2 and Phase 3 — the complete text}

## Rules

1. Write files in this EXACT order:
   a. plans/{name}/index.md
   b. plans/{name}/00-overview.md
   c. plans/{name}/section-01-*.md
   d. plans/{name}/section-02-*.md
   e. ... (continue in order)
   f. plans/{name}/section-NN-*.md (final section)

2. When writing section N, you KNOW what sections 1 through N-1 contain because you just wrote them. Reference their types, APIs, and changes explicitly. Say "Section 02 introduced BorderSides (see 02.1). This section uses it to..." — not "assuming a border type exists."

3. Every file path, type name, function name, and line number must come from the research findings. If you can't find it in the research, flag it as <!-- UNVERIFIED: {claim} --> so the review catches it.

4. YAML frontmatter for sections:
   - `reviewed: true` for Section 01 ONLY
   - `reviewed: false` for ALL other sections
   - `third_party_review: { status: none, updated: null }` for all sections

5. Every section MUST have:
   - `Production code path:` — the specific function/file this section modifies
   - `Observable change:` — what's different in the running terminal
   - `Depends on:` — what prior sections this uses (with specific references)
   - `Establishes:` — what later sections will build on
   - Subsections with `- [ ]` checkboxes referencing specific files and functions
   - Code examples showing CURRENT code (from research) and TARGET code
   - `## {NN}.R Third Party Review Findings` block (initialized with `- None.`)
   - `## {NN}.N Build & Verify` gate as the final subsection

6. Incremental design enforcement: If a section can't name a specific production code path it touches, it needs restructuring.

## File Templates

{Include the relevant sections from plans/_template/plan.md}

## Write all files now.
```

The agent writes every file. This is slower than parallel writing, but produces a coherent plan where:
- Section 02 explicitly references what section 01 introduced
- Section 03 builds on the combined foundation of 01 and 02
- Cross-references are real, not assumed
- No two sections independently invent the same type or solve the same problem
- The dependency chain is reflected in the actual content, not just the frontmatter

### Phase 9: Run /review-plan

After ALL plan files are written, run the actual `/review-plan` skill on the plan directory. This is NOT optional — every plan gets reviewed before delivery.

```
Use the Skill tool to invoke: skill: "review-plan", args: "plans/{name}"
```

**Do NOT recreate the review logic.** Use the actual `/review-plan` skill which runs its own 4 sequential review agents. This ensures the plan gets the same rigorous review that any plan would get, with fresh eyes that have no context from the creation process.

### Phase 10: Report Summary

Show the user:
- Files created (with paths)
- Key findings from the /review-plan verdict
- Any remaining concerns or decisions flagged by the review
- Whether the plan is ready for implementation or needs user attention

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

1. **Must NOT launch parallel agents to write different sections.** Sections are sequential. One agent writes all sections in order. This is the most important rule.
2. **Must NOT skip research.** Both the broad sweep AND the targeted deep dives are mandatory.
3. **Must NOT skip the synthesis agent.** Research findings must be unified by a dedicated agent before writing begins.
4. **Must NOT make assumptions.** If a file path, type name, or function isn't verified by reading the actual code, it doesn't go in the plan.
5. **Must NOT skip user questions.** When research reveals design decisions, ASK the user. Don't pick defaults silently.
6. **Must NOT write plan files before research, synthesis, and user approval complete.** The plan is the OUTPUT of understanding, not the starting point.
7. **Must NOT replace /review-plan with inline agents.** Use the actual Skill tool to run `/review-plan`.
8. **Must NOT ask the user to design the plan structure before research.** You figure out the right structure from the research, then propose it.

## Template Reference

The command uses `plans/_template/plan.md` as the structure reference. See that file for:
- Complete index.md template
- Section file template
- Status conventions
- The roadmap (`plans/roadmap/`) as a working example
