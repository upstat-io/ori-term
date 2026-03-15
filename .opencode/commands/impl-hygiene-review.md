---
description: Review implementation hygiene at module boundaries -- plumbing quality and file organization
---

# Implementation Hygiene Review

Review implementation hygiene against `.claude/rules/impl-hygiene.md` and generate a plan to fix violations.

**Implementation hygiene is NOT architecture** (design decisions are made). It covers the full plumbing layer -- module boundaries, data flow, error propagation, rendering discipline, file organization, naming, comments, visibility, and lint discipline.

## Target

`$ARGUMENTS` specifies the boundary or scope to review. **If empty or blank, default to last commit mode** (equivalent to `/impl-hygiene-review last commit`). Otherwise, there are two modes:

### Path Mode (explicit file/directory targets)
- `/impl-hygiene-review oriterm/src/app/ oriterm/src/session/` -- review app<>session boundary
- `/impl-hygiene-review oriterm_gpu/src/` -- review GPU rendering internals
- `/impl-hygiene-review oriterm_core/src/grid/` -- review grid internals
- `/impl-hygiene-review oriterm/src/` -- review all module boundaries

### Commit Mode (use a commit as a scope selector)
- `/impl-hygiene-review last commit` -- review files touched by the most recent commit
- `/impl-hygiene-review last 3 commits` -- review files touched by the last N commits
- `/impl-hygiene-review <commit-hash>` -- review files touched by a specific commit

**CRITICAL: Commits are scope selectors, NOT content filters.** The commit determines WHICH files and areas to review. Once the files are identified, review them completely -- report ALL hygiene findings in those files, regardless of whether the finding is "related to" or "caused by" the commit. The commit is a lens to focus on a region of the codebase, nothing more. Do NOT annotate findings with whether they relate to the commit. Do NOT deprioritize or exclude findings because they predate the commit.

**Commit scoping procedure:**
1. Use `git diff --name-only HEAD~N..HEAD` (or appropriate range) to get the list of changed `.rs` files
2. Expand to include the full module(s) those files belong to (e.g., if `oriterm_gpu/src/atlas.rs` was touched, include all of `oriterm_gpu/src/`)
3. Proceed with the standard review process using those modules as the target

## Execution

### Step 1: Load Rules

Read the following rule files. These are the source of truth for the review:

**Implementation Hygiene Rules** (`.claude/rules/impl-hygiene.md`):
!`cat .claude/rules/impl-hygiene.md`

**Code Hygiene Rules** (`.claude/rules/code-hygiene.md`):
!`cat .claude/rules/code-hygiene.md`

### Step 2: Load Plan Context

Gather context from active and recently-modified plan files so the review doesn't flag work that is already planned, in-progress, or intentionally deferred.

**Procedure:**
1. Run `git diff --name-only HEAD` and `git diff --name-only --cached` to find uncommitted modified files in `plans/`
2. Run `git diff --name-only HEAD~3..HEAD -- plans/` to find plan files changed in recent commits
3. Combine both lists (deduplicate) to get all recently-touched plan files
4. Read each discovered plan file (skip files > 1000 lines -- read the `00-overview.md` or `index.md` instead)

**How to use plan context:**

Plan context does NOT suppress or deprioritize findings. Instead, it **annotates** them:

- If a finding falls within scope of an active plan, append `-> covered by plans/{plan}/` to the finding
- If a plan has an active reroute or suspension notice (e.g., "all work suspended until X"), note this in the review preamble so the user knows which areas are in flux
- If a plan explicitly describes a refactor that would resolve a finding, mark it as `[PLANNED]` instead of proposing a separate fix -- but still list it so nothing falls through cracks
- Findings NOT covered by any plan are reported normally -- these are the high-value discoveries

**Example annotation:**
```
3. **[DRIFT]** `oriterm_gpu/src/renderer.rs:142` -- Missing atlas invalidation for new glyph variant
   -> covered by plans/gpu_refactor/ (Section 3: Atlas Overhaul)
```

This ensures the review adds value by distinguishing "known debt being addressed" from "unknown debt needing attention."

### Step 3: Identify Review Targets

Determine the distinct modules or boundaries to review based on the target scope from Step 1:

1. List the modules (directories/file groups) in scope
2. Identify which boundaries exist between them (e.g., app<>session, grid<>term_handler, gpu<>app)
3. Group modules into **review units** -- each review unit is either:
   - A single module (for internal review)
   - A pair of modules sharing a boundary (for boundary review)
   - Closely related modules that should be reviewed together

