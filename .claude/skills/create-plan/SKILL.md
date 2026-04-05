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
/create-plan add "<subsection title>" subsection to plans/<plan-dir>
```

- `name`: Directory name for the plan (kebab-case, e.g., `gpu-refactor`, `mux-architecture`)
- `description`: Optional one-line description of the plan's goal
- **Existing plan mode**: If the input references an existing plan directory (e.g., "add X to plans/repr-opt", "add section to roadmap"), this command operates in **Existing Plan Mode** — see the dedicated section below.

---

## Mode Detection

**New Plan Mode** (default): The argument names a new plan directory. Creates `plans/{name}/` from scratch.

**Existing Plan Mode**: The argument indicates adding a section or subsection to an existing plan. Detected when:
- The input contains "roadmap" or references an existing roadmap section (legacy Roadmap Mode — operates on `plans/roadmap/`)
- The input references any existing plan directory (e.g., "add X to plans/gpu-refactor", "add subsection to plans/mux-flatten")
- The input uses the explicit syntax: `add "<title>" subsection to plans/<dir>`

When in Existing Plan Mode, the target is the referenced plan directory. See "Existing Plan Mode" section below for the full workflow.

Both modes follow the SAME research rigor, the SAME iterative deepening, the SAME sequential writing discipline. The difference is the target: a new plan vs. an existing one.

---

## Design Principles

These principles govern the entire plan creation process. When in doubt, consult these:

1. **Research depth > research breadth** — One agent that reads 15 files thoroughly beats 5 agents that scan 50 files superficially. Understanding invariants, control flow, and edge cases matters more than listing type signatures.

2. **Architecture before sections** — The overview isn't boilerplate. It's the load-bearing design document. Sections are *implementations of* the architecture, not independent documents. Design first, detail second.

3. **Sequential section writing is non-negotiable** — Sections depend on each other. Section 3 references decisions made in Section 2. Parallel writing forces each section to *guess* what other sections decided, producing contradictions. Write one section at a time, in order.

4. **User checkpoints at design-level decisions** — Don't ask the user to review 8 completed sections. Ask them to review the architecture *first*, then write sections they've already conceptually agreed to.

5. **Iterative deepening over parallel breadth** — Start wide, then go deep on what matters. Each research pass builds on the findings of the prior pass.

6. **External consultations are SEQUENTIAL and FOREGROUND** — All `/tp-help` and `/tpr-review` invocations MUST run in the foreground (NOT `run_in_background`). MUST wait for each to complete and read its output before proceeding. NEVER launch them in parallel with each other or with other agents/skills. The pipeline is sequential by design — each consultation's feedback informs the next step.

7. **Rules are woven in, not assumed** — Plans cannot assume the implementer has CLAUDE.md or `.claude/rules/*.md` loaded in context. Every section must embed the specific rules that govern its work — TDD discipline, file size limits, crate ordering, test conventions, module boundaries, rendering pipeline purity, cross-platform requirements. The plan is a self-contained execution document. If a rule applies to a section's work, it must appear in that section — either as a checklist constraint, a callout, or an inline requirement. The goal is for rules to appear organically as part of the work description, not as a separate "rules to follow" appendix.

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

### Step 1B: Mission Expansion

The user's input is typically a generic, high-level mission statement. Your job is to expand it into a **full executable mission statement** before any research or plan creation begins.

Take the user's rough goal and expand it into:

1. **Concrete scope**: What crates, subsystems, files, and features are in scope? What's explicitly out?
2. **Deliverables**: What specific, verifiable outcomes does this plan produce? (Not "improve X" — "X does Y where it currently does Z")
3. **Success criteria**: How do you know the mission is complete? What tests pass? What behavior changes?
4. **Boundaries**: What does this plan NOT do? Where does it hand off to other plans or future work?

**Scoping discipline — CRITICAL:**

The terminal emulator is under active development. Plans exist to build out the emulator's feature set. Scoping must reflect this reality:

**Valid reasons to scope something OUT:**
- It doesn't fit ori_term's architecture — would feel tacked on, not organic to the emulator's design
- It doesn't improve the emulator meaningfully — busywork with no architectural payoff
- It's architecturally incoherent with the existing design direction
- It belongs in a different plan that addresses a different subsystem (but must be cross-linked as a dependency)

**INVALID reasons to scope something out:**
- "The UI framework doesn't support X yet" — that's a blocker to resolve (Step 1C), not a scope exclusion
- "The GPU renderer can't handle Y" — same: blocker, not scope
- "We'd need to add Z infrastructure first" — same: blocker, not scope
- Any "missing prerequisite" or "missing feature" argument — if we always scoped out features because prerequisites were missing, no features would ever get built. Missing prerequisites are what Step 1C (Blocker Identification) captures and what the plan resolves.

When evaluating scope, ask: "Is this being excluded because it doesn't belong in ori_term, or because building it requires work?" Only the first is valid. The second is the plan's job.

### Step 1C: Blocker Identification

The mission must remove any blockers in its way. Before the mission can be fulfilled, you must identify what stands between the current codebase state and the mission's goals.

1. **Identify blockers**: What existing bugs, missing features, incomplete infrastructure, or broken subsystems would prevent the mission from being fulfilled?
2. **Check existing tracking**: For each blocker, search:
   - `plans/roadmap/` — is it a roadmap item? Which section?
   - `plans/bug-tracker/` — is it a tracked bug? Which entry?
   - Other `plans/*/` directories — is it queued in another plan? Which section?
   - CLAUDE.md memory entries — is it a known issue?
3. **Resolution strategy**: For each blocker:
   - If tracked elsewhere: this plan MUST include executing/resolving the blocker. Add it as a section or checklist item. When complete, update the original location (roadmap section, bug-tracker entry, other plan) as resolved with a cross-link back to this plan's section.
   - If not tracked anywhere: this plan owns it entirely. Add it as a section or checklist item.
   - If the blocker is too large to include (would double the plan's scope): flag it via `AskUserQuestion` — the user decides whether to expand scope or split into prerequisite plans.
4. **Cross-link format**: When resolving a blocker from another plan, add `<!-- resolved-by: plans/{this-plan}/section-NN -->` to the original location, and `<!-- resolves: plans/{other-plan}/section-MM item description -->` to this plan's item.

### Step 1D: Consensus Loop with Codex (MANDATORY — ITERATE UNTIL AGREEMENT)

**SEQUENTIAL & FOREGROUND — MANDATORY.** Every `/tp-help` call in this loop MUST run in the foreground (NOT `run_in_background`). You MUST wait for each to complete and read its output before proceeding. Do NOT launch these in parallel with any other agent or skill invocation.

This is not a single consultation — it is a **consensus loop**. You and Codex iterate on the mission's direction, approach, and integration points until you reach genuine agreement. The loop runs until one of two outcomes:

1. **Consensus reached**: You and Codex agree on how the plan integrates with ori_term's architecture, what the approach should be, and that it's a good fit.
2. **Agreed rejection**: You and Codex both agree that part or all of the proposed direction is not a good fit for ori_term — in which case, document why and propose an alternative direction.

**Loop protocol:**

**Round 1** — Present the full picture to Codex:

Build a `/tp-help` prompt that includes:
- The user's original generic mission statement
- Your expanded mission (scope, deliverables, success criteria, boundaries) from Step 1B
- The identified blockers and their resolution strategy from Step 1C
- Your proposed direction and approach — how does this integrate with ori_term's existing architecture?
- Any open questions or uncertainties

Ask Codex specifically:
- "Is this mission statement complete and executable? Are there gaps?"
- "Are the identified blockers comprehensive, or am I missing dependencies?"
- "Is the scope right — too broad, too narrow, or just right for a single plan?"
- "Does this direction integrate well with ori_term's architecture? Where are the natural integration points?"
- "What would you change about the approach?"

**Round 2+** — Respond to Codex's feedback:

After each Codex response, evaluate:
- **Points of agreement**: Lock these in. They become part of the consensus.
- **Points of disagreement**: For each, either (a) accept Codex's point and update the mission, or (b) push back with specific reasoning and ask Codex to reconsider. Do NOT silently ignore disagreements.
- **New concerns raised**: Address each one. If Codex identified a blocker or integration issue you missed, incorporate it.
- **Integration fit**: If Codex questions whether something fits ori_term, engage seriously — is there a better organic integration point? Or is this genuinely not the right approach?
- **Scoping pushback**: If Codex suggests scoping something out because "the UI framework doesn't support X yet" or "Y infrastructure is missing," push back — those are blockers to resolve, not scope exclusions. The only valid reason to exclude something is that it doesn't fit ori_term's design. Missing prerequisites are what the plan exists to build.

Call `/tp-help` again with:
- What you agree on so far (locked-in consensus points)
- What you're still iterating on (with your response to Codex's feedback)
- Updated mission statement reflecting changes from this round
- Specific questions for the remaining disagreements

**Loop termination**: The loop ends when BOTH of these are true:
- You and Codex agree on the mission direction, scope, approach, and integration points (or agree that something should be excluded and why)
- There are no unresolved disagreements or open questions between you

**Do NOT cap the loop at a fixed number of rounds.** Most missions will converge in 2-3 rounds. Some may take 4-5. The loop runs until consensus, not until a counter expires.

**After consensus**, compile the results:

1. **Consensus points**: What you and Codex agreed on — direction, approach, integration points, scope
2. **Rejected directions**: What you both agreed is not a good fit for ori_term, and why
3. **Draft execution outline**: A preliminary sketch of how the plan will be executed — approximate section structure, rough ordering, key phases. This is a draft (full planning hasn't run yet), but it gives the user a sense of shape:
   - What gets built first (foundation/prerequisites)
   - What the core implementation phases are
   - What integration/verification looks like
   - Where the major risks and decision points are

### Step 1E: Mission Proposal to User

**MANDATORY — DO NOT SKIP.** Use `AskUserQuestion` to present the consensus results to the user for approval before proceeding.

Present:
1. **Original input**: What the user said
2. **Expanded mission**: The full executable mission statement (scope, deliverables, success criteria, boundaries)
3. **Claude + Codex consensus**: What was agreed on — direction, approach, integration points. Present this as a unified position, not a transcript. The user should see what was decided and why.
4. **Rejected directions** (if any): What was considered and ruled out as not fitting ori_term, with reasoning
5. **Identified blockers**: Each blocker, where it's currently tracked (if anywhere), and how this plan will resolve it
6. **Cross-plan impacts**: Which other plans/roadmap items will be updated as resolved when this plan executes
7. **Draft execution outline**: The preliminary plan shape — section structure, ordering, phases. Flag this as a draft that will be refined during the full research and planning phases.

Ask: "Does this mission and approach accurately capture what you want? Any adjustments to the direction, scope, or approach before I proceed with research and plan creation?"

**Do NOT proceed to Step 2 until the user approves the mission.** If they redirect or adjust, go back to Step 1B with the new direction and repeat Steps 1C-1E (including the consensus loop).

### Step 2: Read the Template & Hygiene Rules

Read `.claude/skills/create-plan/plan-schema.md` for the structure reference.

The full rule set is embedded below (source of truth files — do not maintain separate copies). Use these rules when structuring plan sections to ensure plans account for module boundary discipline, file size limits, rendering pipeline purity, cross-platform requirements, and other hygiene requirements from the start.

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

**This step cannot be parallelized.** Each file read may inform what to look for in the next file. If reading file A reveals that it delegates to file B in a non-obvious way, read file B next.

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

### Step 6B: Third-Party Architectural Consultation

**SEQUENTIAL & FOREGROUND — MANDATORY.** This `/tp-help` call MUST run in the foreground (NOT `run_in_background`). You MUST wait for it to complete and read its output before proceeding to Phase 3. Do NOT launch this in parallel with any other agent or skill invocation.

**After ALL research passes complete**, call `/tp-help` to get a second opinion on the architectural direction before committing to it. This is the highest-leverage consultation point — all research is in, but no architecture is locked.

Build a `/tp-help` prompt that includes:
- The plan's mission/goal
- A condensed summary of key research findings (critical files, sync points, design decisions, analogous patterns, existing bugs)
- The 2-3 most important architectural decisions you're about to make
- Your preliminary architectural direction

Ask Codex specifically:
- "Do you see any architectural risks I'm missing?"
- "Is this the right decomposition for this problem?"
- "Are there better patterns from the reference terminal emulators for this specific case?"

Evaluate Codex's response against your research — you have deeper codebase context, so filter accordingly. Incorporate useful insights into the architecture design.

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
6. **Test strategy** — existing coverage AND planned test requirements per section: what tests exist (from Pass 1-2), what matrix dimensions (types x patterns) each section needs, where semantic pin tests are needed, what harness patterns (WidgetTestHarness, sibling tests.rs, architecture tests)
7. **All unclear items** — things the research couldn't determine
8. **All existing bugs found** — bugs discovered during research (these go into the plan)
9. **Hygiene pre-scan** — files that need splitting or cleanup
10. **Dependency chain** — what must be built first, what gates what, what can be parallelized

### Step 8: Write `00-overview.md` FIRST

The overview is the **load-bearing design document**. It is NOT boilerplate filled in after sections are written — it is the architectural blueprint that DRIVES section content.

Write `00-overview.md` following the template in `.claude/skills/create-plan/plan-schema.md`, grounding every element in research:

- **Mission**: Based on the actual problem discovered during research — what exists, what's broken, what's missing
- **Mission Success Criteria**: Concrete, testable conditions that prove the mission is complete — derived from the approved mission statement (Step 1E). Every criterion must be traceable to at least one section. A criterion with no section delivering it is a plan gap.
- **Architecture diagram**: Based on the actual data flow map from Pass 2's deep read — show how data enters, transforms, and exits
- **Design principles**: Based on patterns observed in analogous features (Pass 3) and prior art (Pass 4) — cite the specific evidence
- **Section dependency graph**: Based on actual crate dependencies and sync points found in Pass 1 — show which sections gate others
- **Implementation sequence**: Based on the analogous feature pattern from Pass 3 — follow the same order that worked before
- **Design decisions**: Include the key design decisions from Pass 4 with recommended approaches and evidence
- **Known bugs**: Include ALL bugs found during research passes
- **Metrics**: Use actual line counts from the hygiene pre-scan

**Also create `index.md`** with keyword clusters using REAL keywords from the research (actual type names, function names, file names — not placeholders).

### Step 8B: Architecture Sanity Check via /tp-help

**SEQUENTIAL & FOREGROUND — MANDATORY.** This `/tp-help` call MUST run in the foreground (NOT `run_in_background`). You MUST wait for it to complete and read its output before proceeding to Step 9. Do NOT launch this in parallel with any other agent or skill invocation.

**Before presenting to the user**, call `/tp-help` to sanity-check the written overview architecture. This catches issues before the user sees them.

Build a `/tp-help` prompt that includes:
- The content of `00-overview.md` (or a focused summary of: mission, dependency graph, implementation sequence, key design decisions)
- The proposed section list with goals and ordering

Ask Codex specifically:
- "Does this section decomposition and ordering make sense?"
- "Are there dependency ordering issues I'm missing?"
- "Would you structure this differently?"

Incorporate feedback into `00-overview.md` before presenting to the user. If Codex flags a fundamental issue, address it now — don't pass known problems to the user review.

### Step 9: User Review of Architecture (MANDATORY — DO NOT SKIP)

**You MUST use `AskUserQuestion` here.** Present the architecture and get explicit buy-in before writing sections.

Present:
1. **The architecture**: Summarize the design from `00-overview.md` — mission, data flow, key design decisions
2. **The proposed sections**: List each section with its goal, what files it touches, and what it depends on. Explain WHY these sections and WHY this order.
3. **Design decisions**: For each key design decision, present the recommended approach with evidence. Ask if the user agrees or wants a different approach.
4. **Analogous pattern**: "Feature X follows this pattern: {pattern}. This plan will follow the same pattern. Does this align with your vision?"
5. **Resolve unclear items**: For every `UNCLEAR` item from research, ask the user.
6. **Report existing bugs**: "During research, I found these existing issues: {list}. Per zero-deferral, these will be included in the plan."
7. **Scope adjustments**: If research revealed the scope is larger or smaller than expected, propose adjustments with rationale.

**Do NOT proceed to Phase 4 until the user responds and approves the architecture.** If they redirect or adjust scope, update the overview and re-present. If they change design decisions, update accordingly. The architecture must be agreed upon before sections are detailed.

---

## Phase 4: Sequential Section Writing (MANDATORY SEQUENTIAL — NO PARALLELISM)

**CRITICAL RULE: Write sections ONE AT A TIME, IN ORDER.** Do NOT launch parallel agents to write sections. Each section depends on decisions and details from prior sections. Section N is not written until Section N-1 is complete.

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
- **Registration sync points**: List ALL sync points from research for any new enum variant/type/entry
- **Analogous pattern**: Reference the analogous feature's implementation pattern — "Follow the same pattern as {feature} in {files}"
- **Code examples**: Show target implementation based on actual code patterns found during research, not invented patterns
- **Test strategy**: Every section that modifies code MUST include matrix testing and pinning requirements per CLAUDE.md — this is not deferred to implementation or review:
  - **Matrix dimensions**: Identify ALL types and ALL control-flow patterns that flow through the changed code path. The plan must name these explicitly (e.g., "test with ASCII, CJK, emoji, combining marks, ZWJ sequences" and "test resize, scroll, reflow, selection, search"). Missing cells are future regressions.
  - **Semantic pin**: At least one test per behavioral change that ONLY passes with the new semantics — a permanent regression guard. The plan must describe what the pin tests.
  - **TDD ordering**: "Write failing test matrix BEFORE implementation" as the section's FIRST checklist item; "Verify all tests pass in debug and release" as the LAST item.
  - **Test types**: Specify which test categories (Rust unit tests in sibling `tests.rs`, widget tests via WidgetTestHarness, architecture tests, visual regression tests).
  - A section without explicit matrix dimensions and semantic pin requirements is NOT executable.
- **Dependencies on prior sections**: Explicitly reference what earlier sections provide. "This section uses the {type} defined in Section {N} ({file path})."
- **What this section provides to later sections**: State what downstream sections will depend on. "Section {M} will use the {API/type/pattern} established here."

- **Success criteria**: Every section MUST have detailed success criteria — concrete, testable conditions that prove the section's work is done. Not "implement X" but "X produces Y when Z is run." Each criterion must connect upward to at least one mission success criterion in `00-overview.md`. A section without success criteria is not executable.
- **Rules woven in**: Every section must embed the CLAUDE.md and `.claude/rules/*.md` rules that apply to its work — not as a "rules" appendix, but woven organically into checklist items, constraints, and callouts. Read CLAUDE.md and the relevant rule files (`.claude/rules/test-organization.md` for test sections, `.claude/rules/impl-hygiene.md` for module boundaries, `.claude/rules/crate-boundaries.md` for crate ownership, `.claude/rules/code-hygiene.md` for surface cleanliness) and embed the applicable constraints directly into the section's tasks. For example: if a section adds a new widget, the checklist item should say "Add `FooWidget` in `oriterm_ui/src/widgets/foo/mod.rs` — implement Widget trait, add to widget registry, create sibling `tests.rs` with WidgetTestHarness tests" rather than "Add widget (remember to follow conventions)." The plan is a self-contained execution document — the implementer should not need to consult external rule files to know what a section requires.

**Frontmatter includes:**
- Section ID, title, status: not-started, goal, `success_criteria` list
- `reviewed` field (see rules below)
- `inspired_by` with actual reference implementations found
- `depends_on` based on actual crate dependency chain AND section content dependencies
- `third_party_review: { status: none, updated: null }`
- `## {NN}.R Third Party Review Findings` block (empty, with `- None.`) before the completion checklist
- Completion checklist at the end — MUST include both `/tpr-review` AND `/impl-hygiene-review last commit` as final gates (hygiene review runs after TPR is clean, per `plan-schema.md`)
- **Mid-section TPR checkpoints** on substantial subsections per `plan-schema.md` TPR Checkpoint Rules — catch issues early, not just at section end

**`reviewed` field rules:**
- **Section 01**: `reviewed: true` — it is the starting point of implementation and was validated during plan creation against the research findings.
- **All other sections (02+)**: `reviewed: false` — they have NOT been validated against actual implementation reality. As Section 01 is implemented, assumptions in later sections may become stale or wrong.

**After writing each section**, briefly verify:
- File paths referenced in this section exist
- Type/function names referenced exist
- References to prior sections are accurate (re-read the referenced section if needed)
- No contradictions with prior sections

Then proceed to the next section.

### Step 12: Update Overview and Index

After all sections are written:
- Update `00-overview.md` with the final section list, dependency graph, and any adjustments that emerged during sequential writing
- Update `index.md` with complete keyword clusters for all sections — using actual type names, function names, and file names from the written sections

---

## Phase 5: Cohesion Review & Finalization

### Step 13: Cohesion Check (before /review-plan)

Launch **one agent** to read the ENTIRE plan front-to-back and check for internal coherence:

```
You are reviewing a newly created plan for internal coherence. Read EVERY file in the plan directory: {plan_dir}/

Check for:
1. CONTRADICTIONS: Does Section X say one thing and Section Y say another? (e.g., Section 2 says "add type Foo to module Bar" but Section 5 says "add type Baz to module Bar" for the same purpose)
2. GAPS: Is there work that falls between sections? (e.g., Section 2 produces a type that Section 4 consumes, but no section handles the transformation between them)
3. REDUNDANCY: Do multiple sections do the same work? (e.g., both Section 3 and Section 5 add the same match arm)
4. BROKEN REFERENCES: Does Section X reference a type/file/function from Section Y that Section Y doesn't actually define?
5. ORDERING ISSUES: Does Section X depend on work described in Section Y, but X comes before Y?
6. SYNC POINT COMPLETENESS: Are ALL sync points (enum variants, match arms, registry entries) accounted for across all sections? Is any sync point mentioned in one section but forgotten in its counterpart section?
7. OVERVIEW ALIGNMENT: Does the overview's architecture diagram, dependency graph, and implementation sequence still match what the sections actually describe?
8. SUCCESS CRITERIA COVERAGE: Does every mission success criterion in 00-overview.md trace to at least one section that delivers it? Does every section have its own success criteria? Does each section criterion connect upward to at least one mission criterion? A mission criterion with no section delivering it is a plan gap. A section without success criteria is not executable.

For each issue found, report:
  ISSUE TYPE: {contradiction/gap/redundancy/broken-ref/ordering/sync-gap/overview-drift/criteria-gap}
  SECTIONS: {which sections are involved}
  DETAILS: {what the issue is}
  FIX: {how to resolve it}
```

Fix all issues found by the cohesion check before proceeding.

### Step 14: Self-Check Before Review

Do a quick self-audit:

1. **Every file path in the plan** — verify it exists in the codebase (use Glob)
2. **Every function/type reference** — verify it exists (use Grep)
3. **Every registration sync point** — verify the list is complete
4. **No placeholder content** — no "TBD", no "placeholder keywords", no "to be determined"
5. **No assumptions** — every technical claim traces to research
6. **No contradictions** — cohesion check passed clean
7. **Test strategy per section** — every code-modifying section has: explicit matrix dimensions (types x patterns), semantic pin requirements, TDD ordering (failing tests first, debug+release last)
8. **Success criteria hierarchy** — `00-overview.md` has mission success criteria; every section has its own success criteria in both frontmatter and body; every mission criterion maps to at least one section; every section criterion maps upward to at least one mission criterion

Fix any issues found.

### Step 15: Report Progress

Show the user:
- Files created (with paths)
- Brief summary of what each section covers
- Any issues found and fixed during cohesion/self-check
- Note: "Running /review-plan for formal review..."

### Step 16: Run /review-plan (MANDATORY — USE THE ACTUAL SKILL)

**SEQUENTIAL & FOREGROUND — MANDATORY.** The `/review-plan` skill internally calls `/tp-help` multiple times (before agents and between agents). All of those internal calls are sequential and foreground — do NOT attempt to optimize by running them in parallel or background. Wait for `/review-plan` to complete fully before proceeding.

**CRITICAL: Run the actual `/review-plan` skill using the Skill tool.** Do NOT reimplement the review logic. Do NOT spawn your own review agents. Use the Skill tool to invoke `/review-plan` with the plan directory path as the argument.

```
Skill: review-plan
Args: plans/{name}/
```

This runs the formal review pipeline as defined in the `/review-plan` skill. It will edit the plan files directly to fix any issues.

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

**Phase 1**: Read CLAUDE.md. Ask user about scope. Expand mission. Identify blockers (missing damage rect type, atlas invalidation). Consensus loop with Codex (2-3 rounds). Present mission proposal to user.

**Phase 2**:
- *Pass 1*: Launch 2 parallel agents — (1) survey `oriterm_gpu`, `oriterm/src/app/` rendering code, all GPU-related files; (2) audit tests, hygiene state, performance invariants.
- *Pass 2*: Deep-read the 12 most critical files. Understand how `GpuRenderer::draw_frame()` works, how the atlas manages glyphs, how damage tracking would integrate.
- *Pass 3*: Trace how the existing cell rendering pipeline works end-to-end.
- *Pass 4*: Study Alacritty's damage tracking, Ghostty's multi-backend approach, WezTerm's texture atlas.

**Phase 3**: Design architecture. `/tp-help` architectural consultation. Write `00-overview.md` with data flow, design decisions, dependency graph. `/tp-help` sanity check. Present to user: "Found N critical files. The existing pattern shows {pattern}. Propose these sections in this order: {list}. The key design decision is {X} — I recommend {Y} because {evidence}."

**Phase 4**: After user approves architecture, write sections sequentially:
- Section 01 (damage types) → read it → write Section 02 (tracking integration, building on 01's types) → read both → write Section 03 (atlas invalidation, building on 01+02).

**Phase 5**: Cohesion check → self-check → report → run `/review-plan plans/gpu-refactor/`.

**Creates:**
```
plans/gpu-refactor/
├── index.md
├── 00-overview.md
├── section-01-damage-types.md
├── section-02-tracking-integration.md
└── section-03-atlas-invalidation.md
```

---

## Section Naming Conventions

| Section Type | Naming Pattern |
|--------------|----------------|
| Setup/Infrastructure | `section-01-setup.md` |
| Core Implementation | `section-02-core.md` |
| Integration | `section-03-integration.md` |
| Testing/Verification | `section-NN-verification.md` |

---

## Expected `/tp-help` Cadence

The planning process includes **structured `/tp-help` checkpoints**. Here's the expected cadence:

| Checkpoint | Phase | When | Purpose |
|------------|-------|------|---------|
| **Step 1D** | Prerequisites | After mission expansion | Consensus on direction and scope |
| **Step 6B** | Research | After all 4 passes | Architectural direction validation |
| **Step 8B** | Architecture | After writing overview | Sanity check written document |
| **Step 16** | Finalization | /review-plan internally | Review agents' /tp-help calls |

**Expected total**: 6-12+ `/tp-help` rounds across a full plan creation. The consensus loop (Step 1D) alone may take 2-5 rounds.

**The rule**: Every major decision point gets challenged by a second brain before it becomes load-bearing. No decision survives on the strength of a single perspective.

**When `/tp-help` is unavailable** (Codex not installed, API errors, timeouts): Note which checkpoints were skipped and flag them in the plan's overview as `<!-- tp-help skipped: {checkpoint} — manual review recommended -->`. Continue without blocking — the feedback is valuable but not a hard gate.

---

## Anti-Deferral Rule for Plan Items

**Every checklist item in a plan must be implementable by the agent executing that section.** When writing plan items:

- Do NOT use soft language that invites skipping: "bonus", "future", "lower priority", "nice to have", "if time permits", "stretch goal".
- Do NOT label items "requires architectural change" — architectural changes are implementation tasks, not deferrals. If a 30-line change across 3 files is needed, describe the change and make it a checkbox.
- Do NOT create items that are descriptions of work rather than work itself. "Investigate whether X" is acceptable; "Document the approach for Y" when Y can be implemented is not.
- If an item genuinely cannot be done within the section (blocked by an unimplemented feature, needs user decision), use `<!-- blocked-by:X -->` with a concrete blocker reference — not vague language.
- Every item must pass this test: "Can the implementing agent, with access to the codebase, complete this item in a single session?" If no, break it into items that can.

## Matrix Testing & Semantic Pinning Rule

**Every section that modifies code must specify its test strategy at plan creation time.** This is not deferred to implementation or to `/review-plan` — the plan itself must describe:

1. **Matrix dimensions**: The types and control-flow patterns that flow through the section's code paths. These are the rows and columns of the test matrix. Name them explicitly — "ASCII, CJK, emoji, combining marks, ZWJ sequences" for character dimension; "resize, scroll, reflow, selection, search" for operation dimension. Missing cells are future regressions.
2. **Semantic pin**: At least one test per behavioral change that ONLY passes with the new semantics. Without a pin, a regression can silently revert the fix. The plan must describe what each pin tests.
3. **TDD discipline**: The section's first checklist item writes the failing test matrix. The section's last checklist item verifies debug + release. Tests frame the implementation, not follow it.
4. **Cross-section coverage**: If a fix in Section N touches code owned by Section M, the test matrix must cover Section M's types and patterns too. Plan boundaries = test boundaries.

A section without these is not executable per CLAUDE.md. The `/review-plan` skill enforces this during review — but catching it at creation time avoids the review rejection cycle.

## Zero Assumptions Rule

**ABSOLUTE — NO EXCEPTIONS.** Every technical claim in the plan must be grounded to something found during research:

- **File paths**: Must exist in the codebase (verified by Glob/Read)
- **Type/function signatures**: Must match actual source (verified by reading the file)
- **Behavior descriptions**: Must match actual code behavior (verified by reading the implementation)
- **Registration sync points**: Must be the complete list (verified by Grep for all match arms / enum variants)
- **Patterns to follow**: Must reference actual analogous implementations (verified by reading them)

If you cannot verify a claim, it MUST be flagged as `<!-- UNVERIFIED: {reason} -->` and reported to the user in Step 9. Unverified claims are not acceptable in the final plan — they must be resolved before Phase 4 or removed.

## Reviewed Field Semantics

The `reviewed: true/false` field in section frontmatter is a **pre-implementation gate** — it tracks whether a section has been validated against the current codebase right before you start implementing it.

**Why this exists:** Plans are written with assumptions about how the code works. But as you implement Section 01, reality changes — deviations, discoveries, refactors, bug fixes. A section written before prior sections were implemented may reference stale file paths, wrong function signatures, or invalid approaches. `reviewed: false` means "not yet validated against implementation reality."

**Rules:**
- **Section 01** is always `reviewed: true` at creation — it's the starting point.
- **All other sections** are `reviewed: false` at creation — plans, not validated reality.
- **Single-section review** (`/review-plan plans/foo/section-03.md`): This is the pre-implementation gate. After confirming accuracy, flip to `reviewed: true`.
- **Whole-plan review** (`/review-plan plans/foo/`): Fixes issues, improves quality, but does NOT change `reviewed` values. You're improving the plan holistically, not gating specific sections.
- **`/continue-roadmap`** starting a `reviewed: false` section: triggers a single-section review first, which flips to `true` after validation.

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

## Existing Plan Mode

When the input indicates adding to an existing plan — whether the roadmap, a rerouted plan, or any other plan directory — this command operates on that plan's directory instead of creating a new one.

**Trigger examples:**
- `/create-plan add selection to roadmap` → operates on `plans/roadmap/`
- `/create-plan add "damage rect tracking" subsection to plans/gpu-refactor` → operates on `plans/gpu-refactor/`
- `/create-plan roadmap: tab bar rendering` → operates on `plans/roadmap/`

**Same rigor, different target.** Every phase applies identically — the research depth, the iterative deepening, the sequential writing, the cohesion review. The only differences are structural: you're inserting into an existing plan, not creating a fresh one.

### Subsection vs Section Granularity

When invoked from `/continue-roadmap` impediment resolution, the work is typically a **subsection** added to an existing section — not a whole new section file. The granularity depends on scope:

- **Subsection** (most common for impediments): Add a `## XX.Y` block to an existing section file. Example: adding `## 03.5b Damage Rect Tracking` to `section-03-gpu-pipeline.md` to resolve missing GPU infrastructure.
- **Section**: Add a new section file when the work is large enough to warrant its own file (100+ lines of plan content, multiple subsections, distinct from existing section scope).

For subsections: update the parent section's YAML frontmatter `sections:` array to include the new subsection entry. For sections: create a new section file and update `00-overview.md` and `index.md`.

### Existing Plan Mode: How It Differs

#### Phase 1 Differences

- **Step 1**: Instead of asking for a plan name, identify:
  1. **What feature/section/subsection** to add to the plan
  2. **Where it fits** — after which existing section? What does it depend on?
  3. **What it might affect** — which existing sections reference related code?
  4. **What it unblocks** — which blocked items will this resolve? (Critical for impediment-driven additions)

- **Step 2**: In addition to the template and hygiene rules, **read the target plan**:
  - `plans/<dir>/00-overview.md` — understand the mission, architecture, dependency graph
  - `plans/<dir>/index.md` — understand the keyword structure and section numbering
  - **The section(s) most related to the new work** — understand what's already planned, what's complete, what's in progress
  - Pay attention to: section dependencies, implementation sequence, cross-section interactions

#### Phase 2 Differences

Research is identical in rigor, but adds a plan-specific dimension:

- **Pass 1**: In addition to the standard inventory, identify:
  - Which existing plan sections touch the same files/types/crates
  - Which existing sections might need updates due to the new section/subsection
  - Whether any completed sections already partially cover the new scope

- **Pass 2**: In addition to deep-reading critical files, deep-read:
  - The 2-3 existing plan sections most related to the new one
  - Any completed sections that the new work builds on (to understand what was actually implemented vs. what was planned)

#### Phase 3 Differences

- **Step 7**: Synthesis must include:
  - **Impact analysis**: How does the new section/subsection affect the existing plan? Does it change dependencies? Does it invalidate assumptions in other sections?
  - **Insertion point**: Where does it go? For subsections: which `## XX.Y` header, what ID? For sections: which section number? (May require renumbering)
  - **Dependency updates**: Which existing sections need `depends_on` updates?
  - **Unblock analysis** (for impediment-driven additions): Which `<!-- blocked: ... -->` comments will this resolve? List them explicitly.

- **Step 8**: Instead of writing a new `00-overview.md`:
  - **For subsections**: Update the parent section's YAML frontmatter `sections:` array to include the new subsection entry. Update the section body with the new `## XX.Y` block.
  - **For sections**: Create the new section file. Update `00-overview.md` — add the new section to the architecture diagram, dependency graph, implementation sequence, quick reference table, and estimated effort. Update `index.md` — add keyword clusters for the new section.
  - If the overview or index format has drifted from the current template (`.claude/skills/create-plan/plan-schema.md`), bring them up to date while you're editing them

- **Step 9**: Present to the user:
  - The proposed new section/subsection with its goals and scope
  - The impact on existing sections (what changes, what doesn't)
  - Which blocked items this will unblock (for impediment-driven additions)
  - Any existing sections that need updates and what those updates are

#### Phase 4 Differences

- **Step 11**: Write the new section(s)/subsection(s) following the same sequential discipline. If multiple are needed, write them in order.

- **After writing**: Update any existing sections that are affected:
  - Update `depends_on` in sections that now depend on the new work
  - Update cross-references in sections that reference related code
  - Update `00-overview.md` dependency graph and implementation sequence (for new sections)
  - Update `index.md` with new keywords (for new sections)
  - **Remove `<!-- blocked: ... -->` comments** from items that the new work will unblock (for impediment-driven additions)
  - If any existing section's content is now stale or contradicted by the new work, fix it. Flag the section as `reviewed: false` if you changed its assumptions.

#### Phase 5 Differences

- **Step 13**: The cohesion check reads the relevant plan sections (all sections for full plans; the parent section + neighbors for subsection additions), checking that:
  - The new work is consistent with existing sections
  - No existing section contradicts the new work
  - The dependency graph in `00-overview.md` is accurate (if modified)
  - The implementation sequence still makes sense
  - Cross-references between sections are all valid

- **Step 16**: Run `/review-plan` on the affected plan directory

- **Step 18**: Skip the reroute question if operating on a plan that is already a reroute or the roadmap itself

### Existing Plan Mode: The "Leave It Better" Rule

**You MUST leave the plan in better shape than you found it.** When operating in existing plan mode:

1. **Format drift**: If the plan's existing sections don't match the current template format (`.claude/skills/create-plan/plan-schema.md`), update them to match. This includes frontmatter fields, section structure, completion checklists, and third-party review blocks.
2. **Stale content**: If you encounter stale file paths, outdated type signatures, or references to code that no longer exists, fix them.
3. **Missing cross-references**: If sections reference each other implicitly but lack explicit `depends_on` or co-implementation callouts, add them.
4. **Incomplete hygiene**: If sections lack completion checklists, exit criteria, or test strategies, add them.
5. **Overview accuracy**: The overview's architecture diagram, dependency graph, and implementation sequence must accurately reflect the current state of the plan after your changes.

This is not optional cleanup — it's a mandatory part of existing plan mode. Every touch of the plan is an opportunity to improve its coherence and accuracy.

### Existing Plan Mode: Example

**Input:** `/create-plan add tab bar rendering to roadmap`

**Phase 1**: Read CLAUDE.md. Read the entire roadmap. Expand mission. Identify blockers. Consensus loop. Present proposal.

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