Each review unit will be reviewed by a **separate subagent** in the next step.

### Step 4: Review Each Target (Separate Subagent Per Review Unit)

For **each review unit** identified in Step 3, spawn a **separate subagent** (using the Task tool). Each agent receives:

1. **The full rule set** -- both hygiene rules and code hygiene rules (from Step 1)
2. **Plan context summary** -- which plans are active and relevant (from Step 2)
3. **The specific module(s)/boundary** to review
4. **The audit checklist** (below)

Each agent performs the following work within its review unit:

#### 4a. Map the Boundary

1. What types cross the boundary? (cells, grid state, render params, events)
2. What functions form the interface? (entry points, draw calls, event handlers)
3. What data flows across? (grid cells, palette colors, font metrics, input events)

Read key files to understand the public API surface.

#### 4b. Trace Data Flow

1. **Read the producer's output types** -- What does the upstream module emit?
2. **Read the consumer's input handling** -- How does the downstream module receive and process it?
3. **Check the boundary types** -- Are they minimal? Do they carry unnecessary baggage?
4. **Check ownership** -- Is data moved, borrowed, or cloned? Are clones necessary?

#### 4c. Audit Each Rule Category

**Module Boundary Discipline:**
- [ ] Data flows one way? (no callbacks to earlier layer, no reaching back)
- [ ] No circular imports between modules?
- [ ] Boundary types are minimal? (only what's needed crosses)
- [ ] Clean ownership transfer? (borrow for rendering, move for ownership changes)
- [ ] No layer bleeding? (grid doesn't render, renderer doesn't parse VTE)

**Data Flow:**
- [ ] Zero-copy where possible? (cell references, not cell copies)
- [ ] No allocation in hot paths? (render loop, VTE input, key encoding)
- [ ] Newtypes for IDs? (`TabId`, not bare `u64`)
- [ ] Instance buffers reused across frames?
- [ ] Glyph cache avoids redundant rasterization?

**Error Handling at Boundaries:**
- [ ] No panics on user input? (bad escapes, invalid UTF-8, unexpected keys)
- [ ] PTY errors recoverable? (close tab, don't crash app)
- [ ] GPU errors surfaced? (surface lost, device lost -> recover or report)
- [ ] Config errors fall back to defaults?
- [ ] Errors carry context? (not bare `unwrap()` or swallowed `Result`)

**Rendering Discipline:**
- [ ] Frame building is pure computation? (no side effects on Grid/Tab/App)
- [ ] No state mutation during render?
- [ ] Color resolution happens once per frame, not per pass?
- [ ] Atlas misses are handled without blocking the frame?

**Event Flow:**
- [ ] Events flow through the event loop? (no bypassing `TermEvent`)
- [ ] Input dispatch is a decision tree? (one handler per event, no fallthrough)
- [ ] State transitions use enums, not booleans?
- [ ] Redraw requests coalesced?

**Platform & External Resource Abstraction:**
- [ ] `#[cfg()]` at module level, not scattered inline?
- [ ] Grid, VTE handler, selection, search are platform-independent?
- [ ] Logic-layer structs free of concrete external-resource types? (no `EventLoopProxy`, `Window`, `wgpu::Device` in logic types)
- [ ] Concrete resource types wired only at the composition root? (`App::new`, entry point)

**Registration Sync Points:**
- [ ] Any enum/variant that must appear in multiple locations has a single source of truth?
- [ ] Parallel lists (match arms, arrays, maps) that must cover the same variants are derived from a shared source?
- [ ] New variants added in one location are present in all parallel locations?
- [ ] When centralization isn't feasible, is there a test enforcing completeness?

**Gap Detection:**
- [ ] Features supported in downstream modules also supported in upstream modules?
- [ ] No silent workarounds for missing capabilities?
- [ ] Full pipeline works end-to-end for each feature? (input -> event -> handler -> grid -> render)

**File Organization:**
- [ ] All production source files under 500 lines? (test files exempt)
- [ ] Each file has a single clear responsibility?
- [ ] Logical groups of 200+ lines within a file extracted to submodules?
- [ ] File names describe what the file does?
- [ ] Directory structure mirrors the logical module structure?

**Naming, Comments, Visibility, Style:**
- [ ] Verb-prefix conventions used? (render_, draw_, handle_, encode_)
- [ ] No decorative banners, no commented-out code, no bare TODOs?
- [ ] Functions < 100 lines? Nesting depth <= 4?
- [ ] pub(crate)/pub(super) used appropriately? No dead pub items?

#### 4d. Return Findings

Each agent must return its findings as a structured list using the categories from `.claude/rules/impl-hygiene.md` (LEAK, DRIFT, GAP, WASTE, EXPOSURE, BLOAT, NOTE) with their default severity levels. Every finding must include `file:line`, the boundary it violates, and a concrete fix.

**Parallelization:** Review agents for independent modules/boundaries should be spawned in parallel. Only serialize agents that share a boundary.

### Step 5: Compile Findings

Collect the findings returned by all review agents. Deduplicate any findings that overlap at shared boundaries. Organize findings by boundary/interface and present them to the user.

### Step 6: Generate Plan (Separate Subagent)

Spawn a **separate subagent** (via Task tool) to generate the fix plan. This agent should use `/create-plan`. Pass it:

1. **All compiled findings** from Step 5
2. **The plan name**: `hygiene-{target-short-name}` (e.g., `hygiene-gpu`, `hygiene-grid-vte`, `hygiene-last-commit`)

The agent should create a plan that:

1. Lists every LEAK, DRIFT, GAP, WASTE, EXPOSURE, and BLOAT finding with `file:line` references
2. Groups by boundary (e.g., "app<>session", "grid<>term_handler", "gpu<>app")
3. Estimates scope: "N boundaries, ~M findings"
4. Orders: leaks first (layer bleeding), then drift (sync), then gaps (feature coverage), then bloat (file organization), then waste (perf), then exposure (encapsulation)

The **final section** of the plan must be a cleanup step:

```markdown
## Cleanup

- [ ] Run `./test-all.sh` to verify no behavior changes
- [ ] Run `./clippy-all.sh` to verify no regressions
- [ ] Run `./build-all.sh` to verify cross-compilation
- [ ] Delete this plan directory: `rm -rf plans/hygiene-{name}/`
```

Hygiene fix plans are disposable -- they exist to track the fixes, then get deleted when complete.

### Plan Section Format

Each section groups findings by boundary:

```
## {Boundary: Module A -> Module B}

**Interface types:** {list types crossing this boundary}
**Entry points:** {list key functions}

### Active Plan Context

{List each plan file read and its relevance. If a plan has a reroute/suspension, note it here.}
- `plans/gpu_refactor/` -- Active reroute: renderer architecture overhaul
- (none) -- if no plan files were found

### Findings

1. **[LEAK]** `file:line` -- {description}
2. **[DRIFT]** `file:line` -- {description}
   -> covered by plans/{plan}/ ({section name})
3. **[DRIFT] [PLANNED]** `file:line` -- {description}
   -> fix described in plans/{plan}/{section}.md
4. **[GAP]** `file:line` -- {description}
5. **[WASTE]** `file:line` -- {description}
6. **[EXPOSURE]** `file:line` -- {description}
```

## Important Rules

1. **No architecture changes** -- Don't propose new modules, new crates, or restructured dependency graphs
2. **Full scope** -- Module boundaries, data flow, naming, comments, visibility, file organization, lint discipline, and code fixes are all in scope. Only new modules, crates, or dependency graph restructures are out of scope (that's architecture).
3. **Trace, don't grep** -- Follow actual data flow through the code, don't just search for patterns
4. **Read both sides** -- Always read both the producer and consumer of a boundary
5. **Understand before flagging** -- Some apparent violations are intentional (e.g., `app.rs` coordinating between tabs and windows is orchestration, not layer bleeding)
6. **Be specific** -- Every finding must have `file:line`, the boundary it violates, and a concrete fix
7. **Compare to reference terminals** -- When in doubt, check how Alacritty/WezTerm/Ghostty handle the same boundary at `~/projects/reference_repos/console_repos/`
8. **Finding targets** -- Scale with scope. Single boundary or single module: **20**. Multi-module or last N commits spanning multiple modules: **30**. Full project: **40**. Dig deep, read broadly, trace more paths. Do NOT fabricate, exaggerate, or inflate findings to hit the target -- every finding must be real and verifiable. If the target area genuinely has fewer issues, report what you find honestly and note the shortfall.
